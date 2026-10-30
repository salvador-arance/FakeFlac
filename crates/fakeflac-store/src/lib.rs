//! See docs/architecture/backend.md. Adapter crate: implements ports defined in fakeflac-core.
//!
//! SQLite result cache (RF-11, `specs/contracts/result-cache.md` sections 6, 7, 9 and 10.2).
//! The cache is an optimisation: every failure is a `CacheError` (during a run) or an
//! `OpenError` (at open), never an `AppError`, and the use case decides what to do with it.

use fakeflac_core::ports::FileEntry;
use fakeflac_core::{CacheError, DetectionParams, ResultStore, TrackAnalysis};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension, TransactionBehavior};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// File name of the cache inside the app's local cache folder (result-cache.md section 5).
pub const FILE_NAME: &str = "results-cache.sqlite3";

/// Schema version, kept in `PRAGMA user_version` (result-cache.md section 6.1).
pub const SCHEMA_VERSION: u32 = 1;

/// `PRAGMA application_id` of a FakeFlac cache: 0x46464C43, "FFLC".
pub const APPLICATION_ID: i32 = 0x4646_4C43;

/// How long every step of `open`, and every `get` and `put`, waits for a lock that another
/// connection holds (result-cache.md 6.2). A setting of the adapter, not detection data.
const BUSY_TIMEOUT: Duration = Duration::from_millis(1000);

/// Schema version 1 (result-cache.md 6.1). Run by `open` step 6 on an empty database, in the
/// same transaction that sets `application_id` and `user_version`.
const SCHEMA_V1: &str = "
CREATE TABLE engine (
  id                 INTEGER PRIMARY KEY,
  params_json        TEXT    NOT NULL,
  extraction_version INTEGER NOT NULL,
  app_version        TEXT    NOT NULL,
  algorithm_version  INTEGER NOT NULL,
  UNIQUE (params_json, extraction_version, app_version)
);
CREATE TABLE track_result (
  path          TEXT    NOT NULL,
  engine_id     INTEGER NOT NULL REFERENCES engine(id) ON DELETE CASCADE,
  size_bytes    INTEGER NOT NULL,
  modified_unix INTEGER NOT NULL,
  analysis_json TEXT    NOT NULL,
  stored_unix   INTEGER NOT NULL,
  PRIMARY KEY (path, engine_id)
) WITHOUT ROWID;
";

/// Migration scripts: `MIGRATIONS[i]` migrates schema version `i + 1` to `i + 2`. None exists
/// before version 1 (result-cache.md section 7, row "older version").
const MIGRATIONS: &[&str] = &[];

const _: () = assert!(MIGRATIONS.len() + 1 == SCHEMA_VERSION as usize);

/// The parts of the engine that the composition root knows (result-cache.md section 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineTag {
    pub app_version: String,
    pub extraction_version: u32,
}

/// Why `SqliteResultStore::open` failed (result-cache.md section 7).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    /// The file is a FakeFlac cache of a newer schema; it was left untouched (Q8).
    #[error("the results cache was written by a newer FakeFlac (schema version {found})")]
    NewerSchema { found: u32 },
    #[error("cannot open the results cache: {0}")]
    Failed(String),
}

/// The result cache in one SQLite file (result-cache.md sections 6 and 7). One connection,
/// guarded by a mutex; the engine is resolved in every call, never kept (6.2).
#[derive(Debug)]
pub struct SqliteResultStore {
    conn: Mutex<Connection>,
    engine: EngineTag,
}

/// What the header check found (result-cache.md section 7, table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Header {
    /// No table, `application_id` 0 and `user_version` 0: a new or empty file.
    Empty,
    /// A FakeFlac cache of this schema version or an older one.
    Current(u32),
    /// A FakeFlac cache of a newer schema version.
    Newer(u32),
    /// Not a FakeFlac cache (Q10).
    Foreign,
}

fn classify(application_id: i64, user_version: i64, has_tables: bool) -> Header {
    let ours = application_id == i64::from(APPLICATION_ID);
    match (application_id, user_version) {
        (0, 0) if !has_tables => Header::Empty,
        (_, v) if ours && v > 0 => match u32::try_from(v) {
            Ok(v) if v <= SCHEMA_VERSION => Header::Current(v),
            Ok(v) => Header::Newer(v),
            Err(_) => Header::Newer(u32::MAX),
        },
        _ => Header::Foreign,
    }
}

/// Reads `application_id`, `user_version` and whether `sqlite_schema` holds any table.
fn read_header(conn: &Connection) -> rusqlite::Result<Header> {
    let application_id: i64 = conn.pragma_query_value(None, "application_id", |r| r.get(0))?;
    let user_version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let has_tables: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_schema WHERE type = 'table')",
        [],
        |r| r.get(0),
    )?;
    Ok(classify(application_id, user_version, has_tables))
}

/// The outcome of one attempt of steps 2 to 6 of `open`.
enum Attempt {
    Opened(Connection),
    /// Not a FakeFlac cache, found in steps 2 to 5: reset and try once more.
    Reset,
    Failed(OpenError),
}

/// SQLite reports a file that is not a database, or a corrupt one (reset, Q10).
fn is_not_a_cache(e: &rusqlite::Error) -> bool {
    matches!(
        e.sqlite_error_code(),
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
    )
}

fn failed(e: impl std::fmt::Display) -> OpenError {
    OpenError::Failed(e.to_string())
}

/// Steps 2 to 5 errors: `Reset` for a file SQLite rejects, `Failed` otherwise.
fn early(e: rusqlite::Error) -> Attempt {
    if is_not_a_cache(&e) {
        Attempt::Reset
    } else {
        Attempt::Failed(failed(e))
    }
}

/// Steps 2 to 6 of `open` (result-cache.md section 7).
fn attempt(file: &Path) -> Attempt {
    // Step 2: read-write, creating the file if it is missing. SQLite writes nothing yet.
    let conn = match Connection::open(file) {
        Ok(conn) => conn,
        Err(e) => return early(e),
    };
    // Step 3: first, so that every later step waits for a lock held by another connection.
    if let Err(e) = conn.busy_timeout(BUSY_TIMEOUT) {
        return Attempt::Failed(failed(e));
    }
    // Step 4: the header check, in a read transaction; it writes nothing.
    let header = conn
        .execute_batch("BEGIN DEFERRED")
        .and_then(|()| read_header(&conn))
        .and_then(|h| conn.execute_batch("COMMIT").map(|()| h));
    match header {
        Ok(Header::Newer(found)) => {
            drop(conn);
            return Attempt::Failed(OpenError::NewerSchema { found });
        }
        Ok(Header::Foreign) => return Attempt::Reset,
        Ok(Header::Empty | Header::Current(_)) => {}
        Err(e) => return early(e),
    }
    // Step 5: the pragmas of a file that `open` goes on to use. If SQLite keeps another journal
    // mode (a network file system), the store goes on in that mode.
    let pragmas = conn
        .query_row("PRAGMA journal_mode = WAL", [], |r| r.get::<_, String>(0))
        .and_then(|_| conn.pragma_update(None, "synchronous", "NORMAL"))
        .and_then(|()| conn.pragma_update(None, "foreign_keys", "ON"));
    if let Err(e) = pragmas {
        return early(e);
    }
    // Step 6: create or migrate in one write transaction; never resets.
    let mut conn = conn;
    match create_or_migrate(&mut conn) {
        Ok(()) => Attempt::Opened(conn),
        Err(e) => Attempt::Failed(e),
    }
}

/// Step 6: reads the header again (another window may have created or migrated the file since
/// step 4) and creates schema version 1 or runs the migrations, in one `BEGIN IMMEDIATE`
/// transaction. Rolls back (by dropping the transaction) on anything else.
fn create_or_migrate(conn: &mut Connection) -> Result<(), OpenError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(failed)?;
    match read_header(&tx).map_err(failed)? {
        Header::Empty => {
            tx.execute_batch(SCHEMA_V1).map_err(failed)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(failed)?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(failed)?;
        }
        Header::Current(version) => {
            let from = usize::try_from(version.saturating_sub(1)).unwrap_or(usize::MAX);
            let scripts = MIGRATIONS.get(from..).unwrap_or_default();
            if !scripts.is_empty() {
                for script in scripts {
                    tx.execute_batch(script).map_err(failed)?;
                }
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)
                    .map_err(failed)?;
            }
        }
        Header::Newer(found) => return Err(OpenError::NewerSchema { found }),
        Header::Foreign => {
            return Err(OpenError::Failed(
                "the results cache file is not a FakeFlac cache".into(),
            ))
        }
    }
    tx.commit().map_err(failed)
}

impl SqliteResultStore {
    /// Opens (creating, migrating or resetting) the cache at `file` (result-cache.md section 7).
    /// Nothing before step 5 writes to the database file.
    pub fn open(file: &Path, engine: EngineTag) -> Result<SqliteResultStore, OpenError> {
        // Step 1.
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(failed)?;
        }
        let conn = match attempt(file) {
            Attempt::Opened(conn) => conn,
            Attempt::Failed(e) => return Err(e),
            // Not a FakeFlac cache: the connection is closed, so delete the files (section 9
            // order) and repeat steps 2 to 6 once on the new empty file.
            Attempt::Reset => {
                clear_cache_files(file).map_err(failed)?;
                match attempt(file) {
                    Attempt::Opened(conn) => conn,
                    Attempt::Failed(e) => return Err(e),
                    Attempt::Reset => {
                        return Err(OpenError::Failed(
                            "the results cache is still not usable after a reset".into(),
                        ))
                    }
                }
            }
        };
        Ok(SqliteResultStore {
            conn: Mutex::new(conn),
            engine,
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, CacheError> {
        self.conn
            .lock()
            .map_err(|_| CacheError("the results cache connection is poisoned".into()))
    }
}

fn cache_error(e: impl std::fmt::Display) -> CacheError {
    CacheError(e.to_string())
}

/// The key of `entry` (result-cache.md sections 2.3 and 3): its canonical path as UTF-8, its
/// size and its mtime. `None` when the entry cannot be keyed: the path, or its canonical form,
/// is not valid Unicode, the path cannot be canonicalised, or the size does not fit SQLite.
/// `entry.path` is checked before anything is serialised.
fn key_of(entry: &FileEntry) -> Option<(String, i64)> {
    entry.path.to_str()?;
    let canonical: PathBuf = std::fs::canonicalize(&entry.path).ok()?;
    let path = canonical.into_os_string().into_string().ok()?;
    let size = i64::try_from(entry.size_bytes).ok()?;
    Some((path, size))
}

fn params_json(params: &DetectionParams) -> Result<String, CacheError> {
    serde_json::to_string(params).map_err(cache_error)
}

impl ResultStore for SqliteResultStore {
    fn get(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
    ) -> Result<Option<TrackAnalysis>, CacheError> {
        let Some((path, size)) = key_of(entry) else {
            return Ok(None);
        };
        let params_json = params_json(params)?;
        let conn = self.lock()?;
        // The engine is looked up in the same read as the row (6.2); a missing engine is a miss.
        let row: Option<(i64, i64, String)> = conn
            .prepare_cached(
                "SELECT t.size_bytes, t.modified_unix, t.analysis_json
                   FROM track_result AS t JOIN engine AS e ON e.id = t.engine_id
                  WHERE e.params_json = ?1 AND e.extraction_version = ?2
                    AND e.app_version = ?3 AND t.path = ?4",
            )
            .and_then(|mut s| {
                s.query_row(
                    params![
                        params_json,
                        self.engine.extraction_version,
                        self.engine.app_version,
                        path
                    ],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
            })
            .map_err(cache_error)?;
        drop(conn);
        Ok(row.and_then(|(stored_size, stored_mtime, json)| {
            if stored_size != size || stored_mtime != entry.modified_unix {
                return None;
            }
            // A row that does not parse is a miss; the next `put` replaces it (6.3).
            serde_json::from_str::<TrackAnalysis>(&json).ok()
        }))
    }

    fn put(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
        analysis: &TrackAnalysis,
    ) -> Result<(), CacheError> {
        let Some((path, size)) = key_of(entry) else {
            return Ok(());
        };
        let params_json = params_json(params)?;
        let analysis_json = serde_json::to_string(analysis).map_err(cache_error)?;
        let stored_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
        let mut conn = self.lock()?;
        // One transaction per put: the result is durable as soon as `put` returns (6.2).
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(cache_error)?;
        let found: Option<i64> = tx
            .query_row(
                "SELECT id FROM engine
                  WHERE params_json = ?1 AND extraction_version = ?2 AND app_version = ?3",
                params![
                    params_json,
                    self.engine.extraction_version,
                    self.engine.app_version
                ],
                |r| r.get(0),
            )
            .optional()
            .map_err(cache_error)?;
        let engine_id = match found {
            Some(id) => id,
            None => {
                // A new engine: insert it and delete every other one (and, by cascade, its rows).
                tx.execute(
                    "INSERT INTO engine
                       (params_json, extraction_version, app_version, algorithm_version)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        params_json,
                        self.engine.extraction_version,
                        self.engine.app_version,
                        params.algorithm_version
                    ],
                )
                .map_err(cache_error)?;
                let id = tx.last_insert_rowid();
                tx.execute("DELETE FROM engine WHERE id <> ?1", params![id])
                    .map_err(cache_error)?;
                id
            }
        };
        tx.execute(
            "INSERT INTO track_result
               (path, engine_id, size_bytes, modified_unix, analysis_json, stored_unix)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (path, engine_id) DO UPDATE SET
               size_bytes = excluded.size_bytes,
               modified_unix = excluded.modified_unix,
               analysis_json = excluded.analysis_json,
               stored_unix = excluded.stored_unix",
            params![
                path,
                engine_id,
                size,
                entry.modified_unix,
                analysis_json,
                stored_unix
            ],
        )
        .map_err(cache_error)?;
        tx.commit().map_err(cache_error)
    }
}

/// `file` with `suffix` appended to its name (`-wal`, `-shm`).
fn sidecar(file: &Path, suffix: &str) -> PathBuf {
    let mut name = file.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// Deletes `<file>-wal`, `<file>-shm` and `<file>`, in this order, each only if it exists
/// (result-cache.md section 9). Stops at the first error. It does not open the database, and
/// it deletes nothing else.
pub fn clear_cache_files(file: &Path) -> std::io::Result<()> {
    for path in [
        sidecar(file, "-wal"),
        sidecar(file, "-shm"),
        file.to_path_buf(),
    ] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

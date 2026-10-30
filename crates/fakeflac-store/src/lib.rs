//! See docs/architecture/backend.md. Adapter crate: implements ports defined in fakeflac-core.
//!
//! SQLite result cache (RF-11, `specs/contracts/result-cache.md` sections 6, 7, 9 and 10.2).

use fakeflac_core::ports::FileEntry;
use fakeflac_core::{CacheError, DetectionParams, ResultStore, TrackAnalysis};
use std::path::Path;

/// File name of the cache inside the app's local cache folder (result-cache.md section 5).
pub const FILE_NAME: &str = "results-cache.sqlite3";

/// Schema version, kept in `PRAGMA user_version` (result-cache.md section 6.1).
pub const SCHEMA_VERSION: u32 = 1;

/// `PRAGMA application_id` of a FakeFlac cache: 0x46464C43, "FFLC".
pub const APPLICATION_ID: i32 = 0x4646_4C43;

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

/// The result cache in one SQLite file (result-cache.md sections 6 and 7).
#[derive(Debug)]
pub struct SqliteResultStore {
    _engine: EngineTag,
}

impl SqliteResultStore {
    /// Opens (creating, migrating or resetting) the cache at `file` (result-cache.md section 7).
    pub fn open(file: &Path, engine: EngineTag) -> Result<SqliteResultStore, OpenError> {
        let _ = (file, engine);
        todo!("RF-11: SqliteResultStore::open (result-cache.md 7)")
    }
}

impl ResultStore for SqliteResultStore {
    fn get(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
    ) -> Result<Option<TrackAnalysis>, CacheError> {
        let _ = (entry, params);
        todo!("RF-11: SqliteResultStore::get (result-cache.md 6.3)")
    }

    fn put(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
        analysis: &TrackAnalysis,
    ) -> Result<(), CacheError> {
        let _ = (entry, params, analysis);
        todo!("RF-11: SqliteResultStore::put (result-cache.md 6.2)")
    }
}

/// Deletes `<file>-wal`, `<file>-shm` and `<file>`, in this order, each only if it exists
/// (result-cache.md section 9).
pub fn clear_cache_files(file: &Path) -> std::io::Result<()> {
    let _ = file;
    todo!("RF-11: clear_cache_files (result-cache.md 9)")
}

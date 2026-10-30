//! Helpers of the SQLite adapter tests of `specs/features/result-cache.feature` (@RF-11, sections
//! "Storage" and "Opening and failures"): a temporary folder per test inside
//! `std::env::temp_dir()` (never the repository, the corpus or `%LOCALAPPDATA%`; ADR-0022), a
//! scanner over real files, runs of `analyze_folder_cached` and read-only inspection of the cache
//! file. The files' content does not matter: the decoder spy analyses nothing. Protected after
//! the `test(RF-11)` commit.

#![allow(dead_code, clippy::expect_used, clippy::unwrap_used)]

#[path = "../../../fakeflac-core/tests/common/cache.rs"]
pub mod cache;

use cache::{DecoderSpy, Recorder};
use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_core::{
    analyze_folder_cached, AnalysisSummary, AppError, AppErrorCode, AudioFormat, CancelToken,
    DetectionParams, ProgressEvent, ResultCache, ResultStore,
};
use fakeflac_store::{EngineTag, OpenError, SqliteResultStore, FILE_NAME};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A folder of the test inside the system temporary folder, removed when dropped.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir()
            .join("fakeflac-tests")
            .join(format!("store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the test folder");
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// `<test folder>/cache/results-cache.sqlite3` (the folder is not created).
    pub fn cache_file(&self) -> PathBuf {
        self.0.join("cache").join(FILE_NAME)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes `len` bytes derived from `seed` (any content: the decoder spy does not read files).
pub fn put_file(dir: &Path, name: &str, len: usize, seed: u8) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    let bytes: Vec<u8> = (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect();
    std::fs::write(&path, bytes).unwrap();
    path
}

pub fn mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

/// Sets the modification time of a test file (never an audio file of the corpus).
pub fn set_mtime(path: &Path, t: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(t)
        .unwrap();
}

/// A scanner over real files, as `fakeflac-fs` would list them: `.flac` and `.wav`, recursively,
/// with the size and the whole-second mtime of each file. Paths are `root` joined with the
/// names read from the folder, so they keep the spelling of `root`.
pub struct DirScanner;

fn walk(dir: &Path, out: &mut Vec<FileEntry>) {
    let mut items: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    items.sort();
    for p in items {
        if p.is_dir() {
            walk(&p, out);
            continue;
        }
        let format = match p.extension().and_then(|e| e.to_str()) {
            Some(e) => AudioFormat::from_extension(e),
            None => None,
        };
        let Some(format) = format else { continue };
        let meta = std::fs::metadata(&p).unwrap();
        let modified_unix = meta
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        out.push(FileEntry {
            folder: p.parent().unwrap().to_path_buf(),
            path: p,
            format,
            size_bytes: meta.len(),
            modified_unix,
        });
    }
}

impl FileScanner for DirScanner {
    fn scan(&self, root: &Path) -> Result<Vec<FileEntry>, AppError> {
        if !root.is_dir() {
            return Err(AppError::new(
                AppErrorCode::FolderUnreadable,
                "not a folder",
            ));
        }
        let mut out = Vec::new();
        walk(root, &mut out);
        Ok(out)
    }
}

pub fn tag() -> EngineTag {
    EngineTag {
        app_version: "0.0.1".into(),
        extraction_version: 2,
    }
}

pub fn open(file: &Path) -> SqliteResultStore {
    SqliteResultStore::open(file, tag()).expect("open the cache")
}

/// A run's outcome.
pub struct Run {
    pub summary: AnalysisSummary,
    pub events: Vec<ProgressEvent>,
}

impl Run {
    /// `(index, analysis)` of every trackDone, sorted by index.
    pub fn done(&self) -> Vec<(u32, fakeflac_core::TrackAnalysis)> {
        cache::done_by_index(&self.events)
    }
}

pub fn run_with(
    cache: ResultCache<'_>,
    spy: &DecoderSpy,
    params: &DetectionParams,
    root: &Path,
    workers: usize,
) -> Run {
    let rec = Recorder::new();
    let summary = analyze_folder_cached(
        &DirScanner,
        spy,
        cache,
        params,
        root,
        workers,
        &CancelToken::new(),
        &|e| rec.push(e),
    )
    .expect("run ok");
    Run {
        summary,
        events: rec.events(),
    }
}

/// "Run N": opens a new `SqliteResultStore` on `file` with `tag`, runs on `root` with `params`
/// and 2 workers, and drops the store.
pub fn run_store(
    file: &Path,
    engine: EngineTag,
    params: &DetectionParams,
    spy: &DecoderSpy,
    root: &Path,
) -> Run {
    let store = SqliteResultStore::open(file, engine).expect("open the cache");
    let run = run_with(ResultCache::Store(&store), spy, params, root, 2);
    drop(store);
    run
}

/// "A run opens the store on the cache file": `Store` if `open` succeeds, `Unavailable` if not.
pub fn run_opening(
    file: &Path,
    spy: &DecoderSpy,
    root: &Path,
) -> (Run, Result<(), OpenError>, Duration) {
    let started = std::time::Instant::now();
    let opened = SqliteResultStore::open(file, tag());
    let open_time = started.elapsed();
    match opened {
        Ok(store) => {
            let run = run_with(
                ResultCache::Store(&store as &dyn ResultStore),
                spy,
                &DetectionParams::default(),
                root,
                2,
            );
            drop(store);
            (run, Ok(()), open_time)
        }
        Err(e) => {
            let run = run_with(
                ResultCache::Unavailable,
                spy,
                &DetectionParams::default(),
                root,
                2,
            );
            (run, Err(e), open_time)
        }
    }
}

// --- Read-only inspection of the cache file ---------------------------------------------------

pub fn read_only(file: &Path) -> Connection {
    Connection::open_with_flags(
        file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the cache read-only")
}

pub fn count(file: &Path, table: &str) -> i64 {
    let c = read_only(file);
    c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

pub fn pragma_i64(file: &Path, name: &str) -> i64 {
    let c = read_only(file);
    c.query_row(&format!("PRAGMA {name}"), [], |r| r.get(0))
        .unwrap()
}

/// The canonical paths stored in `track_result`, sorted.
pub fn stored_paths(file: &Path) -> Vec<String> {
    let c = read_only(file);
    let mut s = c
        .prepare("SELECT path FROM track_result ORDER BY path")
        .unwrap();
    let rows = s.query_map([], |r| r.get::<_, String>(0)).unwrap();
    rows.map(Result::unwrap).collect()
}

/// Names of the tables in `sqlite_schema`, sorted.
pub fn tables(file: &Path) -> Vec<String> {
    let c = read_only(file);
    let mut s = c
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
        .unwrap();
    let rows = s.query_map([], |r| r.get::<_, String>(0)).unwrap();
    rows.map(Result::unwrap).collect()
}

/// `0x46464C43`, "FFLC" (result-cache.md 6.1).
pub const FFLC: i64 = 1_179_012_163;

pub fn sidecar(file: &Path, suffix: &str) -> PathBuf {
    let mut s = file.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

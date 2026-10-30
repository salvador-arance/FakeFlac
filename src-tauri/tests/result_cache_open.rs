//! App layer of the result cache (@RF-11, @RNF-04): the open step and the end of
//! `run::run_analysis_cached` (`specs/contracts/result-cache.md` sections 7 and 10.3, ADR-0044
//! decision 2: a cache failure never fails a run). Driven through
//! `run::run_analysis_cached_with`, which is `run_analysis_cached` with the open step passed in.
//!
//! - Section 7, table row "The folder or the file cannot be created or opened": a failed open
//!   is `unavailable`, and "Never an `AppError`". 10.3 step 3: the run uses
//!   `ResultCache::Unavailable` if the open failed. A panic in the open is such a failure, not
//!   `INTERNAL` (scenario "The cache is optional: verdicts never depend on it", acceptance C5).
//! - Section 7: the store "is closed (dropped) when the run ends, before the slot is freed";
//!   10.3 step 5: "drops the store, then the slot". So a `clear_cache` that takes the slot right
//!   after a run never meets an open store (section 9, acceptance C8).
//!
//! The analysed files are random bytes generated in the test (content does not matter here).
//! Everything is inside the test's own folder in `std::env::temp_dir()`, never the repository,
//! the corpus or `%LOCALAPPDATA%` (ADR-0022).
//!
//! Protected after the `test(RNF-11, RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core::ports::FileEntry;
use fakeflac_core::{AnalysisSlot, CacheError, DetectionParams, ResultStore, TrackAnalysis};
use fakeflac_lib::dto::{AppErrorCode, CacheStatus, ProgressEvent};
use fakeflac_lib::run::{clear_cache, run_analysis_cached_with, ExportState};
use fakeflac_store::OpenError;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// The test's own folder inside the system temporary folder, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir()
            .join("fakeflac-tests")
            .join(format!("app-open-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// `<test folder>/cache/results-cache.sqlite3`; the folder is not created.
    fn cache_file(&self) -> PathBuf {
        self.0.join("cache").join("results-cache.sqlite3")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// "Album A" with 2 files of random-looking bytes.
fn album(t: &TempDir) -> PathBuf {
    let root = t.path().join("Album A");
    std::fs::create_dir_all(&root).unwrap();
    for (name, seed) in [("a.flac", 1u8), ("b.flac", 2u8)] {
        let bytes: Vec<u8> = (0..4096u32)
            .map(|i| (i as u8).wrapping_mul(97).wrapping_add(seed).rotate_left(3))
            .collect();
        std::fs::write(root.join(name), bytes).unwrap();
    }
    root
}

/// A store that misses every `get` and accepts every `put`.
struct NullStore;

impl ResultStore for NullStore {
    fn get(
        &self,
        _entry: &FileEntry,
        _params: &DetectionParams,
    ) -> Result<Option<TrackAnalysis>, CacheError> {
        Ok(None)
    }

    fn put(
        &self,
        _entry: &FileEntry,
        _params: &DetectionParams,
        _analysis: &TrackAnalysis,
    ) -> Result<(), CacheError> {
        Ok(())
    }
}

fn events_sink() -> Mutex<Vec<ProgressEvent>> {
    Mutex::new(Vec::new())
}

// @RF-11 @RNF-04 Scenario Outline: The cache is optional: verdicts never depend on it
// (result-cache.md section 7 "Never an AppError", 10.3 step 3; ADR-0044 decision 2).
// A panic while opening the store is a failed open: the run completes with cache "unavailable",
// it is not INTERNAL and does not unwind out of the run.
#[test]
fn rf11_a_panic_while_opening_the_store_gives_unavailable_and_a_normal_run() {
    let t = TempDir::new("panic");
    let root = album(&t);
    let file = t.cache_file();
    let slot = AnalysisSlot::new();
    let opened = AtomicUsize::new(0);
    let events = events_sink();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_analysis_cached_with(
            &slot,
            &ExportState::new(),
            Some(&file),
            &root,
            &|e| {
                events.lock().unwrap().push(e);
                true
            },
            |_file: &Path| -> Result<NullStore, OpenError> {
                opened.fetch_add(1, Ordering::SeqCst);
                panic!("simulated panic inside SqliteResultStore::open")
            },
        )
    }));

    assert_eq!(opened.load(Ordering::SeqCst), 1, "the open step ran once");
    let result = outcome.expect("a panic in the open step must not unwind out of the run");
    let summary = match result {
        Ok(summary) => summary,
        Err(e) => panic!(
            "a panic in the open step must not fail the run, got {:?}: {}",
            e.code, e.message
        ),
    };
    assert_eq!(summary.done, 2, "both files are reported");
    assert_eq!(summary.from_cache, 0);
    assert_eq!(summary.cache, CacheStatus::Unavailable);
    let events = events.into_inner().unwrap();
    assert!(
        matches!(events.last(), Some(ProgressEvent::Finished { summary }) if summary.done == 2),
        "the run ends with finished: {:?}",
        events.last()
    );
    drop(slot.begin().expect("the slot is free after the run"));
}

/// Records, when it is dropped, what a `clear_cache` that tries to take the slot at that
/// moment gets (`None`: it succeeded, so the slot was already free).
struct DropProbe<'a> {
    slot: &'a AnalysisSlot,
    file: PathBuf,
    seen_at_drop: &'a Mutex<Vec<Option<AppErrorCode>>>,
}

impl ResultStore for DropProbe<'_> {
    fn get(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
    ) -> Result<Option<TrackAnalysis>, CacheError> {
        NullStore.get(entry, params)
    }

    fn put(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
        analysis: &TrackAnalysis,
    ) -> Result<(), CacheError> {
        NullStore.put(entry, params, analysis)
    }
}

impl Drop for DropProbe<'_> {
    fn drop(&mut self) {
        let outcome = clear_cache(self.slot, Some(&self.file))
            .err()
            .map(|e| e.code);
        self.seen_at_drop.lock().unwrap().push(outcome);
    }
}

// @RF-11 Scenario Outline: Clearing the cache (acceptance C8), row "a run holds the analysis
// slot" (result-cache.md section 7 "closed (dropped) when the run ends, before the slot is
// freed", 10.3 step 5 "drops the store, then the slot"). While the store is being dropped the
// run still holds the slot, so a clear_cache at that moment fails with ANALYSIS_RUNNING; once
// run_analysis_cached returns, the store is gone and the slot is free.
#[test]
fn rf11_the_store_is_dropped_before_the_slot_is_freed() {
    let t = TempDir::new("drop-order");
    let root = album(&t);
    let file = t.cache_file();
    let slot = AnalysisSlot::new();
    let seen_at_drop = Mutex::new(Vec::new());

    let summary = run_analysis_cached_with(
        &slot,
        &ExportState::new(),
        Some(&file),
        &root,
        &|_| true,
        |f: &Path| -> Result<DropProbe<'_>, OpenError> {
            Ok(DropProbe {
                slot: &slot,
                file: f.to_path_buf(),
                seen_at_drop: &seen_at_drop,
            })
        },
    )
    .expect("run ok");

    assert_eq!(summary.cache, CacheStatus::Ok, "the run used the store");
    assert_eq!(summary.done, 2);
    assert_eq!(
        *seen_at_drop.lock().unwrap(),
        vec![Some(AppErrorCode::AnalysisRunning)],
        "the store is dropped once, before run_analysis_cached returns, while the run still \
         holds the slot"
    );
    clear_cache(&slot, Some(&file)).expect("after the run the slot is free and clearing works");
}

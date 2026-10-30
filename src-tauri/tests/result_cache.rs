//! App layer of `specs/features/result-cache.feature` (@RF-11, section "App"): real runs of
//! `run::run_analysis_cached` with the symphonia analyzer and `run::clear_cache`
//! (`specs/contracts/result-cache.md` sections 9 and 10.3; acceptance C2, C6 and C8). Audio
//! files are generated inside the test; the analysed folder and the cache file are inside the
//! test's own folder in `std::env::temp_dir()`, never the repository, the corpus or
//! `%LOCALAPPDATA%` (ADR-0022).
//!
//! Readings chosen where the scenarios are open (as in `export_csv.rs` and `album_summary.rs`:
//! the `src-tauri` crate has no FLAC encoder), reported in the test-stage report:
//! - "full_band.flac" is "full_band.wav", 6 s of 16-bit mono white noise (real);
//! - "lp16k.flac" is "lp16k.wav", the same kind of noise through a windowed-sinc low-pass at
//!   16 kHz (fake), the same duration, rate and channels, so the same size as full_band;
//! - "short.flac" is "short.wav", 2 s of the noise (the error tooShort);
//! - "corrupt.flac" is 4096 random bytes (the error corruptStream);
//! - the "2 FLAC files" of the scenarios where content does not matter are random bytes;
//! - "a FakeFlac cache with user_version 99" is a cache written by a run whose header field
//!   `user_version` (offset 60) the test then sets to 99;
//! - "another connection keeps the cache file open" is a handle opened without
//!   `FILE_SHARE_DELETE`, as SQLite opens it on Windows.
//!
//! The test `fixtures_give_the_intended_verdicts` checks these verdicts with `run_analysis`.
//!
//! Protected after the `test(RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, AppErrorCode, CacheStatus, ProgressEvent, TrackError, Verdict};
use fakeflac_lib::run::{
    begin_export, clear_cache, export_csv, run_analysis, run_analysis_cached, run_analysis_kept,
    ExportState,
};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

const CACHE_FILE: &str = "results-cache.sqlite3";

/// The test's own folder inside the system temporary folder, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir()
            .join("fakeflac-tests")
            .join(format!("app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// `<test folder>/cache/results-cache.sqlite3`; the folder is not created.
    fn cache_file(&self) -> PathBuf {
        self.0.join("cache").join(CACHE_FILE)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sidecar(file: &Path, suffix: &str) -> PathBuf {
    let mut s = file.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

// --- Audio files generated in the test --------------------------------------------------------

const RATE: u32 = 44_100;

fn noise(seed: u64, seconds: u32) -> Vec<f64> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..RATE * seconds)
        .map(|_| rng.random::<f64>() * 2.0 - 1.0)
        .collect()
}

/// `noise` through a windowed-sinc low-pass (Blackman-Harris, 511 taps) at `cutoff_hz`:
/// nothing above the transition band but the 16-bit rounding floor.
fn lowpassed(seed: u64, seconds: u32, cutoff_hz: f64) -> Vec<f64> {
    const TAPS: usize = 511;
    let fc = cutoff_hz / f64::from(RATE);
    let mid = (TAPS / 2) as f64;
    let mut h: Vec<f64> = (0..TAPS)
        .map(|n| {
            let x = n as f64 - mid;
            let sinc = if x == 0.0 {
                2.0 * fc
            } else {
                (2.0 * std::f64::consts::PI * fc * x).sin() / (std::f64::consts::PI * x)
            };
            let a = 2.0 * std::f64::consts::PI * n as f64 / (TAPS - 1) as f64;
            let w =
                0.35875 - 0.48829 * a.cos() + 0.14128 * (2.0 * a).cos() - 0.01168 * (3.0 * a).cos();
            sinc * w
        })
        .collect();
    let gain: f64 = h.iter().sum();
    h.iter_mut().for_each(|v| *v /= gain);
    let x = noise(seed, seconds);
    let mut padded = vec![0.0; TAPS - 1];
    padded.extend_from_slice(&x);
    padded
        .windows(TAPS)
        .map(|w| w.iter().zip(&h).map(|(a, b)| a * b).sum::<f64>())
        .collect()
}

fn write_wav(path: &Path, samples: &[f64], scale: f64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("wav");
    for s in samples {
        w.write_sample((s * scale).round() as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

fn full_band_samples() -> &'static [f64] {
    static S: OnceLock<Vec<f64>> = OnceLock::new();
    S.get_or_init(|| noise(1, 6))
}

fn lp16k_samples() -> &'static [f64] {
    static S: OnceLock<Vec<f64>> = OnceLock::new();
    S.get_or_init(|| lowpassed(2, 6, 15_800.0))
}

fn put_full_band(dir: &Path, file: &str) -> PathBuf {
    let p = dir.join(file);
    write_wav(&p, full_band_samples(), 10_000.0);
    p
}

fn put_lp16k(dir: &Path, file: &str) -> PathBuf {
    let p = dir.join(file);
    // The low-pass removes about a quarter of the power: a larger scale keeps a similar level.
    write_wav(&p, lp16k_samples(), 11_000.0);
    p
}

fn put_short(dir: &Path, file: &str) -> PathBuf {
    let p = dir.join(file);
    write_wav(&p, &noise(3, 2), 10_000.0);
    p
}

fn put_garbage(dir: &Path, file: &str, seed: u64) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let bytes: Vec<u8> = (0..4096).map(|_| rng.random::<u8>()).collect();
    let p = dir.join(file);
    std::fs::write(&p, bytes).unwrap();
    p
}

/// Sets the modification time of a generated test file.
fn set_mtime(path: &Path, t: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(t)
        .unwrap();
}

fn snapshot(path: &Path) -> (u64, SystemTime, u64) {
    let m = std::fs::metadata(path).unwrap();
    let mut h = DefaultHasher::new();
    std::fs::read(path).unwrap().hash(&mut h);
    (m.len(), m.modified().unwrap(), h.finish())
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

// --- Runs ---------------------------------------------------------------------------------------

struct Run {
    summary: dto::AnalysisSummary,
    events: Vec<ProgressEvent>,
}

impl Run {
    /// `(index, analysis)` of every trackDone, sorted by index.
    fn done(&self) -> Vec<(u32, dto::TrackAnalysis)> {
        let mut v: Vec<_> = self
            .events
            .iter()
            .filter_map(|e| match e {
                ProgressEvent::TrackDone {
                    index, analysis, ..
                } => Some((*index, analysis.clone())),
                _ => None,
            })
            .collect();
        v.sort_by_key(|(i, _)| *i);
        v
    }

    fn of(&self, file: &str) -> dto::TrackAnalysis {
        self.done()
            .into_iter()
            .map(|(_, a)| a)
            .find(|a| a.info.path.ends_with(file))
            .unwrap_or_else(|| panic!("no trackDone for {file}"))
    }
}

fn cached(slot: &AnalysisSlot, state: &ExportState, cache_file: Option<&Path>, root: &Path) -> Run {
    let events = Mutex::new(Vec::new());
    let summary = run_analysis_cached(slot, state, cache_file, root, &|e| {
        events.lock().unwrap().push(e);
        true
    })
    .expect("run ok");
    Run {
        summary,
        events: events.into_inner().unwrap(),
    }
}

fn cached_run(cache_file: &Path, root: &Path) -> Run {
    cached(
        &AnalysisSlot::new(),
        &ExportState::new(),
        Some(cache_file),
        root,
    )
}

fn csv_of(slot: &AnalysisSlot, state: &ExportState, dest: &Path) -> Vec<u8> {
    let (snapshot, guard) = begin_export(slot, state).expect("begin export");
    export_csv(&snapshot, dest).expect("export");
    drop(guard);
    std::fs::read(dest).unwrap()
}

fn error_kind(a: &dto::TrackAnalysis) -> Option<&'static str> {
    a.error.as_ref().map(|e| match e {
        TrackError::TooShort => "tooShort",
        TrackError::CorruptStream { .. } => "corruptStream",
        TrackError::Unreadable { .. } => "unreadable",
        TrackError::UnsupportedFormat { .. } => "unsupportedFormat",
    })
}

// The generated fixtures give the verdicts the scenarios name (checked without any cache).
#[test]
fn fixtures_give_the_intended_verdicts() {
    let t = TempDir::new("fixtures");
    let root = t.path().join("Mixed");
    put_full_band(&root, "full_band.wav");
    put_garbage(&root, "corrupt.flac", 99);
    put_lp16k(&root, "lp16k.wav");
    put_short(&root, "short.wav");
    assert_eq!(
        std::fs::metadata(root.join("full_band.wav")).unwrap().len(),
        std::fs::metadata(root.join("lp16k.wav")).unwrap().len(),
        "full_band and lp16k WAVs have the same size"
    );
    let events = Mutex::new(Vec::new());
    run_analysis(&AnalysisSlot::new(), &root, &|e| {
        events.lock().unwrap().push(e);
        true
    })
    .expect("run ok");
    let summary = match events.lock().unwrap().last() {
        Some(ProgressEvent::Finished { summary }) => summary.clone(),
        other => panic!("expected finished, got {other:?}"),
    };
    let run = Run {
        summary,
        events: events.into_inner().unwrap(),
    };
    assert_eq!(run.summary.done, 4);
    assert_eq!(run.of("full_band.wav").verdict, Verdict::Real);
    assert_eq!(run.of("lp16k.wav").verdict, Verdict::Fake);
    assert_eq!(error_kind(&run.of("corrupt.flac")), Some("corruptStream"));
    assert_eq!(error_kind(&run.of("short.wav")), Some("tooShort"));
}

// @RF-11 @RNF-04 Scenario: A warm run gives the cold run's events and CSV (acceptance C2)
#[test]
fn rf11_a_warm_run_gives_the_cold_runs_events_and_csv() {
    let t = TempDir::new("warm");
    let root = t.path().join("Mixed");
    put_full_band(&root, "full_band.wav");
    put_garbage(&root, "corrupt.flac", 99);
    put_lp16k(&root, "lp16k.wav");
    put_short(&root, "short.wav");
    let file = t.cache_file();

    let slot = AnalysisSlot::new();
    let state1 = ExportState::new();
    let cold = cached(&slot, &state1, Some(&file), &root);
    assert_eq!(cold.summary.cache, CacheStatus::Ok);
    assert_eq!(cold.summary.from_cache, 0);
    assert_eq!(cold.of("lp16k.wav").verdict, Verdict::Fake);
    let csv1 = csv_of(&slot, &state1, &t.path().join("cold.csv"));

    let state2 = ExportState::new();
    let warm = cached(&slot, &state2, Some(&file), &root);
    assert_eq!(
        warm.done(),
        cold.done(),
        "each trackDone equals the cold run's"
    );
    assert_eq!(
        warm.summary.from_cache, 3,
        "corrupt.flac is analysed again and short (tooShort) is not"
    );
    assert_eq!(warm.summary.cache, CacheStatus::Ok);
    let csv2 = csv_of(&slot, &state2, &t.path().join("warm.csv"));
    assert_eq!(csv2, csv1, "the CSV documents are byte-identical");
}

// @RF-11 Scenario: The cache, not the decoder, gives a warm run's verdict
#[test]
fn rf11_the_cache_not_the_decoder_gives_a_warm_runs_verdict() {
    let t = TempDir::new("cache-not-decoder");
    let root = t.path().join("Album A");
    let x = put_lp16k(&root, "x.wav");
    let file = t.cache_file();
    let run1 = cached_run(&file, &root);
    assert_eq!(run1.of("x.wav").verdict, Verdict::Fake);

    let original = std::fs::metadata(&x).unwrap().modified().unwrap();
    let size = std::fs::metadata(&x).unwrap().len();
    put_full_band(&root, "x.wav");
    set_mtime(&x, original);
    assert_eq!(std::fs::metadata(&x).unwrap().len(), size, "same size");

    let run2 = cached_run(&file, &root);
    assert_eq!(run2.of("x.wav").verdict, Verdict::Fake, "still fake");
    assert_eq!(run2.summary.from_cache, 1);

    clear_cache(&AnalysisSlot::new(), Some(&file)).expect("clear");
    let run3 = cached_run(&file, &root);
    assert_eq!(run3.of("x.wav").verdict, Verdict::Real);
    assert_eq!(run3.summary.from_cache, 0);
}

// @RF-11 Scenario: No cache folder means no cache, not an error
#[test]
fn rf11_no_cache_folder_means_no_cache_not_an_error() {
    let t = TempDir::new("no-cache-folder");
    let root = t.path().join("Album A");
    put_garbage(&root, "a.flac", 1);
    put_garbage(&root, "b.flac", 2);
    let run = cached(&AnalysisSlot::new(), &ExportState::new(), None, &root);
    assert!(
        matches!(run.events.last(), Some(ProgressEvent::Finished { summary }) if summary.done == 2)
    );
    assert_eq!(run.summary.cache, CacheStatus::Unavailable);
}

// @RF-11 Scenario: The existing run functions use no cache
#[test]
fn rf11_the_existing_run_functions_use_no_cache() {
    let t = TempDir::new("existing");
    let root = t.path().join("Album A");
    put_garbage(&root, "a.flac", 1);
    put_garbage(&root, "b.flac", 2);
    let slot = AnalysisSlot::new();
    let plain = run_analysis(&slot, &root, &|_| true).expect("run ok");
    let kept = run_analysis_kept(&slot, &ExportState::new(), &root, &|_| true).expect("run ok");
    for s in [plain, kept] {
        assert_eq!(s.cache, CacheStatus::Off);
        assert_eq!(s.from_cache, 0);
    }
}

// @RF-11 @RNF-05 Scenario: The cache never touches the analysed folder (acceptance C6)
#[test]
fn rf11_the_cache_never_touches_the_analysed_folder() {
    let t = TempDir::new("read-only");
    let root = t.path().join("Album A");
    let a = put_full_band(&root, "full_band.wav");
    let b = put_lp16k(&root, "lp16k.wav");
    let before = (snapshot(&a), snapshot(&b));
    let listing = names_in(&root);
    let file = t.cache_file();

    cached_run(&file, &root);
    let warm = cached_run(&file, &root);
    assert_eq!(warm.summary.from_cache, 2, "the second run is warm");

    assert_eq!((snapshot(&a), snapshot(&b)), before, "size, mtime and hash");
    assert_eq!(
        names_in(&root),
        listing,
        "Album A holds exactly its 2 files"
    );
    assert_eq!(names_in(t.path()), ["Album A", "cache"]);
    let created = names_in(file.parent().unwrap());
    assert!(created.contains(&CACHE_FILE.to_string()), "{created:?}");
    for name in &created {
        assert!(
            [
                CACHE_FILE.to_string(),
                format!("{CACHE_FILE}-wal"),
                format!("{CACHE_FILE}-shm")
            ]
            .contains(name),
            "unexpected file {name}"
        );
    }
}

// @RF-11 @RNF-09 Scenario: Unicode names in a real warm run
#[test]
fn rf11_unicode_names_in_a_real_warm_run() {
    let t = TempDir::new("unicode");
    let root = t.path().join("Álbum ″Ñandú″");
    let names = ["01 – Canción ‘x’.wav", "02 «Ça va» 🎵.wav"];
    for n in names {
        put_full_band(&root, n);
    }
    let file = t.cache_file();
    cached_run(&file, &root);
    let warm = cached_run(&file, &root);
    assert_eq!(warm.summary.from_cache, 2);
    let done = warm.done();
    for (i, a) in &done {
        let expected = root.join(names[*i as usize]);
        assert_eq!(
            a.info.path,
            expected.to_str().unwrap(),
            "code point for code point"
        );
    }
    for e in &warm.events {
        if let ProgressEvent::TrackDone { album, .. } = e {
            assert_eq!(album.name, "Álbum ″Ñandú″");
        }
    }
}

// --- Clearing (acceptance C8) -----------------------------------------------------------------

fn filled_cache(t: &TempDir) -> PathBuf {
    let root = t.path().join("Album A");
    put_garbage(&root, "a.flac", 1);
    put_full_band(&root, "b.wav");
    let file = t.cache_file();
    let run = cached_run(&file, &root);
    assert_eq!(run.summary.cache, CacheStatus::Ok);
    assert!(file.is_file());
    file
}

fn none_of_the_cache_files(file: &Path) {
    for f in [
        file.to_path_buf(),
        sidecar(file, "-wal"),
        sidecar(file, "-shm"),
    ] {
        assert!(!f.exists(), "{} still exists", f.display());
    }
}

// @RF-11 Scenario Outline: Clearing the cache (acceptance C8). Row: a run has filled the cache
#[test]
fn rf11_clearing_a_filled_cache() {
    let t = TempDir::new("clear-filled");
    let file = filled_cache(&t);
    clear_cache(&AnalysisSlot::new(), Some(&file)).expect("clear");
    none_of_the_cache_files(&file);
}

// Row: no cache file exists
#[test]
fn rf11_clearing_an_absent_cache_succeeds() {
    let t = TempDir::new("clear-absent");
    let file = t.cache_file();
    clear_cache(&AnalysisSlot::new(), Some(&file)).expect("clear");
    none_of_the_cache_files(&file);
}

// Row: the cache file holds random bytes
#[test]
fn rf11_clearing_a_cache_of_random_bytes() {
    let t = TempDir::new("clear-random");
    let file = t.cache_file();
    put_garbage(file.parent().unwrap(), CACHE_FILE, 7);
    clear_cache(&AnalysisSlot::new(), Some(&file)).expect("clear");
    assert!(!file.exists());
}

// Row: the cache file is a FakeFlac cache with user_version 99
#[test]
fn rf11_clearing_a_cache_of_a_newer_schema() {
    let t = TempDir::new("clear-newer");
    let file = filled_cache(&t);
    assert!(
        !sidecar(&file, "-wal").exists(),
        "precondition: the store closed cleanly, so the header is in the main file"
    );
    let mut bytes = std::fs::read(&file).unwrap();
    assert_eq!(&bytes[..16], b"SQLite format 3\0");
    bytes[60..64].copy_from_slice(&99u32.to_be_bytes());
    std::fs::write(&file, &bytes).unwrap();
    let unavailable = cached_run(&file, &t.path().join("Album A"));
    assert_eq!(
        unavailable.summary.cache,
        CacheStatus::Unavailable,
        "precondition: a newer schema"
    );

    clear_cache(&AnalysisSlot::new(), Some(&file)).expect("clear");
    assert!(!file.exists());
    let next = cached_run(&file, &t.path().join("Album A"));
    assert_eq!(next.summary.cache, CacheStatus::Ok);
}

// Row: the cache folder holds another file "notes.txt"
#[test]
fn rf11_clearing_keeps_other_files_of_the_cache_folder() {
    let t = TempDir::new("clear-notes");
    let file = filled_cache(&t);
    let notes = file.parent().unwrap().join("notes.txt");
    std::fs::write(&notes, "keep me").unwrap();
    clear_cache(&AnalysisSlot::new(), Some(&file)).expect("clear");
    assert_eq!(std::fs::read_to_string(&notes).unwrap(), "keep me");
    none_of_the_cache_files(&file);
}

// Row: a run holds the analysis slot
#[test]
fn rf11_clearing_while_a_run_holds_the_slot_fails_with_analysis_running() {
    let t = TempDir::new("clear-running");
    let file = filled_cache(&t);
    let files = |f: &Path| {
        [f.to_path_buf(), sidecar(f, "-wal"), sidecar(f, "-shm")]
            .iter()
            .map(|p| std::fs::read(p).ok())
            .collect::<Vec<_>>()
    };
    let before = files(&file);
    let slot = AnalysisSlot::new();
    let held = slot.begin().expect("a run holds the slot");
    let err = clear_cache(&slot, Some(&file)).expect_err("must fail");
    assert_eq!(err.code, AppErrorCode::AnalysisRunning);
    assert_eq!(files(&file), before, "the cache files are unchanged");
    drop(held);
}

// Row: another connection keeps the cache file open (Windows)
#[cfg(windows)]
#[test]
fn rf11_clearing_a_cache_held_open_fails_with_cache_failed() {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    let t = TempDir::new("clear-held");
    let file = filled_cache(&t);
    let handle = std::fs::File::options()
        .read(true)
        .write(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&file)
        .unwrap();
    let err = clear_cache(&AnalysisSlot::new(), Some(&file)).expect_err("must fail");
    assert_eq!(err.code, AppErrorCode::CacheFailed);
    assert!(!err.message.is_empty(), "the OS message");
    drop(handle);
    let again = cached_run(&file, &t.path().join("Album A"));
    assert_eq!(again.summary.cache, CacheStatus::Ok, "still a valid cache");
    assert_eq!(again.summary.from_cache, 1, "its results are still there");
}

// result-cache.md section 9: with no cache folder (`None`), clearing fails with CACHE_FAILED.
#[test]
fn rf11_clearing_without_a_cache_folder_fails_with_cache_failed() {
    let err = clear_cache(&AnalysisSlot::new(), None).expect_err("must fail");
    assert_eq!(err.code, AppErrorCode::CacheFailed);
}

// @RF-11 Scenario: Clearing changes no result kept for the export
#[test]
fn rf11_clearing_changes_no_result_kept_for_the_export() {
    let t = TempDir::new("clear-export");
    let root = t.path().join("Album A");
    put_garbage(&root, "a.flac", 1);
    put_garbage(&root, "b.flac", 2);
    put_full_band(&root, "c.wav");
    let file = t.cache_file();
    let slot = AnalysisSlot::new();
    let state = ExportState::new();
    cached(&slot, &state, Some(&file), &root);
    let (before, guard) = begin_export(&slot, &state).expect("begin export");
    drop(guard);
    assert_eq!(before.tracks.len(), 3);

    clear_cache(&slot, Some(&file)).expect("clear");
    let (after, guard) = begin_export(&slot, &state).expect("begin export");
    drop(guard);
    assert_eq!(after.tracks, before.tracks);
    assert_eq!(after.root, before.root);
    // The slot is free again once clearing is done (it holds it only while it works).
    drop(slot.begin().expect("the slot is free after clearing"));
}

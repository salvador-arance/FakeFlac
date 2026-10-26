//! CSV export around the save dialog (RF-09, RNF-04, RNF-05, RNF-09), derived from the
//! "Command" scenarios of `specs/features/export-csv.feature`. The steps before and after the
//! dialog (`run::begin_export`, `run::export_csv`) run without a Tauri runtime; the dialog is
//! stubbed by choosing the destination in the test. Audio files are generated inside the test
//! (target/tmp), never taken from the corpus.
//!
//! Readings chosen where the scenarios are open (reported in the test-stage report):
//! - the lossless fixtures are written as 16-bit WAV noise (`.wav`; the `src-tauri` crate has
//!   no FLAC encoder and the scenarios say the content does not matter) and "corrupt.flac" is
//!   random bytes; the lossy ones are the writers of `lossy-fixtures.md`;
//! - results are kept through `ExportState::record` / `run::run_analysis_kept`; `begin_export`
//!   returns the snapshot and the guard, which the test drops when "the export ends".
//!
//! Protected after the `test(RF-09)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../crates/fakeflac-audio/tests/common/lossy.rs"]
mod lossy;

#[path = "../../crates/fakeflac-export/tests/common/mod.rs"]
mod csv_support;

use csv_support::{col, read_csv, track, HEADER};
use fakeflac_core as core;
use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, AppErrorCode};
use fakeflac_lib::run::{
    begin_export, export_csv, run_analysis_kept, ExportGuard, ExportSnapshot, ExportState,
};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("export_csv_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

// --- Audio files generated in the test --------------------------------------------------------

fn put_lossless(dir: &Path, file: &str, seed: u64) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(dir.join(file), spec).expect("wav");
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..(44_100 * 6) {
        let s: f64 = rng.random::<f64>() * 2.0 - 1.0;
        w.write_sample((s * 10_000.0) as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

fn put_corrupt(dir: &Path, file: &str) {
    let mut rng = ChaCha8Rng::seed_from_u64(99);
    let bytes: Vec<u8> = (0..4096).map(|_| rng.random::<u8>()).collect();
    std::fs::write(dir.join(file), bytes).unwrap();
}

fn put_lossy(dir: &Path, fixture: &str, file: &str) {
    let bytes =
        lossy::fixture_bytes(fixture).unwrap_or_else(|| panic!("unknown fixture {fixture}"));
    std::fs::write(dir.join(file), bytes).unwrap();
}

// --- Running and exporting ----------------------------------------------------------------------

fn run_kept(
    slot: &AnalysisSlot,
    state: &ExportState,
    root: &Path,
) -> (
    Result<dto::AnalysisSummary, dto::AppError>,
    Vec<dto::ProgressEvent>,
) {
    let events = Mutex::new(Vec::new());
    let result = run_analysis_kept(slot, state, root, &|e| {
        events.lock().unwrap().push(e);
        true
    });
    (result, events.into_inner().unwrap())
}

fn track_done(events: &[dto::ProgressEvent]) -> Vec<(u32, &dto::TrackAnalysis)> {
    events
        .iter()
        .filter_map(|e| match e {
            dto::ProgressEvent::TrackDone {
                index, analysis, ..
            } => Some((*index, analysis)),
            _ => None,
        })
        .collect()
}

/// "I export": `begin_export`, then the dialog (stubbed: `dest`), then `export_csv`.
fn export_to(
    slot: &AnalysisSlot,
    state: &ExportState,
    dest: &Path,
) -> Result<dto::ExportResult, dto::AppError> {
    let (snapshot, guard) = begin_export(slot, state)?;
    let result = export_csv(&snapshot, dest);
    drop(guard);
    result
}

fn read_records(path: &Path) -> Vec<Vec<String>> {
    read_csv(&std::fs::read(path).unwrap())
}

fn files(records: &[Vec<String>]) -> Vec<String> {
    records[1..]
        .iter()
        .map(|r| r[col("file")].clone())
        .collect()
}

// --- Synthetic results (core events) ------------------------------------------------------------

fn core_track(root: &str, name: &str) -> core::TrackAnalysis {
    track(&format!(r"{root}\{name}"))
}

fn scanned(total: u32) -> core::ProgressEvent {
    core::ProgressEvent::Scanned { total }
}

fn done(index: u32, root: &str, name: &str) -> core::ProgressEvent {
    let analysis = core_track(root, name);
    let album = core::AlbumUpdate {
        index: 0,
        name: PathBuf::from("Lib"),
        files: 1,
        summary: core::AlbumSummary {
            folder: analysis.info.folder.clone(),
            total: 1,
            real: 1,
            suspicious: 0,
            fake: 0,
            errors: 0,
            verdict: core::Verdict::Real,
        },
    };
    core::ProgressEvent::TrackDone {
        index,
        analysis,
        album,
    }
}

fn summary(root: &str, total: u32, done: u32, cancelled: bool) -> core::AnalysisSummary {
    core::AnalysisSummary {
        root: PathBuf::from(root),
        total,
        done,
        real: done,
        suspicious: 0,
        fake: 0,
        errors: 0,
        cancelled,
        elapsed_ms: 10,
    }
}

/// A finished run of `root` that reported the files in index order.
fn keep_finished(state: &ExportState, root: &str, files: &[&str]) {
    let root_path = Path::new(root);
    state.record(root_path, &scanned(files.len() as u32));
    for (i, f) in files.iter().enumerate() {
        state.record(root_path, &done(i as u32, root, f));
    }
    state.record(
        root_path,
        &core::ProgressEvent::Finished {
            summary: summary(root, files.len() as u32, files.len() as u32, false),
        },
    );
}

const A: &str = r"C:\Music\Album A";
const B: &str = r"C:\Music\Album B";

fn err_code<T: std::fmt::Debug>(r: Result<T, dto::AppError>) -> AppErrorCode {
    r.unwrap_err().code
}

// ================================================================================================
// Real runs
// ================================================================================================

/// The values of a `trackDone` event as the CSV must carry them (csv-export.md sections 3, 4).
fn json_str<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .unwrap()
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn check_record(r: &[String], a: &dto::TrackAnalysis, algorithm_version: u32) {
    assert_eq!(r.len(), 21);
    assert_eq!(r[col("verdict")], json_str(&a.verdict));
    assert_eq!(
        r[col("cutoff_hz")],
        a.cutoff.as_ref().map_or(String::new(), |c| format!(
            "{}",
            f64::from(c.frequency_hz).round() as i64
        ))
    );
    assert_eq!(
        r[col("declared_bitrate_kbps")],
        a.info
            .declared_bitrate_kbps
            .map_or(String::new(), |v| v.to_string())
    );
    assert_eq!(
        r[col("bitrate_mode")],
        a.info.bitrate_mode.as_ref().map_or(String::new(), json_str)
    );
    assert_eq!(
        r[col("estimated_source_bitrate_kbps")],
        a.estimated_source_bitrate_kbps
            .map_or(String::new(), |v| v.to_string())
    );
    let reasons: Vec<String> = a.reasons.iter().map(json_str).collect();
    assert_eq!(r[col("reasons")], reasons.join(";"));
    let error = serde_json::to_value(&a.error).unwrap();
    assert_eq!(
        r[col("error")],
        error.get("kind").and_then(|k| k.as_str()).unwrap_or("")
    );
    assert_eq!(r[col("path")], a.info.path);
    assert_eq!(r[col("folder")], a.info.folder);
    assert_eq!(
        r[col("file")],
        Path::new(&a.info.path)
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
    assert_eq!(r[col("format")], json_str(&a.info.format));
    assert_eq!(
        r[col("codec")],
        a.info.codec.as_ref().map_or(String::new(), json_str)
    );
    assert_eq!(r[col("size_bytes")], a.info.size_bytes.to_string());
    assert_eq!(r[col("algorithm_version")], algorithm_version.to_string());
}

// @RF-09 @RNF-04 Scenario: Export after a finished analysis
#[test]
fn export_after_a_finished_analysis() {
    let dir = temp_dir("finished");
    let root = dir.join("Mixed");
    std::fs::create_dir_all(&root).unwrap();
    put_lossless(&root, "full_band.wav", 1);
    put_corrupt(&root, "corrupt.flac");
    put_lossless(&root, "lp16k.wav", 2);
    put_lossy(&root, "mp3_320_lp17k", "mp3_320_lp17k.mp3");
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();

    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    let (result, events) = run_kept(&slot, &state, &root);
    result.expect("run ok");
    let tracks = track_done(&events);
    assert_eq!(tracks.len(), 4);

    let dest = out.join("results.csv");
    let exported = export_to(&slot, &state, &dest).expect("export");
    assert_eq!(exported.rows, 4);
    assert_eq!(exported.path, dest.to_string_lossy());

    let records = read_records(&dest);
    assert_eq!(records[0].join(","), HEADER);
    assert_eq!(records.len(), 5);
    let version = core::DetectionParams::default().algorithm_version;

    let mut by_index = tracks.clone();
    by_index.sort_by_key(|(i, _)| *i);
    for (record, (_, analysis)) in records[1..].iter().zip(&by_index) {
        check_record(record, analysis, version);
    }

    let find = |file: &str| {
        records[1..]
            .iter()
            .find(|r| r[col("file")] == file)
            .unwrap_or_else(|| panic!("no record for {file}"))
    };
    let corrupt = find("corrupt.flac");
    assert_eq!(corrupt[col("verdict")], "error");
    for c in ["cutoff_hz", "cutoff_steepness_db_per_khz", "noise_floor_db"] {
        assert_eq!(corrupt[col(c)], "", "{c}");
    }
    let mp3 = find("mp3_320_lp17k.mp3");
    assert_ne!(mp3[col("declared_bitrate_kbps")], "");
    assert_ne!(mp3[col("bitrate_mode")], "");
    let lossless = find("full_band.wav");
    for c in [
        "declared_bitrate_kbps",
        "bitrate_mode",
        "estimated_source_bitrate_kbps",
    ] {
        assert_eq!(lossless[col(c)], "", "{c}");
    }
    assert_eq!(names(&out), vec!["results.csv"]);
}

// @RF-09 @RNF-05 Scenario: Exporting never touches the audio files
#[test]
fn exporting_never_touches_the_audio_files() {
    let root = temp_dir("untouched").join("Album A");
    std::fs::create_dir_all(&root).unwrap();
    put_lossless(&root, "full_band.wav", 1);
    put_lossless(&root, "lp16k.wav", 2);
    put_lossy(&root, "mp3_320_lp20k5", "mp3_320_lp20k5.mp3");
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    run_kept(&slot, &state, &root).0.expect("run ok");

    let fingerprint = || -> Vec<(String, u64, std::time::SystemTime, u64)> {
        names(&root)
            .into_iter()
            .map(|n| {
                let p = root.join(&n);
                let meta = std::fs::metadata(&p).unwrap();
                let mut h = DefaultHasher::new();
                std::fs::read(&p).unwrap().hash(&mut h);
                (n, meta.len(), meta.modified().unwrap(), h.finish())
            })
            .collect()
    };
    let before = fingerprint();
    assert_eq!(before.len(), 3);

    let exported = export_to(&slot, &state, &root.join("results.csv")).expect("export");
    assert_eq!(exported.rows, 3);

    let after = fingerprint();
    assert_eq!(after.len(), 4);
    assert_eq!(&after[..1], &before[..1], "full_band.wav");
    let audio_after: Vec<_> = after
        .iter()
        .filter(|f| f.0 != "results.csv")
        .cloned()
        .collect();
    assert_eq!(audio_after, before);
    assert_eq!(names(&root).len(), 4);
}

// @RF-09 @RNF-05 Scenario: Exporting reads no audio file
#[test]
fn exporting_reads_no_audio_file() {
    let root = temp_dir("deleted").join("Album A");
    std::fs::create_dir_all(&root).unwrap();
    put_lossless(&root, "lp16k.wav", 2);
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    let (result, events) = run_kept(&slot, &state, &root);
    result.expect("run ok");
    let recorded = track_done(&events)[0].1.clone();
    std::fs::remove_file(root.join("lp16k.wav")).unwrap();

    let dest = temp_dir("deleted_out").join("r.csv");
    assert_eq!(export_to(&slot, &state, &dest).expect("export").rows, 1);
    let records = read_records(&dest);
    assert_eq!(records.len(), 2);
    // The defaults carry algorithm_version 7 since detection v7 (RF-16).
    check_record(&records[1], &recorded, 7);
    assert_eq!(records[1][col("file")], "lp16k.wav");
}

// @RF-09 @RNF-09 Scenario: Unicode names survive a round-trip
#[test]
fn unicode_names_survive_a_round_trip() {
    let root = temp_dir("unicode").join("Álbum ″Ñandú″");
    std::fs::create_dir_all(&root).unwrap();
    let files = [
        "01 – Canción ‘x’.wav",
        "02 «Ça va» 🎵.wav",
        "03 Live, 1999.wav",
    ];
    for (i, f) in files.iter().enumerate() {
        put_lossless(&root, f, i as u64);
    }
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    let (result, events) = run_kept(&slot, &state, &root);
    result.expect("run ok");
    let out = temp_dir("unicode_out");
    let dest = out.join("résultats ″1″.csv");
    assert_eq!(export_to(&slot, &state, &dest).expect("export").rows, 3);

    let records = read_records(&dest);
    assert_eq!(files_of(&records), files.to_vec());
    let mut analysed = track_done(&events);
    analysed.sort_by_key(|(i, _)| *i);
    for (r, (_, a)) in records[1..].iter().zip(analysed) {
        assert_eq!(r[col("folder")], root.to_string_lossy());
        assert_eq!(r[col("path")], a.info.path);
        assert_eq!(Path::new(&r[col("path")]), root.join(&r[col("file")]));
    }
}

fn files_of(records: &[Vec<String>]) -> Vec<String> {
    files(records)
}

// ================================================================================================
// Kept results (no audio)
// ================================================================================================

// @RF-09 Scenario: Rows follow the scan order, not the arrival order or the table sort
#[test]
fn rows_follow_the_scan_order() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    let root = Path::new(A);
    state.record(root, &scanned(3));
    for (index, name) in [(2, "c.flac"), (0, "a.flac"), (1, "b.flac")] {
        state.record(root, &done(index, A, name));
    }
    let dest = temp_dir("order").join("r.csv");
    export_to(&slot, &state, &dest).expect("export");
    assert_eq!(files(&read_records(&dest)), ["a.flac", "b.flac", "c.flac"]);
}

// @RF-09 Scenario: A cancelled analysis exports the tracks done
#[test]
fn a_cancelled_analysis_exports_the_tracks_done() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    let root = Path::new(r"C:\Music\Big");
    state.record(root, &scanned(200));
    // 18 tracks done, not the first 18 indexes (parallel workers), arriving out of order
    let indexes: Vec<u32> = (0..18).rev().collect();
    for i in indexes {
        state.record(root, &done(i, r"C:\Music\Big", &format!("t{i:03}.flac")));
    }
    state.record(
        root,
        &core::ProgressEvent::Cancelled {
            summary: summary(r"C:\Music\Big", 200, 18, true),
        },
    );
    let dest = temp_dir("cancelled").join("r.csv");
    let exported = export_to(&slot, &state, &dest).expect("export");
    assert_eq!(exported.rows, 18);
    let records = read_records(&dest);
    assert_eq!(records.len(), 19);
    let expected: Vec<String> = (0..18).map(|i| format!("t{i:03}.flac")).collect();
    assert_eq!(files(&records), expected);
}

// @RF-09 @RNF-04 Scenario: A run that failed unexpectedly exports the tracks it reported
#[test]
fn a_run_that_failed_unexpectedly_exports_the_tracks_it_reported() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    let root = Path::new(A);
    state.record(root, &scanned(5));
    state.record(root, &done(0, A, "a.flac"));
    state.record(root, &done(1, A, "b.flac"));
    // the run then fails with INTERNAL: no terminal event is recorded
    let dest = temp_dir("internal").join("r.csv");
    assert_eq!(export_to(&slot, &state, &dest).expect("export").rows, 2);
    assert_eq!(read_records(&dest).len(), 3);
}

// @RF-09 Scenario: A new analysis replaces the results
#[test]
fn a_new_analysis_replaces_the_results() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a1.flac", "a2.flac", "a3.flac"]);
    keep_finished(&state, B, &["b1.flac", "b2.flac"]);
    let (snapshot, guard) = begin_export(&slot, &state).expect("begin");
    assert_eq!(snapshot.root, PathBuf::from(B));
    drop(guard);
    let dest = temp_dir("replaced").join("r.csv");
    export_to(&slot, &state, &dest).expect("export");
    assert_eq!(files(&read_records(&dest)), ["b1.flac", "b2.flac"]);
    assert_eq!(
        fakeflac_export::default_file_name(&snapshot.root),
        "FakeFlac - Album B.csv"
    );
}

// @RF-09 Scenario: An analysis that fails before it starts keeps the previous results
#[test]
fn a_start_that_fails_keeps_the_previous_results() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a1.flac", "a2.flac", "a3.flac"]);
    let docs = temp_dir("docs_only").join("Docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("notes.txt"), "hello").unwrap();
    let (result, events) = run_kept(&slot, &state, &docs);
    assert_eq!(err_code(result), AppErrorCode::NoAudioFound);
    assert!(events.is_empty());

    let (snapshot, guard) = begin_export(&slot, &state).expect("begin");
    drop(guard);
    assert_eq!(snapshot.root, PathBuf::from(A));
    assert_eq!(snapshot.tracks.len(), 3);
    assert_eq!(
        fakeflac_export::default_file_name(&snapshot.root),
        "FakeFlac - Album A.csv"
    );
    let dest = temp_dir("kept_after_fail").join("r.csv");
    export_to(&slot, &state, &dest).expect("export");
    assert_eq!(read_records(&dest).len(), 4);
}

// @RF-09 Scenario: No export while an analysis is running
#[test]
fn no_export_while_an_analysis_is_running() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac"]);
    let _run = slot.begin().expect("a run holds the slot");
    assert_eq!(
        err_code(begin_export(&slot, &state)),
        AppErrorCode::AnalysisRunning
    );
}

// @RF-09 Scenario: No export before any analysis
#[test]
fn no_export_before_any_analysis() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    assert_eq!(
        err_code(begin_export(&slot, &state)),
        AppErrorCode::NotAnalyzed
    );
}

// @RF-09 Scenario: No export when the latest run reported no track
#[test]
fn no_export_when_the_latest_run_reported_no_track() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac"]);
    let big = Path::new(r"C:\Music\Big");
    state.record(big, &scanned(200));
    state.record(
        big,
        &core::ProgressEvent::Cancelled {
            summary: summary(r"C:\Music\Big", 200, 0, true),
        },
    );
    assert_eq!(
        err_code(begin_export(&slot, &state)),
        AppErrorCode::NotAnalyzed
    );
}

// @RF-09 Scenario: Cancelling the save dialog writes nothing
#[test]
fn cancelling_the_save_dialog_writes_nothing() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac"]);
    let out = temp_dir("dialog_cancelled");
    let (_snapshot, guard) = begin_export(&slot, &state).expect("begin");
    // the dialog answers "cancelled": export_csv is not called
    drop(guard);
    assert!(names(&out).is_empty());
}

fn begin(slot: &AnalysisSlot, state: &ExportState) -> (ExportSnapshot, ExportGuard) {
    begin_export(slot, state).expect("begin_export")
}

// @RF-09 Scenario: A second export while one is in progress
#[test]
fn a_second_export_while_one_is_in_progress() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac", "b.flac", "c.flac"]);
    let (snapshot, guard) = begin(&slot, &state);

    let second = begin_export(&slot, &state).unwrap_err();
    assert_eq!(second.code, AppErrorCode::Internal);
    assert_eq!(second.message, "an export is already in progress");

    let out = temp_dir("second_export");
    let exported = export_csv(&snapshot, &out.join("results.csv")).expect("first export");
    assert_eq!(exported.rows, 3);
    assert_eq!(read_records(&out.join("results.csv")).len(), 4);
    assert_eq!(names(&out), vec!["results.csv"]);
    drop(guard);
}

// @RF-09 Scenario Outline: The export guard is released however the export ends
#[test]
fn the_export_guard_is_released_however_the_export_ends() {
    let out = temp_dir("guard_released");
    for ending in ["cancelled", "rows 3", "EXPORT_FAILED"] {
        let (slot, state) = (AnalysisSlot::new(), ExportState::new());
        keep_finished(&state, A, &["a.flac", "b.flac", "c.flac"]);
        let (snapshot, guard) = begin(&slot, &state);
        match ending {
            "cancelled" => {}
            "rows 3" => {
                let r = export_csv(&snapshot, &out.join("results.csv")).expect("export");
                assert_eq!(r.rows, 3);
            }
            _ => {
                let e = export_csv(&snapshot, &out.join("results.txt")).unwrap_err();
                assert_eq!(e.code, AppErrorCode::ExportFailed);
            }
        }
        drop(guard);
        let (_again, guard) =
            begin_export(&slot, &state).unwrap_or_else(|e| panic!("{ending}: {e:?}"));
        drop(guard);
    }
    assert!(!out.join("results.txt").exists());
}

// @RF-09 Scenario: An export in progress is reported before a running analysis
#[test]
fn an_export_in_progress_is_reported_before_a_running_analysis() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac", "b.flac", "c.flac"]);
    let (_snapshot, _guard) = begin(&slot, &state);
    let _run = slot.begin().expect("a run holds the slot");
    assert_eq!(
        err_code(begin_export(&slot, &state)),
        AppErrorCode::Internal
    );
}

// @RF-09 Scenario: A run started while the dialog is open does not change the export
#[test]
fn a_run_started_while_the_dialog_is_open_does_not_change_the_export() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a1.flac", "a2.flac", "a3.flac"]);
    let (snapshot, guard) = begin(&slot, &state);
    // the dialog is open; an analysis of Album B starts and reports a track
    state.record(Path::new(B), &scanned(2));
    state.record(Path::new(B), &done(0, B, "b1.flac"));
    let dest = temp_dir("run_during_dialog").join("results.csv");
    assert_eq!(export_csv(&snapshot, &dest).expect("export").rows, 3);
    drop(guard);
    assert_eq!(
        files(&read_records(&dest)),
        ["a1.flac", "a2.flac", "a3.flac"]
    );
}

// @RF-09 Scenario Outline: A file that cannot be written is a typed error (command level)
#[test]
fn run_export_csv_refuses_what_it_cannot_write() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac", "b.flac"]);
    let out = temp_dir("typed_errors");

    let missing = export_to(&slot, &state, &out.join("missing").join("results.csv"));
    assert_eq!(err_code(missing), AppErrorCode::ExportFailed);
    assert!(!out.join("missing").exists());

    let wrong_extension = export_to(&slot, &state, &out.join("results.txt"));
    assert_eq!(err_code(wrong_extension), AppErrorCode::ExportFailed);

    let relative = export_to(&slot, &state, Path::new(r"out\results.csv"));
    assert_eq!(err_code(relative), AppErrorCode::ExportFailed);

    assert!(
        names(&out).is_empty(),
        "nothing was created: {:?}",
        names(&out)
    );
}

// @RF-09 Scenario: Replacing an existing file (command level)
#[test]
fn run_export_csv_replaces_an_existing_file_as_a_whole() {
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    keep_finished(&state, A, &["a.flac", "b.flac"]);
    let out = temp_dir("replace_command");
    std::fs::write(out.join("results.csv"), vec![b'x'; 100_000]).unwrap();
    export_to(&slot, &state, &out.join("results.csv")).expect("export");
    let bytes = std::fs::read(out.join("results.csv")).unwrap();
    assert!(bytes.starts_with(HEADER.as_bytes()));
    assert!(bytes.len() < 10_000);
    assert_eq!(read_csv(&bytes).len(), 3);
    assert_eq!(names(&out), vec!["results.csv"]);
}

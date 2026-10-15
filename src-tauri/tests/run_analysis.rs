//! Wiring of the analysis to the real adapters (RF-01, RF-02, RF-05, RF-06, RNF-04, RNF-05).
//! Real temp folders with synthetic files. Protected after the `test(RF-01,RF-05)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, AppErrorCode, ProgressEvent, Verdict};
use fakeflac_lib::run::{run_analysis, worker_count};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("run_analysis_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn garbage(seed: u64, len: usize) -> Vec<u8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..len).map(|_| rng.random::<u8>()).collect()
}

fn write_noise_wav(path: &Path, seconds: u32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("wav");
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    for _ in 0..(44100 * seconds * 2) {
        let s: f64 = rng.random::<f64>() * 2.0 - 1.0;
        w.write_sample((s * 16000.0) as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

type Collected = (
    Result<dto::AnalysisSummary, dto::AppError>,
    Vec<ProgressEvent>,
);

fn collect(slot: &AnalysisSlot, root: &Path) -> Collected {
    let events = Mutex::new(Vec::new());
    let result = run_analysis(slot, root, &|e| {
        events.lock().unwrap().push(e);
        true
    });
    (result, events.into_inner().unwrap())
}

// @RF-02 Folder without supported audio
#[test]
fn folder_without_audio_is_no_audio_found() {
    let dir = temp_dir("no_audio");
    std::fs::write(dir.join("notes.txt"), "hello").unwrap();
    let (result, events) = collect(&AnalysisSlot::new(), &dir);
    assert_eq!(result.unwrap_err().code, AppErrorCode::NoAudioFound);
    assert!(events.is_empty());
}

// @RF-02
#[test]
fn missing_folder_is_folder_unreadable() {
    let dir = temp_dir("missing").join("does-not-exist");
    let (result, events) = collect(&AnalysisSlot::new(), &dir);
    assert_eq!(result.unwrap_err().code, AppErrorCode::FolderUnreadable);
    assert!(events.is_empty());
}

// @RF-01 Analysing a file instead of a folder (contract v2)
#[test]
fn file_root_is_not_a_folder() {
    let dir = temp_dir("file-root");
    std::fs::write(dir.join("a.flac"), garbage(7, 512)).unwrap();
    let (result, events) = collect(&AnalysisSlot::new(), &dir.join("a.flac"));
    assert_eq!(result.unwrap_err().code, AppErrorCode::NotAFolder);
    assert!(events.is_empty());
}

// @RF-01 Relative paths are rejected (contract v2)
#[test]
fn relative_root_is_folder_unreadable() {
    assert!(
        Path::new("tests").is_dir(),
        "precondition: relative folder exists"
    );
    let (result, events) = collect(&AnalysisSlot::new(), Path::new("tests"));
    assert_eq!(result.unwrap_err().code, AppErrorCode::FolderUnreadable);
    assert!(events.is_empty());
}

// @RNF-04 A corrupt file does not stop the analysis; @RF-06 row content
#[test]
fn corrupt_file_does_not_stop_the_analysis() {
    let dir = temp_dir("corrupt");
    std::fs::write(dir.join("bad.flac"), garbage(1, 4096)).unwrap();
    write_noise_wav(&dir.join("good.wav"), 6);
    let (result, events) = collect(&AnalysisSlot::new(), &dir);
    let summary = result.expect("run ok");
    assert_eq!(summary.total, 2);
    assert_eq!(summary.errors, 1);

    assert!(matches!(
        events.first(),
        Some(ProgressEvent::Scanned { total: 2 })
    ));
    match events.last() {
        Some(ProgressEvent::Finished { summary }) => assert_eq!(summary.errors, 1),
        other => panic!("expected finished, got {other:?}"),
    }
    let started = events
        .iter()
        .filter(|e| matches!(e, ProgressEvent::TrackStarted { .. }))
        .count();
    assert_eq!(started, 2);

    let analyses: Vec<&dto::TrackAnalysis> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone { analysis, .. } => Some(analysis),
            _ => None,
        })
        .collect();
    assert_eq!(analyses.len(), 2);
    let bad = analyses
        .iter()
        .find(|a| a.info.path.ends_with("bad.flac"))
        .expect("bad row");
    assert_eq!(bad.verdict, Verdict::Error);
    // The scenario expects the reason "Corrupt stream".
    assert!(
        matches!(bad.error, Some(dto::TrackError::CorruptStream { .. })),
        "{:?}",
        bad.error
    );
    let good = analyses
        .iter()
        .find(|a| a.info.path.ends_with("good.wav"))
        .expect("good row");
    assert_eq!(good.verdict, Verdict::Real);
    assert_eq!(good.info.format, dto::AudioFormat::Wav);
    assert_eq!(good.info.sample_rate, 44100);
    assert_eq!(good.info.bit_depth, Some(16));
    assert_eq!(good.info.channels, 2);
    assert!(
        good.info.duration_ms.abs_diff(6000) <= 50,
        "{}",
        good.info.duration_ms
    );
}

// @RF-05 Only one analysis at a time
#[test]
fn only_one_analysis_at_a_time() {
    let dir = temp_dir("one_at_a_time");
    write_noise_wav(&dir.join("a.wav"), 6);
    let slot = AnalysisSlot::new();
    let held = slot.begin().expect("slot free");
    let (result, events) = collect(&slot, &dir);
    assert_eq!(result.unwrap_err().code, AppErrorCode::AnalysisRunning);
    assert!(events.is_empty());
    drop(held);

    assert!(collect(&slot, &dir).0.is_ok());
    // The slot is free again after the run.
    assert!(collect(&slot, &dir).0.is_ok());
}

// @RF-05 window gone: `send` returning false cancels the run
#[test]
fn send_returning_false_cancels_the_run() {
    let dir = temp_dir("window_gone");
    for i in 0..200u64 {
        std::fs::write(dir.join(format!("t{i:03}.flac")), garbage(i, 64)).unwrap();
    }
    let summary = run_analysis(&AnalysisSlot::new(), &dir, &|e| {
        !matches!(e, ProgressEvent::TrackDone { .. })
    })
    .expect("run ok");
    assert!(summary.cancelled);
    assert!(summary.done < 200, "done = {}", summary.done);
}

// @RNF-05 Audio files are never modified
#[test]
fn files_are_left_untouched() {
    let dir = temp_dir("readonly");
    std::fs::write(dir.join("bad.flac"), garbage(3, 2048)).unwrap();
    write_noise_wav(&dir.join("good.wav"), 6);
    let snapshot = || {
        let mut v = Vec::new();
        for name in ["bad.flac", "good.wav"] {
            let p = dir.join(name);
            let m = std::fs::metadata(&p).unwrap();
            v.push((m.len(), m.modified().unwrap(), std::fs::read(&p).unwrap()));
        }
        v
    };
    let before = snapshot();
    collect(&AnalysisSlot::new(), &dir).0.expect("run ok");
    assert_eq!(before, snapshot());
}

#[test]
fn worker_count_is_between_one_and_the_cores() {
    let n = worker_count();
    let cores = std::thread::available_parallelism().map_or(1, |c| c.get());
    assert!((1..=cores).contains(&n), "n = {n}, cores = {cores}");
}

// @RF-05 Progress is reported per track and globally: trackStarted in index order 0..n and
// exactly one trackDone per index.
#[test]
fn progress_events_cover_every_index_once() {
    let dir = temp_dir("progress");
    for i in 0..12u64 {
        std::fs::write(dir.join(format!("t{i:02}.flac")), garbage(100 + i, 2048)).unwrap();
    }
    let (result, events) = collect(&AnalysisSlot::new(), &dir);
    let summary = result.expect("run ok");
    assert_eq!(summary.total, 12);
    let started: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackStarted { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(started, (0..12).collect::<Vec<u32>>());
    let mut done: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    done.sort_unstable();
    assert_eq!(done, (0..12).collect::<Vec<u32>>());
    assert!(matches!(
        events.last(),
        Some(ProgressEvent::Finished { .. })
    ));
}

//! Use-case tests of `specs/features/analyze-folder.feature` (RF-02, RF-05, RNF-04) with fakes of
//! the ports. No real audio. Protected after the `test(RF-02,RF-05)` commit.

#![allow(clippy::expect_used)]

use fakeflac_core::ports::{AudioProbe, FileEntry, FileScanner, SpectrumAnalyzer};
use fakeflac_core::{
    analyze_folder, AnalysisSlot, AnalysisSummary, AppError, AppErrorCode, AudioFormat,
    AverageSpectrum, CancelToken, DetectionParams, ProgressEvent, TrackError, Verdict,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct FakeScanner(Result<Vec<FileEntry>, AppError>);

impl FileScanner for FakeScanner {
    fn scan(&self, _root: &Path) -> Result<Vec<FileEntry>, AppError> {
        self.0.clone()
    }
}

fn entry(name: &str) -> FileEntry {
    let format = if name.to_ascii_lowercase().ends_with(".wav") {
        AudioFormat::Wav
    } else {
        AudioFormat::Flac
    };
    FileEntry {
        path: PathBuf::from("music").join(name),
        folder: PathBuf::from("music"),
        format,
        size_bytes: 1000,
        modified_unix: 1_700_000_000,
    }
}

fn scanner(names: &[&str]) -> FakeScanner {
    FakeScanner(Ok(names.iter().map(|n| entry(n)).collect()))
}

fn spectrum(db: impl Fn(f64) -> f64) -> AverageSpectrum {
    let sample_rate = 44_100;
    let n = 8192usize;
    let bin_hz = f64::from(sample_rate) / n as f64;
    let power_db = (0..=n / 2).map(|k| db(k as f64 * bin_hz)).collect();
    AverageSpectrum {
        sample_rate,
        bin_hz,
        power_db,
    }
}

/// Behaviour chosen by file name: `fake*` cliff at 16 kHz, `susp*` cliff at 20.9 kHz,
/// `corrupt*` corrupt stream, `panic*` panics (a decoder bug), anything else flat (real). Optional sleep per track.
struct FakeAnalyzer {
    delay: Duration,
    calls: AtomicUsize,
}

impl FakeAnalyzer {
    fn new(delay_ms: u64) -> Self {
        FakeAnalyzer {
            delay: Duration::from_millis(delay_ms),
            calls: AtomicUsize::new(0),
        }
    }
}

impl SpectrumAnalyzer for FakeAnalyzer {
    fn analyze(&self, path: &Path, _p: &DetectionParams) -> Result<AudioProbe, TrackError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if name.starts_with("corrupt") {
            return Err(TrackError::CorruptStream("bad frame".into()));
        }
        if name.starts_with("panic") {
            panic!("decoder bug (expected by the test)");
        }
        let s = if name.starts_with("fake") {
            spectrum(|f| if f < 16_000.0 { 0.0 } else { -100.0 })
        } else if name.starts_with("susp") {
            spectrum(|f| if f < 20_900.0 { 0.0 } else { -100.0 })
        } else {
            spectrum(|_| 0.0)
        };
        Ok(AudioProbe {
            sample_rate: 44_100,
            bit_depth: Some(16),
            channels: 2,
            duration_ms: 180_000,
            spectrum: Some(s),
        })
    }
}

type Events = Mutex<Vec<ProgressEvent>>;

fn run(
    sc: &FakeScanner,
    an: &FakeAnalyzer,
    workers: usize,
    cancel: &CancelToken,
    events: &Events,
) -> Result<AnalysisSummary, AppError> {
    let sink = |e: ProgressEvent| events.lock().expect("lock").push(e);
    analyze_folder(
        sc,
        an,
        &DetectionParams::default(),
        Path::new("music"),
        workers,
        cancel,
        &sink,
    )
}

fn done_by_index(events: &[ProgressEvent]) -> Vec<(u32, fakeflac_core::TrackAnalysis)> {
    let mut v: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone { index, analysis } => Some((*index, analysis.clone())),
            _ => None,
        })
        .collect();
    v.sort_by_key(|(i, _)| *i);
    v
}

fn terminal_count(events: &[ProgressEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                ProgressEvent::Finished { .. } | ProgressEvent::Cancelled { .. }
            )
        })
        .count()
}

// Scenario: Progress is reported per track and globally (@RF-05)
#[test]
fn rf05_progress_per_track_and_globally() {
    let sc = scanner(&["real1.flac", "fake1.flac", "susp1.flac", "real2.wav"]);
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let summary = run(&sc, &an, 3, &CancelToken::new(), &events).expect("ok");
    let ev = events.into_inner().expect("lock");

    assert_eq!(ev.first(), Some(&ProgressEvent::Scanned { total: 4 }));
    for i in 0..4u32 {
        let started = ev
            .iter()
            .position(|e| matches!(e, ProgressEvent::TrackStarted { index, .. } if *index == i))
            .expect("started");
        let done = ev
            .iter()
            .position(|e| matches!(e, ProgressEvent::TrackDone { index, .. } if *index == i))
            .expect("done");
        assert!(started < done, "track {i} started before done");
    }
    assert_eq!(terminal_count(&ev), 1);
    match ev.last() {
        Some(ProgressEvent::Finished { summary: s }) => assert_eq!(s, &summary),
        other => panic!("last event not Finished: {other:?}"),
    }
    assert_eq!(summary.total, 4);
    assert_eq!(summary.done, 4);
    assert!(!summary.cancelled);
    assert_eq!(summary.real, 2);
    assert_eq!(summary.suspicious, 1);
    assert_eq!(summary.fake, 1);
    assert_eq!(summary.errors, 0);
    assert_eq!(summary.root, PathBuf::from("music"));
}

// Index is the position in case-insensitive path order (RF-05, analyze_folder doc)
#[test]
fn rf05_index_follows_case_insensitive_path_order() {
    let sc = scanner(&["b.flac", "A.flac", "c.wav"]);
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    run(&sc, &an, 2, &CancelToken::new(), &events).expect("ok");
    let ev = events.into_inner().expect("lock");
    let expected = ["A.flac", "b.flac", "c.wav"];
    for (i, name) in expected.iter().enumerate() {
        let want = PathBuf::from("music").join(name);
        let started = ev.iter().find_map(|e| match e {
            ProgressEvent::TrackStarted { index, path } if *index as usize == i => Some(path),
            _ => None,
        });
        assert_eq!(started, Some(&want));
    }
    let done = done_by_index(&ev);
    assert_eq!(done.len(), 3);
    for (i, a) in done {
        assert_eq!(
            a.info.path,
            PathBuf::from("music").join(expected[i as usize])
        );
    }
}

// Scenario: Cancel a running analysis (@RF-05)
#[test]
fn rf05_cancel_running_analysis() {
    let names: Vec<String> = (0..200).map(|i| format!("t{i:03}.flac")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let sc = scanner(&refs);
    let an = FakeAnalyzer::new(20);
    let cancel = CancelToken::new();
    let events = Events::default();
    let cancelled_at: Mutex<Option<Instant>> = Mutex::new(None);

    let (result, returned_at) = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            *cancelled_at.lock().expect("lock") = Some(Instant::now());
            cancel.cancel();
        });
        let r = run(&sc, &an, 4, &cancel, &events);
        (r, Instant::now())
    });
    let summary = result.expect("ok");
    let at = cancelled_at.lock().expect("lock").expect("cancel fired");
    assert!(
        returned_at.saturating_duration_since(at) < Duration::from_millis(500),
        "returned too late after cancel"
    );

    let ev = events.into_inner().expect("lock");
    assert_eq!(terminal_count(&ev), 1);
    match ev.last() {
        Some(ProgressEvent::Cancelled { summary: s }) => assert_eq!(s, &summary),
        other => panic!("last event not Cancelled: {other:?}"),
    }
    assert!(summary.cancelled);
    assert_eq!(summary.total, 200);
    let done = ev
        .iter()
        .filter(|e| matches!(e, ProgressEvent::TrackDone { .. }))
        .count();
    let started = ev
        .iter()
        .filter(|e| matches!(e, ProgressEvent::TrackStarted { .. }))
        .count();
    assert!(summary.done < 200);
    assert_eq!(summary.done as usize, done);
    assert_eq!(started, done, "every started track finishes");
}

// Cancel before start (@RF-05)
#[test]
fn rf05_cancel_before_start_starts_nothing() {
    let sc = scanner(&["a.flac", "b.flac", "c.flac"]);
    let an = FakeAnalyzer::new(0);
    let cancel = CancelToken::new();
    cancel.cancel();
    let events = Events::default();
    let summary = run(&sc, &an, 2, &cancel, &events).expect("ok");
    let ev = events.into_inner().expect("lock");
    assert_eq!(ev.first(), Some(&ProgressEvent::Scanned { total: 3 }));
    assert!(!ev
        .iter()
        .any(|e| matches!(e, ProgressEvent::TrackStarted { .. })));
    assert_eq!(terminal_count(&ev), 1);
    assert!(matches!(ev.last(), Some(ProgressEvent::Cancelled { .. })));
    assert!(summary.cancelled);
    assert_eq!(summary.done, 0);
    assert_eq!(summary.total, 3);
    assert_eq!(an.calls.load(Ordering::SeqCst), 0);
}

// Scenario: Only one analysis at a time (@RF-05)
#[test]
fn rf05_only_one_analysis_at_a_time() {
    let slot = AnalysisSlot::new();
    let first = slot.begin().expect("first");
    let second = slot.begin().expect_err("second must fail");
    assert_eq!(second.code, AppErrorCode::AnalysisRunning);
    drop(first);
    assert!(slot.begin().is_ok());
}

#[test]
fn rf05_slot_cancel_cancels_active_run() {
    let slot = AnalysisSlot::new();
    let run = slot.begin().expect("begin");
    assert!(!run.token().is_cancelled());
    slot.cancel();
    assert!(run.token().is_cancelled());
}

#[test]
fn rf05_slot_cancel_without_run_is_noop_and_does_not_precancel() {
    let slot = AnalysisSlot::new();
    slot.cancel();
    let run = slot.begin().expect("begin");
    assert!(!run.token().is_cancelled());
}

// Scenario: Folder without supported audio (@RF-02)
#[test]
fn rf02_no_audio_found_is_error_without_events() {
    let sc = FakeScanner(Ok(vec![]));
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let err = run(&sc, &an, 2, &CancelToken::new(), &events).expect_err("error");
    assert_eq!(err.code, AppErrorCode::NoAudioFound);
    assert!(events.into_inner().expect("lock").is_empty());
}

#[test]
fn rf02_folder_unreadable_is_propagated_without_events() {
    let sc = FakeScanner(Err(AppError::new(AppErrorCode::FolderUnreadable, "nope")));
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let err = run(&sc, &an, 2, &CancelToken::new(), &events).expect_err("error");
    assert_eq!(err.code, AppErrorCode::FolderUnreadable);
    assert!(events.into_inner().expect("lock").is_empty());
}

// Scenario: A corrupt file does not stop the analysis (@RNF-04)
#[test]
fn rnf04_corrupt_file_does_not_stop_analysis() {
    // Path order: a_real, corrupt, z_real (the fake recognises "corrupt*").
    let sc = scanner(&["a_real.flac", "corrupt.flac", "z_real.flac"]);
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let summary = run(&sc, &an, 2, &CancelToken::new(), &events).expect("ok");
    let ev = events.into_inner().expect("lock");
    let done = done_by_index(&ev);
    assert_eq!(done.len(), 3);
    assert_eq!(done[1].1.verdict, Verdict::Error);
    assert!(matches!(
        done[1].1.error,
        Some(TrackError::CorruptStream(_))
    ));
    assert_eq!(done[0].1.verdict, Verdict::Real);
    assert_eq!(done[2].1.verdict, Verdict::Real);
    assert!(matches!(ev.last(), Some(ProgressEvent::Finished { .. })));
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.real, 2);
    assert_eq!(summary.done, 3);
    assert!(!summary.cancelled);
}

// Workers: 0 is treated as 1; the result does not depend on the worker count
#[test]
fn workers_zero_completes() {
    let sc = scanner(&["a.flac", "b.flac"]);
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let summary = run(&sc, &an, 0, &CancelToken::new(), &events).expect("ok");
    assert_eq!(summary.done, 2);
}

#[test]
fn workers_count_does_not_change_the_analyses() {
    let names: Vec<String> = (0..12)
        .map(|i| match i % 3 {
            0 => format!("real{i:02}.flac"),
            1 => format!("fake{i:02}.flac"),
            _ => format!("susp{i:02}.wav"),
        })
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let sc = scanner(&refs);
    let mut results = Vec::new();
    for workers in [1usize, 8] {
        let an = FakeAnalyzer::new(0);
        let events = Events::default();
        run(&sc, &an, workers, &CancelToken::new(), &events).expect("ok");
        results.push(done_by_index(&events.into_inner().expect("lock")));
    }
    assert_eq!(results[0].len(), 12);
    assert_eq!(results[0], results[1]);
}

// Serialization matches specs/contracts/commands.md
#[test]
fn contract_serialization() {
    let scanned = serde_json::to_string(&ProgressEvent::Scanned { total: 4 }).expect("json");
    assert_eq!(scanned, r#"{"kind":"scanned","total":4}"#);

    let started = serde_json::to_value(ProgressEvent::TrackStarted {
        index: 1,
        path: PathBuf::from("a.flac"),
    })
    .expect("json");
    assert_eq!(started["kind"], "trackStarted");
    assert_eq!(started["index"], 1);

    let summary = AnalysisSummary {
        root: PathBuf::from("music"),
        total: 1,
        done: 1,
        real: 1,
        suspicious: 0,
        fake: 0,
        errors: 0,
        cancelled: false,
        elapsed_ms: 5,
    };
    let v = serde_json::to_value(&summary).expect("json");
    assert_eq!(v["elapsedMs"], 5);
    assert_eq!(v["cancelled"], false);
    let fin = serde_json::to_value(ProgressEvent::Finished { summary }).expect("json");
    assert_eq!(fin["kind"], "finished");
    assert_eq!(fin["summary"]["elapsedMs"], 5);

    assert_eq!(
        serde_json::to_string(&AppErrorCode::NoAudioFound).expect("json"),
        "\"NO_AUDIO_FOUND\""
    );
    assert_eq!(
        serde_json::to_string(&AppErrorCode::AnalysisRunning).expect("json"),
        "\"ANALYSIS_RUNNING\""
    );
}

// Sanity: the fake spectra yield the intended verdicts under the v1 rules (detect_rules.rs V6/V3/V4)
#[test]
fn fake_spectra_yield_intended_verdicts() {
    use fakeflac_core::detect::detect;
    let p = DetectionParams::default();
    let flat = spectrum(|_| 0.0);
    let cliff16 = spectrum(|f| if f < 16_000.0 { 0.0 } else { -100.0 });
    let cliff209 = spectrum(|f| if f < 20_900.0 { 0.0 } else { -100.0 });
    assert_eq!(detect(Some(&flat), &p).verdict, Verdict::Real);
    assert_eq!(detect(Some(&cliff16), &p).verdict, Verdict::Fake);
    assert_eq!(detect(Some(&cliff209), &p).verdict, Verdict::Suspicious);
}

fn count_started(events: &[ProgressEvent], i: u32) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ProgressEvent::TrackStarted { index, .. } if *index == i))
        .count()
}

fn count_done(events: &[ProgressEvent], i: u32) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ProgressEvent::TrackDone { index, .. } if *index == i))
        .count()
}

// Scenario: Progress is reported per track and globally (@RF-05): exactly one
// trackStarted and one trackDone per file, and 2 + 2 * total events in all.
#[test]
fn rf05_exactly_one_started_and_done_per_track() {
    let sc = scanner(&["real1.flac", "fake1.flac", "susp1.flac", "real2.wav"]);
    let an = FakeAnalyzer::new(1);
    let events = Events::default();
    run(&sc, &an, 3, &CancelToken::new(), &events).expect("ok");
    let ev = events.into_inner().expect("lock");
    for i in 0..4u32 {
        assert_eq!(count_started(&ev, i), 1, "trackStarted for {i}");
        assert_eq!(count_done(&ev, i), 1, "trackDone for {i}");
    }
    assert_eq!(ev.len(), 2 + 2 * 4);
}

// Contract (`specs/contracts/commands.md`): only `trackDone` may arrive out of index
// order, so `trackStarted` events arrive in index order even with several workers.
#[test]
fn rf05_track_started_in_index_order() {
    let names: Vec<String> = (0..64).map(|i| format!("t{i:02}.flac")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let sc = scanner(&refs);
    let an = FakeAnalyzer::new(0);
    // The race is between taking an index and reporting it: repeat to make it show up.
    for _ in 0..50 {
        let events = Events::default();
        run(&sc, &an, 8, &CancelToken::new(), &events).expect("ok");
        let started: Vec<u32> = events
            .into_inner()
            .expect("lock")
            .iter()
            .filter_map(|e| match e {
                ProgressEvent::TrackStarted { index, .. } => Some(*index),
                _ => None,
            })
            .collect();
        assert_eq!(started, (0..64).collect::<Vec<u32>>());
    }
}

// Scenario: A corrupt file does not stop the analysis (@RNF-04), ADR-0013: a panic in the
// decoder of one track becomes an error track and the run still finishes.
#[test]
fn rnf04_decoder_panic_does_not_stop_analysis() {
    let sc = scanner(&["a_real.flac", "panic.flac", "z_real.flac"]);
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let summary = run(&sc, &an, 2, &CancelToken::new(), &events).expect("ok");
    let ev = events.into_inner().expect("lock");

    let done = done_by_index(&ev);
    assert_eq!(done.len(), 3);
    assert_eq!(done[1].1.verdict, Verdict::Error);
    assert!(
        matches!(done[1].1.error, Some(TrackError::CorruptStream(_))),
        "{:?}",
        done[1].1.error
    );
    assert_eq!(done[0].1.verdict, Verdict::Real);
    assert_eq!(done[2].1.verdict, Verdict::Real);
    assert!(matches!(ev.last(), Some(ProgressEvent::Finished { .. })));
    assert_eq!(summary.done, 3);
    assert_eq!(summary.errors, 1);
}

// Scenario: Analysing a file instead of a folder (@RF-01): the scanner's NOT_A_FOLDER is
// propagated before any event, like the other pre-run errors.
#[test]
fn rf01_not_a_folder_is_propagated_without_events() {
    let sc = FakeScanner(Err(AppError::new(AppErrorCode::NotAFolder, "a file")));
    let an = FakeAnalyzer::new(0);
    let events = Events::default();
    let err = run(&sc, &an, 2, &CancelToken::new(), &events).expect_err("error");
    assert_eq!(err.code, AppErrorCode::NotAFolder);
    assert!(events.into_inner().expect("lock").is_empty());
}

#[test]
fn not_a_folder_serializes_as_in_the_contract() {
    let json = serde_json::to_string(&AppErrorCode::NotAFolder).expect("serialize");
    assert_eq!(json, "\"NOT_A_FOLDER\"");
}

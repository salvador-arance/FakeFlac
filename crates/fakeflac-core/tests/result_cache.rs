//! Use-case layer of `specs/features/result-cache.feature` (@RF-11, section "Use case"):
//! `analyze_folder_cached`, `ResultCache`, the `ResultStore` port and `is_cacheable`, with a fake
//! scanner, the decoder spy and the in-memory store fake (`tests/common/cache.rs`). Spec:
//! `specs/contracts/result-cache.md` sections 2.2, 3, 7 and 8. No real audio, no I/O.
//!
//! Readings chosen where the scenarios are open (reported in the test-stage report):
//! - "the store fake holds a result" is a hand-built analysis whose verdict differs from what
//!   the decoder spy would give, and whose `info.path` / `info.folder` name another folder, so a
//!   reused result can be told from a fresh one;
//! - "the store fake returns for it an analysis whose <field> is <value>" forces that analysis
//!   for the path, whatever the key;
//! - scenarios that do not name a worker count use 2 workers.
//!
//! Protected after the `test(RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "common/cache.rs"]
mod cache;

use cache::{
    assert_started_then_done, done_by_index, done_of, file_name_of, Behaviour, DecoderSpy, Op,
    Recorder, StoreFake,
};
use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_core::{
    analyze_folder, analyze_folder_cached, is_cacheable, summarize_album, AnalysisSummary,
    AppError, AudioCodec, AudioFormat, CacheStatus, CancelToken, CutoffEstimate, DetectionParams,
    ProgressEvent, ResultCache, SuspicionReason, TrackAnalysis, TrackError, TrackInfo, Verdict,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

struct FakeScanner(Vec<FileEntry>);

impl FileScanner for FakeScanner {
    fn scan(&self, _root: &Path) -> Result<Vec<FileEntry>, AppError> {
        Ok(self.0.clone())
    }
}

const MTIME: i64 = 1_700_000_000;

fn entry_in(folder: &str, name: &str) -> FileEntry {
    let folder = PathBuf::from(folder);
    FileEntry {
        path: folder.join(name),
        folder,
        format: if name.ends_with(".wav") {
            AudioFormat::Wav
        } else {
            AudioFormat::Flac
        },
        size_bytes: 1000,
        modified_unix: MTIME,
    }
}

fn entry(name: &str) -> FileEntry {
    entry_in("music", name)
}

fn entries(names: &[&str]) -> Vec<FileEntry> {
    names.iter().map(|n| entry(n)).collect()
}

fn params() -> DetectionParams {
    DetectionParams::default()
}

/// A hand-built analysis of `e`: `verdict`, a finite cutoff, `info.path` / `info.folder` in
/// another folder ("elsewhere"), the size, mtime and format of `e`, and `algorithm_version`.
fn stored_for(e: &FileEntry, verdict: Verdict, algorithm_version: u32) -> TrackAnalysis {
    let elsewhere = PathBuf::from("elsewhere");
    TrackAnalysis {
        info: TrackInfo {
            path: elsewhere.join(e.path.file_name().unwrap()),
            folder: elsewhere,
            format: e.format,
            codec: Some(AudioCodec::Flac),
            sample_rate: 44_100,
            bit_depth: Some(16),
            channels: 2,
            declared_bitrate_kbps: None,
            bitrate_mode: None,
            duration_ms: 123_456,
            size_bytes: e.size_bytes,
            modified_unix: e.modified_unix,
        },
        verdict,
        cutoff: Some(CutoffEstimate {
            frequency_hz: 16_000.4,
            steepness_db_per_khz: 42.06,
            noise_floor_db: -101.94,
        }),
        estimated_source_bitrate_kbps: None,
        reasons: if verdict == Verdict::Suspicious {
            vec![SuspicionReason::GradualRolloff]
        } else {
            vec![]
        },
        error: None,
        algorithm_version,
    }
}

/// `stored` as the run must report it: `info.path` and `info.folder` of the scan entry.
fn as_reported(stored: &TrackAnalysis, e: &FileEntry) -> TrackAnalysis {
    let mut a = stored.clone();
    a.info.path = e.path.clone();
    a.info.folder = e.folder.clone();
    a
}

fn run_cached(
    scanner: &FakeScanner,
    spy: &DecoderSpy,
    cache: ResultCache<'_>,
    p: &DetectionParams,
    workers: usize,
    cancel: &CancelToken,
    rec: &Recorder,
) -> AnalysisSummary {
    analyze_folder_cached(
        scanner,
        spy,
        cache,
        p,
        Path::new("music"),
        workers,
        cancel,
        &|e| rec.push(e),
    )
    .expect("run ok")
}

fn run_with_store(
    scanner: &FakeScanner,
    spy: &DecoderSpy,
    store: &StoreFake,
    workers: usize,
) -> (AnalysisSummary, Vec<ProgressEvent>) {
    let rec = Recorder::new();
    let summary = run_cached(
        scanner,
        spy,
        ResultCache::Store(store),
        &params(),
        workers,
        &CancelToken::new(),
        &rec,
    );
    (summary, rec.events())
}

fn finished_done(events: &[ProgressEvent]) -> Option<u32> {
    match events.last() {
        Some(ProgressEvent::Finished { summary }) => Some(summary.done),
        _ => None,
    }
}

// @RF-11 Scenario: A hit is not analysed and is reported like an analysed track
#[test]
fn rf11_a_hit_is_not_analysed_and_is_reported_like_an_analysed_track() {
    let es = entries(&["a.flac", "b.flac", "c.flac", "d.flac"]);
    let scanner = FakeScanner(es.clone());
    let p = params();
    let store = StoreFake::new();
    let b = stored_for(&es[1], Verdict::Fake, p.algorithm_version);
    let d = stored_for(&es[3], Verdict::Suspicious, p.algorithm_version);
    store.hold(&es[1], p.algorithm_version, b.clone());
    store.hold(&es[3], p.algorithm_version, d.clone());
    let spy = DecoderSpy::new();

    let (summary, events) = run_with_store(&scanner, &spy, &store, 2);

    assert_eq!(spy.names(), ["a.flac", "c.flac"]);
    assert_started_then_done(&events, 4);
    assert_eq!(finished_done(&events), Some(4));
    assert_eq!(done_of(&events, "b.flac"), as_reported(&b, &es[1]));
    assert_eq!(done_of(&events, "d.flac"), as_reported(&d, &es[3]));
    assert_eq!(summary.from_cache, 2);
    assert_eq!(summary.cache, CacheStatus::Ok);

    let puts = store.puts();
    let mut put_names: Vec<String> = puts.iter().map(|c| file_name_of(&c.path)).collect();
    put_names.sort();
    assert_eq!(put_names, ["a.flac", "c.flac"]);
    for put in &puts {
        let name = file_name_of(&put.path);
        assert_eq!(
            put.analysis.as_ref(),
            Some(&done_of(&events, &name)),
            "the put of {name} carries the analysis of its trackDone"
        );
    }
}

// @RF-11 Scenario: Hits count in their album like analysed tracks
#[test]
fn rf11_hits_count_in_their_album_like_analysed_tracks() {
    let folder = "Lib/Album A";
    let es: Vec<FileEntry> = ["01.flac", "02.flac", "03.flac"]
        .iter()
        .map(|n| entry_in(folder, n))
        .collect();
    let scanner = FakeScanner(es.clone());
    let p = params();
    let store = StoreFake::new();
    let mut cached = Vec::new();
    for (e, v) in es
        .iter()
        .zip([Verdict::Real, Verdict::Fake, Verdict::Suspicious])
    {
        let mut a = stored_for(e, v, p.algorithm_version);
        a.info.path = e.path.clone();
        a.info.folder = e.folder.clone();
        store.hold(e, p.algorithm_version, a.clone());
        cached.push(a);
    }
    let spy = DecoderSpy::new();
    let (summary, events) = run_with_store(&scanner, &spy, &store, 2);

    assert_eq!(spy.count(), 0);
    assert_eq!(summary.from_cache, 3);
    let updates: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone { album, .. } => Some(album.clone()),
            _ => None,
        })
        .collect();
    let totals: Vec<u32> = updates.iter().map(|u| u.summary.total).collect();
    assert_eq!(totals, [1, 2, 3], "album totals in channel order");
    assert_eq!(
        Some(updates[2].summary.clone()),
        summarize_album(Path::new(folder), &cached)
    );
}

// @RF-11 Scenario Outline: A returned analysis that does not match the entry is a miss
#[test]
fn rf11_a_returned_analysis_that_does_not_match_the_entry_is_a_miss() {
    type Change = fn(&mut TrackAnalysis);
    let rows: [(&str, Change); 4] = [
        ("algorithm_version 6", |a| a.algorithm_version = 6),
        ("info.size_bytes 999", |a| a.info.size_bytes = 999),
        ("info.modified_unix 1699999999", |a| {
            a.info.modified_unix = 1_699_999_999
        }),
        ("info.format wav", |a| a.info.format = AudioFormat::Wav),
    ];
    for (row, change) in rows {
        let e = entry("a.flac");
        assert_eq!((e.size_bytes, e.modified_unix), (1000, MTIME));
        let scanner = FakeScanner(vec![e.clone()]);
        let mut p = params();
        p.algorithm_version = 7;
        let mut returned = stored_for(&e, Verdict::Fake, 7);
        change(&mut returned);
        let store = StoreFake::new();
        store.force(&e.path, returned);
        let spy = DecoderSpy::new();
        let rec = Recorder::new();
        let summary = run_cached(
            &scanner,
            &spy,
            ResultCache::Store(&store),
            &p,
            2,
            &CancelToken::new(),
            &rec,
        );
        assert_eq!(spy.count_of("a.flac"), 1, "{row}: analysed once");
        assert_eq!(spy.count(), 1, "{row}");
        assert_eq!(summary.from_cache, 0, "{row}");
        assert_eq!(summary.cache, CacheStatus::Ok, "{row}");
        assert_eq!(
            done_of(&rec.events(), "a.flac").verdict,
            Verdict::Real,
            "{row}: the fresh analysis is reported"
        );
    }
}

// --- is_cacheable -----------------------------------------------------------------------------

fn analysis(verdict: Verdict, cutoff: Option<(f32, f32, f32)>) -> TrackAnalysis {
    let e = entry("x.flac");
    let mut a = stored_for(&e, verdict, 7);
    a.info.path = e.path;
    a.info.folder = e.folder;
    a.cutoff = cutoff.map(|(f, s, n)| CutoffEstimate {
        frequency_hz: f,
        steepness_db_per_khz: s,
        noise_floor_db: n,
    });
    a
}

fn error_analysis(error: TrackError) -> TrackAnalysis {
    let mut a = analysis(Verdict::Error, None);
    a.reasons = vec![];
    a.error = Some(error);
    a
}

// @RF-11 Scenario Outline: Which analyses are cacheable
#[test]
fn rf11_which_analyses_are_cacheable() {
    let finite = Some((16_000.0, 40.0, -90.0));
    let mut streaming = analysis(Verdict::Suspicious, Some((20_000.0, 80.0, -95.0)));
    streaming.reasons = vec![SuspicionReason::StreamingSampleRate];
    let rows: Vec<(&str, TrackAnalysis, bool)> = vec![
        (
            "verdict real and a finite cutoff",
            analysis(Verdict::Real, finite),
            true,
        ),
        (
            "verdict real and no cutoff",
            analysis(Verdict::Real, None),
            true,
        ),
        ("verdict fake", analysis(Verdict::Fake, finite), true),
        (
            "verdict suspicious with reason streamingSampleRate",
            streaming,
            true,
        ),
        (
            "the error tooShort",
            error_analysis(TrackError::TooShort),
            true,
        ),
        (
            "the error unsupportedFormat",
            error_analysis(TrackError::UnsupportedFormat("codec".into())),
            true,
        ),
        (
            "the error unreadable",
            error_analysis(TrackError::Unreadable("locked".into())),
            false,
        ),
        (
            "the error corruptStream",
            error_analysis(TrackError::CorruptStream("bad frame".into())),
            false,
        ),
        (
            "the error corruptStream of a caught decoder panic",
            error_analysis(TrackError::CorruptStream(
                "the decoder failed unexpectedly".into(),
            )),
            false,
        ),
        (
            "verdict real and a cutoff whose noise floor is NaN",
            analysis(Verdict::Real, Some((16_000.0, 40.0, f32::NAN))),
            false,
        ),
        (
            "verdict suspicious and a cutoff whose steepness is +infinity",
            analysis(Verdict::Suspicious, Some((16_000.0, f32::INFINITY, -90.0))),
            false,
        ),
        (
            "verdict fake and a cutoff whose frequency is -infinity",
            analysis(Verdict::Fake, Some((f32::NEG_INFINITY, 40.0, -90.0))),
            false,
        ),
    ];
    for (row, a, cacheable) in rows {
        assert_eq!(is_cacheable(&a), cacheable, "{row}");
    }
}

// --- Which results are stored ---------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Outcome {
    Verdict(Verdict),
    Error(&'static str),
}

fn outcome_of(a: &TrackAnalysis) -> Outcome {
    match &a.error {
        None => Outcome::Verdict(a.verdict),
        Some(TrackError::TooShort) => Outcome::Error("tooShort"),
        Some(TrackError::UnsupportedFormat(_)) => Outcome::Error("unsupportedFormat"),
        Some(TrackError::Unreadable(_)) => Outcome::Error("unreadable"),
        Some(TrackError::CorruptStream(_)) => Outcome::Error("corruptStream"),
    }
}

// @RF-11 Scenario Outline: Which results are stored
#[test]
fn rf11_which_results_are_stored() {
    let rows: Vec<(&str, Behaviour, Outcome, usize)> = vec![
        (
            "returns the flat spectrum",
            Behaviour::Flat,
            Outcome::Verdict(Verdict::Real),
            1,
        ),
        (
            "returns the cliff at 16 kHz",
            Behaviour::Cliff16k,
            Outcome::Verdict(Verdict::Fake),
            1,
        ),
        (
            "returns the cliff at 20.5 kHz",
            Behaviour::Cliff20k5,
            Outcome::Verdict(Verdict::Suspicious),
            1,
        ),
        (
            "returns a probe lasting 500 ms",
            Behaviour::Short,
            Outcome::Error("tooShort"),
            1,
        ),
        (
            "returns the error UnsupportedFormat",
            Behaviour::Fail(TrackError::UnsupportedFormat("codec".into())),
            Outcome::Error("unsupportedFormat"),
            1,
        ),
        (
            "returns the error Unreadable",
            Behaviour::Fail(TrackError::Unreadable("locked".into())),
            Outcome::Error("unreadable"),
            0,
        ),
        (
            "returns the error CorruptStream",
            Behaviour::Fail(TrackError::CorruptStream("bad frame".into())),
            Outcome::Error("corruptStream"),
            0,
        ),
        (
            "panics",
            Behaviour::Panic,
            Outcome::Error("corruptStream"),
            0,
        ),
    ];
    for (row, behaviour, outcome, puts) in rows {
        let e = entry("x.flac");
        let scanner = FakeScanner(vec![e.clone()]);
        let spy = DecoderSpy::new();
        spy.on("x.flac", behaviour);
        let store = StoreFake::new();
        let (_, events) = run_with_store(&scanner, &spy, &store, 2);
        let done = done_of(&events, "x.flac");
        assert_eq!(outcome_of(&done), outcome, "{row}: outcome");
        let x_puts: Vec<_> = store
            .puts()
            .into_iter()
            .filter(|c| c.path == e.path)
            .collect();
        assert_eq!(x_puts.len(), puts, "{row}: puts");
        for put in x_puts {
            assert_eq!(put.analysis.as_ref(), Some(&done), "{row}: put = trackDone");
        }
    }
}

// @RF-11 Scenario: An entry without an mtime is never looked up or stored
#[test]
fn rf11_an_entry_without_an_mtime_is_never_looked_up_or_stored() {
    let mut e = entry("a.flac");
    e.modified_unix = 0;
    let scanner = FakeScanner(vec![e.clone()]);
    let spy = DecoderSpy::new();
    let store = StoreFake::new();
    let (summary, _) = run_with_store(&scanner, &spy, &store, 2);
    assert!(
        store.calls_for(&e.path).is_empty(),
        "no get and no put for a.flac: {:?}",
        store.log()
    );
    assert_eq!(spy.count_of("a.flac"), 1);
    assert_eq!(summary.cache, CacheStatus::Ok);
}

// @RF-11 @RNF-04 Scenario: The first store failure stops the use of the store for the rest of
// the run (bound F3 of result-cache.md section 7, exact with one worker)
#[test]
fn rf11_the_first_store_failure_stops_the_use_of_the_store() {
    let names: Vec<String> = (0..10).map(|i| format!("t{i:02}.flac")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let scanner = FakeScanner(entries(&refs));
    let store = StoreFake::failing_on_call(3);
    let spy = DecoderSpy::new();
    let (summary, events) = run_with_store(&scanner, &spy, &store, 1);

    assert_eq!(store.log().len(), 3, "store calls: {:?}", store.log());
    assert_eq!(spy.count(), 10);
    let done = done_by_index(&events);
    assert_eq!(done.len(), 10);
    for (i, a) in &done {
        assert_eq!(a.verdict, Verdict::Real, "track {i}");
    }
    assert_eq!(summary.cache, CacheStatus::Degraded);
    assert_eq!(finished_done(&events), Some(10));
}

// @RF-11 @RNF-04 Scenario: A panic inside the store is a store failure, not a crash
#[test]
fn rf11_a_panic_inside_the_store_is_a_store_failure() {
    let scanner = FakeScanner(entries(&["a.flac", "b.flac", "c.flac"]));
    let store = StoreFake::panicking_on_get(1);
    let spy = DecoderSpy::new();
    let (summary, events) = run_with_store(&scanner, &spy, &store, 2);
    assert_eq!(finished_done(&events), Some(3));
    assert_eq!(summary.cache, CacheStatus::Degraded);
    assert_eq!(summary.done, 3);
}

// @RF-11 Scenario Outline: The cache status of a run
#[test]
fn rf11_the_cache_status_of_a_run() {
    let store = StoreFake::new();
    let rows: [(&str, ResultCache<'_>, CacheStatus); 3] = [
        ("ResultCache::Off", ResultCache::Off, CacheStatus::Off),
        (
            "ResultCache::Unavailable",
            ResultCache::Unavailable,
            CacheStatus::Unavailable,
        ),
        (
            "ResultCache::Store of an empty store fake",
            ResultCache::Store(&store),
            CacheStatus::Ok,
        ),
    ];
    for (row, cache, status) in rows {
        let scanner = FakeScanner(entries(&["a.flac", "b.flac"]));
        let spy = DecoderSpy::new();
        let rec = Recorder::new();
        let summary = run_cached(
            &scanner,
            &spy,
            cache,
            &params(),
            2,
            &CancelToken::new(),
            &rec,
        );
        assert_eq!(summary.cache, status, "{row}");
        assert_eq!(summary.from_cache, 0, "{row}");
        assert_eq!(spy.count(), 2, "{row}");
    }
}

/// The events with `elapsed_ms` set to 0 (it is a time, not a result).
fn without_elapsed(events: Vec<ProgressEvent>) -> Vec<ProgressEvent> {
    events
        .into_iter()
        .map(|e| match e {
            ProgressEvent::Finished { mut summary } => {
                summary.elapsed_ms = 0;
                ProgressEvent::Finished { summary }
            }
            ProgressEvent::Cancelled { mut summary } => {
                summary.elapsed_ms = 0;
                ProgressEvent::Cancelled { summary }
            }
            other => other,
        })
        .collect()
}

// @RF-11 Scenario: analyze_folder keeps working without a cache
#[test]
fn rf11_analyze_folder_keeps_working_without_a_cache() {
    // One worker, so both runs send their events in the same order.
    let scanner = FakeScanner(entries(&["a.flac", "fake.flac"]));
    let spy = DecoderSpy::new();
    let plain = Recorder::new();
    let mut s1 = analyze_folder(
        &scanner,
        &spy,
        &params(),
        Path::new("music"),
        1,
        &CancelToken::new(),
        &|e| plain.push(e),
    )
    .expect("run ok");
    let cached = Recorder::new();
    let mut s2 = run_cached(
        &scanner,
        &spy,
        ResultCache::Off,
        &params(),
        1,
        &CancelToken::new(),
        &cached,
    );
    assert_eq!(
        without_elapsed(plain.events()),
        without_elapsed(cached.events())
    );
    s1.elapsed_ms = 0;
    s2.elapsed_ms = 0;
    assert_eq!(s1, s2);
    assert_eq!(s1.from_cache, 0);
    assert_eq!(s1.cache, CacheStatus::Off);
}

// @RF-11 Scenario: Tracks finished before a cancel are stored (and acceptance C7: a rerun
// analyses exactly the tracks that were not reported)
#[test]
fn rf11_tracks_finished_before_a_cancel_are_stored() {
    let names: Vec<String> = (0..50).map(|i| format!("t{i:02}.flac")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let es = entries(&refs);
    let scanner = FakeScanner(es.clone());
    let store = StoreFake::new();
    let spy = DecoderSpy::with_delay(5);
    let cancel = CancelToken::new();
    let rec = Recorder::new();
    let dones = AtomicU32::new(0);
    let summary = analyze_folder_cached(
        &scanner,
        &spy,
        ResultCache::Store(&store),
        &params(),
        Path::new("music"),
        2,
        &cancel,
        &|e| {
            let is_done = matches!(e, ProgressEvent::TrackDone { .. });
            rec.push(e);
            if is_done && dones.fetch_add(1, Ordering::SeqCst) + 1 == 10 {
                cancel.cancel();
            }
        },
    )
    .expect("run ok");
    let stamped = rec.stamped();
    assert!(
        matches!(stamped.last(), Some((_, ProgressEvent::Cancelled { .. }))),
        "the run ends with cancelled"
    );
    assert!(summary.cancelled);

    let started: Vec<u32> = stamped
        .iter()
        .filter_map(|(_, e)| match e {
            ProgressEvent::TrackStarted { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    let done: Vec<(usize, u32, PathBuf)> = stamped
        .iter()
        .filter_map(|(t, e)| match e {
            ProgressEvent::TrackDone {
                index, analysis, ..
            } => Some((*t, *index, analysis.info.path.clone())),
            _ => None,
        })
        .collect();
    assert!(started.len() < 50, "the cancel stopped the run early");
    for i in &started {
        assert!(
            done.iter().any(|(_, d, _)| d == i),
            "track {i} has a trackStarted and a trackDone"
        );
    }
    let puts = store.puts();
    for (t, i, path) in &done {
        assert!(
            puts.iter().any(|c| &c.path == path && c.ok && c.stamp < *t),
            "track {i} was put in the store before its trackDone"
        );
    }
    for (i, e) in es.iter().enumerate() {
        if !started.contains(&(i as u32)) {
            assert!(
                store.calls_for(&e.path).is_empty(),
                "no store call for {} (no trackStarted)",
                e.path.display()
            );
        }
    }

    // C7: a rerun makes exactly total - k analyzer calls (every reported track is cacheable).
    let k = done.len();
    let spy2 = DecoderSpy::new();
    let (s2, _) = run_with_store(&scanner, &spy2, &store, 2);
    assert_eq!(spy2.count(), 50 - k);
    assert_eq!(s2.from_cache as usize, k);
    assert!(
        store.log().iter().all(|c| c.op == Op::Get || c.ok),
        "no failed put"
    );
}

// Contract version 7: the core `CacheStatus` serialises camelCase (result-cache.md 8.3).
#[test]
fn rf11_cache_status_serialises_camel_case() {
    for (s, json) in [
        (CacheStatus::Off, "\"off\""),
        (CacheStatus::Ok, "\"ok\""),
        (CacheStatus::Degraded, "\"degraded\""),
        (CacheStatus::Unavailable, "\"unavailable\""),
    ] {
        assert_eq!(serde_json::to_string(&s).expect("json"), json);
    }
}

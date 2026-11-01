//! Wiring of the analysis to the real adapters for MP3 and M4A files (RF-02, RF-14, RNF-04, RNF-05),
//! derived from `specs/features/decode-lossy.feature`. Real temp folders with synthetic lossy
//! fixtures written by the bitstream writers of `specs/detection/lossy-fixtures.md` (included by
//! path from `crates/fakeflac-audio/tests/common/lossy.rs`, ADR-0036).
//!
//! The scenarios whose assertions need the detection v5 verdicts (the table of a mixed folder, the
//! Unicode names with verdict "real") belong to RF-15.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../crates/fakeflac-audio/tests/common/lossy.rs"]
mod lossy;
#[path = "common/opus_support.rs"]
mod opus_support;

use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, AppErrorCode, ProgressEvent, Verdict};
use fakeflac_lib::run::run_analysis;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("run_analysis_lossy_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

/// Writes the fixture `name` of lossy-fixtures.md section 2 as `dir/file`.
fn put(dir: &Path, name: &str, file: &str) {
    let bytes = lossy::fixture_bytes(name).unwrap_or_else(|| panic!("unknown fixture {name}"));
    std::fs::write(dir.join(file), bytes).unwrap();
}

type Collected = (
    Result<dto::AnalysisSummary, dto::AppError>,
    Vec<ProgressEvent>,
);

fn collect(root: &Path) -> Collected {
    let events = Mutex::new(Vec::new());
    let result = run_analysis(&AnalysisSlot::new(), root, &|e| {
        events.lock().unwrap().push(e);
        true
    });
    (result, events.into_inner().unwrap())
}

fn analyses(events: &[ProgressEvent]) -> Vec<&dto::TrackAnalysis> {
    events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone { analysis, .. } => Some(analysis),
            _ => None,
        })
        .collect()
}

// @RF-02 @RF-14 Scenario: A folder with only Opus files has no supported audio yet
#[test]
fn rf14_a_folder_with_only_other_ogg_files_has_no_supported_audio() {
    let dir = temp_dir("opus_only");
    std::fs::write(dir.join("a.ogg"), b"OggS dummy").unwrap();
    let (result, events) = collect(&dir);
    assert_eq!(result.unwrap_err().code, AppErrorCode::NoAudioFound);
    assert!(events.is_empty());
}

// @RNF-04 Scenario: A corrupt lossy file does not stop the analysis
#[test]
fn rnf04_a_corrupt_lossy_file_does_not_stop_the_analysis() {
    let dir = temp_dir("mixed");
    put(&dir, "mp3_320_lp20k5", "mp3_320_lp20k5.mp3");
    put(&dir, "mp3_corrupt", "mp3_corrupt.mp3");
    put(&dir, "m4a_aac_128_lp16k", "m4a_aac_128_lp16k.m4a");
    let (result, events) = collect(&dir);
    let summary = result.expect("run ok");
    assert_eq!(summary.total, 3);
    assert_eq!(summary.errors, 1);

    // "the last event is finished with errors 1"
    match events.last() {
        Some(ProgressEvent::Finished { summary }) => assert_eq!(summary.errors, 1),
        other => panic!("expected finished, got {other:?}"),
    }
    let rows = analyses(&events);
    assert_eq!(rows.len(), 3);
    // "mp3_corrupt.mp3 has state Error with the reason Corrupt stream"
    let bad = rows
        .iter()
        .find(|a| a.info.path.ends_with("mp3_corrupt.mp3"))
        .expect("corrupt row");
    assert_eq!(bad.verdict, Verdict::Error);
    assert!(
        matches!(bad.error, Some(dto::TrackError::CorruptStream { .. })),
        "{:?}",
        bad.error
    );
    // "the other two files are analysed normally"
    for (file, format) in [
        ("mp3_320_lp20k5.mp3", dto::AudioFormat::Mp3),
        ("m4a_aac_128_lp16k.m4a", dto::AudioFormat::M4a),
    ] {
        let row = rows
            .iter()
            .find(|a| a.info.path.ends_with(file))
            .unwrap_or_else(|| panic!("row of {file}"));
        assert_ne!(row.verdict, Verdict::Error, "{file}: {row:?}");
        assert_eq!(row.error, None, "{file}");
        assert_eq!(row.info.format, format, "{file}");
        assert!(row.cutoff.is_some(), "{file}: a cutoff is measured");
        assert_eq!(row.info.sample_rate, 44_100, "{file}");
    }
}

// @RNF-05 Scenario: Analysed lossy files are never modified
#[test]
fn rnf05_analysed_lossy_files_are_never_modified() {
    let dir = temp_dir("readonly");
    put(&dir, "mp3_320_lp20k5", "mp3_320_lp20k5.mp3");
    put(&dir, "m4a_aac_128_lp16k", "m4a_aac_128_lp16k.m4a");
    let snapshot = || {
        ["mp3_320_lp20k5.mp3", "m4a_aac_128_lp16k.m4a"].map(|name| {
            let p = dir.join(name);
            let m = std::fs::metadata(&p).unwrap();
            let mut hasher = DefaultHasher::new();
            std::fs::read(&p).unwrap().hash(&mut hasher);
            (m.len(), m.modified().unwrap(), hasher.finish())
        })
    };
    let before = snapshot();
    let (result, events) = collect(&dir);
    let summary = result.expect("run ok");
    assert_eq!(summary.total, 2, "both lossy files are analysed");
    assert_eq!(analyses(&events).len(), 2);
    assert_eq!(before, snapshot());
}

// @RF-14 @RF-15 @RF-06 @RNF-04 @RNF-05 @RNF-09 opus.feature:
// Opus-only folders, complete table events, corrupt-track isolation, read-only and Unicode paths.
#[test]
fn rf14_opus_analysis_events_and_audio_are_preserved() {
    let dir = temp_dir("opus_mixed").join("Álbum ″Ñandú″");
    std::fs::create_dir_all(&dir).unwrap();
    let good = "01 – Canción ‘x’.opus";
    let bad = "02 – Ópera «Ça va» 🎵.opus";
    for (fixture, name) in [
        ("opus_128_fb", good),
        ("opus_bad_crc", bad),
        ("opus_160_src16k", "fake.opus"),
    ] {
        std::fs::copy(opus_support::opus::fixture(fixture), dir.join(name)).unwrap();
    }
    let snapshot = || {
        [good, bad, "fake.opus"].map(|name| {
            let p = dir.join(name);
            let m = std::fs::metadata(&p).unwrap();
            (m.len(), m.modified().unwrap(), std::fs::read(&p).unwrap())
        })
    };
    let before = snapshot();
    let (summary, events) = collect(&dir);
    let summary = summary.unwrap();
    assert_eq!((summary.total, summary.errors), (3, 1));
    assert_eq!(snapshot(), before);
    assert!(
        matches!(events.last(), Some(ProgressEvent::Finished { summary }) if summary.errors == 1)
    );
    let rows = analyses(&events);
    for a in &rows {
        assert_eq!(a.algorithm_version, 8);
        assert_eq!(a.info.format, dto::AudioFormat::Opus);
        assert_eq!(a.info.codec, Some(dto::AudioCodec::Opus));
    }
    let good_row = rows
        .iter()
        .find(|a| a.info.path == dir.join(good).to_string_lossy())
        .unwrap();
    assert_eq!(good_row.verdict, Verdict::Real);
    assert_eq!(good_row.info.sample_rate, 48000);
    // @RF-14 "Stream information of Opus fixtures": VBR fixture targets allow ±1 kbps.
    // Compare the event against its exact packet-byte/duration formula, then check the target.
    let encoded = opus_support::opus::encoded("opus_128_fb");
    let packet_bytes: usize = encoded.packets.iter().map(Vec::len).sum();
    let declared =
        (8.0 * packet_bytes as f64 * 48000.0 / encoded.samples as f64 / 1000.0).round() as u32;
    assert_eq!(good_row.info.duration_ms, 30000);
    assert!(declared.abs_diff(128) <= 1, "fixture bitrate {declared}");
    assert_eq!(good_row.info.declared_bitrate_kbps, Some(declared));
    // @RF-15 "Verdict of Opus fixtures": a real Opus estimate equals its declared bitrate.
    assert_eq!(good_row.estimated_source_bitrate_kbps, Some(declared));
    let bad_row = rows
        .iter()
        .find(|a| a.info.path == dir.join(bad).to_string_lossy())
        .unwrap();
    assert!(matches!(
        bad_row.error,
        Some(dto::TrackError::CorruptStream { .. })
    ));
    let fake = rows
        .iter()
        .find(|a| a.info.path.ends_with("fake.opus"))
        .unwrap();
    assert_eq!(fake.verdict, Verdict::Fake);
    assert_eq!(fake.estimated_source_bitrate_kbps, None);
    assert!((f64::from(fake.cutoff.as_ref().unwrap().frequency_hz) - 16000.0).abs() <= 200.0);
}

// @RF-02 @RF-14 opus.feature: A folder with only Opus files is analysed.
#[test]
fn rf14_a_folder_with_only_opus_is_analysed() {
    let dir = temp_dir("opus_only_valid");
    std::fs::copy(
        opus_support::opus::fixture("opus_128_fb"),
        dir.join("a.opus"),
    )
    .unwrap();
    let (result, _) = collect(&dir);
    let s = result.unwrap();
    assert_eq!((s.total, s.errors), (1, 0));
}

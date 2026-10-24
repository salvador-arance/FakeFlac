//! Wiring of the analysis to the real adapters for lossy files with detection v5 (RF-14, RF-15,
//! RF-06, RNF-09), derived from `specs/features/decode-lossy.feature`: the scenarios whose
//! assertions need the v5 verdicts and the estimated source bitrate ("A lossy row has the same
//! fields as a lossless row", "A variable bitrate is marked in the table", "Unicode names of lossy
//! files"). `run_analysis_lossy.rs` (RF-14) holds the scenarios that need no verdict.
//!
//! The text of the table cells ("≈128 kbps", "240 kbps VBR", "—") is formatted by the frontend from
//! these `TrackAnalysis` values (`specs/ui/lossy-formats.md`): the frontend author derives it.
//! Values the table needs, per row:
//!   mp3_320_lp17k.mp3      info.format mp3, info.codec mp3, declaredBitrateKbps 320, bitrateMode cbr,
//!                          estimatedSourceBitrateKbps 128, verdict fake, cutoff 17.0 kHz ± 200 Hz
//!   m4a_aac_128_lp16k.m4a  format m4a, codec aac, declaredBitrateKbps 128, bitrateMode null,
//!                          estimatedSourceBitrateKbps 128, verdict real
//!   a WAV file             declaredBitrateKbps, bitrateMode and estimatedSourceBitrateKbps null
//!   mp3_vbr_lp19k5.mp3     declaredBitrateKbps 240, bitrateMode vbr, estimate 240, verdict real
//! Every row carries algorithmVersion 6.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../crates/fakeflac-audio/tests/common/lossy.rs"]
mod lossy;

use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, ProgressEvent, Verdict};
use fakeflac_lib::run::run_analysis;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("run_analysis_lossy_v5_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

/// Writes the fixture `name` of lossy-fixtures.md section 2 as `dir/file`.
fn put(dir: &Path, name: &str, file: &str) {
    let bytes = lossy::fixture_bytes(name).unwrap_or_else(|| panic!("unknown fixture {name}"));
    std::fs::write(dir.join(file), bytes).unwrap();
}

/// A lossless file for the mixed folder: 6 s of mono noise as WAV (the scenario names
/// "full_band.flac"; the table cells it checks do not depend on the signal).
fn put_lossless(dir: &Path, file: &str) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(dir.join(file), spec).expect("wav");
    let mut state = 12_345_u32;
    for _ in 0..(44_100 * 6) {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let s = (state >> 16) as i32 - 32_768;
        w.write_sample((s / 3) as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

fn run(root: &Path) -> (dto::AnalysisSummary, Vec<ProgressEvent>) {
    let events = Mutex::new(Vec::new());
    let summary = run_analysis(&AnalysisSlot::new(), root, &|e| {
        events.lock().unwrap().push(e);
        true
    })
    .expect("run ok");
    (summary, events.into_inner().unwrap())
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

fn row<'a>(rows: &[&'a dto::TrackAnalysis], file: &str) -> &'a dto::TrackAnalysis {
    rows.iter()
        .copied()
        .find(|a| a.info.path.ends_with(file))
        .unwrap_or_else(|| panic!("no row for {file}"))
}

// @RF-14 @RF-06 Scenario: A lossy row has the same fields as a lossless row
#[test]
fn rf15_a_lossy_row_has_the_same_fields_as_a_lossless_row() {
    let dir = temp_dir("mixed").join("Mixed");
    std::fs::create_dir_all(&dir).unwrap();
    put_lossless(&dir, "full_band.wav");
    put(&dir, "mp3_320_lp17k", "mp3_320_lp17k.mp3");
    put(&dir, "m4a_aac_128_lp16k", "m4a_aac_128_lp16k.m4a");
    let (summary, events) = run(&dir);
    assert_eq!(summary.total, 3);

    // "every trackDone event carries a complete TrackAnalysis with algorithmVersion 6"
    let rows = analyses(&events);
    assert_eq!(rows.len(), 3);
    for a in &rows {
        assert_eq!(a.algorithm_version, 6, "{:?}", a.info.path);
        assert_eq!(a.error, None, "{:?}", a.info.path);
        assert!(a.cutoff.is_some(), "{:?}: a cutoff", a.info.path);
        assert!(a.info.sample_rate > 0 && a.info.duration_ms > 0);
    }

    // "mp3_320_lp17k.mp3": format MP3, bitrate 320, real bitrate 128, Fake, cutoff near 17.0 kHz
    let mp3 = row(&rows, "mp3_320_lp17k.mp3");
    assert_eq!(mp3.info.format, dto::AudioFormat::Mp3);
    assert_eq!(mp3.info.codec, Some(dto::AudioCodec::Mp3));
    assert_eq!(mp3.info.declared_bitrate_kbps, Some(320));
    assert_eq!(mp3.info.bitrate_mode, Some(dto::BitrateMode::Cbr));
    assert_eq!(mp3.estimated_source_bitrate_kbps, Some(128));
    assert_eq!(mp3.verdict, Verdict::Fake, "{mp3:#?}");
    assert!(mp3.reasons.is_empty());
    let hz = mp3.cutoff.as_ref().unwrap().frequency_hz;
    assert!((hz - 17_000.0).abs() <= 200.0, "cutoff {hz} Hz");

    // "m4a_aac_128_lp16k.m4a": format M4A, bitrate 128, real bitrate 128, Real
    let m4a = row(&rows, "m4a_aac_128_lp16k.m4a");
    assert_eq!(m4a.info.format, dto::AudioFormat::M4a);
    assert_eq!(m4a.info.codec, Some(dto::AudioCodec::Aac));
    assert_eq!(m4a.info.declared_bitrate_kbps, Some(128));
    assert_eq!(m4a.info.bitrate_mode, None);
    assert_eq!(m4a.estimated_source_bitrate_kbps, Some(128));
    assert_eq!(m4a.verdict, Verdict::Real, "{m4a:#?}");

    // the lossless file: "—" in its bitrate and real bitrate cells
    let wav = row(&rows, "full_band.wav");
    assert_eq!(wav.info.declared_bitrate_kbps, None);
    assert_eq!(wav.info.bitrate_mode, None);
    assert_eq!(wav.estimated_source_bitrate_kbps, None);
}

// @RF-14 @RF-06 Scenario: A variable bitrate is marked in the table
#[test]
fn rf15_a_variable_bitrate_is_marked_in_the_table() {
    let dir = temp_dir("vbr").join("VBR");
    std::fs::create_dir_all(&dir).unwrap();
    put(&dir, "mp3_vbr_lp19k5", "mp3_vbr_lp19k5.mp3");
    let (_, events) = run(&dir);
    let rows = analyses(&events);
    assert_eq!(rows.len(), 1);
    let a = rows[0];
    // The table shows "240 kbps VBR": the average and the mode.
    assert_eq!(a.info.declared_bitrate_kbps, Some(240));
    assert_eq!(a.info.bitrate_mode, Some(dto::BitrateMode::Vbr));
    assert_eq!(a.estimated_source_bitrate_kbps, Some(240));
    assert_eq!(a.verdict, Verdict::Real, "{a:#?}");
    assert_eq!(a.algorithm_version, 6);
}

// @RNF-09 Scenario: Unicode names of lossy files
#[test]
fn rnf09_unicode_names_of_lossy_files() {
    let dir = temp_dir("unicode").join("Álbum ″Ñandú″");
    std::fs::create_dir_all(&dir).unwrap();
    put(&dir, "mp3_320_lp20k5", "01 – Canción ‘x’.mp3");
    let (summary, events) = run(&dir);
    assert_eq!(summary.total, 1);
    let rows = analyses(&events);
    assert_eq!(rows.len(), 1);
    let a = rows[0];
    // "the table shows the file name exactly"
    assert!(
        a.info.path.ends_with("01 – Canción ‘x’.mp3"),
        "{}",
        a.info.path
    );
    assert!(
        a.info.folder.ends_with("Álbum ″Ñandú″"),
        "{}",
        a.info.folder
    );
    // "its verdict is real"
    assert_eq!(a.verdict, Verdict::Real, "{a:#?}");
    assert_eq!(a.error, None);
}

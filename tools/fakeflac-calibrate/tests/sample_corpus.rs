//! Checks of the test support itself (@RP-11, `specs/calibration/sample-corpus.md`): the synthetic
//! signals written by `tests/common` have the verdicts that `specs/features/detection-lossless.feature`
//! requires with the default DetectionParams (v4). These tests exercise only `fakeflac-audio` and `fakeflac-core`,
//! so they pass before the tool exists. Protected after the `test(RP-11)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_audio::{SymphoniaAnalyzer, EXTRACTION_VERSION};
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::ports::FileEntry;
use fakeflac_core::{AudioFormat, DetectionParams, SuspicionReason, TrackError, Verdict};
use std::path::Path;

fn analyze(path: &Path) -> fakeflac_core::TrackAnalysis {
    let meta = std::fs::metadata(path).unwrap();
    let ext = path.extension().unwrap().to_string_lossy().into_owned();
    let entry = FileEntry {
        path: path.to_path_buf(),
        folder: path.parent().unwrap().to_path_buf(),
        format: AudioFormat::from_extension(&ext).unwrap(),
        size_bytes: meta.len(),
        modified_unix: 0,
    };
    analyze_track(&SymphoniaAnalyzer, &entry, &DetectionParams::default())
}

#[test]
fn rp11_extraction_version_is_declared() {
    assert_eq!(EXTRACTION_VERSION, 2);
}

#[test]
fn rp11_sample_fixtures_have_the_v2_verdicts() {
    let verdict = |name: &str, c: Container| analyze(&fixture(name, c));
    for (name, container, expected, reasons) in [
        ("full_band", Container::Flac, Verdict::Real, vec![]),
        ("full_band", Container::Wav, Verdict::Real, vec![]),
        ("hf_bursts", Container::Flac, Verdict::Real, vec![]),
        (
            "low_hf",
            Container::Flac,
            Verdict::Suspicious,
            vec![SuspicionReason::LowHighFrequencyContent],
        ),
        ("lp16k_dither", Container::Flac, Verdict::Fake, vec![]),
        // v4 (fixtures.md, "Verdicts with v4"): the 6 dB mean step at 16 kHz is in the lossy range,
        // so the track is not full band and the gap rule gives Fake, as in v2.
        ("sfb21_gaps", Container::Flac, Verdict::Fake, vec![]),
        ("lp20k_floor", Container::Flac, Verdict::Fake, vec![]),
        (
            "sfb21_gaps_weak",
            Container::Flac,
            Verdict::Suspicious,
            vec![SuspicionReason::HighBandGaps],
        ),
    ] {
        let a = verdict(name, container);
        assert_eq!(a.verdict, expected, "{name}: {a:#?}");
        assert_eq!(a.reasons, reasons, "{name}");
    }
    let short = verdict("short", Container::Flac);
    assert_eq!(short.verdict, Verdict::Error);
    assert_eq!(short.error, Some(TrackError::TooShort));
    let corrupt = verdict("corrupt", Container::Flac);
    assert_eq!(corrupt.verdict, Verdict::Error);
    assert!(
        matches!(corrupt.error, Some(TrackError::CorruptStream(_))),
        "{:?}",
        corrupt.error
    );
}

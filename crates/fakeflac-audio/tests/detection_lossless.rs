//! Tests derived from `specs/features/detection-lossless.feature` (@RF-03 @RF-04 @RF-14 @RNF-04).
//! Spec: `specs/detection/rules.md` (algorithm_version 2), parameters v2, fixtures of
//! `specs/detection/fixtures.md`. One test per example row of each Scenario Outline.
//! Protected after the `test(RF-03,RF-04)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::{DetectionParams, SuspicionReason, TrackAnalysis, TrackError, Verdict};

fn analyze(name: &str, container: Container) -> TrackAnalysis {
    let path = fixture(name, container);
    analyze_track(
        &SymphoniaAnalyzer,
        &entry(&path),
        &DetectionParams::default(),
    )
}

fn assert_verdict(name: &str, container: Container, verdict: Verdict, reasons: &[SuspicionReason]) {
    let a = analyze(name, container);
    assert_eq!(a.verdict, verdict, "{name} ({container:?}): {a:#?}");
    assert_eq!(a.reasons, reasons, "{name} ({container:?}) reasons");
}

/// Cutoff assertion of the feature: expected ± 200 Hz (2 · band_hz).
fn assert_cutoff(name: &str, expected_hz: f32) {
    let a = analyze(name, Container::Flac);
    let cutoff = a.cutoff.as_ref().expect("cutoff reported").frequency_hz;
    assert!(
        (cutoff - expected_hz).abs() <= 200.0,
        "{name}: cutoff {cutoff} Hz, expected {expected_hz} ± 200 Hz"
    );
}

/// "Signal without a cutoff step reports the Nyquist frequency as cutoff".
fn assert_nyquist_cutoff(name: &str) {
    let a = analyze(name, Container::Flac);
    assert_eq!(
        a.cutoff.expect("cutoff reported").frequency_hz,
        22_050.0,
        "{name}"
    );
}

fn assert_not_real(name: &str) {
    let a = analyze(name, Container::Flac);
    assert_ne!(a.verdict, Verdict::Real, "{name}: {a:#?}");
}

use Container::{Flac, Wav};
use SuspicionReason::{GradualRolloff, HighBandGaps, LowHighFrequencyContent};

// ---- Scenario Outline: Verdict by signal (@RF-04) ----------------------------------------------
#[test]
fn verdict_full_band_flac() {
    assert_verdict("full_band", Flac, Verdict::Real, &[]);
}
#[test]
fn verdict_full_band_wav() {
    assert_verdict("full_band", Wav, Verdict::Real, &[]);
}
#[test]
fn verdict_lp16k_flac() {
    assert_verdict("lp16k", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_lp19k_flac() {
    assert_verdict("lp19k", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_lp20k_wav() {
    assert_verdict("lp20k", Wav, Verdict::Fake, &[]);
}
#[test]
fn verdict_lp16k_96k_flac() {
    assert_verdict("lp16k_96k", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_bass_tone_lp16k_flac() {
    assert_verdict("bass_tone_lp16k", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_gradual_flac() {
    assert_verdict("gradual", Flac, Verdict::Suspicious, &[GradualRolloff]);
}
#[test]
fn verdict_low_hf_flac() {
    assert_verdict(
        "low_hf",
        Flac,
        Verdict::Suspicious,
        &[LowHighFrequencyContent],
    );
}
#[test]
fn verdict_silence_flac() {
    assert_verdict(
        "silence",
        Flac,
        Verdict::Suspicious,
        &[LowHighFrequencyContent],
    );
}
#[test]
fn verdict_lp16k_dither_flac() {
    assert_verdict("lp16k_dither", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_lp20k_floor_flac() {
    assert_verdict("lp20k_floor", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_sfb21_gaps_flac() {
    assert_verdict("sfb21_gaps", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_sfb21_gaps_weak_flac() {
    assert_verdict(
        "sfb21_gaps_weak",
        Flac,
        Verdict::Suspicious,
        &[HighBandGaps],
    );
}
#[test]
fn verdict_hf_bursts_flac() {
    assert_verdict("hf_bursts", Flac, Verdict::Real, &[]);
}

// ---- Scenario Outline: Cutoff estimate (@RF-03) ------------------------------------------------
#[test]
fn cutoff_lp16k() {
    assert_cutoff("lp16k", 16_000.0);
}
#[test]
fn cutoff_lp19k() {
    assert_cutoff("lp19k", 19_000.0);
}
#[test]
fn cutoff_lp20k() {
    assert_cutoff("lp20k", 20_000.0);
}
#[test]
fn cutoff_lp16k_96k() {
    assert_cutoff("lp16k_96k", 16_000.0);
}
#[test]
fn cutoff_bass_tone_lp16k() {
    assert_cutoff("bass_tone_lp16k", 16_000.0);
}
#[test]
fn cutoff_lp16k_dither() {
    assert_cutoff("lp16k_dither", 16_000.0);
}
#[test]
fn cutoff_lp20k_floor() {
    assert_cutoff("lp20k_floor", 20_000.0);
}

// ---- Scenario Outline: Signal without a cutoff step reports the Nyquist frequency (@RF-03) -----
#[test]
fn nyquist_cutoff_full_band() {
    assert_nyquist_cutoff("full_band");
}
#[test]
fn nyquist_cutoff_sfb21_gaps() {
    assert_nyquist_cutoff("sfb21_gaps");
}
#[test]
fn nyquist_cutoff_hf_bursts() {
    assert_nyquist_cutoff("hf_bursts");
}

// ---- Scenario Outline: Noise above the low-pass does not hide a transcode (v1 regression, @RF-04)
#[test]
fn v1_regression_lp16k_dither_is_not_real() {
    assert_not_real("lp16k_dither");
}
#[test]
fn v1_regression_lp20k_floor_is_not_real() {
    assert_not_real("lp20k_floor");
}
#[test]
fn v1_regression_sfb21_gaps_is_not_real() {
    assert_not_real("sfb21_gaps");
}

// ---- Scenario: Too short (@RNF-04) -------------------------------------------------------------
#[test]
fn too_short_is_error() {
    let a = analyze("short", Flac);
    assert_eq!(a.verdict, Verdict::Error);
    assert_eq!(a.error, Some(TrackError::TooShort));
}

// ---- Scenario: Corrupt file (@RNF-04) ----------------------------------------------------------
#[test]
fn corrupt_is_error() {
    let a = analyze("corrupt", Flac);
    assert_eq!(a.verdict, Verdict::Error);
    assert!(
        matches!(a.error, Some(TrackError::CorruptStream(_))),
        "expected CorruptStream, got {:?}",
        a.error
    );
}

// ---- Scenario: Deterministic result (@RF-04) ---------------------------------------------------
#[test]
fn deterministic() {
    assert_eq!(analyze("lp19k", Flac), analyze("lp19k", Flac));
}

// ---- Scenario: Parameters come from the versioned file (@RF-04) --------------------------------
#[test]
fn default_params_equal_versioned_file() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection/detection-params.v2.toml");
    let text = std::fs::read_to_string(file).unwrap();
    let from_file: DetectionParams = toml::from_str(&text).unwrap();
    assert_eq!(DetectionParams::default(), from_file);
    assert_eq!(from_file.algorithm_version, 2);
}

#[test]
fn every_analysis_carries_algorithm_version_2() {
    // Analysed, too short and corrupt tracks alike.
    for name in ["lp16k", "silence", "short", "corrupt"] {
        assert_eq!(analyze(name, Flac).algorithm_version, 2, "{name}");
    }
}

// Track metadata reported with the analysis (RF-06 columns).
#[test]
fn track_info_is_filled() {
    let a = analyze("full_band", Flac);
    assert_eq!(a.info.sample_rate, 44_100);
    assert_eq!(a.info.bit_depth, Some(16));
    assert_eq!(a.info.channels, 1);
    assert!(
        (29_900..=30_100).contains(&a.info.duration_ms),
        "duration {}",
        a.info.duration_ms
    );
    assert_eq!(a.info.format, fakeflac_core::AudioFormat::Flac);
}

//! Tests derived from `specs/features/detection-lossless.feature` (@RF-03 @RF-04 @RF-14 @RNF-04).
//! Spec: `specs/detection/rules.md` (algorithm_version 3), parameters v3, fixtures of
//! `specs/detection/fixtures.md`. One test per example row of each Scenario Outline.
//! Protected after the `test(RF-03,RF-04)` commit; updated for detection v3 in the `test(RF-04)`
//! commit that follows ADR-0038. The rule-level scenarios of the full-band guard are in
//! `crates/fakeflac-core/tests/detect_rules.rs`, the parameter files in `params_files.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::measure;
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{DetectionParams, SuspicionReason, TrackAnalysis, TrackError, Verdict};

fn analyze(name: &str, container: Container) -> TrackAnalysis {
    analyze_with(name, container, &DetectionParams::default())
}

fn analyze_with(name: &str, container: Container, params: &DetectionParams) -> TrackAnalysis {
    let path = fixture(name, container);
    analyze_track(&SymphoniaAnalyzer, &entry(&path), params)
}

/// The parameters of `specs/detection/detection-params.v2.toml` (no `[full_band]` section).
fn v2_params() -> DetectionParams {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection/detection-params.v2.toml");
    toml::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// "The track is full band": `FullBand` of the measurements of the FLAC fixture with the default
/// (v3) parameters.
fn is_full_band(name: &str) -> bool {
    let params = DetectionParams::default();
    let path = fixture(name, Container::Flac);
    let probe = SymphoniaAnalyzer.analyze(&path, &params).unwrap();
    let spectra = probe.spectrum.expect("the fixture has spectra");
    let m = measure(&spectra, &params);
    fakeflac_core::detect::is_full_band(&m, &params)
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
use SuspicionReason::{AmbiguousCutoff, GradualRolloff, HighBandGaps, LowHighFrequencyContent};

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
    // v3: the gated content reaches the upper window, so the track is full band (ADR-0038).
    assert_verdict("sfb21_gaps", Flac, Verdict::Suspicious, &[HighBandGaps]);
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

#[test]
fn verdict_sfb21_gaps_lp20k5_flac() {
    assert_verdict("sfb21_gaps_lp20k5", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_sfb21_gaps_aa21k_flac() {
    assert_verdict(
        "sfb21_gaps_aa21k",
        Flac,
        Verdict::Suspicious,
        &[HighBandGaps],
    );
}
#[test]
fn verdict_shelf19k5_flac() {
    assert_verdict("shelf19k5", Flac, Verdict::Fake, &[]);
}
#[test]
fn verdict_shelf19k5_bursts_flac() {
    assert_verdict(
        "shelf19k5_bursts",
        Flac,
        Verdict::Suspicious,
        &[AmbiguousCutoff],
    );
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

#[test]
fn cutoff_sfb21_gaps_lp20k5() {
    assert_cutoff("sfb21_gaps_lp20k5", 20_500.0);
}
#[test]
fn cutoff_sfb21_gaps_aa21k() {
    assert_cutoff("sfb21_gaps_aa21k", 21_000.0);
}
#[test]
fn cutoff_shelf19k5() {
    assert_cutoff("shelf19k5", 19_500.0);
}
#[test]
fn cutoff_shelf19k5_bursts() {
    assert_cutoff("shelf19k5_bursts", 19_500.0);
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

// ---- Scenario Outline: A stationary floor above the low-pass is not full band (v3 guard, @RF-04)
fn assert_floor_is_not_full_band(name: &str) {
    assert!(!is_full_band(name), "{name}: not full band");
    assert_verdict(name, Flac, Verdict::Fake, &[]);
}
#[test]
fn guard_floor_lp16k_dither_is_not_full_band() {
    assert_floor_is_not_full_band("lp16k_dither");
}
#[test]
fn guard_floor_lp20k_floor_is_not_full_band() {
    assert_floor_is_not_full_band("lp20k_floor");
}
#[test]
fn guard_floor_sfb21_gaps_lp20k5_is_not_full_band() {
    assert_floor_is_not_full_band("sfb21_gaps_lp20k5");
}
#[test]
fn guard_floor_shelf19k5_is_not_full_band() {
    assert_floor_is_not_full_band("shelf19k5");
}

// ---- Scenario Outline: Intermittent content above the lossy range is full band (v3 guard) ------
fn assert_bursts_are_full_band(name: &str) {
    assert!(is_full_band(name), "{name}: full band");
    assert_eq!(analyze(name, Flac).verdict, Verdict::Suspicious, "{name}");
}
#[test]
fn guard_bursts_sfb21_gaps_is_full_band() {
    assert_bursts_are_full_band("sfb21_gaps");
}
#[test]
fn guard_bursts_sfb21_gaps_aa21k_is_full_band() {
    assert_bursts_are_full_band("sfb21_gaps_aa21k");
}
#[test]
fn guard_bursts_shelf19k5_bursts_is_full_band() {
    assert_bursts_are_full_band("shelf19k5_bursts");
}

// ---- Scenario Outline: A v2 params file still gives the v2 verdicts (@RF-04) ------------------
fn assert_v2_is_fake(name: &str) {
    let a = analyze_with(name, Flac, &v2_params());
    assert_eq!(a.verdict, Verdict::Fake, "{name}: {a:#?}");
    assert_eq!(a.algorithm_version, 2, "{name}");
}
#[test]
fn v2_params_sfb21_gaps_is_fake() {
    assert_v2_is_fake("sfb21_gaps");
}
#[test]
fn v2_params_sfb21_gaps_aa21k_is_fake() {
    assert_v2_is_fake("sfb21_gaps_aa21k");
}
#[test]
fn v2_params_shelf19k5_bursts_is_fake() {
    assert_v2_is_fake("shelf19k5_bursts");
}
#[test]
fn v2_params_lp16k_dither_is_fake() {
    assert_v2_is_fake("lp16k_dither");
}

// ---- Scenario: A v2 params file changes only the guarded verdicts (@RF-04) ---------------------
#[test]
fn v2_params_change_only_the_guarded_verdicts() {
    const GUARDED: [&str; 3] = ["sfb21_gaps", "sfb21_gaps_aa21k", "shelf19k5_bursts"];
    let (v2, v3) = (v2_params(), DetectionParams::default());
    for name in [
        "full_band",
        "lp16k",
        "lp19k",
        "lp20k",
        "lp16k_96k",
        "gradual",
        "low_hf",
        "bass_tone_lp16k",
        "silence",
        "short",
        "corrupt",
        "lp16k_unknown_length",
        "lp16k_dither",
        "lp20k_floor",
        "sfb21_gaps",
        "sfb21_gaps_weak",
        "hf_bursts",
        "sfb21_gaps_lp20k5",
        "sfb21_gaps_aa21k",
        "shelf19k5",
        "shelf19k5_bursts",
    ] {
        let (a2, a3) = (analyze_with(name, Flac, &v2), analyze_with(name, Flac, &v3));
        assert_eq!(a2.cutoff, a3.cutoff, "{name}: cutoff");
        assert_eq!(a2.error, a3.error, "{name}: error");
        if GUARDED.contains(&name) {
            assert_eq!(a2.verdict, Verdict::Fake, "{name}: v2");
            assert!(a2.reasons.is_empty(), "{name}: v2 reasons");
            assert_ne!(a3.verdict, Verdict::Fake, "{name}: v3");
        } else {
            assert_eq!(a2.verdict, a3.verdict, "{name}: verdict");
            assert_eq!(a2.reasons, a3.reasons, "{name}: reasons");
        }
    }
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

// ---- Scenario: Stream without a declared length (@RF-03 @RNF-04) ------------------------------
#[test]
fn unknown_length_stream_is_analysed() {
    let path = fixture("lp16k_unknown_length", Flac);
    // Sanity: the fixture really exercises the undeclared-length path, and the original does not.
    assert_eq!(common::declared_total_samples(&path), 0);
    assert!(common::declared_total_samples(&fixture("lp16k", Flac)) > 0);
    let a = analyze("lp16k_unknown_length", Flac);
    assert_eq!(a.verdict, Verdict::Fake, "{a:#?}");
    let cutoff = a.cutoff.as_ref().expect("cutoff reported").frequency_hz;
    assert!(
        (cutoff - 16_000.0).abs() <= 200.0,
        "cutoff {cutoff} Hz, expected 16000 ± 200 Hz"
    );
}

// ---- Scenario: A silent track reports no cutoff (@RF-03) ---------------------------------------
#[test]
fn silent_track_reports_no_cutoff() {
    let a = analyze("silence", Flac);
    assert_eq!(a.verdict, Verdict::Suspicious, "{a:#?}");
    assert_eq!(a.reasons, &[LowHighFrequencyContent]);
    assert_eq!(a.cutoff, None, "{a:#?}");
}

// ---- Scenario: Deterministic result (@RF-04) ---------------------------------------------------
#[test]
fn deterministic() {
    assert_eq!(
        analyze("shelf19k5_bursts", Flac),
        analyze("shelf19k5_bursts", Flac)
    );
}

// ---- Scenario: Parameters come from the versioned file (@RF-04) --------------------------------
#[test]
fn default_params_equal_versioned_file() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection/detection-params.v3.toml");
    let text = std::fs::read_to_string(file).unwrap();
    let from_file: DetectionParams = toml::from_str(&text).unwrap();
    assert_eq!(DetectionParams::default(), from_file);
    assert_eq!(from_file.algorithm_version, 3);
}

#[test]
fn every_analysis_carries_algorithm_version_3() {
    // Analysed, too short and corrupt tracks alike.
    for name in ["lp16k", "silence", "short", "corrupt"] {
        assert_eq!(analyze(name, Flac).algorithm_version, 3, "{name}");
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

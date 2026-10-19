//! Tests derived from `specs/features/detection-lossless.feature` (@RF-03 @RF-04 @RF-14 @RNF-04).
//! Spec: `specs/detection/rules.md` (algorithm_version 4), parameters v4, fixtures of
//! `specs/detection/fixtures.md`. One test per example row of each Scenario Outline.
//! Protected after the `test(RF-03,RF-04)` commit; updated for detection v3 in the `test(RF-04)`
//! commit that follows ADR-0038 and for v4 in the one that follows ADR-0039. The rule-level
//! scenarios of the full-band guard are in
//! `crates/fakeflac-core/tests/detect_rules.rs`, the parameter files in `params_files.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{measure, Measurements};
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

/// The parameters of `specs/detection/detection-params.v3.toml` (a `[full_band]` section without
/// `max_lossy_step_db`).
fn v3_params() -> DetectionParams {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection/detection-params.v3.toml");
    toml::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// The measurements of the FLAC fixture with `params`.
fn measurements(name: &str, params: &DetectionParams) -> Measurements {
    let path = fixture(name, Container::Flac);
    let probe = SymphoniaAnalyzer.analyze(&path, params).unwrap();
    let spectra = probe.spectrum.expect("the fixture has spectra");
    measure(&spectra, params)
}

/// "The track is full band": `FullBand` of the measurements of the FLAC fixture with `params`.
fn is_full_band(name: &str, params: &DetectionParams) -> bool {
    let m = measurements(name, params);
    fakeflac_core::detect::is_full_band(&m, params)
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
    // v4: its 6 dB mean step at 16 kHz is in the lossy range, so it is not full band (ADR-0039).
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
    // v4: its 21 dB step at 19.5 kHz is in the lossy range, so it is not full band (ADR-0039).
    assert_verdict("shelf19k5_bursts", Flac, Verdict::Fake, &[]);
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

// ---- Scenario Outline: A stationary floor above the low-pass is not full band (guard, @RF-04)
fn assert_floor_is_not_full_band(name: &str) {
    assert!(
        !is_full_band(name, &DetectionParams::default()),
        "{name}: not full band"
    );
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

// ---- Scenario Outline: Intermittent content above the lossy range, with no step of
// max_lossy_step_db or more inside it, is full band (@RF-04) -------------------------------------
fn assert_bursts_are_full_band(name: &str, verdict: Verdict) {
    assert!(
        is_full_band(name, &DetectionParams::default()),
        "{name}: full band"
    );
    assert_eq!(analyze(name, Flac).verdict, verdict, "{name}");
}
#[test]
fn guard_bursts_sfb21_gaps_aa21k_is_full_band() {
    assert_bursts_are_full_band("sfb21_gaps_aa21k", Verdict::Suspicious);
}
#[test]
fn guard_bursts_hf_bursts_is_full_band() {
    assert_bursts_are_full_band("hf_bursts", Verdict::Real);
}

// ---- Scenario Outline: Intermittent content above a cutoff step in the lossy range is not full
// band (v4, @RF-04). Also confirms the measured step of fixtures.md, "Verdicts with v4". ---------
fn assert_lossy_step_removes_the_guard(name: &str) {
    let v3 = v3_params();
    assert!(
        is_full_band(name, &v3),
        "{name}: full band with the v3 parameters"
    );
    let v4 = DetectionParams::default();
    let m = measurements(name, &v4);
    let fs = m.step_hz.expect("the fixture has a cutoff step");
    assert!(
        (v4.verdict.lossy_min_cutoff_hz..v4.verdict.fake_max_cutoff_hz).contains(&fs),
        "{name}: fs {fs} Hz is in the lossy range"
    );
    assert!(
        m.step_db >= 3.0,
        "{name}: D {} dB is at least 3 dB",
        m.step_db
    );
    assert!(
        !fakeflac_core::detect::is_full_band(&m, &v4),
        "{name}: not full band with v4 (D {} dB at {fs} Hz)",
        m.step_db
    );
    assert_verdict(name, Flac, Verdict::Fake, &[]);
}
#[test]
fn lossy_step_sfb21_gaps_is_not_full_band() {
    assert_lossy_step_removes_the_guard("sfb21_gaps");
}
#[test]
fn lossy_step_shelf19k5_bursts_is_not_full_band() {
    assert_lossy_step_removes_the_guard("shelf19k5_bursts");
}

// hf_bursts has no step of 3 dB or more in the lossy range (its gate starts at 8 kHz, below the
// step search range): the precondition of the full-band row above (fixtures.md, "Verdicts with v4").
#[test]
fn hf_bursts_has_no_lossy_range_step_of_max_lossy_step_db() {
    let v4 = DetectionParams::default();
    let m = measurements("hf_bursts", &v4);
    let in_range = m.step_hz.is_some_and(|fs| {
        (v4.verdict.lossy_min_cutoff_hz..v4.verdict.fake_max_cutoff_hz).contains(&fs)
    });
    assert!(
        !(in_range && m.step_db >= 3.0),
        "hf_bursts: D {} dB at {:?} Hz",
        m.step_db,
        m.step_hz
    );
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

// ---- Scenario Outline: A v3 params file still gives the v3 verdicts (@RF-04) -------------------
fn assert_v3_verdict(name: &str, verdict: Verdict, reasons: &[SuspicionReason]) {
    let a = analyze_with(name, Flac, &v3_params());
    assert_eq!(a.verdict, verdict, "{name}: {a:#?}");
    assert_eq!(a.reasons, reasons, "{name}: reasons");
    assert_eq!(a.algorithm_version, 3, "{name}");
}
#[test]
fn v3_params_sfb21_gaps_is_suspicious() {
    assert_v3_verdict("sfb21_gaps", Verdict::Suspicious, &[HighBandGaps]);
}
#[test]
fn v3_params_sfb21_gaps_aa21k_is_suspicious() {
    assert_v3_verdict("sfb21_gaps_aa21k", Verdict::Suspicious, &[HighBandGaps]);
}
#[test]
fn v3_params_shelf19k5_bursts_is_suspicious() {
    assert_v3_verdict("shelf19k5_bursts", Verdict::Suspicious, &[AmbiguousCutoff]);
}
#[test]
fn v3_params_lp16k_dither_is_fake() {
    assert_v3_verdict("lp16k_dither", Verdict::Fake, &[]);
}

// ---- Scenario: A v3 params file differs only where a lossy-range step removes the guard --------
#[test]
fn v3_params_change_only_where_a_lossy_step_removes_the_guard() {
    let (v3, v4) = (v3_params(), DetectionParams::default());
    for name in ALL_FIXTURES {
        let (a3, a4) = (analyze_with(name, Flac, &v3), analyze_with(name, Flac, &v4));
        assert_eq!(a3.cutoff, a4.cutoff, "{name}: cutoff");
        assert_eq!(a3.error, a4.error, "{name}: error");
        if ["sfb21_gaps", "shelf19k5_bursts"].contains(&name) {
            assert_eq!(a3.verdict, Verdict::Suspicious, "{name}: v3");
            assert_eq!(a4.verdict, Verdict::Fake, "{name}: v4");
            assert!(a4.reasons.is_empty(), "{name}: v4 reasons");
        } else {
            assert_eq!(a3.verdict, a4.verdict, "{name}: verdict");
            assert_eq!(a3.reasons, a4.reasons, "{name}: reasons");
        }
    }
}

const ALL_FIXTURES: [&str; 21] = [
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
];

// ---- Scenario: A v2 params file changes only the guarded verdicts (@RF-04) ---------------------
// With v4 only sfb21_gaps_aa21k is guarded: sfb21_gaps and shelf19k5_bursts are Fake with both.
#[test]
fn v2_params_change_only_the_guarded_verdicts() {
    const GUARDED: [&str; 1] = ["sfb21_gaps_aa21k"];
    let (v2, v4) = (v2_params(), DetectionParams::default());
    for name in ALL_FIXTURES {
        let (a2, a4) = (analyze_with(name, Flac, &v2), analyze_with(name, Flac, &v4));
        assert_eq!(a2.cutoff, a4.cutoff, "{name}: cutoff");
        assert_eq!(a2.error, a4.error, "{name}: error");
        if GUARDED.contains(&name) {
            assert_eq!(a2.verdict, Verdict::Fake, "{name}: v2");
            assert!(a2.reasons.is_empty(), "{name}: v2 reasons");
            assert_eq!(a4.verdict, Verdict::Suspicious, "{name}: v4");
            assert_eq!(a4.reasons, [HighBandGaps], "{name}: v4 reasons");
        } else {
            assert_eq!(a2.verdict, a4.verdict, "{name}: verdict");
            assert_eq!(a2.reasons, a4.reasons, "{name}: reasons");
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
        .join("../../specs/detection/detection-params.v4.toml");
    let text = std::fs::read_to_string(file).unwrap();
    let from_file: DetectionParams = toml::from_str(&text).unwrap();
    assert_eq!(DetectionParams::default(), from_file);
    assert_eq!(from_file.algorithm_version, 4);
}

#[test]
fn every_analysis_carries_algorithm_version_4() {
    // Analysed, too short and corrupt tracks alike.
    for name in ["lp16k", "silence", "short", "corrupt"] {
        assert_eq!(analyze(name, Flac).algorithm_version, 4, "{name}");
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

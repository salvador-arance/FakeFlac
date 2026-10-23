//! Fixture-level scenarios of `specs/features/detection-lossy.feature` (@RF-15): the synthetic lossy
//! fixtures of `specs/detection/lossy-fixtures.md` (bitstream writers in `tests/common/lossy.rs`)
//! analysed with DetectionParams v5, plus the older params files and the v4 compatibility of the
//! lossless signals. Rule-level scenarios (given measurements) are in
//! `crates/fakeflac-core/tests/detect_lossy.rs`; the parameter-file scenarios are in
//! `params_files.rs`. Spec: `specs/detection/lossy-source.md`, ADR-0041.
//!
//! The analyses use the parameter files of `specs/detection/` (not only `Default`) so that "DetectionParams
//! v5" and "the parameters of specs/detection/detection-params.v4.toml" are read from the spec.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, lossy_fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::ports::FileEntry;
use fakeflac_core::{
    AudioFormat, DetectionParams, SuspicionReason, TrackAnalysis, TrackError, Verdict,
};
use std::path::Path;

fn params_file(name: &str) -> DetectionParams {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection")
        .join(name);
    toml::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

fn v5() -> DetectionParams {
    params_file("detection-params.v5.toml")
}

/// The `FileEntry` the scanner would produce for a lossy fixture (the format comes from the extension).
fn lossy_entry(path: &Path) -> FileEntry {
    let meta = std::fs::metadata(path).unwrap();
    let format = match path.extension().unwrap().to_str().unwrap() {
        "mp3" => AudioFormat::Mp3,
        "m4a" => AudioFormat::M4a,
        other => panic!("not a lossy fixture: {other}"),
    };
    FileEntry {
        path: path.to_path_buf(),
        folder: path.parent().unwrap().to_path_buf(),
        format,
        size_bytes: meta.len(),
        modified_unix: meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    }
}

/// "Given the lossy fixture <name> ... When it is analysed with <params>".
fn analyze_with(name: &str, params: &DetectionParams) -> TrackAnalysis {
    analyze_track(
        &SymphoniaAnalyzer,
        &lossy_entry(&lossy_fixture(name)),
        params,
    )
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Verdict and estimated source bitrate of lossy fixtures (@RF-15)
// ---------------------------------------------------------------------------------------------

fn verdict_row(
    fixture: &str,
    verdict: Verdict,
    reasons: &[SuspicionReason],
    estimate: u32,
    cutoff_hz: f32,
) {
    let a = analyze_with(fixture, &v5());
    assert_eq!(a.verdict, verdict, "{fixture}: {a:#?}");
    assert_eq!(a.reasons, reasons, "{fixture}");
    assert_eq!(a.estimated_source_bitrate_kbps, Some(estimate), "{fixture}");
    assert_eq!(a.error, None, "{fixture}");
    assert_eq!(a.algorithm_version, 5, "{fixture}");
    let c = a
        .cutoff
        .as_ref()
        .unwrap_or_else(|| panic!("{fixture}: a cutoff"));
    assert!(
        (c.frequency_hz - cutoff_hz).abs() <= 200.0,
        "{fixture}: cutoff {} Hz, want {cutoff_hz} ± 200",
        c.frequency_hz
    );
}

#[test]
fn rf15_mp3_320_lp20k5_is_real_with_the_declared_bitrate() {
    verdict_row("mp3_320_lp20k5", Verdict::Real, &[], 320, 20_500.0);
}
#[test]
fn rf15_mp3_320_lp17k_is_fake_with_an_estimate_of_128() {
    verdict_row("mp3_320_lp17k", Verdict::Fake, &[], 128, 17_000.0);
}
#[test]
fn rf15_mp3_128_lp17k_is_real() {
    verdict_row("mp3_128_lp17k", Verdict::Real, &[], 128, 17_000.0);
}
#[test]
fn rf15_mp3_192_lp17k_notag_is_suspicious_cutoff_below_declared() {
    verdict_row(
        "mp3_192_lp17k_notag",
        Verdict::Suspicious,
        &[SuspicionReason::CutoffBelowDeclared],
        128,
        17_000.0,
    );
}
#[test]
fn rf15_mp3_320_lp18k5_is_suspicious_cutoff_below_declared() {
    verdict_row(
        "mp3_320_lp18k5",
        Verdict::Suspicious,
        &[SuspicionReason::CutoffBelowDeclared],
        192,
        18_500.0,
    );
}
#[test]
fn rf15_mp3_320_tag16k_honours_the_user_chosen_low_pass() {
    verdict_row("mp3_320_tag16k", Verdict::Real, &[], 320, 16_000.0);
}
#[test]
fn rf15_mp3_vbr_lp19k5_is_real_with_the_average_as_estimate() {
    verdict_row("mp3_vbr_lp19k5", Verdict::Real, &[], 240, 19_500.0);
}
#[test]
fn rf15_mp3_abr_lp18k6_is_real_with_the_average_as_estimate() {
    verdict_row("mp3_abr_lp18k6", Verdict::Real, &[], 192, 18_600.0);
}
#[test]
fn rf15_mp3_320_48k_lp20k_is_real_a_known_limitation() {
    verdict_row("mp3_320_48k_lp20k", Verdict::Real, &[], 320, 20_000.0);
}
#[test]
fn rf15_m4a_aac_128_lp16k_is_real() {
    verdict_row("m4a_aac_128_lp16k", Verdict::Real, &[], 128, 16_000.0);
}
#[test]
fn rf15_m4a_aac_256_lp15k3_is_fake_with_an_estimate_of_128() {
    verdict_row("m4a_aac_256_lp15k3", Verdict::Fake, &[], 128, 15_300.0);
}

// ---------------------------------------------------------------------------------------------
// Scenario: Little treble gives no estimate (@RF-15)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_little_treble_gives_no_estimate() {
    let a = analyze_with("mp3_320_low_hf", &v5());
    assert_eq!(a.verdict, Verdict::Suspicious, "{a:#?}");
    assert_eq!(a.reasons, vec![SuspicionReason::LowHighFrequencyContent]);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert_eq!(a.algorithm_version, 5);
}

// ---------------------------------------------------------------------------------------------
// Scenario: A silent lossy file reports no cutoff and no estimate (@RF-15)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_a_silent_lossy_file_reports_no_cutoff_and_no_estimate() {
    let a = analyze_with("mp3_silence", &v5());
    assert_eq!(a.verdict, Verdict::Suspicious, "{a:#?}");
    assert_eq!(a.reasons, vec![SuspicionReason::LowHighFrequencyContent]);
    assert_eq!(a.cutoff, None);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert_eq!(a.algorithm_version, 5);
}

// ---------------------------------------------------------------------------------------------
// Scenario: Deterministic result (@RF-15)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_deterministic_result() {
    let first = analyze_with("mp3_320_lp17k", &v5());
    let second = analyze_with("mp3_320_lp17k", &v5());
    assert_eq!(first, second);
    // Not a vacuous equality: the analysis is a real verdict with an estimate.
    assert_eq!(first.estimated_source_bitrate_kbps, Some(128), "{first:#?}");
}

// The defaults of the code are the v5 file, so "analysed with DetectionParams v5" and
// "analysed by the app" agree (scenario "Parameters come from the versioned file").
#[test]
fn rf15_the_default_params_give_the_same_analysis_as_the_v5_file() {
    assert_eq!(
        analyze_with("mp3_320_lp17k", &DetectionParams::default()),
        analyze_with("mp3_320_lp17k", &v5())
    );
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Params without a [lossy] section do not judge lossy files (@RF-15)
// ---------------------------------------------------------------------------------------------

fn without_lossy_section(file: &str, version: u32) {
    let a = analyze_with("mp3_320_lp17k", &params_file(file));
    assert_eq!(a.verdict, Verdict::Error, "{file}: {a:#?}");
    assert_eq!(
        a.error,
        Some(TrackError::UnsupportedFormat(
            "these DetectionParams have no [lossy] section".into()
        )),
        "{file}"
    );
    assert_eq!(a.algorithm_version, version, "{file}");
    assert_eq!(a.estimated_source_bitrate_kbps, None, "{file}");
    assert_eq!(a.cutoff, None, "{file}");
}

#[test]
fn rf15_v4_params_do_not_judge_lossy_files() {
    without_lossy_section("detection-params.v4.toml", 4);
}
#[test]
fn rf15_v2_params_do_not_judge_lossy_files() {
    without_lossy_section("detection-params.v2.toml", 2);
}

// An M4A is probed first (L0 depends on the codec): AAC in M4A without [lossy] is an error, ALAC
// in M4A is still judged as lossless with v4 params.
#[test]
fn rf15_v4_params_still_judge_alac_and_do_not_judge_aac() {
    let v4 = params_file("detection-params.v4.toml");
    let aac = analyze_with("m4a_aac_128_lp16k", &v4);
    assert_eq!(aac.verdict, Verdict::Error, "{aac:#?}");
    let alac = analyze_with("m4a_alac_lp16k", &v4);
    assert_eq!(alac.verdict, Verdict::Fake, "{alac:#?}");
    assert_eq!(alac.algorithm_version, 4);
}

// ---------------------------------------------------------------------------------------------
// Scenario: v5 gives the v4 lossless verdicts (@RF-15 @RF-04)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_v5_gives_the_v4_lossless_verdicts() {
    let v4 = params_file("detection-params.v4.toml");
    let v5 = v5();
    assert_eq!((v4.algorithm_version, v5.algorithm_version), (4, 5));
    // Every synthetic signal of specs/detection/fixtures.md saved as FLAC.
    let signals = [
        "full_band",
        "lp16k",
        "lp16k_96k",
        "lp19k",
        "lp20k",
        "gradual",
        "low_hf",
        "bass_tone_lp16k",
        "silence",
        "lp16k_dither",
        "lp20k_floor",
        "sfb21_gaps",
        "sfb21_gaps_weak",
        "hf_bursts",
        "sfb21_gaps_lp20k5",
        "sfb21_gaps_aa21k",
        "shelf19k5",
        "shelf19k5_bursts",
        "short",
        "corrupt",
    ];
    let mut failures = Vec::new();
    for name in signals {
        let path = fixture(name, Container::Flac);
        let old = analyze_track(&SymphoniaAnalyzer, &entry(&path), &v4);
        let new = analyze_track(&SymphoniaAnalyzer, &entry(&path), &v5);
        let same = old.verdict == new.verdict
            && old.reasons == new.reasons
            && old.cutoff == new.cutoff
            && old.error == new.error
            && old.estimated_source_bitrate_kbps == new.estimated_source_bitrate_kbps
            && new.estimated_source_bitrate_kbps.is_none();
        if !same || old.algorithm_version != 4 || new.algorithm_version != 5 {
            failures.push(format!("{name}: v4 {old:?} / v5 {new:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

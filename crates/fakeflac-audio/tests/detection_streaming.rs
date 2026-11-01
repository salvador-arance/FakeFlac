//! Fixture-level and file-level scenarios of `specs/features/detection-streaming-source.feature`
//! (@RF-16, rule L6a of `specs/detection/lossy-source.md` section 11, detection v7): the synthetic
//! lossy fixtures of `specs/detection/lossy-fixtures.md` (bitstream writers in
//! `tests/common/lossy.rs`; the three RF-16 fixtures are checked in `lossy_fixtures_streaming.rs`)
//! analysed with DetectionParams v7 (the defaults) and with the parameter files of
//! `specs/detection/` (v5, v6), plus the rule-level rows that name a params file. The rule-level
//! rows on the defaults are in `crates/fakeflac-core/tests/detect_streaming.rs`; the parameter-file
//! scenarios are in `params_streaming.rs`. Spec: lossy-source.md section 11, ADR-0043.
//!
//! Names chosen by the test author: `SuspicionReason::StreamingSampleRate` (the contract's
//! `streamingSampleRate`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, lossy, lossy_fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{cutoff_estimate, decide_track, measure, Measurements, StreamInfo};
use fakeflac_core::ports::{FileEntry, SpectrumAnalyzer};
use fakeflac_core::{
    AudioCodec, AudioFormat, DetectionParams, SuspicionReason, TrackAnalysis, TrackError, Verdict,
};
use std::path::Path;
use SuspicionReason::StreamingSampleRate;

fn spec_text(name: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection")
        .join(name);
    std::fs::read_to_string(file).unwrap().replace("\r\n", "\n")
}

fn params_file(name: &str) -> DetectionParams {
    toml::from_str(&spec_text(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// DetectionParams v7: the defaults (lossy-source.md section 11.4).
fn v7() -> DetectionParams {
    params_file("detection-params.v7.toml")
}

fn v6() -> DetectionParams {
    params_file("detection-params.v6.toml")
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
// Scenario Outline: Verdict of lossy fixtures with v7 (@RF-16)
// ---------------------------------------------------------------------------------------------

fn verdict_row(
    fixture: &str,
    verdict: Verdict,
    reasons: &[SuspicionReason],
    estimate: Option<u32>,
    cutoff_hz: f32,
) {
    let a = analyze_with(fixture, &v7());
    assert_eq!(a.verdict, verdict, "{fixture}: {a:#?}");
    assert_eq!(a.reasons, reasons, "{fixture}");
    assert_eq!(a.estimated_source_bitrate_kbps, estimate, "{fixture}");
    assert_eq!(a.error, None, "{fixture}");
    assert_eq!(a.algorithm_version, 7, "{fixture}");
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
fn rf16_mp3_320_48k_lp20k_is_suspicious_streaming_sample_rate_with_no_estimate() {
    verdict_row(
        "mp3_320_48k_lp20k",
        Verdict::Suspicious,
        &[StreamingSampleRate],
        None,
        20_000.0,
    );
}
#[test]
fn rf16_mp3_320_lp20k_at_44_1_khz_is_real() {
    verdict_row("mp3_320_lp20k", Verdict::Real, &[], Some(320), 20_000.0);
}
#[test]
fn rf16_m4a_aac_256_48k_lp20k_is_suspicious_streaming_sample_rate_with_no_estimate() {
    verdict_row(
        "m4a_aac_256_48k_lp20k",
        Verdict::Suspicious,
        &[StreamingSampleRate],
        None,
        20_000.0,
    );
}
#[test]
fn rf16_mp3_320_lp20k5_is_real_with_v7() {
    verdict_row("mp3_320_lp20k5", Verdict::Real, &[], Some(320), 20_500.0);
}
#[test]
fn rf16_mp3_320_lp17k_is_fake_with_v7() {
    verdict_row("mp3_320_lp17k", Verdict::Fake, &[], Some(128), 17_000.0);
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: The same 48 kHz fixtures are real with params without [lossy.streaming] (@RF-16)
// ---------------------------------------------------------------------------------------------

fn real_without_streaming(fixture: &str, params: &DetectionParams, estimate: u32, version: u32) {
    let a = analyze_with(fixture, params);
    assert_eq!(a.verdict, Verdict::Real, "{fixture} v{version}: {a:#?}");
    assert!(a.reasons.is_empty(), "{fixture} v{version}");
    assert_eq!(
        a.estimated_source_bitrate_kbps,
        Some(estimate),
        "{fixture} v{version}"
    );
    assert_eq!(a.algorithm_version, version, "{fixture}");
}

#[test]
fn rf16_mp3_320_48k_lp20k_is_real_with_v6() {
    real_without_streaming("mp3_320_48k_lp20k", &v6(), 320, 6);
}
#[test]
fn rf16_m4a_aac_256_48k_lp20k_is_real_with_v6() {
    real_without_streaming("m4a_aac_256_48k_lp20k", &v6(), 256, 6);
}
#[test]
fn rf16_mp3_320_48k_lp20k_is_real_with_v5() {
    real_without_streaming("mp3_320_48k_lp20k", &v5(), 320, 5);
}

// ---------------------------------------------------------------------------------------------
// Scenario: v7 changes no other lossy fixture (@RF-16 @RF-15)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_v7_changes_no_other_lossy_fixture() {
    let (old_params, new_params) = (v6(), v7());
    // Every lossy fixture of lossy-fixtures.md section 2: the byte fixtures and the two ALAC ones.
    let mut names: Vec<&str> = lossy::BYTE_FIXTURES.to_vec();
    names.extend(["m4a_alac_full_band", "m4a_alac_lp16k"]);
    assert_eq!(names.len(), 22, "the fixtures of section 2");
    names.retain(|n| !["mp3_320_48k_lp20k", "m4a_aac_256_48k_lp20k"].contains(n));
    let mut failures = Vec::new();
    for name in names {
        let (old, new) = (
            analyze_with(name, &old_params),
            analyze_with(name, &new_params),
        );
        let same = old.verdict == new.verdict
            && old.reasons == new.reasons
            && old.error == new.error
            && old.cutoff == new.cutoff
            && old.estimated_source_bitrate_kbps == new.estimated_source_bitrate_kbps;
        if !same || old.algorithm_version != 6 || new.algorithm_version != 7 {
            failures.push(format!("{name}: v6 {old:?} / v7 {new:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Params without [lossy.streaming] never apply L6a (@RF-16), from the files
// ---------------------------------------------------------------------------------------------

fn measurements(h: f64, g: f64, d: f64, fs: Option<f64>, fe: f64) -> Measurements {
    Measurements {
        noise_floor_db: -80.0,
        high_frequency_db: h,
        extent_hz: fe,
        step_db: d,
        step_hz: fs,
        gap_db: g,
        upper_level_db: Some(-40.0),
        upper_burst_db: Some(8.0),
        median_step_db: None,
        top_floor_db: None,
        residual_db: None,
    }
}

fn stream(codec: AudioCodec, rate: u32, b: u32) -> StreamInfo {
    StreamInfo {
        codec,
        sample_rate: rate,
        declared_bitrate_kbps: Some(b),
        encoder_lowpass_hz: None,
    }
}

#[test]
fn rf16_params_files_without_a_streaming_section_never_apply_l6a() {
    let rows = [
        (AudioCodec::Mp3, 320, 20_050.0, "detection-params.v6.toml"),
        (AudioCodec::Mp3, 320, 20_450.0, "detection-params.v6.toml"),
        (AudioCodec::Aac, 256, 19_950.0, "detection-params.v6.toml"),
        (AudioCodec::Mp3, 320, 20_050.0, "detection-params.v5.toml"),
    ];
    let mut failures = Vec::new();
    for (codec, b, fs, file) in rows {
        let p = params_file(file);
        let d = decide_track(
            &stream(codec, 48_000, b),
            &measurements(-20.0, 0.0, 40.0, Some(fs), 24_000.0),
            &p,
        );
        if d.verdict != Verdict::Real
            || !d.reasons.is_empty()
            || d.estimated_source_bitrate_kbps != Some(b)
            || d.error.is_some()
        {
            failures.push(format!("{codec:?} {b} fs {fs} with {file}: {d:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Scenario: Opus tracks are not judged by L6a even when the params have Opus rows (@RF-16)
// ---------------------------------------------------------------------------------------------

/// The text of v7 with an `opus` array in `[lossy.cutoffs]` equal to its `mp3` array.
fn v7_with_opus_rows() -> String {
    let text = spec_text("detection-params.v7.toml");
    let start = text.find("\nmp3 = [").expect("the mp3 array") + 1;
    let end = start + text[start..].find("\n]").expect("the end of the mp3 array") + 2;
    let opus = text[start..end].replacen("mp3 = [", "opus = [", 1);
    let aac = text.find("\naac = [").expect("the aac array") + 1;
    let mut out = text.clone();
    out.insert_str(aac, &format!("{opus}\n"));
    out
}

#[test]
fn rf16_opus_tracks_are_not_judged_by_l6a_even_with_opus_rows() {
    let p: DetectionParams = toml::from_str(&v7_with_opus_rows()).expect("the text deserialises");
    p.validate().expect("the text is valid");
    assert_eq!(
        p.lossy.as_ref().unwrap().cutoffs.opus.as_ref(),
        Some(&p.lossy.as_ref().unwrap().cutoffs.mp3),
        "the opus rows equal the mp3 rows"
    );
    let d = decide_track(
        &stream(AudioCodec::Opus, 48_000, 160),
        &measurements(-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
        &p,
    );
    assert_eq!(d.verdict, Verdict::Real, "{d:?}");
    assert!(d.reasons.is_empty(), "{d:?}");
    assert_eq!(d.estimated_source_bitrate_kbps, Some(160));
    assert_eq!(d.error, None);
    // The same measurements make an MP3 at 48 kHz suspicious with these params: only the codec
    // keeps L6a away.
    let mp3 = decide_track(
        &stream(AudioCodec::Mp3, 48_000, 320),
        &measurements(-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
        &p,
    );
    assert_eq!(mp3.reasons, [StreamingSampleRate], "{mp3:?}");
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Lossless codecs at 48 kHz are never judged by L6a (@RF-16 @RF-04)
// ---------------------------------------------------------------------------------------------

fn full_band_measurements(params: &DetectionParams) -> Measurements {
    let path = fixture("full_band", Container::Flac);
    let probe = SymphoniaAnalyzer.analyze(&path, params).unwrap();
    measure(&probe.spectrum.expect("the fixture has spectra"), params)
}

#[test]
fn rf16_lossless_codecs_at_48_khz_are_never_judged_by_l6a() {
    let (p6, p7) = (v6(), v7());
    let (m6, m7) = (full_band_measurements(&p6), full_band_measurements(&p7));
    for codec in [AudioCodec::Pcm, AudioCodec::Flac, AudioCodec::Alac] {
        let s = StreamInfo {
            codec,
            sample_rate: 48_000,
            declared_bitrate_kbps: None,
            encoder_lowpass_hz: None,
        };
        let (old, new) = (decide_track(&s, &m6, &p6), decide_track(&s, &m7, &p7));
        assert_eq!(new.verdict, old.verdict, "{codec:?}");
        assert_eq!(new.reasons, old.reasons, "{codec:?}");
        assert_eq!(
            cutoff_estimate(&m7, &p7),
            cutoff_estimate(&m6, &p6),
            "{codec:?}: cutoff"
        );
        assert!(!new.reasons.contains(&StreamingSampleRate), "{codec:?}");
        assert_eq!(new.estimated_source_bitrate_kbps, None, "{codec:?}");
        assert_eq!(new.error, None, "{codec:?}");
    }
    // Not vacuous: v7 is the default and differs from v6 by its [lossy.streaming] section.
    assert_eq!(p7.algorithm_version, 7);
    assert!(p7.lossy.as_ref().unwrap().streaming.is_some());
}

// ---------------------------------------------------------------------------------------------
// Scenario: v7 gives the v6 lossless verdicts (@RF-16 @RF-04)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_v7_gives_the_v6_lossless_verdicts() {
    let (old_params, new_params) = (v6(), v7());
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
        "edge_bump19k3",
        "edge_bump19k3_lp19k5",
        "holes_lp19k5",
    ];
    let mut failures = Vec::new();
    for name in signals {
        let path = fixture(name, Container::Flac);
        let old = analyze_track(&SymphoniaAnalyzer, &entry(&path), &old_params);
        let new = analyze_track(&SymphoniaAnalyzer, &entry(&path), &new_params);
        let same = old.verdict == new.verdict
            && old.reasons == new.reasons
            && old.cutoff == new.cutoff
            && old.error == new.error;
        if !same || old.algorithm_version != 6 || new.algorithm_version != 7 {
            failures.push(format!("{name}: v6 {old:?} / v7 {new:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Broken lossy files stay errors with v7 (@RF-16 @RNF-04)
// ---------------------------------------------------------------------------------------------

fn broken(fixture: &str, matches_error: impl Fn(&TrackError) -> bool) {
    let a = analyze_with(fixture, &v7());
    assert_eq!(a.verdict, Verdict::Error, "{fixture}: {a:#?}");
    let e = a.error.as_ref().expect("an error");
    assert!(matches_error(e), "{fixture}: {e:?}");
    assert!(a.reasons.is_empty(), "{fixture}");
    assert_eq!(a.cutoff, None, "{fixture}");
    assert_eq!(a.estimated_source_bitrate_kbps, None, "{fixture}");
}

#[test]
fn rf16_mp3_320_48k_short_is_too_short_with_v7() {
    broken("mp3_320_48k_short", |e| *e == TrackError::TooShort);
}
#[test]
fn rf16_mp3_corrupt_is_a_corrupt_stream_with_v7() {
    broken("mp3_corrupt", |e| matches!(e, TrackError::CorruptStream(_)));
}
#[test]
fn rf16_m4a_corrupt_is_a_corrupt_stream_with_v7() {
    broken("m4a_corrupt", |e| matches!(e, TrackError::CorruptStream(_)));
}

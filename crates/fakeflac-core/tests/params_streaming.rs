//! The `[lossy.streaming]` section of `DetectionParams` v7 (`specs/detection/lossy-source.md`
//! section 11.3), scenarios of `specs/features/detection-streaming-source.feature` (@RF-16): "The
//! defaults are the v7 file" (its values, on the struct) and "Valid parameter files and their
//! [lossy.streaming] section" (the code's defaults row). The text of the files, the deserialisation
//! rows and the validation rows are in `crates/fakeflac-audio/tests/params_streaming.rs` (the core
//! has no `toml` dependency); the equality of the defaults with `detection-params.v7.toml` is
//! checked there and in `detection_lossless.rs`.
//!
//! Names chosen by the test author: `LossyParams.streaming: Option<StreamingParams>`,
//! `fakeflac_core::params::StreamingParams { sample_rates_hz: Vec<u32>, codecs:
//! Vec<StreamingCodec>, min_declared_bitrate_kbps: u32, min_cutoff_hz: f64, max_cutoff_hz: f64 }`
//! (the five keys of the file) and `fakeflac_core::params::StreamingCodec { Mp3, Aac }` (the only
//! two values the shape accepts, section 11.3).

#![allow(clippy::expect_used)]

use fakeflac_core::params::{StreamingCodec, StreamingParams};
use fakeflac_core::DetectionParams;

fn streaming(p: &DetectionParams) -> &StreamingParams {
    p.lossy
        .as_ref()
        .expect("the defaults have a [lossy] section")
        .streaming
        .as_ref()
        .expect("the defaults have a [lossy.streaming] section")
}

// @RF-16 Scenario: The defaults are the v7 file ("lossy.streaming.sample_rates_hz is [48000], ...",
// "every TrackAnalysis computed with them carries algorithmVersion 7")
#[test]
fn rf16_the_defaults_hold_the_v7_streaming_values() {
    let p = DetectionParams::default();
    assert_eq!(p.algorithm_version, 7);
    assert_eq!(
        streaming(&p),
        &StreamingParams {
            sample_rates_hz: vec![48_000],
            codecs: vec![StreamingCodec::Mp3, StreamingCodec::Aac],
            min_declared_bitrate_kbps: 192,
            min_cutoff_hz: 19_800.0,
            max_cutoff_hz: 20_200.0,
        }
    );
}

// @RF-16 Scenario Outline: Valid parameter files and their [lossy.streaming] section (the default
// DetectionParams of the code: accepted, section present)
#[test]
fn rf16_the_default_params_are_valid_and_have_a_streaming_section() {
    let p = DetectionParams::default();
    p.validate().expect("the defaults are valid");
    assert!(p
        .lossy
        .as_ref()
        .expect("the defaults have a [lossy] section")
        .streaming
        .is_some());
}

fn rejection(p: &DetectionParams) -> String {
    p.validate()
        .expect_err("the parameters are invalid")
        .to_string()
}

fn edited(edit: impl FnOnce(&mut StreamingParams)) -> DetectionParams {
    let mut p = DetectionParams::default();
    edit(
        p.lossy
            .as_mut()
            .expect("the defaults have a [lossy] section")
            .streaming
            .as_mut()
            .expect("the defaults have a [lossy.streaming] section"),
    );
    p
}

// @RF-16 Scenario Outline: An invalid [lossy.streaming] section is rejected, validation rows on the
// struct: the error names the check of lossy-source.md section 11.3, written as there.
#[test]
fn rf16_validate_names_each_streaming_check() {
    let cases: Vec<(&str, DetectionParams, &str)> = vec![
        (
            "sample_rates_hz []",
            edited(|s| s.sample_rates_hz = vec![]),
            "lossy.streaming.sample_rates_hz is not empty",
        ),
        (
            "sample_rates_hz [0]",
            edited(|s| s.sample_rates_hz = vec![0]),
            "lossy.streaming.sample_rates_hz: sample rate > 0",
        ),
        (
            "sample_rates_hz [48000, 44100]",
            edited(|s| s.sample_rates_hz = vec![48_000, 44_100]),
            "lossy.streaming.sample_rates_hz increases",
        ),
        (
            "sample_rates_hz [48000, 48000]",
            edited(|s| s.sample_rates_hz = vec![48_000, 48_000]),
            "lossy.streaming.sample_rates_hz increases",
        ),
        (
            "codecs []",
            edited(|s| s.codecs = vec![]),
            "lossy.streaming.codecs is not empty",
        ),
        (
            "codecs [mp3, mp3]",
            edited(|s| s.codecs = vec![StreamingCodec::Mp3, StreamingCodec::Mp3]),
            "lossy.streaming.codecs has no duplicates",
        ),
        (
            "min_declared_bitrate_kbps 0",
            edited(|s| s.min_declared_bitrate_kbps = 0),
            "lossy.streaming.min_declared_bitrate_kbps > 0",
        ),
        (
            "min_cutoff_hz 0",
            edited(|s| s.min_cutoff_hz = 0.0),
            "lossy.streaming.min_cutoff_hz > 0",
        ),
        (
            "min_cutoff_hz NaN",
            edited(|s| s.min_cutoff_hz = f64::NAN),
            "lossy.streaming.min_cutoff_hz > 0",
        ),
        (
            "min_cutoff_hz = max_cutoff_hz",
            edited(|s| s.min_cutoff_hz = 20_200.0),
            "lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz",
        ),
        (
            "max_cutoff_hz NaN",
            edited(|s| s.max_cutoff_hz = f64::NAN),
            "lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz",
        ),
    ];
    let mut failures = Vec::new();
    for (case, p, what) in cases {
        match p.validate() {
            Ok(()) => failures.push(format!("{case}: accepted")),
            Err(e) if !e.to_string().contains(what) => {
                failures.push(format!("{case}: {e} (want it to name \"{what}\")"))
            }
            Err(_) => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// @RF-16 Scenario Outline: An invalid [lossy.streaming] section is rejected, last row: the checks of
// [lossy] (lossy-source.md section 6) come first.
#[test]
fn rf16_the_lossy_checks_come_before_the_streaming_checks() {
    let mut p = edited(|s| s.codecs = vec![]);
    p.lossy
        .as_mut()
        .expect("the defaults have a [lossy] section")
        .suspicious_shortfall_hz = 0.0;
    let t = rejection(&p);
    assert!(t.contains("lossy.suspicious_shortfall_hz > 0"), "{t}");
    assert!(!t.contains("lossy.streaming"), "{t}");
}

// @RF-16 Scenario Outline: Values of [lossy.streaming] at the edge of validity are accepted, on the
// struct.
#[test]
fn rf16_streaming_values_at_the_edge_of_validity_are_accepted() {
    let cases: Vec<(&str, DetectionParams)> = vec![
        (
            "sample_rates_hz [44100, 48000]",
            edited(|s| s.sample_rates_hz = vec![44_100, 48_000]),
        ),
        (
            "codecs [aac]",
            edited(|s| s.codecs = vec![StreamingCodec::Aac]),
        ),
        (
            "codecs [aac, mp3]",
            edited(|s| s.codecs = vec![StreamingCodec::Aac, StreamingCodec::Mp3]),
        ),
        (
            "min_declared_bitrate_kbps 1",
            edited(|s| s.min_declared_bitrate_kbps = 1),
        ),
        (
            "min_cutoff_hz 20199",
            edited(|s| s.min_cutoff_hz = 20_199.0),
        ),
    ];
    for (case, p) in cases {
        p.validate()
            .unwrap_or_else(|e| panic!("{case}: rejected with {e}"));
    }
}

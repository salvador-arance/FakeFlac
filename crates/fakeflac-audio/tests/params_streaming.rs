//! Parameter-file scenarios of `specs/features/detection-streaming-source.feature` (@RF-16): the
//! optional `[lossy.streaming]` section of `DetectionParams` v7 (`specs/detection/lossy-source.md`
//! section 11.3). The shape is checked when a file is deserialised (serde), the values by
//! `DetectionParams::validate`, after the checks of lossy-source.md section 6. Lives in this crate
//! because `toml` is a dev-dependency here, not in `fakeflac-core`; the struct-level checks of the
//! defaults are in `crates/fakeflac-core/tests/params_streaming.rs`.
//!
//! Names chosen by the test author: `LossyParams.streaming: Option<StreamingParams>` (see the core
//! test for the fields).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use fakeflac_core::DetectionParams;
use std::path::Path;

const V5: &str = "detection-params.v5.toml";
const V6: &str = "detection-params.v6.toml";
const V7: &str = "detection-params.v7.toml";

fn spec_text(name: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection")
        .join(name);
    std::fs::read_to_string(file).unwrap().replace("\r\n", "\n")
}

/// The section name of a TOML header line (`[lossy.streaming]   # ...` -> `lossy.streaming`).
fn header(line: &str) -> Option<String> {
    line.trim()
        .strip_prefix('[')
        .and_then(|r| r.split(']').next())
        .map(str::to_string)
}

/// The text of v7 with the (section, key) line replaced by `new_line`, or removed when `None`. The
/// line is found by section and key, so `min_cutoff_hz` of `[lossy.streaming]` is not confused with
/// `lossy_min_cutoff_hz` of `[verdict]`.
fn v7_edited(section: &str, key: &str, new_line: Option<&str>) -> String {
    let mut current = String::new();
    let mut found = false;
    let mut out = Vec::new();
    for line in spec_text(V7).lines() {
        if let Some(name) = header(line) {
            current = name;
        }
        if current == section && line.trim().starts_with(&format!("{key} ")) {
            found = true;
            if let Some(n) = new_line {
                out.push(n.to_string());
            }
            continue;
        }
        out.push(line.to_string());
    }
    assert!(found, "{section}.{key} not found in {V7}");
    out.join("\n")
}

/// `v7_edited` for a key of `[lossy.streaming]`.
fn streaming_line(key: &str, new_line: Option<&str>) -> String {
    v7_edited("lossy.streaming", key, new_line)
}

/// The text of v7 with `extra` as the first line of `[lossy.streaming]`.
fn v7_with_extra_streaming_line(extra: &str) -> String {
    let mut out = Vec::new();
    let mut found = false;
    for line in spec_text(V7).lines() {
        out.push(line.to_string());
        if header(line).as_deref() == Some("lossy.streaming") {
            found = true;
            out.push(extra.to_string());
        }
    }
    assert!(found, "[lossy.streaming] not found in {V7}");
    out.join("\n")
}

/// The text of v7 with every line of `[lossy]` and `[lossy.cutoffs]` removed (their headers
/// included) and `[lossy.streaming]` kept.
fn v7_with_only_the_streaming_subsection() -> String {
    let mut current = String::new();
    let mut out = Vec::new();
    for line in spec_text(V7).lines() {
        if let Some(name) = header(line) {
            current = name;
        }
        if current == "lossy" || current == "lossy.cutoffs" {
            continue;
        }
        out.push(line.to_string());
    }
    let text = out.join("\n");
    assert!(text.contains("[lossy.streaming]") && !text.contains("[lossy.cutoffs]"));
    text
}

fn has_streaming(p: &DetectionParams) -> bool {
    p.lossy.as_ref().is_some_and(|l| l.streaming.is_some())
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Valid parameter files and their [lossy.streaming] section (@RF-16)
// ---------------------------------------------------------------------------------------------

fn accepted_file(name: &str) -> DetectionParams {
    let p: DetectionParams =
        toml::from_str(&spec_text(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    p.validate().unwrap_or_else(|e| panic!("{name}: {e}"));
    p
}

#[test]
fn rf16_the_v7_file_is_accepted_and_has_a_streaming_section() {
    let p = accepted_file(V7);
    assert_eq!(p.algorithm_version, 7);
    assert!(has_streaming(&p));
}
#[test]
fn rf16_the_v6_file_is_accepted_and_has_no_streaming_section() {
    let p = accepted_file(V6);
    assert_eq!(p.algorithm_version, 6);
    assert!(p.lossy.is_some() && !has_streaming(&p));
}
#[test]
fn rf16_the_v5_file_is_accepted_and_has_no_streaming_section() {
    let p = accepted_file(V5);
    assert_eq!(p.algorithm_version, 5);
    assert!(p.lossy.is_some() && !has_streaming(&p));
}
#[test]
fn rf16_the_default_params_are_accepted_and_have_a_streaming_section() {
    let p = DetectionParams::default();
    p.validate().expect("the defaults are valid");
    assert!(has_streaming(&p));
}

// ---------------------------------------------------------------------------------------------
// Scenario: The defaults are the v7 file (@RF-16)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_the_defaults_are_the_v7_file() {
    let v7 = accepted_file(V7);
    assert_eq!(DetectionParams::default(), v7);
    // "their sections other than [lossy.streaming] are equal to those of v6"
    let mut without = v7.clone();
    without.algorithm_version = 6;
    without.lossy.as_mut().unwrap().streaming = None;
    assert_eq!(without, accepted_file(V6));
    // The values are checked on the struct in the core's params_streaming.rs; here, on the text.
    for line in [
        "sample_rates_hz = [48000]",
        "codecs = [\"mp3\", \"aac\"]",
        "min_declared_bitrate_kbps = 192",
        "min_cutoff_hz = 19800.0",
        "max_cutoff_hz = 20200.0",
    ] {
        let text = spec_text(V7);
        let section = &text[text.find("[lossy.streaming]").unwrap()..];
        assert!(
            section.lines().any(|l| l.starts_with(line)),
            "{line} in [lossy.streaming]"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Values of [lossy.streaming] at the edge of validity are accepted (@RF-16)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_streaming_values_at_the_edge_of_validity_are_accepted() {
    let cases = [
        streaming_line("sample_rates_hz", Some("sample_rates_hz = [44100, 48000]")),
        streaming_line("codecs", Some("codecs = [\"aac\"]")),
        streaming_line("codecs", Some("codecs = [\"aac\", \"mp3\"]")),
        streaming_line(
            "min_declared_bitrate_kbps",
            Some("min_declared_bitrate_kbps = 1"),
        ),
        streaming_line("min_cutoff_hz", Some("min_cutoff_hz = 20199.0")),
    ];
    let mut failures = Vec::new();
    for text in cases {
        match toml::from_str::<DetectionParams>(&text) {
            Err(e) => failures.push(format!("deserialisation: {e}")),
            Ok(p) => {
                if let Err(e) = p.validate() {
                    failures.push(format!("validation: {e}"));
                } else if !has_streaming(&p) {
                    failures.push("no [lossy.streaming] section".to_string());
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: An invalid [lossy.streaming] section is rejected (@RF-16)
// ---------------------------------------------------------------------------------------------

fn assert_rejected_by_deserialisation(text: &str, what: &str) {
    let e = toml::from_str::<DetectionParams>(text).expect_err("the shape is invalid");
    assert!(e.to_string().contains(what), "{what}: {e}");
}

fn assert_rejected_by_validation(text: &str, what: &str) {
    let p: DetectionParams = toml::from_str(text).expect("the shape is valid");
    let e = p.validate().expect_err("the values are invalid");
    assert!(e.to_string().contains(what), "{what}: {e}");
}

#[test]
fn rf16_a_streaming_section_without_min_cutoff_hz_is_rejected() {
    assert_rejected_by_deserialisation(&streaming_line("min_cutoff_hz", None), "min_cutoff_hz");
}
#[test]
fn rf16_a_streaming_section_without_codecs_is_rejected() {
    assert_rejected_by_deserialisation(&streaming_line("codecs", None), "codecs");
}
#[test]
fn rf16_a_streaming_section_without_sample_rates_hz_is_rejected() {
    assert_rejected_by_deserialisation(&streaming_line("sample_rates_hz", None), "sample_rates_hz");
}
#[test]
fn rf16_a_streaming_section_with_an_unknown_field_is_rejected() {
    assert_rejected_by_deserialisation(
        &v7_with_extra_streaming_line("max_declared_bitrate_kbps = 320"),
        "max_declared_bitrate_kbps",
    );
}
#[test]
fn rf16_opus_cannot_be_listed_as_a_streaming_codec() {
    assert_rejected_by_deserialisation(
        &streaming_line("codecs", Some("codecs = [\"mp3\", \"opus\"]")),
        "opus",
    );
}
#[test]
fn rf16_a_lossless_codec_cannot_be_listed_as_a_streaming_codec() {
    assert_rejected_by_deserialisation(
        &streaming_line("codecs", Some("codecs = [\"mp3\", \"flac\"]")),
        "flac",
    );
}
#[test]
fn rf16_a_float_sample_rate_is_rejected() {
    assert_rejected_by_deserialisation(
        &streaming_line("sample_rates_hz", Some("sample_rates_hz = [48000.0]")),
        "sample_rates_hz",
    );
}
#[test]
fn rf16_a_streaming_section_without_the_lossy_values_is_rejected() {
    assert_rejected_by_deserialisation(&v7_with_only_the_streaming_subsection(), "missing field");
}

#[test]
fn rf16_an_empty_sample_rate_list_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("sample_rates_hz", Some("sample_rates_hz = []")),
        "lossy.streaming.sample_rates_hz is not empty",
    );
}
#[test]
fn rf16_a_zero_sample_rate_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("sample_rates_hz", Some("sample_rates_hz = [0]")),
        "lossy.streaming.sample_rates_hz: sample rate > 0",
    );
}
#[test]
fn rf16_decreasing_sample_rates_are_rejected() {
    assert_rejected_by_validation(
        &streaming_line("sample_rates_hz", Some("sample_rates_hz = [48000, 44100]")),
        "lossy.streaming.sample_rates_hz increases",
    );
}
#[test]
fn rf16_a_repeated_sample_rate_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("sample_rates_hz", Some("sample_rates_hz = [48000, 48000]")),
        "lossy.streaming.sample_rates_hz increases",
    );
}
#[test]
fn rf16_an_empty_codec_list_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("codecs", Some("codecs = []")),
        "lossy.streaming.codecs is not empty",
    );
}
#[test]
fn rf16_a_repeated_codec_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("codecs", Some("codecs = [\"mp3\", \"mp3\"]")),
        "lossy.streaming.codecs has no duplicates",
    );
}
#[test]
fn rf16_a_zero_bitrate_floor_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line(
            "min_declared_bitrate_kbps",
            Some("min_declared_bitrate_kbps = 0"),
        ),
        "lossy.streaming.min_declared_bitrate_kbps > 0",
    );
}
#[test]
fn rf16_a_zero_min_cutoff_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("min_cutoff_hz", Some("min_cutoff_hz = 0.0")),
        "lossy.streaming.min_cutoff_hz > 0",
    );
}
#[test]
fn rf16_a_nan_min_cutoff_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("min_cutoff_hz", Some("min_cutoff_hz = nan")),
        "lossy.streaming.min_cutoff_hz > 0",
    );
}
#[test]
fn rf16_an_empty_window_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("min_cutoff_hz", Some("min_cutoff_hz = 20200.0")),
        "lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz",
    );
}
#[test]
fn rf16_a_nan_max_cutoff_is_rejected() {
    assert_rejected_by_validation(
        &streaming_line("max_cutoff_hz", Some("max_cutoff_hz = nan")),
        "lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz",
    );
}
#[test]
fn rf16_the_lossy_checks_come_before_the_streaming_checks() {
    let text = streaming_line("codecs", Some("codecs = []")).replacen(
        "suspicious_shortfall_hz = 1000.0",
        "suspicious_shortfall_hz = 0.0",
        1,
    );
    assert_rejected_by_validation(&text, "lossy.suspicious_shortfall_hz > 0");
    let p: DetectionParams = toml::from_str(&text).unwrap();
    let e = p.validate().unwrap_err().to_string();
    assert!(!e.contains("lossy.streaming"), "{e}");
}

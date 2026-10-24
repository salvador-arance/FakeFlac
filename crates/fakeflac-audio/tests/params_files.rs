//! Parameter-file scenarios of `specs/features/detection-full-band.feature` (@RF-04): the shape of
//! `DetectionParams` is checked when a file is deserialised (serde), the order of the thresholds by
//! `DetectionParams::validate` (rules.md section 4). Lives in this crate because `toml` is a
//! dev-dependency here, not in `fakeflac-core` (the core has no file loader).
//! Spec: `specs/detection/rules.md` (algorithm_version 4), ADR-0038 and ADR-0039. The tests that
//! edit the text of v3 are the v3 scenarios and stay as they are; the ones that edit the text of
//! v4 cover `full_band.max_lossy_step_db`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use fakeflac_core::DetectionParams;
use std::path::Path;

fn spec_text(name: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection")
        .join(name);
    std::fs::read_to_string(file).unwrap().replace("\r\n", "\n")
}

/// The text of v3 with `edit` applied to the (section, key) line: `Some(new line)` replaces it,
/// `None` removes it. The line is found by section and key, so `low_hz` of `[full_band]` is not
/// confused with `search_low_hz` or `ref_band_low_hz`.
fn edited(section: &str, key: &str, new_line: Option<&str>) -> String {
    edited_file("detection-params.v3.toml", section, key, new_line)
}

fn edited_file(file: &str, section: &str, key: &str, new_line: Option<&str>) -> String {
    let mut current = String::new();
    let mut found = false;
    let mut out = Vec::new();
    for line in spec_text(file).lines() {
        let t = line.trim();
        if let Some(name) = t.strip_prefix('[').and_then(|r| r.split(']').next()) {
            current = name.to_string();
        }
        if current == section && t.starts_with(&format!("{key} ")) {
            found = true;
            if let Some(n) = new_line {
                out.push(n.to_string());
            }
            continue;
        }
        out.push(line.to_string());
    }
    assert!(found, "{section}.{key} not found in {file}");
    out.join("\n")
}

fn with_extra_line(section: &str, extra: &str) -> String {
    let mut out = Vec::new();
    for line in spec_text("detection-params.v3.toml").lines() {
        out.push(line.to_string());
        if line.trim_start().starts_with(&format!("[{section}]")) {
            out.push(extra.to_string());
        }
    }
    out.join("\n")
}

// Scenario Outline: Valid parameter files deserialise and validate (@RF-04)
#[test]
fn v3_file_is_accepted_and_has_a_full_band_section() {
    let p: DetectionParams = toml::from_str(&spec_text("detection-params.v3.toml")).unwrap();
    p.validate().expect("v3 is valid");
    assert!(p.full_band.is_some());
    assert_eq!(p.algorithm_version, 3);
    // v4 (ADR-0039): a v3 file has no max_lossy_step_db.
    assert_eq!(p.full_band.unwrap().max_lossy_step_db, None);
}

#[test]
fn v4_file_is_accepted_and_has_max_lossy_step_db() {
    let p: DetectionParams = toml::from_str(&spec_text("detection-params.v4.toml")).unwrap();
    p.validate().expect("v4 is valid");
    assert_eq!(p.algorithm_version, 4);
    assert_eq!(p.full_band.unwrap().max_lossy_step_db, Some(3.0));
}

// The v3 stand-in of `crates/fakeflac-core/tests/detect_rules.rs` (`v3_params()`: the defaults, v4,
// with `max_lossy_step_db = None` and `algorithm_version = 3`) is exactly what the v3 file says.
// Built here because tests cannot be shared across crates.
#[test]
fn v3_file_equals_the_v4_defaults_without_max_lossy_step_db() {
    // The defaults are v6 (ADR-0042); a v3 file has no [lossy] or [edge] section.
    let mut expected = DetectionParams {
        algorithm_version: 3,
        lossy: None,
        edge: None,
        ..DetectionParams::default()
    };
    expected.full_band.as_mut().unwrap().max_lossy_step_db = None;
    let p: DetectionParams = toml::from_str(&spec_text("detection-params.v3.toml")).unwrap();
    assert_eq!(p, expected);
}

#[test]
fn v2_file_is_accepted_and_has_no_full_band_section() {
    let p: DetectionParams = toml::from_str(&spec_text("detection-params.v2.toml")).unwrap();
    p.validate().expect("v2 is valid");
    assert!(p.full_band.is_none());
    assert_eq!(p.algorithm_version, 2);
}

#[test]
fn default_params_are_accepted_and_have_a_full_band_section() {
    let p = DetectionParams::default();
    p.validate().expect("the defaults are valid");
    assert!(p.full_band.is_some());
    assert_eq!(p.full_band.unwrap().max_lossy_step_db, Some(3.0));
}

// Scenario Outline: The bounds of max_lossy_step_db are inclusive (v4 text, value set).
fn v4_with_lossy_step(value: &str) -> String {
    edited_file(
        "detection-params.v4.toml",
        "full_band",
        "max_lossy_step_db",
        Some(&format!("max_lossy_step_db = {value}")),
    )
}

#[test]
fn max_lossy_step_db_lower_bound_is_accepted() {
    let p: DetectionParams = toml::from_str(&v4_with_lossy_step("0.0")).unwrap();
    p.validate().expect("0.0 is the lower bound");
}
#[test]
fn max_lossy_step_db_equal_to_step_fake_db_is_accepted() {
    let p: DetectionParams = toml::from_str(&v4_with_lossy_step("10.0")).unwrap();
    p.validate().expect("10.0 equals step_fake_db");
}

// Scenario Outline: An invalid max_lossy_step_db is rejected by validation.
#[test]
fn negative_max_lossy_step_db_is_rejected() {
    assert_rejected_by_validation(
        &v4_with_lossy_step("-0.5"),
        "full_band.max_lossy_step_db ≥ 0",
    );
}
#[test]
fn max_lossy_step_db_above_step_fake_db_is_rejected() {
    assert_rejected_by_validation(
        &v4_with_lossy_step("10.5"),
        "full_band.max_lossy_step_db ≤ step_fake_db",
    );
}
#[test]
fn step_fake_db_below_max_lossy_step_db_is_rejected() {
    let text = edited_file(
        "detection-params.v4.toml",
        "verdict",
        "step_fake_db",
        Some("step_fake_db = 2.5"),
    );
    let text = text
        .lines()
        .map(|l| {
            if l.trim_start().starts_with("step_suspicious_db ") {
                "step_suspicious_db = 2.5".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_rejected_by_validation(&text, "full_band.max_lossy_step_db ≤ step_fake_db");
}
#[test]
fn nan_max_lossy_step_db_is_rejected() {
    assert_rejected_by_validation(&v4_with_lossy_step("nan"), "full_band.max_lossy_step_db");
}

// Scenario Outline: Invalid parameter files are rejected: shape, at deserialisation.
fn assert_rejected_by_deserialisation(text: &str, what: &str) {
    let e = toml::from_str::<DetectionParams>(text).expect_err("the shape is invalid");
    assert!(e.to_string().contains(what), "{what}: {e}");
}

#[test]
fn full_band_without_low_hz_is_rejected() {
    assert_rejected_by_deserialisation(&edited("full_band", "low_hz", None), "low_hz");
}
#[test]
fn full_band_without_high_hz_is_rejected() {
    assert_rejected_by_deserialisation(&edited("full_band", "high_hz", None), "high_hz");
}
#[test]
fn full_band_without_min_level_db_is_rejected() {
    assert_rejected_by_deserialisation(&edited("full_band", "min_level_db", None), "min_level_db");
}
#[test]
fn full_band_without_min_burst_db_is_rejected() {
    assert_rejected_by_deserialisation(&edited("full_band", "min_burst_db", None), "min_burst_db");
}
#[test]
fn full_band_with_an_unknown_field_is_rejected() {
    assert_rejected_by_deserialisation(
        &with_extra_line("full_band", "max_level_db = 0.0"),
        "max_level_db",
    );
}

// Scenario Outline: Invalid parameter files are rejected: values, by validate.
fn assert_rejected_by_validation(text: &str, what: &str) {
    let p: DetectionParams = toml::from_str(text).expect("the shape is valid");
    let e = p.validate().expect_err("the values are invalid");
    assert!(e.to_string().contains(what), "{what}: {e}");
}

#[test]
fn high_hz_equal_to_low_hz_is_rejected() {
    assert_rejected_by_validation(
        &edited("full_band", "high_hz", Some("high_hz = 20700.0")),
        "full_band.low_hz < full_band.high_hz",
    );
}
#[test]
fn low_hz_equal_to_fake_max_cutoff_hz_is_rejected() {
    assert_rejected_by_validation(
        &edited("full_band", "low_hz", Some("low_hz = 20300.0")),
        "fake_max_cutoff_hz < full_band.low_hz",
    );
}
#[test]
fn gap_suspicious_above_gap_fake_is_rejected() {
    assert_rejected_by_validation(
        &edited(
            "verdict",
            "gap_suspicious_db",
            Some("gap_suspicious_db = 6.5"),
        ),
        "gap_suspicious_db ≤ gap_fake_db",
    );
}
#[test]
fn step_suspicious_above_step_fake_is_rejected() {
    assert_rejected_by_validation(
        &edited(
            "verdict",
            "step_suspicious_db",
            Some("step_suspicious_db = 10.5"),
        ),
        "step_suspicious_db ≤ step_fake_db",
    );
}

// =============================================================================================
// The [lossy] section of detection v5 (specs/features/detection-lossy.feature, @RF-15;
// specs/detection/lossy-source.md section 6, ADR-0041)
// =============================================================================================

const V5: &str = "detection-params.v5.toml";

// Scenario Outline: Valid parameter files and their [lossy] section (@RF-15)
#[test]
fn rf15_v5_file_is_accepted_and_has_a_lossy_section() {
    let p: DetectionParams = toml::from_str(&spec_text(V5)).unwrap();
    p.validate().expect("v5 is valid");
    assert_eq!(p.algorithm_version, 5);
    assert!(p.lossy.is_some());
}

#[test]
fn rf15_v4_file_is_accepted_and_has_no_lossy_section() {
    let p: DetectionParams = toml::from_str(&spec_text("detection-params.v4.toml")).unwrap();
    p.validate().expect("v4 is valid");
    assert!(p.lossy.is_none());
}

#[test]
fn rf15_v2_and_v3_files_have_no_lossy_section() {
    for file in ["detection-params.v2.toml", "detection-params.v3.toml"] {
        let p: DetectionParams = toml::from_str(&spec_text(file)).unwrap();
        p.validate().unwrap();
        assert!(p.lossy.is_none(), "{file}");
    }
}

#[test]
fn rf15_the_default_params_have_a_lossy_section() {
    let p = DetectionParams::default();
    p.validate().expect("the defaults are valid");
    assert!(p.lossy.is_some());
}

// v5 is v4 plus [lossy]: every other section and value is unchanged (lossy-source.md section 1).
#[test]
fn rf15_v5_is_v4_plus_the_lossy_section() {
    let v4: DetectionParams = toml::from_str(&spec_text("detection-params.v4.toml")).unwrap();
    let v5: DetectionParams = toml::from_str(&spec_text(V5)).unwrap();
    let expected = DetectionParams {
        algorithm_version: 5,
        lossy: v5.lossy.clone(),
        ..v4.clone()
    };
    assert_eq!(v5, expected);
    // And the v4 file is the defaults without [lossy] (the stand-in of the core's tests).
    assert_eq!(
        v4,
        DetectionParams {
            algorithm_version: 4,
            lossy: None,
            edge: None, // the defaults are v6 since ADR-0042
            ..DetectionParams::default()
        }
    );
}

/// The text of v5 with the exact `old` replaced by `new` (once), searching after `from` when given:
/// the `mp3` and `aac` tables share some rows.
fn v5_replace(from: Option<&str>, old: &str, new: &str) -> String {
    let text = spec_text(V5);
    let start = from.map_or(0, |marker| {
        text.find(marker)
            .unwrap_or_else(|| panic!("{marker} not in {V5}"))
    });
    let at = text[start..]
        .find(old)
        .unwrap_or_else(|| panic!("{old:?} not found in {V5}"))
        + start;
    format!("{}{}{}", &text[..at], new, &text[at + old.len()..])
}

/// The text of v5 with the whole array `name = [ ... ]` of `[lossy.cutoffs]` replaced.
fn v5_with_array(name: &str, replacement: &str) -> String {
    let text = spec_text(V5);
    let start = text
        .find(&format!("\n{name} = ["))
        .unwrap_or_else(|| panic!("{name} array not found"))
        + 1;
    let end = start + text[start..].find("\n]").unwrap() + 2;
    format!("{}{}{}", &text[..start], replacement, &text[end..])
}

const MP3_112: &str = "  { bitrate_kbps = 112, cutoff_hz = 15600.0 },";
const MP3_128: &str = "  { bitrate_kbps = 128, cutoff_hz = 17000.0 },";

// Scenario Outline: Invalid [lossy] sections are rejected: by deserialisation (@RF-15)
#[test]
fn rf15_lossy_without_suspicious_shortfall_hz_is_rejected() {
    assert_rejected_by_deserialisation(
        &edited_file(V5, "lossy", "suspicious_shortfall_hz", None),
        "suspicious_shortfall_hz",
    );
}

#[test]
fn rf15_lossy_with_an_unknown_field_is_rejected() {
    let text = spec_text(V5).replace("\n[lossy]", "\n[lossy]\nmax_shortfall_hz = 1.0");
    assert!(text.contains("max_shortfall_hz"));
    assert_rejected_by_deserialisation(&text, "max_shortfall_hz");
}

#[test]
fn rf15_lossy_cutoffs_without_the_aac_array_is_rejected() {
    assert_rejected_by_deserialisation(&v5_with_array("aac", ""), "aac");
}

#[test]
fn rf15_lossy_cutoffs_with_an_unknown_codec_is_rejected() {
    let text = spec_text(V5).replace("\n[lossy.cutoffs]", "\n[lossy.cutoffs]\nvorbis = []");
    assert!(text.contains("vorbis = []"));
    assert_rejected_by_deserialisation(&text, "vorbis");
}

#[test]
fn rf15_a_cutoff_row_without_cutoff_hz_is_rejected() {
    assert_rejected_by_deserialisation(
        &v5_replace(None, MP3_128, "  { bitrate_kbps = 128 },"),
        "cutoff_hz",
    );
}

// Scenario Outline: Invalid [lossy] sections are rejected: by validation (@RF-15)
#[test]
fn rf15_zero_suspicious_shortfall_hz_is_rejected() {
    assert_rejected_by_validation(
        &edited_file(
            V5,
            "lossy",
            "suspicious_shortfall_hz",
            Some("suspicious_shortfall_hz = 0.0"),
        ),
        "lossy.suspicious_shortfall_hz > 0",
    );
}

#[test]
fn rf15_suspicious_shortfall_above_fake_shortfall_is_rejected() {
    assert_rejected_by_validation(
        &edited_file(
            V5,
            "lossy",
            "suspicious_shortfall_hz",
            Some("suspicious_shortfall_hz = 3000.0"),
        ),
        "lossy.suspicious_shortfall_hz ≤ lossy.fake_shortfall_hz",
    );
}

#[test]
fn rf15_nan_fake_shortfall_hz_is_rejected() {
    assert_rejected_by_validation(
        &edited_file(
            V5,
            "lossy",
            "fake_shortfall_hz",
            Some("fake_shortfall_hz = nan"),
        ),
        "lossy.fake_shortfall_hz",
    );
}

#[test]
fn rf15_fake_min_step_below_step_suspicious_is_rejected() {
    assert_rejected_by_validation(
        &edited_file(
            V5,
            "lossy",
            "fake_min_step_db",
            Some("fake_min_step_db = 6.5"),
        ),
        "step_suspicious_db ≤ lossy.fake_min_step_db",
    );
}

#[test]
fn rf15_an_empty_mp3_array_is_rejected() {
    assert_rejected_by_validation(
        &v5_with_array("mp3", "mp3 = []"),
        "lossy.cutoffs.mp3 is not empty",
    );
}

#[test]
fn rf15_an_empty_opus_array_is_rejected() {
    let text = spec_text(V5).replace("\n[lossy.cutoffs]", "\n[lossy.cutoffs]\nopus = []");
    assert!(text.contains("opus = []"));
    assert_rejected_by_validation(&text, "lossy.cutoffs.opus is not empty");
}

#[test]
fn rf15_swapped_mp3_rows_are_rejected_by_the_bitrate_check() {
    assert_rejected_by_validation(
        &v5_replace(
            None,
            &format!("{MP3_112}\n{MP3_128}"),
            &format!("{MP3_128}\n{MP3_112}"),
        ),
        "lossy.cutoffs.mp3: bitrate_kbps increases",
    );
}

#[test]
fn rf15_a_repeated_mp3_bitrate_is_rejected() {
    assert_rejected_by_validation(
        &v5_replace(None, MP3_128, &format!("{MP3_128}\n{MP3_128}")),
        "lossy.cutoffs.mp3: bitrate_kbps increases",
    );
}

#[test]
fn rf15_a_decreasing_mp3_cutoff_is_rejected() {
    assert_rejected_by_validation(
        &v5_replace(
            None,
            "{ bitrate_kbps = 160, cutoff_hz = 17500.0 }",
            "{ bitrate_kbps = 160, cutoff_hz = 16500.0 }",
        ),
        "lossy.cutoffs.mp3: cutoff_hz does not decrease",
    );
}

#[test]
fn rf15_a_zero_aac_bitrate_is_rejected() {
    assert_rejected_by_validation(
        &v5_replace(
            Some("\naac = ["),
            "{ bitrate_kbps = 64,  cutoff_hz = 11000.0 }",
            "{ bitrate_kbps = 0,  cutoff_hz = 11000.0 }",
        ),
        "lossy.cutoffs.aac: bitrate_kbps > 0",
    );
}

#[test]
fn rf15_a_zero_aac_cutoff_is_rejected() {
    assert_rejected_by_validation(
        &v5_replace(
            Some("\naac = ["),
            "{ bitrate_kbps = 64,  cutoff_hz = 11000.0 }",
            "{ bitrate_kbps = 64,  cutoff_hz = 0.0 }",
        ),
        "lossy.cutoffs.aac: cutoff_hz > 0",
    );
}

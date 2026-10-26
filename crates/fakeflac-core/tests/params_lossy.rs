//! The `[lossy]` section of `DetectionParams` v5 (`specs/detection/lossy-source.md` section 6),
//! scenarios of `specs/features/detection-lossy.feature` (@RF-15): "The defaults hold the v5 lossy
//! values", "Valid parameter files and their [lossy] section" (the code's defaults row) and the
//! validation rows of "Invalid [lossy] sections are rejected" built on the struct (the text of the
//! file, the deserialisation rows and the file rows are in `crates/fakeflac-audio/tests/params_files.rs`).
//! Names chosen by the test author: `DetectionParams.lossy: Option<LossyParams>`,
//! `LossyParams { suspicious_shortfall_hz, fake_shortfall_hz, fake_min_step_db, cutoffs }`,
//! `LossyCutoffs { mp3: Vec<CutoffRow>, aac: Vec<CutoffRow>, opus: Option<Vec<CutoffRow>> }`,
//! `CutoffRow { bitrate_kbps: u32, cutoff_hz: f64 }`.

#![allow(clippy::expect_used)]

use fakeflac_core::params::{CutoffRow, LossyParams};
use fakeflac_core::DetectionParams;

fn lossy(p: &DetectionParams) -> &LossyParams {
    p.lossy
        .as_ref()
        .expect("the defaults have a [lossy] section")
}

// @RF-15 Scenario: The defaults hold the v5 lossy values (since detection v7 the defaults carry
// algorithm_version 7 and add [lossy.streaming], RF-16; lossy-source.md Q25 (j))
#[test]
fn rf15_the_defaults_hold_the_v5_lossy_values() {
    let p = DetectionParams::default();
    assert_eq!(p.algorithm_version, 7);
    let l = lossy(&p);
    assert_eq!(l.suspicious_shortfall_hz, 1000.0);
    assert_eq!(l.fake_shortfall_hz, 2500.0);
    assert_eq!(l.fake_min_step_db, 10.0);
    assert_eq!(l.cutoffs.mp3.len(), 14);
    assert_eq!(l.cutoffs.aac.len(), 7);
    assert_eq!(l.cutoffs.opus, None);
}

// @RF-15 Scenario: The defaults hold the v5 lossy values (the table rows of lossy-source.md section 6)
#[test]
fn rf15_the_default_tables_are_those_of_the_spec() {
    let row = |bitrate_kbps, cutoff_hz| CutoffRow {
        bitrate_kbps,
        cutoff_hz,
    };
    let p = DetectionParams::default();
    let l = lossy(&p);
    assert_eq!(
        l.cutoffs.mp3,
        vec![
            row(32, 5500.0),
            row(40, 7000.0),
            row(48, 7500.0),
            row(56, 10000.0),
            row(64, 11000.0),
            row(80, 13500.0),
            row(96, 15100.0),
            row(112, 15600.0),
            row(128, 17000.0),
            row(160, 17500.0),
            row(192, 18600.0),
            row(224, 19400.0),
            row(256, 19700.0),
            row(320, 20500.0),
        ]
    );
    assert_eq!(
        l.cutoffs.aac,
        vec![
            row(64, 11000.0),
            row(96, 13000.0),
            row(128, 15500.0),
            row(160, 16000.0),
            row(192, 17000.0),
            row(256, 19000.0),
            row(320, 19500.0),
        ]
    );
}

// @RF-15 Scenario Outline: Valid parameter files and their [lossy] section (the default DetectionParams of the code)
#[test]
fn rf15_the_default_params_are_valid() {
    DetectionParams::default()
        .validate()
        .expect("the defaults are valid");
}

fn rejection(p: &DetectionParams) -> String {
    p.validate()
        .expect_err("the parameters are invalid")
        .to_string()
}

fn edited(edit: impl FnOnce(&mut LossyParams)) -> DetectionParams {
    let mut p = DetectionParams::default();
    edit(
        p.lossy
            .as_mut()
            .expect("the defaults have a [lossy] section"),
    );
    p
}

// @RF-15 Scenario Outline: Invalid [lossy] sections are rejected (validation rows, on the struct)
#[test]
fn rf15_a_zero_suspicious_shortfall_is_rejected() {
    let t = rejection(&edited(|l| l.suspicious_shortfall_hz = 0.0));
    assert!(t.contains("lossy.suspicious_shortfall_hz > 0"), "{t}");
}

#[test]
fn rf15_a_suspicious_shortfall_above_the_fake_shortfall_is_rejected() {
    let t = rejection(&edited(|l| l.suspicious_shortfall_hz = 3000.0));
    assert!(
        t.contains("lossy.suspicious_shortfall_hz ≤ lossy.fake_shortfall_hz"),
        "{t}"
    );
}

#[test]
fn rf15_a_suspicious_shortfall_equal_to_the_fake_shortfall_is_accepted() {
    edited(|l| l.suspicious_shortfall_hz = 2500.0)
        .validate()
        .expect("the inequality is not strict");
}

#[test]
fn rf15_a_fake_min_step_below_step_suspicious_db_is_rejected() {
    let t = rejection(&edited(|l| l.fake_min_step_db = 6.5));
    assert!(
        t.contains("step_suspicious_db ≤ lossy.fake_min_step_db"),
        "{t}"
    );
}

#[test]
fn rf15_a_fake_min_step_equal_to_step_suspicious_db_is_accepted() {
    edited(|l| l.fake_min_step_db = 7.0)
        .validate()
        .expect("the inequality is not strict");
}

#[test]
fn rf15_an_empty_mp3_table_is_rejected() {
    let t = rejection(&edited(|l| l.cutoffs.mp3.clear()));
    assert!(t.contains("lossy.cutoffs.mp3 is not empty"), "{t}");
}

#[test]
fn rf15_an_empty_opus_table_is_rejected_but_an_absent_one_is_not() {
    let t = rejection(&edited(|l| l.cutoffs.opus = Some(vec![])));
    assert!(t.contains("lossy.cutoffs.opus is not empty"), "{t}");
    assert!(DetectionParams::default().validate().is_ok());
}

#[test]
fn rf15_swapped_rows_report_the_bitrate_check_first() {
    // Swapping 112 and 128 breaks both row checks; the bitrate one is reported (section 6, "Order").
    let t = rejection(&edited(|l| l.cutoffs.mp3.swap(7, 8)));
    assert!(
        t.contains("lossy.cutoffs.mp3: bitrate_kbps increases"),
        "{t}"
    );
}

#[test]
fn rf15_a_repeated_bitrate_is_rejected() {
    let t = rejection(&edited(|l| {
        let again = l.cutoffs.mp3[8].clone();
        l.cutoffs.mp3.insert(9, again);
    }));
    assert!(
        t.contains("lossy.cutoffs.mp3: bitrate_kbps increases"),
        "{t}"
    );
}

#[test]
fn rf15_a_decreasing_cutoff_is_rejected_and_an_equal_one_is_accepted() {
    let t = rejection(&edited(|l| l.cutoffs.mp3[9].cutoff_hz = 16500.0));
    assert!(
        t.contains("lossy.cutoffs.mp3: cutoff_hz does not decrease"),
        "{t}"
    );
    edited(|l| l.cutoffs.mp3[9].cutoff_hz = 17000.0)
        .validate()
        .expect("equal cutoffs are accepted");
}

#[test]
fn rf15_a_zero_bitrate_is_rejected() {
    let t = rejection(&edited(|l| l.cutoffs.aac[0].bitrate_kbps = 0));
    assert!(t.contains("lossy.cutoffs.aac: bitrate_kbps > 0"), "{t}");
}

#[test]
fn rf15_a_zero_cutoff_is_rejected() {
    let t = rejection(&edited(|l| l.cutoffs.aac[0].cutoff_hz = 0.0));
    assert!(t.contains("lossy.cutoffs.aac: cutoff_hz > 0"), "{t}");
}

#[test]
fn rf15_an_aac_error_is_reported_after_the_mp3_checks() {
    // Both tables broken: the mp3 check comes first (codec order mp3, aac, opus).
    let t = rejection(&edited(|l| {
        l.cutoffs.aac[0].bitrate_kbps = 0;
        l.cutoffs.mp3.clear();
    }));
    assert!(t.contains("lossy.cutoffs.mp3 is not empty"), "{t}");
}

// @RF-15 Scenario Outline: Invalid [lossy] sections are rejected ("lossy.fake_shortfall_hz set to nan")
// and the rule "a comparison with a NaN operand does not hold" (section 6).
#[test]
fn rf15_validate_rejects_nan_in_every_lossy_value() {
    type Poke = fn(&mut LossyParams);
    let cases: [(&str, Poke); 6] = [
        ("lossy.suspicious_shortfall_hz", |l| {
            l.suspicious_shortfall_hz = f64::NAN
        }),
        ("lossy.fake_shortfall_hz", |l| {
            l.fake_shortfall_hz = f64::NAN
        }),
        ("lossy.fake_min_step_db", |l| l.fake_min_step_db = f64::NAN),
        ("lossy.cutoffs.mp3 cutoff_hz", |l| {
            l.cutoffs.mp3[3].cutoff_hz = f64::NAN
        }),
        ("lossy.cutoffs.aac cutoff_hz", |l| {
            l.cutoffs.aac[3].cutoff_hz = f64::NAN
        }),
        ("lossy.cutoffs.opus cutoff_hz", |l| {
            l.cutoffs.opus = Some(vec![CutoffRow {
                bitrate_kbps: 64,
                cutoff_hz: f64::NAN,
            }])
        }),
    ];
    let mut accepted = Vec::new();
    for (name, poke) in cases {
        if edited(poke).validate().is_ok() {
            accepted.push(name);
        }
    }
    assert!(
        accepted.is_empty(),
        "validate accepted NaN in: {accepted:?}"
    );
    let t = rejection(&edited(|l| l.fake_shortfall_hz = f64::NAN));
    assert!(t.contains("lossy.fake_shortfall_hz"), "{t}");
}

// @RF-15 Scenario Outline: Valid parameter files (v4 row, on the struct): without [lossy] the
// params are valid and the lossless checks still apply.
#[test]
fn rf15_params_without_a_lossy_section_are_valid() {
    let p = DetectionParams {
        lossy: None,
        ..DetectionParams::default()
    };
    p.validate().expect("a v2-v4 file has no [lossy] section");
}

//! `DetectionParams::validate` and non-finite values (specs/detection/rules.md section 4,
//! @RF-04). Rule text: "`ParamsError` names the inequality that does not hold. It checks
//! `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db` and, when `[full_band]`
//! is present, `fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz`" and (v4), when
//! `full_band.max_lossy_step_db` is present, `0 ≤ full_band.max_lossy_step_db ≤ step_fake_db`." An ordering with a NaN
//! operand does not hold, so the parameters are invalid. (TOML accepts `nan`, so a params file can
//! reach the validator with one.)

#![allow(clippy::expect_used)]

use fakeflac_core::DetectionParams;

#[test]
fn rf04_validate_rejects_nan_in_every_ordered_field() {
    type Poke = fn(&mut DetectionParams);
    let cases: [(&str, Poke); 8] = [
        ("verdict.gap_suspicious_db", |p| {
            p.verdict.gap_suspicious_db = f64::NAN
        }),
        ("verdict.gap_fake_db", |p| p.verdict.gap_fake_db = f64::NAN),
        ("verdict.step_suspicious_db", |p| {
            p.verdict.step_suspicious_db = f64::NAN
        }),
        ("verdict.step_fake_db", |p| {
            p.verdict.step_fake_db = f64::NAN
        }),
        ("verdict.fake_max_cutoff_hz", |p| {
            p.verdict.fake_max_cutoff_hz = f64::NAN
        }),
        ("full_band.low_hz", |p| {
            p.full_band.as_mut().expect("full_band").low_hz = f64::NAN
        }),
        ("full_band.high_hz", |p| {
            p.full_band.as_mut().expect("full_band").high_hz = f64::NAN
        }),
        ("full_band.max_lossy_step_db", |p| {
            p.full_band.as_mut().expect("full_band").max_lossy_step_db = Some(f64::NAN)
        }),
    ];
    let mut accepted = Vec::new();
    for (name, poke) in cases {
        let mut p = DetectionParams::default();
        assert!(p.full_band.is_some(), "the defaults have [full_band]");
        poke(&mut p);
        if p.validate().is_ok() {
            accepted.push(name);
        }
    }
    assert!(
        accepted.is_empty(),
        "validate accepted NaN in: {accepted:?}"
    );
}

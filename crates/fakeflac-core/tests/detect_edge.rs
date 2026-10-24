//! Rule-level tests of the edge step of detection v6 (`specs/detection/rules.md` sections 2 to 4,
//! ADR-0042) on hand-built band spectra and hand-built measurements. Scenarios:
//! `specs/features/detection-edge-step.feature` (@RF-04), "Measurements", "The conjunction and
//! its boundaries" and "Watch list". The fixtures and the parameter files are in the audio crate
//! (`detection_edge_step.rs`, `params_edge.rs`).
//!
//! The spectra are written for bands of 100 Hz at 44.1 kHz: band `b` has its centre at
//! `(b + 0.5) * 100` Hz, there are 220 bands and `top` is band 219 (centre 21 950 Hz).

#![allow(clippy::expect_used)]

use fakeflac_core::detect::{decide, is_edge_step, measure, Measurements};
use fakeflac_core::{BandSpectra, DetectionParams, SuspicionReason, Verdict};

const TOL: f64 = 1e-6;

fn v6() -> DetectionParams {
    DetectionParams::default()
}

/// The defaults as v6 defines them: the `[edge]` section of `detection-params.v6.toml`. The
/// default of the code is checked against the file in the audio crate; the rule tests here must
/// not depend on it, so they set the section explicitly.
fn v6_explicit() -> DetectionParams {
    use fakeflac_core::params::EdgeParams;
    DetectionParams {
        algorithm_version: 6,
        edge: Some(EdgeParams {
            floor_low_hz: 20_900.0,
            floor_high_hz: 22_000.0,
            floor_min_db: -104.0,
            max_median_step_db: 3.0,
            min_residual_db: 15.0,
            max_step_db: 16.0,
        }),
        ..v6()
    }
}

/// The parameters of `detection-params.v5.toml`: the same without `[edge]`.
fn v5_like() -> DetectionParams {
    DetectionParams {
        algorithm_version: 5,
        edge: None,
        ..v6()
    }
}

fn spectra(
    sample_rate: u32,
    mean: impl Fn(usize) -> f64,
    median: impl Fn(usize) -> f64,
) -> BandSpectra {
    let band_hz = 100.0;
    let n = (f64::from(sample_rate) / 2.0 / band_hz).floor() as usize;
    BandSpectra {
        sample_rate,
        band_hz,
        mean_db: (0..n).map(&mean).collect(),
        median_db: (0..n).map(&median).collect(),
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < TOL
}

// ---------------------------------------------------------------------------------------------
// Scenario: Median step, top floor and post-step residual at the step of the mean spectrum
// ---------------------------------------------------------------------------------------------

#[test]
fn rf04_edge_measurements_at_the_step_of_the_mean_spectrum() {
    // M = -30 below 19450 Hz (bands 0..=193), -45 at 19450 (194), -55 from 19550 to 20850
    // (195..=208), -70 from 20950 up (209..).
    let mean = |b: usize| match b {
        0..=193 => -30.0,
        194 => -45.0,
        195..=208 => -55.0,
        _ => -70.0,
    };
    // Q = -50 on the bands centred 18850..19350 (187..=193), -51 on 19550..19750 (195..=197);
    // everywhere else it follows M (the unspecified bands do not enter D_Q at fs = 19450).
    let median = |b: usize| match b {
        187..=194 => -50.0,
        195..=197 => -51.0,
        _ => mean(b),
    };
    let s = spectra(44_100, mean, median);
    let p = v6_explicit();
    let m = measure(&s, &p);

    assert_eq!(m.step_hz, Some(19_450.0));
    assert!(close(m.step_db, 25.0), "D = {}", m.step_db);
    assert!(
        close(m.median_step_db.expect("D_Q is defined"), 1.0),
        "D_Q = {:?}",
        m.median_step_db
    );
    assert!(
        close(m.top_floor_db.expect("F_top is defined"), -40.0),
        "F_top = {:?}",
        m.top_floor_db
    );
    assert!(
        close(m.residual_db.expect("R is defined"), 15.0),
        "R = {:?}",
        m.residual_db
    );
    // D = 25 dB is not below max_step_db = 16: not an edge step.
    assert!(!is_edge_step(&m, &p));
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: The top floor is never below the 16-bit rounding floor
// ---------------------------------------------------------------------------------------------

#[test]
fn rf04_top_floor_is_never_below_the_16_bit_rounding_floor() {
    // (floor dB absolute, expected F_top, expected R); L_ref = -30 dB, so -104 dB absolute is -74.
    let rows = [
        (-140.0, -74.0, 9.0),
        (-104.0, -74.0, 9.0),
        (-100.0, -70.0, 5.0),
    ];
    let mut failures = Vec::new();
    for (floor, f_top, r) in rows {
        let mean = |b: usize| match b {
            0..=193 => -30.0,
            194 => -60.0,
            195..=208 => -95.0,
            _ => floor,
        };
        let s = spectra(44_100, mean, mean);
        let m = measure(&s, &v6_explicit());
        if !close(m.step_db, 65.0) || m.step_hz != Some(19_450.0) {
            failures.push(format!(
                "floor {floor}: D = {} at {:?}",
                m.step_db, m.step_hz
            ));
        }
        match (m.top_floor_db, m.residual_db) {
            (Some(ft), Some(res)) if close(ft, f_top) && close(res, r) => {}
            other => failures.push(format!(
                "floor {floor}: (F_top, R) = {other:?}, expected ({f_top}, {r})"
            )),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: The edge measurements are undefined without a step, a floor window or an
// [edge] section
// ---------------------------------------------------------------------------------------------

#[test]
fn rf04_edge_measurements_at_32_khz_have_d_q_but_no_floor() {
    // 32 kHz: Nyquist 16 kHz, the floor window [20900, 22000) holds no band. A step at 15050 Hz.
    let mean = |b: usize| match b {
        0..=149 => -30.0,
        150 => -45.0,
        _ => -60.0,
    };
    let s = spectra(32_000, mean, mean);
    let m = measure(&s, &v6_explicit());
    assert_eq!(m.step_hz, Some(15_050.0), "D = {}", m.step_db);
    assert!(m.median_step_db.is_some(), "D_Q is defined");
    assert_eq!(m.top_floor_db, None);
    assert_eq!(m.residual_db, None);
    assert!(!is_edge_step(&m, &v6_explicit()));
}

#[test]
fn rf04_edge_measurements_without_a_step_are_undefined() {
    let flat = |_b: usize| -30.0;
    let s = spectra(44_100, flat, flat);
    let m = measure(&s, &v6_explicit());
    assert_eq!((m.step_db, m.step_hz), (0.0, None));
    assert_eq!(m.median_step_db, None);
    assert_eq!(m.top_floor_db, None);
    assert_eq!(m.residual_db, None);
    assert!(!is_edge_step(&m, &v6_explicit()));
}

#[test]
fn rf04_edge_measurements_without_an_edge_section_are_undefined() {
    let mean = |b: usize| match b {
        0..=193 => -30.0,
        194 => -45.0,
        195..=208 => -55.0,
        _ => -70.0,
    };
    let s = spectra(44_100, mean, mean);
    let p = v5_like();
    let m = measure(&s, &p);
    assert_eq!(m.step_hz, Some(19_450.0));
    assert_eq!(m.median_step_db, None);
    assert_eq!(m.top_floor_db, None);
    assert_eq!(m.residual_db, None);
    assert!(!is_edge_step(&m, &p));
}

// ---------------------------------------------------------------------------------------------
// Hand-built measurements
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn tab(h: f64, g: f64, d: f64, fs: f64, u: f64, b: f64, d_q: f64, r: f64) -> Measurements {
    Measurements {
        noise_floor_db: -80.0,
        high_frequency_db: h,
        extent_hz: 22_050.0,
        step_db: d,
        step_hz: Some(fs),
        gap_db: g,
        upper_level_db: Some(u),
        upper_burst_db: Some(b),
        median_step_db: Some(d_q),
        top_floor_db: None,
        residual_db: Some(r),
    }
}

use SuspicionReason::{AmbiguousCutoff, HighBandGaps, LowHighFrequencyContent};
use Verdict::{Fake, Real, Suspicious};

/// One row of the conjunction table: case, D, fs, D_Q, R, edge step, verdict, reasons.
type Row = (
    &'static str,
    f64,
    f64,
    f64,
    f64,
    bool,
    Verdict,
    Vec<SuspicionReason>,
);

fn conjunction_rows() -> Vec<Row> {
    let a = vec![AmbiguousCutoff];
    vec![
        (
            "W4",
            14.22,
            19450.0,
            0.32,
            17.48,
            true,
            Suspicious,
            a.clone(),
        ),
        (
            "D_Q at max_median_step_db",
            14.22,
            19450.0,
            3.0,
            17.48,
            false,
            Fake,
            vec![],
        ),
        (
            "D_Q just below",
            14.22,
            19450.0,
            2.99,
            17.48,
            true,
            Suspicious,
            a.clone(),
        ),
        (
            "R at min_residual_db",
            14.22,
            19450.0,
            0.32,
            15.0,
            true,
            Suspicious,
            a.clone(),
        ),
        (
            "R just below",
            14.22,
            19450.0,
            0.32,
            14.99,
            false,
            Fake,
            vec![],
        ),
        (
            "D at max_step_db",
            16.0,
            19450.0,
            0.32,
            17.48,
            false,
            Fake,
            vec![],
        ),
        (
            "D just below max_step_db",
            15.99,
            19450.0,
            0.32,
            17.48,
            true,
            Suspicious,
            a.clone(),
        ),
        (
            "fs just inside the upper edge",
            14.22,
            20250.0,
            0.32,
            17.48,
            true,
            Suspicious,
            a.clone(),
        ),
        (
            "fs at the upper edge (V8)",
            14.22,
            20300.0,
            0.32,
            17.48,
            false,
            Suspicious,
            a.clone(),
        ),
        (
            "fs just below the range (V7)",
            14.22,
            14450.0,
            0.32,
            17.48,
            false,
            Suspicious,
            a.clone(),
        ),
        (
            "low-pass: floor right above",
            30.0,
            19450.0,
            -5.0,
            0.5,
            false,
            Fake,
            vec![],
        ),
        (
            "low-pass with holes",
            13.0,
            19450.0,
            0.0,
            0.5,
            false,
            Fake,
            vec![],
        ),
        (
            "low-pass that shows in the median",
            13.0,
            19450.0,
            12.0,
            20.0,
            false,
            Fake,
            vec![],
        ),
        (
            "deep low-pass with a residue",
            35.0,
            19450.0,
            -8.0,
            22.0,
            false,
            Fake,
            vec![],
        ),
    ]
}

// Scenario Outline: A lossy-range step is an edge step only when the three conditions hold
#[test]
fn rf04_edge_step_conjunction_flag() {
    let p = v6_explicit();
    let mut failures = Vec::new();
    for (case, d, fs, d_q, r, edge, _, _) in conjunction_rows() {
        let m = tab(-20.0, -0.25, d, fs, -47.06, 2.86, d_q, r);
        if is_edge_step(&m, &p) != edge {
            failures.push(format!("{case}: expected edge step = {edge}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn rf04_edge_step_conjunction_verdicts() {
    let p = v6_explicit();
    let mut failures = Vec::new();
    for (case, d, fs, d_q, r, _, verdict, reasons) in conjunction_rows() {
        let m = tab(-20.0, -0.25, d, fs, -47.06, 2.86, d_q, r);
        let got = decide(&m, &p);
        if got != (verdict, reasons.clone()) {
            failures.push(format!("{case}: {got:?}, expected {verdict:?} {reasons:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

// Scenario Outline: An edge step only turns Fake into Suspicious
#[test]
fn rf04_edge_step_only_turns_fake_into_suspicious() {
    // (case, H, G, D, fs, U, B, D_Q, R, verdict, reasons)
    #[allow(clippy::type_complexity)]
    let rows: Vec<(&str, [f64; 8], Verdict, Vec<SuspicionReason>)> = vec![
        (
            "edge step with a gap: V3 comes first",
            [-20.0, 8.0, 14.22, 19450.0, -47.06, 2.86, 0.32, 17.48],
            Fake,
            vec![],
        ),
        (
            "edge step with a weak gap: V5",
            [-20.0, 5.0, 14.22, 19450.0, -47.06, 2.86, 0.32, 17.48],
            Suspicious,
            vec![HighBandGaps],
        ),
        (
            "edge step with little treble: V2",
            [-65.0, -0.25, 14.22, 19450.0, -47.06, 2.86, 0.32, 17.48],
            Suspicious,
            vec![LowHighFrequencyContent],
        ),
        (
            "W5: below step_fake_db (V6)",
            [-20.0, -2.51, 9.88, 20150.0, -32.64, 17.29, 1.02, 22.35],
            Suspicious,
            vec![AmbiguousCutoff],
        ),
        (
            "W6: gap rule V5",
            [-20.0, 5.65, 7.39, 16550.0, -51.13, 2.37, -0.24, 26.72],
            Suspicious,
            vec![HighBandGaps],
        ),
        (
            "W1: deepest step above the lossy range",
            [-20.0, 18.39, 8.93, 20450.0, -56.64, 9.53, 1.11, 14.15],
            Suspicious,
            vec![HighBandGaps],
        ),
        (
            "small lossy step, no gap: nothing fires",
            [-20.0, 1.0, 5.0, 18050.0, -47.06, 2.86, 0.32, 17.48],
            Real,
            vec![],
        ),
    ];
    let p = v6_explicit();
    let mut failures = Vec::new();
    for (case, v, verdict, reasons) in rows {
        let m = tab(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]);
        let got = decide(&m, &p);
        if got != (verdict, reasons.clone()) {
            failures.push(format!("{case}: {got:?}, expected {verdict:?} {reasons:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

// Scenario: Without an [edge] section the edge step is disabled whatever D_Q and R are
#[test]
fn rf04_without_an_edge_section_the_edge_step_is_disabled() {
    let p = v5_like();
    let m = tab(-20.0, -0.25, 14.22, 19450.0, -47.06, 2.86, 0.32, 17.48);
    assert!(!is_edge_step(&m, &p));
    assert_eq!(decide(&m, &p), (Fake, vec![]));
}

//! Fixture-level scenarios of `specs/features/detection-edge-step.feature` (@RF-04): the synthetic
//! signals of `specs/detection/fixtures.md`, "Edge step" (generators in `tests/common/mod.rs`),
//! analysed with the parameters of `detection-params.v5.toml` and with DetectionParams v6 (the
//! defaults). The rule-level scenarios are in `crates/fakeflac-core/tests/detect_edge.rs`, the
//! parameter files in `params_edge.rs`. Spec: `specs/detection/rules.md`, ADR-0042.
//!
//! The two "Confirmed by the test stage" scenarios measure the fixtures first; if they do not hold,
//! the spec returns to G1.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{is_edge_step, measure, Measurements};
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{DetectionParams, SuspicionReason, TrackAnalysis, Verdict};

fn v5() -> DetectionParams {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection/detection-params.v5.toml");
    toml::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

fn analyze(name: &str, params: &DetectionParams) -> TrackAnalysis {
    let path = fixture(name, Container::Flac);
    analyze_track(&SymphoniaAnalyzer, &entry(&path), params)
}

fn measurements(name: &str, params: &DetectionParams) -> Measurements {
    let path = fixture(name, Container::Flac);
    let probe = SymphoniaAnalyzer.analyze(&path, params).unwrap();
    measure(&probe.spectrum.expect("the fixture has spectra"), params)
}

// ---- Scenario Outline: Edge-step fixtures -------------------------------------------------------

fn edge_fixture(id: &str, v6_verdict: Verdict, v6_reasons: &[SuspicionReason], edge: bool, cutoff: f32) {
    let (old, new) = (analyze(id, &v5()), analyze(id, &DetectionParams::default()));
    assert_eq!(old.verdict, Verdict::Fake, "{id}: v5 {old:#?}");
    assert!(old.reasons.is_empty(), "{id}: v5 reasons");
    assert_eq!(new.verdict, v6_verdict, "{id}: v6 {new:#?}");
    assert_eq!(new.reasons, v6_reasons, "{id}: v6 reasons");
    let m = measurements(id, &DetectionParams::default());
    assert_eq!(is_edge_step(&m, &DetectionParams::default()), edge, "{id}: edge step");
    for (which, a) in [("v5", &old), ("v6", &new)] {
        let c = a.cutoff.as_ref().expect("cutoff reported").frequency_hz;
        assert!((c - cutoff).abs() <= 200.0, "{id} {which}: cutoff {c}, expected {cutoff} ± 200");
    }
}

#[test]
fn rf04_edge_bump19k3() {
    edge_fixture(
        "edge_bump19k3",
        Verdict::Suspicious,
        &[SuspicionReason::AmbiguousCutoff],
        true,
        19_450.0,
    );
}
#[test]
fn rf04_edge_bump19k3_lp19k5() {
    edge_fixture("edge_bump19k3_lp19k5", Verdict::Fake, &[], false, 19_500.0);
}
#[test]
fn rf04_holes_lp19k5() {
    edge_fixture("holes_lp19k5", Verdict::Fake, &[], false, 19_500.0);
}
#[test]
fn rf04_shelf19k5_edge_row() {
    edge_fixture("shelf19k5", Verdict::Fake, &[], false, 19_500.0);
}

// ---- Scenario: The content-edge fixture meets the three conditions ------------------------------

#[test]
fn rf04_the_content_edge_fixture_meets_the_three_conditions() {
    let m = measurements("edge_bump19k3", &DetectionParams::default());
    let fs = m.step_hz.expect("fs defined");
    assert!((19_350.0..=19_550.0).contains(&fs), "fs {fs}");
    assert!((10.0..16.0).contains(&m.step_db), "D {}", m.step_db);
    let d_q = m.median_step_db.expect("D_Q defined");
    assert!(d_q < 3.0, "D_Q {d_q}");
    let r = m.residual_db.expect("R defined");
    assert!(r >= 15.0, "R {r}");
}

// ---- Scenario: A low-pass with quantiser holes is stopped by the residual alone ------------------

#[test]
fn rf04_a_low_pass_with_holes_is_stopped_by_the_residual_alone() {
    let p = DetectionParams::default();
    let m = measurements("holes_lp19k5", &p);
    let fs = m.step_hz.expect("fs defined");
    assert!((p.verdict.lossy_min_cutoff_hz..p.verdict.fake_max_cutoff_hz).contains(&fs), "fs {fs}");
    assert!((10.0..16.0).contains(&m.step_db), "D {}", m.step_db);
    let d_q = m.median_step_db.expect("D_Q defined");
    assert!(d_q < 3.0, "D_Q {d_q}");
    let r = m.residual_db.expect("R defined");
    assert!(r < 15.0, "R {r}");
    assert_eq!(analyze("holes_lp19k5", &p).verdict, Verdict::Fake);
}

// ---- Scenario: v6 gives the v5 verdicts on every other fixture ------------------------------------

#[test]
fn rf04_v6_gives_the_v5_verdicts_on_every_other_fixture() {
    let signals = [
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
        "edge_bump19k3_lp19k5",
        "holes_lp19k5",
    ];
    let (old_params, new_params) = (v5(), DetectionParams::default());
    let mut failures = Vec::new();
    for name in signals {
        let (old, new) = (analyze(name, &old_params), analyze(name, &new_params));
        let same = old.verdict == new.verdict
            && old.reasons == new.reasons
            && old.cutoff == new.cutoff
            && old.error == new.error;
        if !same || old.algorithm_version != 5 || new.algorithm_version != 6 {
            failures.push(format!("{name}: v5 {old:?} / v6 {new:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---- Fixture preconditions, by a reference computation in the test --------------------------------
//
// "The test stage confirms the measurements first; if they do not hold, it stops for G1." These
// tests compute D_Q, F_top and R straight from the band spectra with the definitions of rules.md
// section 2 (independent of the production code, which has not implemented them yet), so they
// pass as soon as the generators in `tests/common/mod.rs` are right.

struct Reference {
    fs: f64,
    d: f64,
    d_q: f64,
    r: f64,
}

fn reference(name: &str) -> Reference {
    let p = DetectionParams::default();
    let path = fixture(name, Container::Flac);
    let spectra = SymphoniaAnalyzer
        .analyze(&path, &p)
        .unwrap()
        .spectrum
        .expect("spectra");
    let m = measure(&spectra, &p);
    let fs = m.step_hz.expect("fs defined");
    let band_hz = spectra.band_hz;
    let i = (fs / band_hz).floor() as usize; // band whose centre is fs
    let w = p.step.window_bands as usize;
    let mean = |v: &[f64], a: usize, b: usize| v[a..=b].iter().sum::<f64>() / (b - a + 1) as f64;
    let q = &spectra.median_db;
    let below_q = mean(q, i - w, i - 1);
    let above_q = mean(q, i + 1, i + w);
    let slope_q = (mean(q, i - 2 * w, i - w - 1) - below_q).max(0.0);
    let d_q = below_q - above_q - slope_q;
    // L_ref: median of M over the reference bands.
    let centre = |b: usize| (b as f64 + 0.5) * band_hz;
    let mut ref_values: Vec<f64> = (0..spectra.mean_db.len())
        .filter(|&b| centre(b) >= p.bands.ref_band_low_hz && centre(b) <= p.bands.ref_band_high_hz)
        .map(|b| spectra.mean_db[b])
        .collect();
    ref_values.sort_by(f64::total_cmp);
    let l_ref = ref_values[(ref_values.len() - 1) / 2 + usize::from(ref_values.len() % 2 == 0)];
    let edge = p.edge.clone().unwrap_or(fakeflac_core::params::EdgeParams {
        floor_low_hz: 20_900.0,
        floor_high_hz: 22_000.0,
        floor_min_db: -104.0,
        max_median_step_db: 3.0,
        min_residual_db: 15.0,
        max_step_db: 16.0,
    });
    let top_hz = spectra.nyquist_hz() - p.bands.nyquist_guard_hz;
    let mut floor: Vec<f64> = (0..spectra.mean_db.len())
        .filter(|&b| {
            centre(b) >= edge.floor_low_hz && centre(b) < edge.floor_high_hz && centre(b) <= top_hz
        })
        .map(|b| spectra.mean_db[b] - l_ref)
        .collect();
    floor.sort_by(f64::total_cmp);
    let f_top = floor[floor.len() / 2].max(edge.floor_min_db - l_ref);
    let above = mean(&spectra.mean_db, i + 1, i + w) - l_ref;
    Reference {
        fs,
        d: m.step_db,
        d_q,
        r: above - f_top,
    }
}

#[test]
fn rf04_precondition_edge_bump19k3_by_reference_computation() {
    let r = reference("edge_bump19k3");
    assert!((19_350.0..=19_550.0).contains(&r.fs), "fs {}", r.fs);
    assert!((10.0..16.0).contains(&r.d), "D {}", r.d);
    assert!(r.d_q < 3.0, "D_Q {}", r.d_q);
    assert!(r.r >= 15.0, "R {}", r.r);
}

#[test]
fn rf04_precondition_holes_lp19k5_by_reference_computation() {
    let p = DetectionParams::default();
    let r = reference("holes_lp19k5");
    assert!(
        (p.verdict.lossy_min_cutoff_hz..p.verdict.fake_max_cutoff_hz).contains(&r.fs),
        "fs {}",
        r.fs
    );
    assert!((10.0..16.0).contains(&r.d), "D {}", r.d);
    assert!(r.d_q < 3.0, "D_Q {}", r.d_q);
    assert!(r.r < 15.0, "R {}", r.r);
}

#[test]
fn rf04_precondition_edge_bump19k3_lp19k5_by_reference_computation() {
    let r = reference("edge_bump19k3_lp19k5");
    assert!(r.d >= 16.0, "D {} (the cap)", r.d);
    assert!(r.r < 15.0, "R {}", r.r);
}

//! Rule-level tests of `specs/detection/rules.md` (sections 1 item 6, 2, 3 and 4) on hand-built
//! band spectra, hand-built measurements and hand-built parameters, plus the supported extensions
//! (RF-14). Scenarios: `specs/features/detection-lossless.feature` (@RF-03 @RF-04) and, for the
//! full-band guard (v3, amended by v4), `specs/features/detection-full-band.feature` (@RF-04), at
//! the end of this file. Protected after the `test(RF-03,RF-04)` commit; the v3 changes to the
//! v2-derived tests listed in ADR-0038 and the v4 changes listed in ADR-0039 ("Consequences") were
//! made in the `test(RF-04)` commits of detection v3 and v4.
//!
//! The spectra are written for bands of 100 Hz at 44.1 kHz: band `b` has its centre at
//! `(b + 0.5) * 100` Hz, there are 220 bands and `top` is band 219 (centre 21 950 Hz).
//! Useful landmarks: band 100 starts at 10 kHz, band 160 starts at 16 kHz.

#![allow(clippy::expect_used)]

use fakeflac_core::detect::{cutoff_estimate, decide, detect, is_full_band, measure, Measurements};
use fakeflac_core::params::{FullBandParams, ParamsError};
use fakeflac_core::{AudioFormat, BandSpectra, DetectionParams, SuspicionReason, Verdict};

fn params() -> DetectionParams {
    DetectionParams::default()
}

/// Band spectra with `mean(b)` and `median(b)` per band index, for any sample rate.
fn spectra_at(
    sample_rate: u32,
    mean: impl Fn(usize) -> f64,
    median: impl Fn(usize) -> f64,
) -> BandSpectra {
    let band_hz = params().fft.band_hz;
    assert_eq!(band_hz, 100.0, "these tests are written for 100 Hz bands");
    let n = (f64::from(sample_rate) / 2.0 / band_hz).floor() as usize;
    BandSpectra {
        sample_rate,
        band_hz,
        mean_db: (0..n).map(&mean).collect(),
        median_db: (0..n).map(&median).collect(),
    }
}

/// 44.1 kHz, the median spectrum equal to the mean spectrum.
fn spectra(level: impl Fn(usize) -> f64 + Copy) -> BandSpectra {
    spectra_at(44_100, level, level)
}

/// 44.1 kHz, flat median spectrum: only the mean spectrum carries a shape.
fn mean_only(level: impl Fn(usize) -> f64) -> BandSpectra {
    spectra_at(44_100, level, |_| 0.0)
}

/// 44.1 kHz, flat mean spectrum: only the median spectrum carries a shape.
fn median_only(level: impl Fn(usize) -> f64) -> BandSpectra {
    spectra_at(44_100, |_| 0.0, level)
}

fn near(got: f64, want: f64) {
    assert!((got - want).abs() < 1e-9, "got {got}, want {want}");
}

// ---------------------------------------------------------------------------------------------
// Section 2: measurements (@RF-03)
// ---------------------------------------------------------------------------------------------

// Scenario Outline: Cutoff estimate (@RF-03). L_ref is the median of M over the bands with centre
// in [ref_band_low_hz, ref_band_high_hz] and all levels are relative to it. 60 reference bands at
// +10 dB and 30 at -5 dB give L_ref = +10 (their mean would be +5); the treble sits 30 dB below.
// Bands below ref_band_low_hz are loud and must be ignored.
#[test]
fn rf03_levels_are_relative_to_the_median_of_the_reference_bands() {
    let s = spectra(|b| match b {
        0..=9 => 50.0,
        10..=69 => 10.0,
        70..=99 => -5.0,
        _ => -20.0,
    });
    near(measure(&s, &params()).high_frequency_db, -30.0);
}

// H is the median of M over the bands with centre in [hf_band_low_hz, hf_band_high_hz]:
// 60 bands (100..=159), 25 of them at -40 dB and 35 at -80 dB give a median of -80; bands outside
// that range (b >= 160) are loud and must be ignored.
#[test]
fn rf03_high_frequency_content_is_the_median_over_the_hf_range() {
    let s = spectra(|b| match b {
        0..=99 => 0.0,
        100..=124 => -40.0,
        125..=159 => -80.0,
        _ => 20.0,
    });
    near(measure(&s, &params()).high_frequency_db, -80.0);
}

// F is the noise_floor_percentile percentile of M over the bands with centre in
// [ref_band_low_hz, nyquist - nyquist_guard_hz]: 3 outliers at -90 and 30 bands at -50 among 210
// bands put the 5th percentile at -50. The 10 bands below 1 kHz at -200 are outside the range.
#[test]
fn rf03_noise_floor_is_a_low_percentile_over_the_analysed_range() {
    let s = spectra(|b| match b {
        0..=9 => -200.0,
        10..=186 => 0.0,
        190..=192 => -90.0,
        _ => -50.0,
    });
    near(measure(&s, &params()).noise_floor_db, -50.0);
}

// fe: a spectrum that reaches the top band has its extent at Nyquist.
#[test]
fn rf03_extent_of_a_full_band_spectrum_is_nyquist() {
    let s = spectra(|_| 0.0);
    near(measure(&s, &params()).extent_hz, 22_050.0);
}

// fe: scanning down from `top`, the lower edge of the first band of the first wide-enough run
// (the highest band of the run). Content up to band 159 -> 15 900 Hz.
#[test]
fn rf03_extent_is_the_lower_edge_of_the_highest_band_of_the_run() {
    let s = spectra(|b| if b < 160 { 0.0 } else { -100.0 });
    near(measure(&s, &params()).extent_hz, 15_900.0);
}

// fe: a run narrower than content_min_width_hz (one 100 Hz band) is not content.
#[test]
fn rf03_extent_ignores_a_run_narrower_than_the_minimum_width() {
    let s = spectra(|b| match b {
        0..=99 => 0.0,
        219 => -30.0,
        _ => -100.0,
    });
    near(measure(&s, &params()).extent_hz, 9_900.0);
}

// fe: a run of exactly content_min_width_hz that starts at `top` means Nyquist.
#[test]
fn rf03_extent_run_starting_at_top_is_nyquist() {
    let s = spectra(|b| if b <= 99 || b >= 218 { 0.0 } else { -100.0 });
    near(measure(&s, &params()).extent_hz, 22_050.0);
}

// fe: content is "M > -content_drop_db" (60 dB): -59 dB is content, -61 dB is not.
#[test]
fn rf03_extent_threshold_is_content_drop_db() {
    let above = spectra(|b| if b < 160 { 0.0 } else { -59.0 });
    near(measure(&above, &params()).extent_hz, 22_050.0);
    let below = spectra(|b| if b < 160 { 0.0 } else { -61.0 });
    near(measure(&below, &params()).extent_hz, 15_900.0);
}

// d(i): the transition band is left out of both windows. Band 150 sits half-way (-20 dB) between
// 0 dB below and -40 dB above: d(150) = 0 - (-40) = 40, while the neighbours see the half-way
// band inside a window and score 33.3.
#[test]
fn rf03_step_leaves_the_transition_band_out_of_both_windows() {
    let s = mean_only(|b| match b {
        ..=149 => 0.0,
        150 => -20.0,
        _ => -40.0,
    });
    let m = measure(&s, &params());
    near(m.step_db, 40.0);
    assert_eq!(m.step_hz, Some(15_050.0));
}

// Cutoff step D: the lowest band wins on ties. A brick wall between bands 159 and 160 scores
// d = 100 at both (a window of three bands on each side of the transition band).
#[test]
fn rf03_step_ties_go_to_the_lowest_band() {
    let s = mean_only(|b| if b < 160 { 0.0 } else { -100.0 });
    let m = measure(&s, &params());
    near(m.step_db, 100.0);
    assert_eq!(m.step_hz, Some(15_950.0));
}

// Slope discount: a roll-off that was already under way (3 dB per band from band 100) is not a
// step. Each candidate scores 4 * 3 - 3 * 3 = 3 dB instead of 12.
#[test]
fn rf03_step_discounts_a_rolloff_already_under_way() {
    let s = mean_only(|b| {
        if b <= 100 {
            0.0
        } else {
            -3.0 * (b as f64 - 100.0)
        }
    });
    let m = measure(&s, &params());
    near(m.step_db, 3.0);
    assert!(m.step_hz.is_some());
}

// Slope discount: max(0, ...) means that a level rising into the step does not add to it.
// Bands 147..=149 are 20 dB above their surroundings; d(150) = 20 (not 40).
#[test]
fn rf03_step_slope_discount_never_increases_the_step() {
    let s = mean_only(|b| match b {
        147..=149 => 0.0,
        _ => -20.0,
    });
    let m = measure(&s, &params());
    near(m.step_db, 20.0);
    assert_eq!(m.step_hz, Some(15_050.0));
}

// Rebound: content that comes back 6 dB or more above the 'above' level higher up means the drop
// is not where the content ends. The 16 kHz drop is rejected; the real end is band 199/200.
#[test]
fn rf03_step_rejects_a_drop_when_content_comes_back_higher_up() {
    let s = mean_only(|b| match b {
        ..=159 => 0.0,
        190..=199 => -20.0,
        _ => -40.0,
    });
    let m = measure(&s, &params());
    near(m.step_db, 20.0);
    assert_eq!(m.step_hz, Some(19_950.0));
}

// Rebound: content that comes back by less than rebound_db (4 dB here) does not disqualify.
#[test]
fn rf03_step_keeps_a_drop_when_the_rebound_is_below_the_threshold() {
    let s = mean_only(|b| match b {
        ..=159 => 0.0,
        190..=199 => -36.0,
        _ => -40.0,
    });
    let m = measure(&s, &params());
    near(m.step_db, 40.0);
    assert_eq!(m.step_hz, Some(15_950.0));
}

// Search range: only transition bands with centre >= search_low_hz (11 kHz) are candidates; a
// drop at 10 kHz is not one. Nothing else varies, so D = 0 and fs is undefined.
#[test]
fn rf03_step_below_the_search_range_is_not_a_candidate() {
    let s = mean_only(|b| if b < 100 { 0.0 } else { -40.0 });
    let m = measure(&s, &params());
    near(m.step_db, 0.0);
    assert_eq!(m.step_hz, None);
}

// Search range: the highest candidate has its centre at nyquist - search_top_margin_hz
// (21 650 Hz, band 216). Its 'above' window (217, 218, 219) sees a drop at band 218.
// A larger margin takes that band out of the search: nothing else varies.
#[test]
fn rf03_step_search_stops_at_the_top_margin() {
    let s = mean_only(|b| if b <= 217 { 0.0 } else { -40.0 });
    let m = measure(&s, &params());
    near(m.step_db, 80.0 / 3.0);
    assert_eq!(m.step_hz, Some(21_650.0));

    let mut p = params();
    p.step.search_top_margin_hz = 1_000.0;
    let m = measure(&s, &p);
    near(m.step_db, 0.0);
    assert_eq!(m.step_hz, None);
}

// D <= 0 -> D = 0 and fs undefined: a flat spectrum.
#[test]
fn rf03_flat_spectrum_has_no_step() {
    let m = measure(&spectra(|_| 0.0), &params());
    near(m.step_db, 0.0);
    assert_eq!(m.step_hz, None);
}

// D <= 0 -> D = 0 and fs undefined: every candidate scores below zero when the level rises.
#[test]
fn rf03_rising_spectrum_has_no_step() {
    let m = measure(&mean_only(|b| 0.1 * b as f64), &params());
    near(m.step_db, 0.0);
    assert_eq!(m.step_hz, None);
}

// The window length comes from the parameters (step.window_bands). A two-stage drop (0 dB, then
// -30 dB at bands 150-151, then -60 dB) gives D = 50 at band 150 with three bands per window, and
// D = 30 at band 149 (lowest of the ties) with one band per window.
#[test]
fn rf03_step_window_is_a_parameter() {
    let s = mean_only(|b| match b {
        ..=149 => 0.0,
        150..=151 => -30.0,
        _ => -60.0,
    });
    let m = measure(&s, &params());
    near(m.step_db, 50.0);
    assert_eq!(m.step_hz, Some(15_050.0));

    let mut p = params();
    p.step.window_bands = 1;
    let m = measure(&s, &p);
    near(m.step_db, 30.0);
    assert_eq!(m.step_hz, Some(14_950.0));
}

// Higher sample rates use the same absolute frequencies (rules.md section 3): at 96 kHz there
// are 480 bands and a brick wall at 16 kHz is still found at the same place.
#[test]
fn rf03_hi_res_spectra_use_absolute_frequencies() {
    let s = spectra_at(96_000, |b| if b < 160 { 0.0 } else { -100.0 }, |_| 0.0);
    let m = measure(&s, &params());
    near(m.step_db, 100.0);
    assert_eq!(m.step_hz, Some(15_950.0));
    near(m.extent_hz, 15_900.0);
}

// G at 16 kHz on the median spectrum: q(15 300, 16 000) - q(16 000, 16 700) = 0 - (-20).
#[test]
fn rf03_gap_step_is_the_drop_of_the_median_spectrum_at_the_edge() {
    let s = median_only(|b| if b < 160 { 0.0 } else { -20.0 });
    near(measure(&s, &params()).gap_db, 20.0);
}

// G uses the median spectrum Q, not the mean spectrum M: a step only in M gives G = 0.
#[test]
fn rf03_gap_step_ignores_the_mean_spectrum() {
    let s = mean_only(|b| if b < 160 { 0.0 } else { -20.0 });
    near(measure(&s, &params()).gap_db, 0.0);
}

// G slope discount: the 700 Hz band below the comparison band is 10 dB higher, so a roll-off was
// already under way: G = 20 - 10.
#[test]
fn rf03_gap_step_discounts_a_rolloff_already_under_way() {
    let s = median_only(|b| match b {
        146..=152 => 10.0,
        ..=159 => 0.0,
        _ => -20.0,
    });
    near(measure(&s, &params()).gap_db, 10.0);
}

// G slope discount: max(0, ...) means a lower level below does not add to G.
#[test]
fn rf03_gap_step_slope_discount_never_increases_the_gap() {
    let s = median_only(|b| match b {
        146..=152 => -10.0,
        ..=159 => 0.0,
        _ => -20.0,
    });
    near(measure(&s, &params()).gap_db, 20.0);
}

// G is a median across bands: a single tone on either side of the edge does not count as a step.
#[test]
fn rf03_gap_step_is_robust_to_a_single_tone() {
    let s = median_only(|b| match b {
        156 | 163 => 40.0,
        ..=159 => 0.0,
        _ => -20.0,
    });
    near(measure(&s, &params()).gap_db, 20.0);
}

// G edge: gaps.edge_hz is an absolute frequency read from the parameters.
#[test]
fn rf03_gap_edge_comes_from_the_parameters() {
    let s = median_only(|b| if b < 80 { 0.0 } else { -20.0 });
    near(measure(&s, &params()).gap_db, 0.0);
    let mut p = params();
    p.gaps.edge_hz = 8_000.0;
    near(measure(&s, &p).gap_db, 20.0);
}

// nyquist < e + g -> G = 0. At 33 kHz Nyquist is 16 500 Hz, below 16 000 + 700, even though the
// median spectrum drops at 16 kHz.
#[test]
fn rf03_gap_step_is_zero_when_nyquist_is_below_edge_plus_band() {
    let s = spectra_at(33_000, |_| 0.0, |b| if b < 160 { 0.0 } else { -20.0 });
    near(measure(&s, &params()).gap_db, 0.0);
}

// ---------------------------------------------------------------------------------------------
// Section 2: reported cutoff (@RF-03)
// ---------------------------------------------------------------------------------------------

fn clean() -> Measurements {
    Measurements {
        noise_floor_db: -80.0,
        high_frequency_db: -20.0,
        extent_hz: 22_050.0,
        step_db: 0.0,
        step_hz: None,
        gap_db: 0.0,
        upper_level_db: None,
        upper_burst_db: None,
        median_step_db: None,
        top_floor_db: None,
        residual_db: None,
    }
}

// Scenario Outline: Cutoff estimate (@RF-03). D >= step_suspicious_db: the cutoff is fs and its
// steepness is D over the distance between the centres of the two windows,
// (3 + 1) * 100 Hz = 0.4 kHz: 8 dB / 0.4 kHz = 20 dB/kHz.
#[test]
fn rf03_reported_cutoff_is_the_step_when_it_is_deep_enough() {
    let m = Measurements {
        step_db: 8.0,
        step_hz: Some(15_950.0),
        noise_floor_db: -63.0,
        ..clean()
    };
    let c = cutoff_estimate(&m, &params());
    assert_eq!(c.frequency_hz, 15_950.0);
    assert!((c.steepness_db_per_khz - 20.0).abs() < 1e-3);
    assert_eq!(c.noise_floor_db, -63.0);
}

// The limit is inclusive: D == step_suspicious_db reports the step.
#[test]
fn rf03_reported_cutoff_step_limit_is_inclusive() {
    let p = params();
    let m = Measurements {
        step_db: p.verdict.step_suspicious_db,
        step_hz: Some(17_050.0),
        ..clean()
    };
    assert_eq!(cutoff_estimate(&m, &p).frequency_hz, 17_050.0);
}

// Scenario: Signal without a cutoff step reports the Nyquist frequency as cutoff (@RF-03):
// D below step_suspicious_db: the cutoff is fe and the steepness 0, even if fs is defined.
#[test]
fn rf03_reported_cutoff_is_the_extent_when_there_is_no_deep_step() {
    let p = params();
    let m = Measurements {
        step_db: p.verdict.step_suspicious_db - 0.01,
        step_hz: Some(15_950.0),
        extent_hz: 22_050.0,
        noise_floor_db: -70.0,
        ..clean()
    };
    let c = cutoff_estimate(&m, &p);
    assert_eq!(c.frequency_hz, 22_050.0);
    assert_eq!(c.steepness_db_per_khz, 0.0);
    assert_eq!(c.noise_floor_db, -70.0);
}

// The steepness distance comes from the parameters: a window of 1 band is 0.2 kHz.
#[test]
fn rf03_reported_steepness_uses_the_window_and_band_parameters() {
    let mut p = params();
    p.step.window_bands = 1;
    let m = Measurements {
        step_db: 10.0,
        step_hz: Some(15_950.0),
        ..clean()
    };
    assert!((cutoff_estimate(&m, &p).steepness_db_per_khz - 50.0).abs() < 1e-3);
}

// ---------------------------------------------------------------------------------------------
// Section 3: verdict table V2 to V10 (@RF-04). First match wins.
// ---------------------------------------------------------------------------------------------

use SuspicionReason::{AmbiguousCutoff, GradualRolloff, HighBandGaps, LowHighFrequencyContent};

fn verdict_of(m: &Measurements) -> (Verdict, Vec<SuspicionReason>) {
    decide(m, &params())
}

/// A step of depth `d` at `fs`, everything else clean.
fn step(d: f64, fs: f64) -> Measurements {
    Measurements {
        step_db: d,
        step_hz: Some(fs),
        ..clean()
    }
}

// Scenario Outline: Verdict by signal (@RF-04): V10, nothing found.
#[test]
fn rf04_v10_clean_measurements_are_real() {
    assert_eq!(verdict_of(&clean()), (Verdict::Real, vec![]));
}

// V2: H below low_hf_threshold_db -> Suspicious(LowHighFrequencyContent).
#[test]
fn rf04_v2_little_treble_is_suspicious() {
    let p = params();
    let m = Measurements {
        high_frequency_db: p.verdict.low_hf_threshold_db - 0.01,
        ..clean()
    };
    assert_eq!(
        decide(&m, &p),
        (Verdict::Suspicious, vec![LowHighFrequencyContent])
    );
}

// V2: the limit is strict: H == low_hf_threshold_db is not "little treble".
#[test]
fn rf04_v2_limit_is_strict() {
    let p = params();
    let m = Measurements {
        high_frequency_db: p.verdict.low_hf_threshold_db,
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// V2 beats V3, V4 and V5: never Fake only because of the spectrum shape.
#[test]
fn rf04_v2_beats_every_later_row() {
    let m = Measurements {
        high_frequency_db: -90.0,
        gap_db: 100.0,
        step_db: 100.0,
        step_hz: Some(16_000.0),
        ..clean()
    };
    assert_eq!(
        verdict_of(&m),
        (Verdict::Suspicious, vec![LowHighFrequencyContent])
    );
}

// V3: G >= gap_fake_db -> Fake, whatever the step (fs undefined here).
#[test]
fn rf04_v3_large_gap_step_is_fake() {
    let p = params();
    let m = Measurements {
        gap_db: p.verdict.gap_fake_db,
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// V3 beats V5 and V6: a gap step at the fake limit with a step that would only be suspicious
// gives Fake, with no reasons.
#[test]
fn rf04_v3_beats_v5_and_v6() {
    let p = params();
    let m = Measurements {
        gap_db: p.verdict.gap_fake_db,
        step_db: p.verdict.step_suspicious_db,
        step_hz: Some(16_000.0),
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// V4: D >= step_fake_db with fs in the lossy range -> Fake.
#[test]
fn rf04_v4_deep_step_in_the_lossy_range_is_fake() {
    let p = params();
    let m = step(p.verdict.step_fake_db, 16_000.0);
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// V4: the lossy range is [lossy_min_cutoff_hz, fake_max_cutoff_hz): lower limit inclusive...
#[test]
fn rf04_v4_lower_limit_of_the_range_is_inclusive() {
    let p = params();
    let m = step(p.verdict.step_fake_db, p.verdict.lossy_min_cutoff_hz);
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// ...and the upper limit exclusive: fs == fake_max_cutoff_hz with D = 10 is in no row.
#[test]
fn rf04_v4_upper_limit_of_the_range_is_exclusive() {
    let p = params();
    let m = step(p.verdict.step_fake_db, p.verdict.fake_max_cutoff_hz);
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
    let m = step(p.verdict.step_fake_db, p.verdict.fake_max_cutoff_hz - 1.0);
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// V4 beats V5: a deep step in range with a gap step in the suspicious band is Fake.
#[test]
fn rf04_v4_beats_v5() {
    let p = params();
    let m = Measurements {
        gap_db: p.verdict.gap_suspicious_db,
        ..step(p.verdict.step_fake_db, 16_000.0)
    };
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// V5: G >= gap_suspicious_db -> Suspicious(HighBandGaps).
#[test]
fn rf04_v5_moderate_gap_step_is_suspicious() {
    let p = params();
    let m = Measurements {
        gap_db: p.verdict.gap_suspicious_db,
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![HighBandGaps]));
}

// V5: just below gap_suspicious_db and no step -> Real.
#[test]
fn rf04_v5_limit_is_inclusive_and_below_it_is_real() {
    let p = params();
    let m = Measurements {
        gap_db: p.verdict.gap_suspicious_db - 0.01,
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// V5 comes before V6: both apply, only HighBandGaps is reported.
#[test]
fn rf04_v5_beats_v6() {
    let p = params();
    let m = Measurements {
        gap_db: p.verdict.gap_suspicious_db,
        ..step(p.verdict.step_suspicious_db + 1.0, 16_000.0)
    };
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![HighBandGaps]));
}

// V6: D >= step_suspicious_db in the lossy range -> Suspicious(AmbiguousCutoff).
#[test]
fn rf04_v6_moderate_step_in_the_lossy_range_is_suspicious() {
    let p = params();
    for d in [p.verdict.step_suspicious_db, p.verdict.step_fake_db - 0.01] {
        let m = step(d, 16_000.0);
        assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![AmbiguousCutoff]));
    }
}

// V6: below step_suspicious_db the step is not enough (and fe is high) -> Real.
#[test]
fn rf04_v6_below_the_limit_is_real() {
    let p = params();
    let m = step(p.verdict.step_suspicious_db - 0.01, 16_000.0);
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// V7: D >= step_low_suspicious_db with fs below the lossy range -> Suspicious(AmbiguousCutoff).
#[test]
fn rf04_v7_deep_step_below_the_lossy_range_is_suspicious() {
    let p = params();
    let m = step(p.verdict.step_low_suspicious_db, 14_000.0);
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![AmbiguousCutoff]));
    // fs just under the lossy range
    let m = step(
        p.verdict.step_low_suspicious_db,
        p.verdict.lossy_min_cutoff_hz - 1.0,
    );
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![AmbiguousCutoff]));
}

// V7: a step shallower than step_low_suspicious_db below the range matches nothing.
#[test]
fn rf04_v7_shallower_step_below_the_range_is_real() {
    let p = params();
    let m = step(p.verdict.step_low_suspicious_db - 0.01, 14_000.0);
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// V8: D >= step_high_suspicious_db with fs in [fake_max_cutoff_hz, ambiguous_max_cutoff_hz).
#[test]
fn rf04_v8_deep_step_just_above_the_lossy_range_is_suspicious() {
    let p = params();
    for fs in [
        p.verdict.fake_max_cutoff_hz,
        p.verdict.ambiguous_max_cutoff_hz - 1.0,
    ] {
        let m = step(p.verdict.step_high_suspicious_db, fs);
        assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![AmbiguousCutoff]));
    }
}

// V8: shallower than step_high_suspicious_db, or fs at ambiguous_max_cutoff_hz or above -> Real.
#[test]
fn rf04_v8_limits() {
    let p = params();
    let m = step(p.verdict.step_high_suspicious_db - 0.01, 20_500.0);
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
    let m = step(
        p.verdict.step_high_suspicious_db,
        p.verdict.ambiguous_max_cutoff_hz,
    );
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// V9: no deep step and content ends below gradual_min_cutoff_hz -> Suspicious(GradualRolloff).
#[test]
fn rf04_v9_gradual_rolloff_is_suspicious() {
    let p = params();
    let m = Measurements {
        extent_hz: p.verdict.gradual_min_cutoff_hz - 1.0,
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![GradualRolloff]));
    // a step just under step_suspicious_db is still "no step"
    let m = Measurements {
        extent_hz: 15_000.0,
        ..step(p.verdict.step_suspicious_db - 0.01, 16_000.0)
    };
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![GradualRolloff]));
}

// V9: fe at gradual_min_cutoff_hz is not below it -> Real.
#[test]
fn rf04_v9_limit_is_strict() {
    let p = params();
    let m = Measurements {
        extent_hz: p.verdict.gradual_min_cutoff_hz,
        ..clean()
    };
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// V9 needs D < step_suspicious_db: with a step of 8 dB at 20.5 kHz (in no earlier row) the
// verdict is Real even though fe is low.
#[test]
fn rf04_v9_requires_the_step_to_be_below_the_suspicious_limit() {
    let p = params();
    let m = Measurements {
        extent_hz: 15_000.0,
        ..step(p.verdict.step_suspicious_db + 1.0, 20_500.0)
    };
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// S4: the verdict depends on parameters, not on constants in the code.
#[test]
fn rf04_thresholds_come_from_params() {
    let m = Measurements {
        gap_db: 6.0,
        ..clean()
    };
    assert_eq!(decide(&m, &params()), (Verdict::Fake, vec![]));
    let mut p = params();
    p.verdict.gap_fake_db = 100.0;
    assert_eq!(decide(&m, &p), (Verdict::Suspicious, vec![HighBandGaps]));

    let m = step(10.0, 16_000.0);
    let mut p = params();
    p.verdict.fake_max_cutoff_hz = 15_000.0;
    assert_eq!(decide(&m, &p), (Verdict::Real, vec![]));
}

// ---------------------------------------------------------------------------------------------
// Whole pipeline of the pure rules: spectra -> Detection (@RF-03 @RF-04)
// ---------------------------------------------------------------------------------------------

// Scenario Outline: Signal without a cutoff step reports the Nyquist frequency as cutoff.
#[test]
fn rf03_flat_full_band_is_real_with_nyquist_cutoff() {
    let d = detect(Some(&spectra(|_| 0.0)), &params());
    assert_eq!(d.verdict, Verdict::Real);
    assert!(d.reasons.is_empty());
    let c = d.cutoff.expect("cutoff");
    assert_eq!(c.frequency_hz, 22_050.0);
    assert_eq!(c.steepness_db_per_khz, 0.0);
}

// Scenario Outline: Verdict by signal / Cutoff estimate (lp16k): V4, cutoff at fs.
#[test]
fn rf04_cliff_at_16k_is_fake() {
    let d = detect(
        Some(&mean_only(|b| if b < 160 { 0.0 } else { -100.0 })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Fake);
    assert!(d.reasons.is_empty());
    let c = d.cutoff.expect("cutoff");
    assert!(
        (c.frequency_hz - 16_000.0).abs() <= 200.0,
        "{}",
        c.frequency_hz
    );
    assert!((c.steepness_db_per_khz - 250.0).abs() < 1e-3);
    assert!(c.noise_floor_db < -90.0);
}

// A brick wall in the ambiguous zone just above the lossy range (V8): cliff at 20.5 kHz.
#[test]
fn rf04_cliff_at_20_5k_is_ambiguous() {
    let d = detect(
        Some(&mean_only(|b| if b < 205 { 0.0 } else { -100.0 })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![AmbiguousCutoff]);
}

// Scenario Outline: Verdict by signal (gradual): 60 dB/octave from 8 kHz -> V9.
#[test]
fn rf04_gentle_rolloff_is_gradual() {
    let d = detect(
        Some(&spectra(|b| {
            let f = (b as f64 + 0.5) * 100.0;
            if f <= 8_000.0 {
                0.0
            } else {
                -200.0 * (f / 8_000.0).log10()
            }
        })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![GradualRolloff]);
}

// Scenario Outline: Verdict by signal (low_hf): little treble wins over the cliff (V2).
#[test]
fn rf04_low_treble_is_never_fake() {
    let d = detect(
        Some(&spectra(|b| if b < 60 { 0.0 } else { -90.0 })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![LowHighFrequencyContent]);
}

// Scenario Outline: Verdict by signal (sfb21_gaps): gated content up to Nyquist. The median
// spectrum drops 19 dB at 16 kHz (G = 19), the mean spectrum only 6 dB, which is its cutoff step
// (fs 15950 Hz, in the lossy range, D = 6 >= max_lossy_step_db = 3). The upper window has M = -6 dB and
// Q = -19 dB: U = -6 dB, B = 13 dB pass the thresholds, but with v4 defaults (ADR-0039) the lossy
// step removes the guard, so the gap rule V3 stands: Fake, no reasons. With the v3 file's values
// (no max_lossy_step_db) the track is full band and Suspicious(HighBandGaps). Cutoff: Nyquist.
#[test]
fn rf04_sfb21_gaps_are_fake_because_of_the_lossy_step() {
    let s = spectra_at(
        44_100,
        |b| if b < 160 { 0.0 } else { -6.0 },
        |b| if b < 160 { 0.0 } else { -19.0 },
    );
    let p = params();
    let m = measure(&s, &p);
    near(m.upper_level_db.expect("U"), -6.0);
    near(m.upper_burst_db.expect("B"), 13.0);
    near(m.step_db, 6.0);
    assert_eq!(m.step_hz, Some(15_950.0));
    assert!(!is_full_band(&m, &p), "the lossy step removes the guard");
    let d = detect(Some(&s), &p);
    assert_eq!(d.verdict, Verdict::Fake);
    assert!(d.reasons.is_empty());
    assert_eq!(d.cutoff.expect("cutoff").frequency_hz, 22_050.0);

    let v3 = v3_params();
    assert!(is_full_band(&m, &v3), "full band with the v3 values");
    let d = detect(Some(&s), &v3);
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![HighBandGaps]);
}

// Scenario Outline: Verdict by signal (sfb21_gaps_weak): G = 5 dB -> V5.
#[test]
fn rf04_weak_gaps_are_suspicious() {
    let s = spectra_at(44_100, |_| 0.0, |b| if b < 160 { 0.0 } else { -5.0 });
    let d = detect(Some(&s), &params());
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![HighBandGaps]);
}

// Scenario Outline: Verdict by signal (hf_bursts): the same gate from 8 kHz has no edge at
// 16 kHz -> Real.
#[test]
fn rf04_bursts_without_an_edge_at_16k_are_real() {
    let s = median_only(|b| if b < 80 { 0.0 } else { -19.0 });
    let d = detect(Some(&s), &params());
    assert_eq!(d.verdict, Verdict::Real);
    assert!(d.reasons.is_empty());
    assert_eq!(d.cutoff.expect("cutoff").frequency_hz, 22_050.0);
}

// rules.md section 1 item 6 (Scenario Outline: Verdict by signal, silence): no frame was kept ->
// Suspicious(LowHighFrequencyContent). The cutoff reported in that case is not specified.
#[test]
fn rf04_no_kept_frames_is_suspicious_low_hf() {
    let d = detect(None, &params());
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![LowHighFrequencyContent]);
}

// RF-14: supported extensions (H1: WAV and FLAC; H2, ADR-0040: MP3 and M4A; Opus is still skipped).
// Scenario "MP3 and M4A files are found; Opus files are not yet" (decode-lossy.feature, @RF-02 @RF-14).
#[test]
fn supported_extensions() {
    assert_eq!(AudioFormat::from_extension("flac"), Some(AudioFormat::Flac));
    assert_eq!(AudioFormat::from_extension("FLAC"), Some(AudioFormat::Flac));
    assert_eq!(AudioFormat::from_extension("wav"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("Wav"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("mp3"), Some(AudioFormat::Mp3));
    assert_eq!(AudioFormat::from_extension("MP3"), Some(AudioFormat::Mp3));
    assert_eq!(AudioFormat::from_extension("m4a"), Some(AudioFormat::M4a));
    assert_eq!(AudioFormat::from_extension("M4A"), Some(AudioFormat::M4a));
    assert_eq!(AudioFormat::from_extension("opus"), Some(AudioFormat::Opus));
    assert_eq!(AudioFormat::from_extension("OPUS"), Some(AudioFormat::Opus));
    assert_eq!(AudioFormat::from_extension("ogg"), None);
    assert_eq!(AudioFormat::from_extension("mp4"), None);
    assert_eq!(AudioFormat::from_extension("aac"), None);
    assert_eq!(AudioFormat::from_extension("txt"), None);
    assert_eq!(AudioFormat::from_extension(""), None);
}
// =============================================================================================
// Full-band guard, detection v3 amended by v4 (specs/features/detection-full-band.feature, @RF-04)
// Rules: specs/detection/rules.md sections 2 to 4, DetectionParams v4 (ADR-0038, ADR-0039). The
// tests named rf04_v3_* were written for v3 and keep their assertions under the v4 defaults; the
// rf04_v4_* tests cover what v4 adds or changes.
// Names chosen by the test author (not pinned by the spec):
//   Measurements.upper_level_db / upper_burst_db: Option<f64>  (U and B, None = undefined)
//   fakeflac_core::detect::is_full_band(&Measurements, &DetectionParams) -> bool
//   fakeflac_core::params::{FullBandParams { low_hz, high_hz, min_level_db, min_burst_db,
//                           max_lossy_step_db: Option<f64> }, ParamsError}  and  DetectionParams.full_band: Option<FullBandParams>
//   DetectionParams::validate(&self) -> Result<(), ParamsError>, whose Display text contains the
//   inequality that does not hold, written as in the feature ("full_band.low_hz < full_band.high_hz").
// =============================================================================================

/// The parameters of `specs/detection/detection-params.v2.toml` as far as the core can express
/// them: v4 without the `[full_band]` section (the audio crate's tests load the real file).
fn v2_params() -> DetectionParams {
    DetectionParams {
        algorithm_version: 2,
        full_band: None,
        lossy: None,
        ..DetectionParams::default()
    }
}

/// The parameters of `specs/detection/detection-params.v3.toml`: v4 without `max_lossy_step_db`,
/// so the guard has no step condition (the audio crate's tests load the real file).
fn v3_params() -> DetectionParams {
    let mut p = DetectionParams {
        algorithm_version: 3,
        lossy: None,
        ..DetectionParams::default()
    };
    p.full_band
        .as_mut()
        .expect("the defaults have a [full_band] section")
        .max_lossy_step_db = None;
    p
}

// ---- Measurements ---------------------------------------------------------------------------

// Scenario: Upper level and burstiness are medians over the upper window.
// L_ref = -30 dB (all bands at -30 dB in the reference range). The window [20700, 21000) holds the
// bands centred at 20750, 20850 and 20950 Hz (bands 207 to 209); the neighbours at 20650 and
// 21050 Hz (bands 206 and 210) are outside and would change the medians if they were counted.
#[test]
fn rf04_v3_upper_level_and_burstiness_are_medians_over_the_upper_window() {
    let mean = |b: usize| match b {
        207 => -70.0,
        208 => -80.0,
        209 => -75.0,
        _ => -30.0,
    };
    let median_ = |b: usize| match b {
        207 => -80.0,
        208 => -82.0,
        209 => -80.0,
        206 | 210 => -90.0,
        _ => -30.0,
    };
    let s = spectra_at(44_100, mean, median_);
    let p = params();
    let m = measure(&s, &p);
    // U: median of M - L_ref = (-40, -50, -45) = -45. B: median of M - Q = (10, 2, 5) = 5.
    near(m.upper_level_db.expect("U is defined"), -45.0);
    near(m.upper_burst_db.expect("B is defined"), 5.0);
    assert!(is_full_band(&m, &p), "the track is full band");
}

// Scenario: An even number of window bands takes the upper middle value.
// full_band.low_hz = 20800 leaves the bands centred at 20850 and 20950 Hz.
#[test]
fn rf04_v3_even_window_takes_the_upper_middle_value() {
    let mean = |b: usize| match b {
        208 => -80.0,
        209 => -70.0,
        _ => -30.0,
    };
    let median_ = |b: usize| match b {
        208 => -82.0,
        209 => -74.0,
        _ => -30.0,
    };
    let s = spectra_at(44_100, mean, median_);
    let mut p = params();
    p.full_band
        .as_mut()
        .expect("v3 has a [full_band] section")
        .low_hz = 20_800.0;
    let m = measure(&s, &p);
    // U: (-50, -40) -> the larger, -40. B: (2, 4) -> the larger, 4.
    near(m.upper_level_db.expect("U is defined"), -40.0);
    near(m.upper_burst_db.expect("B is defined"), 4.0);
}

// Scenario Outline: Upper level and burstiness are undefined without an upper window.
// Example 1: 32 kHz (Nyquist 16 kHz) with v3 parameters: no band in [20700, 21000).
#[test]
fn rf04_v3_no_upper_window_at_32k_leaves_u_and_b_undefined() {
    let s = spectra_at(32_000, |_| 0.0, |_| 0.0);
    let p = params();
    let m = measure(&s, &p);
    assert_eq!(m.upper_level_db, None);
    assert_eq!(m.upper_burst_db, None);
    assert!(!is_full_band(&m, &p), "the track is not full band");
}

// Example 2: 44.1 kHz with the parameters of detection-params.v2.toml (no [full_band] section).
#[test]
fn rf04_v3_no_full_band_section_leaves_u_and_b_undefined() {
    let s = spectra_at(44_100, |_| 0.0, |_| -10.0);
    let p = v2_params();
    let m = measure(&s, &p);
    assert_eq!(m.upper_level_db, None);
    assert_eq!(m.upper_burst_db, None);
    assert!(!is_full_band(&m, &p), "the track is not full band");
}

// ---- Verdict rules on given measurements ------------------------------------------------------

/// The measurements of the feature's tables: H, G, D, fs, fe, U and B.
#[allow(clippy::too_many_arguments)]
fn tab(h: f64, g: f64, d: f64, fs: Option<f64>, fe: f64, u: f64, b: f64) -> Measurements {
    Measurements {
        noise_floor_db: -80.0,
        high_frequency_db: h,
        extent_hz: fe,
        step_db: d,
        step_hz: fs,
        gap_db: g,
        upper_level_db: Some(u),
        upper_burst_db: Some(b),
        median_step_db: None,
        top_floor_db: None,
        residual_db: None,
    }
}

// Scenario Outline: Confirmed false positives whose deepest step is above the lossy range are not
// Fake. U and B are the values measured on the real tracks (rules.md, "Why v3 was replaced").
#[test]
fn rf04_v3_confirmed_false_positives_are_suspicious() {
    let p = params();
    for (id, g, d, fs, fe, u, b) in [
        ("FP1", 18.39, 8.93, 20_450.0, 20_900.0, -56.64, 9.53),
        ("FP2", 13.18, 14.79, 21_450.0, 21_400.0, -44.74, 30.60),
        ("FP3", 10.26, 14.70, 21_250.0, 21_100.0, -51.46, 10.76),
    ] {
        let m = tab(-20.0, g, d, Some(fs), fe, u, b);
        assert!(is_full_band(&m, &p), "{id}: full band");
        assert_eq!(
            decide(&m, &p),
            (Verdict::Suspicious, vec![HighBandGaps]),
            "{id}"
        );
    }
}

// Scenario Outline: FP4 stays Fake (known limitation). Its deepest step (14.22 dB at 19.45 kHz) is
// in the lossy range, so v4 never guards it, whatever its upper window holds. The first row has
// the U and B measured on the real track, the second illustrative bursty content.
#[test]
fn rf04_v4_fp4_stays_fake() {
    let p = params();
    for (u, b) in [(-47.06, 2.86), (-45.0, 8.0)] {
        let m = tab(-20.0, -0.25, 14.22, Some(19_450.0), 22_050.0, u, b);
        assert!(!is_full_band(&m, &p), "U = {u}, B = {b}: not full band");
        assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]), "U = {u}, B = {b}");
    }
}

// Scenario Outline: The same evidence without content above the lossy range is still Fake.
#[test]
fn rf04_v3_the_same_evidence_without_content_above_is_fake() {
    let p = params();
    for (pattern, g, d, fs, fe, u, b) in [
        (
            "FP1 gap over a floor",
            18.39,
            8.93,
            20_450.0,
            22_050.0,
            -50.0,
            0.3,
        ),
        (
            "FP4 step over a floor",
            -0.25,
            14.22,
            19_450.0,
            22_050.0,
            -45.0,
            0.3,
        ),
        (
            "320 kbps: gap, step at 20.5 kHz",
            12.0,
            25.0,
            20_450.0,
            22_050.0,
            -55.0,
            0.3,
        ),
        (
            "128 kbps: gap, step at 16 kHz",
            15.0,
            30.0,
            16_050.0,
            22_050.0,
            -48.0,
            0.2,
        ),
        (
            "AAC/Opus: step only at 20 kHz",
            -0.5,
            20.0,
            19_950.0,
            22_050.0,
            -60.0,
            0.4,
        ),
        (
            "gap, near digital silence above",
            9.0,
            40.0,
            20_450.0,
            20_600.0,
            -95.0,
            6.0,
        ),
    ] {
        let m = tab(-20.0, g, d, Some(fs), fe, u, b);
        assert!(!is_full_band(&m, &p), "{pattern}: not full band");
        assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]), "{pattern}");
    }
}

// ---- Boundaries at the thresholds -------------------------------------------------------------

// Scenario Outline: The guard holds at its thresholds and not below them.
#[test]
fn rf04_v3_the_guard_holds_at_its_thresholds_and_not_below_them() {
    let p = params();
    for (u, b, full_band, verdict, reasons) in [
        (-70.0, 3.0, true, Verdict::Suspicious, vec![HighBandGaps]),
        (-70.1, 3.0, false, Verdict::Fake, vec![]),
        (-70.0, 2.9, false, Verdict::Fake, vec![]),
        (-20.0, 2.9, false, Verdict::Fake, vec![]),
        (-90.0, 9.0, false, Verdict::Fake, vec![]),
    ] {
        let m = tab(-20.0, 10.0, 0.0, None, 22_050.0, u, b);
        assert_eq!(is_full_band(&m, &p), full_band, "U = {u}, B = {b}");
        assert_eq!(decide(&m, &p), (verdict, reasons), "U = {u}, B = {b}");
    }
}

// Scenario Outline: The gap and step thresholds keep their v2 boundaries when the track is not
// full band (U = -50 dB, B = 0.3 dB).
#[test]
fn rf04_v3_gap_and_step_thresholds_keep_their_v2_boundaries() {
    let p = params();
    for (g, d, fs, verdict, reasons) in [
        (6.0, 0.0, None, Verdict::Fake, vec![]),
        (5.99, 0.0, None, Verdict::Suspicious, vec![HighBandGaps]),
        (0.0, 10.0, Some(20_250.0), Verdict::Fake, vec![]),
        (
            0.0,
            9.99,
            Some(20_250.0),
            Verdict::Suspicious,
            vec![AmbiguousCutoff],
        ),
        (0.0, 10.0, Some(20_300.0), Verdict::Real, vec![]),
    ] {
        let m = tab(-20.0, g, d, fs, 22_050.0, -50.0, 0.3);
        assert_eq!(
            decide(&m, &p),
            (verdict, reasons),
            "G = {g}, D = {d}, fs = {fs:?}"
        );
    }
}

// ---- What the guard never changes ---------------------------------------------------------------

// Scenario Outline: The guard only turns Fake into Suspicious (U = -40 dB, B = 8 dB). The v3 row
// "gap and lossy step: V5 comes first" is Fake with v4: see the scenarios "A cutoff step in the
// lossy range removes the guard" and "A v3 params file still gives the v3 verdicts" below.
#[test]
fn rf04_v3_the_guard_only_turns_fake_into_suspicious() {
    let p = params();
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        &str,
        f64,
        f64,
        f64,
        Option<f64>,
        f64,
        Verdict,
        Vec<SuspicionReason>,
    )> = vec![
        (
            "weak gap (V5 as in v2)",
            -20.0,
            5.0,
            0.0,
            None,
            22_050.0,
            Verdict::Suspicious,
            vec![HighBandGaps],
        ),
        (
            "step above the lossy range (V8)",
            -20.0,
            0.0,
            13.0,
            Some(20_450.0),
            22_050.0,
            Verdict::Suspicious,
            vec![AmbiguousCutoff],
        ),
        (
            "anti-aliasing step at 21.5 kHz",
            -20.0,
            0.0,
            30.0,
            Some(21_550.0),
            21_500.0,
            Verdict::Real,
            vec![],
        ),
        (
            "little treble (V2 comes first)",
            -65.0,
            12.0,
            20.0,
            Some(19_450.0),
            22_050.0,
            Verdict::Suspicious,
            vec![LowHighFrequencyContent],
        ),
        (
            "no evidence",
            -20.0,
            1.0,
            3.0,
            Some(18_050.0),
            22_050.0,
            Verdict::Real,
            vec![],
        ),
    ];
    for (case, h, g, d, fs, fe, verdict, reasons) in rows {
        let m = tab(h, g, d, fs, fe, -40.0, 8.0);
        assert_eq!(decide(&m, &p), (verdict, reasons), "{case}");
    }
}

// Scenario: Without a [full_band] section the guard is disabled whatever U and B are.
#[test]
fn rf04_v3_without_a_full_band_section_the_guard_is_disabled() {
    let p = v2_params();
    let m = tab(-20.0, 18.39, 8.93, Some(20_450.0), 20_900.0, -45.0, 8.0);
    assert!(!is_full_band(&m, &p), "the track is not full band");
    assert_eq!(decide(&m, &p), (Verdict::Fake, vec![]));
}

// ---- Parameters (rules.md section 4) --------------------------------------------------------------

// The defaults of the code are v7 (RF-16, lossy-source.md section 11; v6 in ADR-0042, v5 in
// ADR-0041; the audio crate's test compares them with the file). Their [full_band] section is that
// of v4.
#[test]
fn rf04_v3_defaults_are_v3() {
    let p = DetectionParams::default();
    assert_eq!(p.algorithm_version, 8);
    assert_eq!(
        p.full_band,
        Some(FullBandParams {
            low_hz: 20_700.0,
            high_hz: 21_000.0,
            min_level_db: -70.0,
            min_burst_db: 3.0,
            max_lossy_step_db: Some(3.0),
        })
    );
}

// Scenario Outline: Valid parameter files deserialise and validate (the validation half):
// the defaults (v4), v3 (no max_lossy_step_db) and v2 (no [full_band] section).
#[test]
fn rf04_v3_valid_parameters_are_accepted() {
    DetectionParams::default()
        .validate()
        .expect("the defaults are valid");
    v3_params()
        .validate()
        .expect("params without max_lossy_step_db are valid");
    v2_params()
        .validate()
        .expect("params without [full_band] are valid");
}

/// Validates `p` and returns the text of the error.
fn rejection(p: &DetectionParams) -> String {
    let e: ParamsError = p.validate().expect_err("the parameters are invalid");
    e.to_string()
}

// Scenario Outline: Invalid parameter files are rejected (the validation rows).
#[test]
fn rf04_v3_full_band_high_must_exceed_low() {
    let mut p = params();
    p.full_band.as_mut().expect("full_band").high_hz = 20_700.0; // equal to low_hz
    let text = rejection(&p);
    assert!(
        text.contains("full_band.low_hz < full_band.high_hz"),
        "{text}"
    );
}

#[test]
fn rf04_v3_full_band_low_must_exceed_fake_max_cutoff() {
    let mut p = params();
    p.full_band.as_mut().expect("full_band").low_hz = 20_300.0; // equal to fake_max_cutoff_hz
    let text = rejection(&p);
    assert!(
        text.contains("fake_max_cutoff_hz < full_band.low_hz"),
        "{text}"
    );
}

#[test]
fn rf04_v3_gap_suspicious_must_not_exceed_gap_fake() {
    let mut p = params();
    p.verdict.gap_suspicious_db = 6.5;
    let text = rejection(&p);
    assert!(text.contains("gap_suspicious_db ≤ gap_fake_db"), "{text}");
}

#[test]
fn rf04_v3_step_suspicious_must_not_exceed_step_fake() {
    let mut p = params();
    p.verdict.step_suspicious_db = 10.5;
    let text = rejection(&p);
    assert!(text.contains("step_suspicious_db ≤ step_fake_db"), "{text}");
}

// The threshold checks also apply without a [full_band] section (the full-band inequalities are
// skipped then).
#[test]
fn rf04_v3_validation_without_full_band_checks_the_thresholds_only() {
    let mut p = v2_params();
    p.verdict.gap_suspicious_db = 6.5;
    assert!(rejection(&p).contains("gap_suspicious_db ≤ gap_fake_db"));
}

// ---- Detection v4: the lossy-step condition (ADR-0039) --------------------------------------------

// Scenario Outline: A cutoff step in the lossy range removes the guard (H = -20, fe = 22050,
// U = -40 dB, B = 8 dB: only D and fs decide).
#[test]
fn rf04_v4_a_lossy_step_removes_the_guard() {
    let p = params();
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        &str,
        f64,
        f64,
        Option<f64>,
        bool,
        Verdict,
        Vec<SuspicionReason>,
    )> = vec![
        (
            "gap, no cutoff step",
            10.0,
            0.0,
            None,
            true,
            Verdict::Suspicious,
            vec![HighBandGaps],
        ),
        (
            "gap, lossy step at max_lossy_step_db",
            10.0,
            3.0,
            Some(18_050.0),
            false,
            Verdict::Fake,
            vec![],
        ),
        (
            "gap, lossy step just below max_lossy_step_db",
            10.0,
            2.99,
            Some(18_050.0),
            true,
            Verdict::Suspicious,
            vec![HighBandGaps],
        ),
        (
            "gap, step at the lower edge of the lossy range",
            10.0,
            5.0,
            Some(14_500.0),
            false,
            Verdict::Fake,
            vec![],
        ),
        (
            "gap, step just below the lossy range",
            10.0,
            5.0,
            Some(14_450.0),
            true,
            Verdict::Suspicious,
            vec![HighBandGaps],
        ),
        (
            "gap, step just inside the upper edge of the range",
            10.0,
            5.0,
            Some(20_250.0),
            false,
            Verdict::Fake,
            vec![],
        ),
        (
            "gap, step at the upper edge of the range",
            10.0,
            5.0,
            Some(20_300.0),
            true,
            Verdict::Suspicious,
            vec![HighBandGaps],
        ),
        (
            "FP1 pattern: deepest step 150 Hz above the range",
            18.39,
            8.93,
            Some(20_450.0),
            true,
            Verdict::Suspicious,
            vec![HighBandGaps],
        ),
        (
            "gap and a deep lossy step",
            12.0,
            20.0,
            Some(19_450.0),
            false,
            Verdict::Fake,
            vec![],
        ),
        (
            "confirmed fake with the smallest lossy step",
            8.0,
            5.63,
            Some(17_050.0),
            false,
            Verdict::Fake,
            vec![],
        ),
        (
            "deep lossy step, no gap: V4 is never guarded",
            -0.25,
            14.22,
            Some(19_450.0),
            false,
            Verdict::Fake,
            vec![],
        ),
        (
            "small lossy step, no gap: nothing fires",
            1.0,
            5.0,
            Some(18_050.0),
            false,
            Verdict::Real,
            vec![],
        ),
    ];
    for (case, g, d, fs, full_band, verdict, reasons) in rows {
        let m = tab(-20.0, g, d, fs, 22_050.0, -40.0, 8.0);
        assert_eq!(is_full_band(&m, &p), full_band, "{case}: full band");
        assert_eq!(decide(&m, &p), (verdict, reasons), "{case}");
    }
}

// Rules.md section 2 ("LossyStep"): with max_lossy_step_db = 0 any defined fs in the lossy range
// removes the guard (D > 0 whenever fs is defined); no step at all (fs undefined) does not.
#[test]
fn rf04_v4_a_zero_threshold_means_any_lossy_range_step() {
    let mut p = params();
    p.full_band.as_mut().expect("full_band").max_lossy_step_db = Some(0.0);
    p.validate().expect("0 is a valid max_lossy_step_db");
    let m = tab(-20.0, 10.0, 0.01, Some(18_050.0), 22_050.0, -40.0, 8.0);
    assert!(!is_full_band(&m, &p));
    let m = tab(-20.0, 10.0, 0.0, None, 22_050.0, -40.0, 8.0);
    assert!(is_full_band(&m, &p), "no step at all: still full band");
}

// Scenario Outline: A v3 params file still gives the v3 verdicts. The params of v3 have no
// max_lossy_step_db, so the guard has no step condition and also covers V4.
#[test]
fn rf04_v4_a_v3_params_file_still_gives_the_v3_verdicts() {
    let v3 = v3_params();
    let v4 = params();
    type Outcome = (Verdict, Vec<SuspicionReason>);
    type Measures = (f64, f64, f64, f64, f64, f64);
    // (case, (G, D, fs, fe, U, B), v3 outcome, v4 outcome)
    let rows: Vec<(&str, Measures, Outcome, Outcome)> = vec![
        (
            "FP4 pattern with bursty content",
            (-0.25, 14.22, 19_450.0, 22_050.0, -45.0, 8.0),
            (Verdict::Suspicious, vec![AmbiguousCutoff]),
            (Verdict::Fake, vec![]),
        ),
        (
            "gap and a deep lossy step",
            (12.0, 20.0, 19_450.0, 22_050.0, -40.0, 8.0),
            (Verdict::Suspicious, vec![HighBandGaps]),
            (Verdict::Fake, vec![]),
        ),
        (
            "gap and a small lossy step",
            (10.0, 5.0, 16_050.0, 22_050.0, -6.0, 13.0),
            (Verdict::Suspicious, vec![HighBandGaps]),
            (Verdict::Fake, vec![]),
        ),
        (
            "FP1",
            (18.39, 8.93, 20_450.0, 20_900.0, -56.64, 9.53),
            (Verdict::Suspicious, vec![HighBandGaps]),
            (Verdict::Suspicious, vec![HighBandGaps]),
        ),
        (
            "FP1 gap over a floor",
            (18.39, 8.93, 20_450.0, 22_050.0, -50.0, 0.3),
            (Verdict::Fake, vec![]),
            (Verdict::Fake, vec![]),
        ),
    ];
    for (case, (g, d, fs, fe, u, b), want3, want4) in rows {
        let m = tab(-20.0, g, d, Some(fs), fe, u, b);
        assert_eq!(decide(&m, &v3), want3, "{case}: v3");
        assert_eq!(decide(&m, &v4), want4, "{case}: v4");
    }
}

// Scenario Outline: The bounds of max_lossy_step_db are inclusive (the validation half).
#[test]
fn rf04_v4_max_lossy_step_bounds_are_inclusive() {
    for value in [0.0, 10.0] {
        let mut p = params();
        p.full_band.as_mut().expect("full_band").max_lossy_step_db = Some(value);
        p.validate()
            .unwrap_or_else(|e| panic!("{value} must be accepted: {e}"));
    }
}

// Scenario Outline: An invalid max_lossy_step_db is rejected by validation.
#[test]
fn rf04_v4_a_negative_max_lossy_step_is_rejected() {
    let mut p = params();
    p.full_band.as_mut().expect("full_band").max_lossy_step_db = Some(-0.5);
    let text = rejection(&p);
    assert!(text.contains("full_band.max_lossy_step_db ≥ 0"), "{text}");
}

#[test]
fn rf04_v4_max_lossy_step_above_step_fake_is_rejected() {
    let mut p = params();
    p.full_band.as_mut().expect("full_band").max_lossy_step_db = Some(10.5);
    let text = rejection(&p);
    assert!(
        text.contains("full_band.max_lossy_step_db ≤ step_fake_db"),
        "{text}"
    );
}

#[test]
fn rf04_v4_lowering_step_fake_below_max_lossy_step_is_rejected() {
    let mut p = params();
    p.verdict.step_fake_db = 2.5;
    p.verdict.step_suspicious_db = 2.5;
    let text = rejection(&p);
    assert!(
        text.contains("full_band.max_lossy_step_db ≤ step_fake_db"),
        "{text}"
    );
}

#[test]
fn rf04_v4_a_nan_max_lossy_step_is_rejected() {
    let mut p = params();
    p.full_band.as_mut().expect("full_band").max_lossy_step_db = Some(f64::NAN);
    let text = rejection(&p);
    assert!(text.contains("full_band.max_lossy_step_db"), "{text}");
}

// Without the field (a v3 file) the step_fake_db bound does not apply (rules.md section 4).
#[test]
fn rf04_v4_without_max_lossy_step_the_bounds_are_not_checked() {
    let mut p = v3_params();
    p.verdict.step_fake_db = 2.5;
    p.verdict.step_suspicious_db = 2.5;
    p.validate()
        .expect("no max_lossy_step_db, nothing to bound");
}

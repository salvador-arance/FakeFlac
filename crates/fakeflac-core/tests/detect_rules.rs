//! Rule-level tests of `specs/detection/rules.md` v2 (sections 1 item 6, 2 and 3) on hand-built
//! band spectra and hand-built measurements, plus the supported extensions (RF-14).
//! Scenarios: `specs/features/detection-lossless.feature` (@RF-03 @RF-04).
//! Protected after the `test(RF-03,RF-04)` commit.
//!
//! The spectra are written for bands of 100 Hz at 44.1 kHz: band `b` has its centre at
//! `(b + 0.5) * 100` Hz, there are 220 bands and `top` is band 219 (centre 21 950 Hz).
//! Useful landmarks: band 100 starts at 10 kHz, band 160 starts at 16 kHz.

#![allow(clippy::expect_used)]

use fakeflac_core::detect::{cutoff_estimate, decide, detect, measure, Measurements};
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

// Scenario Outline: Verdict by signal (sfb21_gaps): no cliff in the mean, a 20 dB drop in the
// median at 16 kHz -> V3 Fake, and the cutoff is Nyquist (there is no step).
#[test]
fn rf04_sfb21_gaps_are_fake_without_a_cutoff_step() {
    let s = spectra_at(
        44_100,
        |b| if b < 160 { 0.0 } else { -6.0 },
        |b| if b < 160 { 0.0 } else { -19.0 },
    );
    let d = detect(Some(&s), &params());
    assert_eq!(d.verdict, Verdict::Fake);
    assert!(d.reasons.is_empty());
    assert_eq!(d.cutoff.expect("cutoff").frequency_hz, 22_050.0);
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

// RF-14: supported extensions in H1
#[test]
fn supported_extensions() {
    assert_eq!(AudioFormat::from_extension("flac"), Some(AudioFormat::Flac));
    assert_eq!(AudioFormat::from_extension("FLAC"), Some(AudioFormat::Flac));
    assert_eq!(AudioFormat::from_extension("wav"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("Wav"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("mp3"), None);
    assert_eq!(AudioFormat::from_extension("txt"), None);
    assert_eq!(AudioFormat::from_extension(""), None);
}

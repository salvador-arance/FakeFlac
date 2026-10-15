//! Detection rules v1 for lossless files (RF-03, RF-04). Spec: `specs/detection/rules.md`.
//! Pure functions: no I/O, every number comes from `DetectionParams`.

use crate::domain::{CutoffEstimate, SuspicionReason, Verdict};
use crate::params::DetectionParams;
use crate::spectrum::AverageSpectrum;

/// Measurements of section 2 of the spec, relative to the reference level `L_ref`.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurements {
    pub noise_floor_db: f64,
    pub high_frequency_db: f64,
    pub cutoff_hz: f64,
    pub steepness_db_per_khz: f64,
    pub cliff: bool,
}

/// Outcome of the rules for one analysed (decodable, long enough) track.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub verdict: Verdict,
    pub reasons: Vec<SuspicionReason>,
    pub cutoff: Option<CutoffEstimate>,
}

/// Section 2: measurements on the smoothed spectrum.
pub fn measure(spectrum: &AverageSpectrum, params: &DetectionParams) -> Measurements {
    let bands = &params.bands;
    let bin_hz = spectrum.bin_hz;
    let nyquist = spectrum.nyquist_hz();
    let last = spectrum.power_db.len().saturating_sub(1);
    let bin_of = |f: f64| ((f / bin_hz).round().max(0.0) as usize).min(last);

    // Section 1.7: running median of width `smoothing_hz`, then levels relative to L_ref.
    let half = ((params.fft.smoothing_hz / bin_hz) / 2.0).round() as usize;
    let smoothed = running_median(&spectrum.power_db, half);
    let l_ref = median(&smoothed[bin_of(bands.ref_band_low_hz)..=bin_of(bands.ref_band_high_hz)]);
    let rel: Vec<f64> = smoothed.iter().map(|v| v - l_ref).collect();

    let top = bin_of(nyquist - bands.nyquist_guard_hz);
    let noise_floor_db = percentile(
        &rel[bin_of(bands.ref_band_low_hz)..=top],
        bands.noise_floor_percentile,
    );
    let high_frequency_db =
        median(&rel[bin_of(bands.hf_band_low_hz)..=bin_of(bands.hf_band_high_hz)]);

    // Cutoff: scanning down from the top, the first run of content at least
    // `content_min_width_hz` wide. A run that starts at the top means full band.
    let threshold = -params.cutoff.content_drop_db;
    let min_bins = ((params.cutoff.content_min_width_hz / bin_hz).ceil() as usize).max(1);
    let mut run = 0usize;
    let mut run_top = top;
    let mut cutoff_hz = 0.0;
    for k in (0..=top).rev() {
        if rel[k] > threshold {
            if run == 0 {
                run_top = k;
            }
            run += 1;
            if run >= min_bins {
                cutoff_hz = if run_top == top {
                    nyquist
                } else {
                    run_top as f64 * bin_hz
                };
                break;
            }
        } else {
            run = 0;
        }
    }

    let span = params.cutoff.steepness_span_hz;
    let steepness_db_per_khz = if cutoff_hz > 0.0 && cutoff_hz < nyquist {
        (rel[bin_of(cutoff_hz - span)] - rel[bin_of(cutoff_hz + span)]) / (2.0 * span / 1000.0)
    } else {
        0.0
    };
    let cliff = cutoff_hz > 0.0
        && steepness_db_per_khz >= params.verdict.cliff_min_steepness_db_per_khz
        && cutoff_hz < nyquist - params.cutoff.full_band_margin_hz;

    Measurements {
        noise_floor_db,
        high_frequency_db,
        cutoff_hz,
        steepness_db_per_khz,
        cliff,
    }
}

/// Sections 1 (silence) and 3: verdict. `None` means every frame was silent.
pub fn detect(spectrum: Option<&AverageSpectrum>, params: &DetectionParams) -> Detection {
    let Some(spectrum) = spectrum else {
        return Detection {
            verdict: Verdict::Suspicious,
            reasons: vec![SuspicionReason::LowHighFrequencyContent],
            cutoff: None,
        };
    };
    let m = measure(spectrum, params);
    let v = &params.verdict;
    let (verdict, reasons) = if m.high_frequency_db < v.low_hf_threshold_db {
        (
            Verdict::Suspicious,
            vec![SuspicionReason::LowHighFrequencyContent],
        )
    } else if m.cliff && m.cutoff_hz <= v.fake_max_cutoff_hz {
        (Verdict::Fake, vec![])
    } else if m.cliff {
        (Verdict::Suspicious, vec![SuspicionReason::AmbiguousCutoff])
    } else if m.cutoff_hz < v.gradual_min_cutoff_hz {
        (Verdict::Suspicious, vec![SuspicionReason::GradualRolloff])
    } else {
        (Verdict::Real, vec![])
    };
    Detection {
        verdict,
        reasons,
        cutoff: Some(CutoffEstimate {
            frequency_hz: m.cutoff_hz as f32,
            steepness_db_per_khz: m.steepness_db_per_khz as f32,
            noise_floor_db: m.noise_floor_db as f32,
        }),
    }
}

fn running_median(values: &[f64], half: usize) -> Vec<f64> {
    if half == 0 {
        return values.to_vec();
    }
    (0..values.len())
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half).min(values.len() - 1);
            median(&values[lo..=hi])
        })
        .collect()
}

fn median(values: &[f64]) -> f64 {
    percentile(values, 50.0)
}

/// Nearest-rank percentile; 0 for an empty slice.
fn percentile(values: &[f64], pct: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

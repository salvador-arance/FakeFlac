//! Detection rules v4 for lossless files (RF-03, RF-04). Spec: `specs/detection/rules.md`.
//! Pure functions: no I/O, every number comes from `DetectionParams`.

use crate::domain::{CutoffEstimate, SuspicionReason, Verdict};
use crate::params::DetectionParams;
use crate::spectrum::BandSpectra;

/// Tolerance for comparing frequencies and levels that come out of floating-point arithmetic.
const EPS: f64 = 1e-9;

/// Measurements of section 2 of the spec. Levels are relative to `L_ref`; `gap_db` is a
/// difference of levels of the median spectrum.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurements {
    /// `F`: noise floor (reported, not used to decide).
    pub noise_floor_db: f64,
    /// `H`: high-frequency content.
    pub high_frequency_db: f64,
    /// `fe`: content extent in Hz (Nyquist when the content reaches the top).
    pub extent_hz: f64,
    /// `D`: depth of the deepest cutoff step, 0 when there is none.
    pub step_db: f64,
    /// `fs`: centre of the transition band of that step; `None` when `D` is 0.
    pub step_hz: Option<f64>,
    /// `G`: gap step at `gaps.edge_hz`.
    pub gap_db: f64,
    /// `U`: upper level (median of `M - L_ref` in the upper window); `None` when undefined.
    pub upper_level_db: Option<f64>,
    /// `B`: upper burstiness (median of `M - Q` in the upper window); `None` when undefined.
    pub upper_burst_db: Option<f64>,
}

/// Outcome of the rules for one analysed (decodable, long enough) track.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub verdict: Verdict,
    pub reasons: Vec<SuspicionReason>,
    pub cutoff: Option<CutoffEstimate>,
}

/// Section 2: measurements on the band spectra.
pub fn measure(spectra: &BandSpectra, params: &DetectionParams) -> Measurements {
    let bands = &params.bands;
    let nyquist = spectra.nyquist_hz();
    let band_hz = spectra.band_hz;
    let count = spectra.mean_db.len().min(spectra.median_db.len());
    let centre = |b: usize| spectra.centre_hz(b);
    // Bands whose centre lies in `[lo, hi]`.
    let in_range = |lo: f64, hi: f64| -> Vec<usize> {
        (0..count)
            .filter(|&b| centre(b) >= lo - EPS && centre(b) <= hi + EPS)
            .collect()
    };

    // L_ref: median of M over the reference bands; every level below is relative to it.
    let ref_values: Vec<f64> = in_range(bands.ref_band_low_hz, bands.ref_band_high_hz)
        .into_iter()
        .map(|b| spectra.mean_db[b])
        .collect();
    let l_ref = median(&ref_values);
    let m: Vec<f64> = spectra.mean_db[..count].iter().map(|v| v - l_ref).collect();

    // `top`: last band whose centre is at or below `nyquist - nyquist_guard_hz`.
    let top_hz = nyquist - bands.nyquist_guard_hz;
    let top = (0..count).rev().find(|&b| centre(b) <= top_hz + EPS);

    // Noise floor F and high-frequency content H.
    let floor_values: Vec<f64> = in_range(bands.ref_band_low_hz, top_hz)
        .into_iter()
        .map(|b| m[b])
        .collect();
    let noise_floor_db = percentile(&floor_values, bands.noise_floor_percentile);
    let hf_values: Vec<f64> = in_range(bands.hf_band_low_hz, bands.hf_band_high_hz)
        .into_iter()
        .map(|b| m[b])
        .collect();
    let high_frequency_db = median(&hf_values);

    // Content extent `fe`: scanning down from `top`, the first run of content bands that is wide
    // enough. It is the lower edge of the highest band of the run (Nyquist if it starts at `top`).
    let extent_hz = top.map_or(0.0, |top| {
        let threshold = -params.extent.content_drop_db;
        let min_bands =
            ((params.extent.content_min_width_hz / band_hz - EPS).ceil() as usize).max(1);
        let mut run = 0usize;
        let mut run_top = top;
        for b in (0..=top).rev() {
            if m[b] > threshold {
                if run == 0 {
                    run_top = b;
                }
                run += 1;
                if run >= min_bands {
                    return if run_top == top {
                        nyquist
                    } else {
                        run_top as f64 * band_hz
                    };
                }
            } else {
                run = 0;
            }
        }
        0.0
    });

    // Cutoff step `D` at `fs` on the mean spectrum.
    let w = params.step.window_bands as usize;
    let mean_of =
        |from: usize, to: usize| m[from..=to].iter().sum::<f64>() / (to - from + 1) as f64;
    let mut best: Option<(f64, usize)> = None;
    if let (Some(top), true) = (top, w > 0) {
        let search_top = nyquist - params.step.search_top_margin_hz;
        // Bands whose windows would run off the spectrum are not candidates. With v2 parameters this
        // never drops one: search_top_margin_hz >= window_bands * band_hz + nyquist_guard_hz.
        for i in 2 * w..count.saturating_sub(w) {
            let c = centre(i);
            if c < params.step.search_low_hz - EPS || c > search_top + EPS {
                continue;
            }
            let below = mean_of(i - w, i - 1);
            let above = mean_of(i + 1, i + w);
            let slope = (mean_of(i - 2 * w, i - w - 1) - below).max(0.0);
            let d = below - above - slope;
            let rebound = (i + w + 1..=top).any(|b| m[b] > above + params.step.rebound_db);
            if !rebound && d > 0.0 && best.is_none_or(|(bd, _)| d > bd + EPS) {
                best = Some((d, i));
            }
        }
    }
    let (step_db, step_hz) = match best {
        Some((d, i)) => (d, Some(centre(i))),
        None => (0.0, None),
    };

    // Gap step `G` at `gaps.edge_hz` on the median spectrum.
    let (e, g) = (params.gaps.edge_hz, params.gaps.band_hz);
    let q = |lo: f64, hi: f64| -> f64 {
        let values: Vec<f64> = (0..count)
            .filter(|&b| centre(b) >= lo && centre(b) < hi)
            .map(|b| spectra.median_db[b])
            .collect();
        median(&values)
    };
    let gap_db = if nyquist < e + g {
        0.0
    } else {
        (q(e - g, e) - q(e, e + g)) - (q(e - 2.0 * g, e - g) - q(e - g, e)).max(0.0)
    };

    // Upper window (v3): bands with centre in `[low_hz, high_hz)` at or below `top`.
    let (upper_level_db, upper_burst_db) = match (&params.full_band, top) {
        (Some(fb), Some(top)) => {
            // Half-open bounds are exact: band centres are `(b + 0.5) * band_hz`, as in `q()`.
            let window: Vec<usize> = (0..=top)
                .filter(|&b| centre(b) >= fb.low_hz && centre(b) < fb.high_hz)
                .collect();
            if window.is_empty() {
                (None, None)
            } else {
                let levels: Vec<f64> = window.iter().map(|&b| m[b]).collect();
                let bursts: Vec<f64> = window
                    .iter()
                    .map(|&b| spectra.mean_db[b] - spectra.median_db[b])
                    .collect();
                (Some(median(&levels)), Some(median(&bursts)))
            }
        }
        _ => (None, None),
    };

    Measurements {
        noise_floor_db,
        high_frequency_db,
        extent_hz,
        step_db,
        step_hz,
        gap_db,
        upper_level_db,
        upper_burst_db,
    }
}

/// Section 2, "Reported cutoff": `fs` and its steepness when `D >= step_suspicious_db`,
/// otherwise `fe` and 0. `noise_floor_db` is `F`. `params.fft.band_hz` must equal the `band_hz` of the spectra
/// that produced `m` (the audio adapter builds them from it).
pub fn cutoff_estimate(m: &Measurements, params: &DetectionParams) -> CutoffEstimate {
    let (frequency_hz, steepness_db_per_khz) = match m.step_hz {
        Some(fs) if m.step_db >= params.verdict.step_suspicious_db => {
            let span_khz = f64::from(params.step.window_bands + 1) * params.fft.band_hz / 1000.0;
            (fs, m.step_db / span_khz)
        }
        _ => (m.extent_hz, 0.0),
    };
    CutoffEstimate {
        frequency_hz: frequency_hz as f32,
        steepness_db_per_khz: steepness_db_per_khz as f32,
        noise_floor_db: m.noise_floor_db as f32,
    }
}

/// Section 2, `FullBand`: the params have a `[full_band]` section, `U` and `B` are defined and
/// reach their minimums and (v4) the cutoff step is not `LossyStep`. Without the section it is
/// `false` whatever `U` and `B` are.
pub fn is_full_band(m: &Measurements, params: &DetectionParams) -> bool {
    match (&params.full_band, m.upper_level_db, m.upper_burst_db) {
        (Some(fb), Some(u), Some(b)) => {
            let v = &params.verdict;
            let lossy_step = match (fb.max_lossy_step_db, m.step_hz) {
                (Some(max), Some(fs)) => {
                    fs >= v.lossy_min_cutoff_hz && fs < v.fake_max_cutoff_hz && m.step_db >= max
                }
                _ => false,
            };
            u >= fb.min_level_db && b >= fb.min_burst_db && !lossy_step
        }
        _ => false,
    }
}

/// Section 3, rows V2 to V10, evaluated in order (V1 is decided before the spectra exist).
pub fn decide(m: &Measurements, params: &DetectionParams) -> (Verdict, Vec<SuspicionReason>) {
    let v = &params.verdict;
    let d = m.step_db;
    let fs = m.step_hz;
    let full_band = is_full_band(m, params);
    let in_range = fs.is_some_and(|f| f >= v.lossy_min_cutoff_hz && f < v.fake_max_cutoff_hz);
    if m.high_frequency_db < v.low_hf_threshold_db {
        // V2
        (
            Verdict::Suspicious,
            vec![SuspicionReason::LowHighFrequencyContent],
        )
    } else if !full_band && (m.gap_db >= v.gap_fake_db || (d >= v.step_fake_db && in_range)) {
        // V3, V4
        (Verdict::Fake, vec![])
    } else if m.gap_db >= v.gap_suspicious_db {
        // V5
        (Verdict::Suspicious, vec![SuspicionReason::HighBandGaps])
    } else if (d >= v.step_suspicious_db && in_range)
        || (d >= v.step_low_suspicious_db && fs.is_some_and(|f| f < v.lossy_min_cutoff_hz))
        || (d >= v.step_high_suspicious_db
            && fs.is_some_and(|f| f >= v.fake_max_cutoff_hz && f < v.ambiguous_max_cutoff_hz))
    {
        // V6, V7, V8
        (Verdict::Suspicious, vec![SuspicionReason::AmbiguousCutoff])
    } else if d < v.step_suspicious_db && m.extent_hz < v.gradual_min_cutoff_hz {
        // V9
        (Verdict::Suspicious, vec![SuspicionReason::GradualRolloff])
    } else {
        // V10
        (Verdict::Real, vec![])
    }
}

/// Sections 1 (silence) and 3: verdict. `None` means every frame was silent.
pub fn detect(spectra: Option<&BandSpectra>, params: &DetectionParams) -> Detection {
    let Some(spectra) = spectra else {
        return Detection {
            verdict: Verdict::Suspicious,
            reasons: vec![SuspicionReason::LowHighFrequencyContent],
            cutoff: None,
        };
    };
    let m = measure(spectra, params);
    let (verdict, reasons) = decide(&m, params);
    Detection {
        verdict,
        reasons,
        cutoff: Some(cutoff_estimate(&m, params)),
    }
}

fn median(values: &[f64]) -> f64 {
    percentile(values, 50.0)
}

/// Nearest-index percentile (`round(p · (n − 1))`; the upper middle value for an even-count
/// median); 0 for an empty slice.
fn percentile(values: &[f64], pct: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

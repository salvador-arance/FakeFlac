//! `DetectionParams` (S4, ADR-0011). Mirrors `specs/detection/detection-params.v3.toml`;
//! a test checks that `Default` equals that file. Values are data, never constants in the algorithm.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowKind {
    #[serde(rename = "hann")]
    Hann,
    #[serde(rename = "blackman_harris_4")]
    BlackmanHarris4,
    #[serde(rename = "nuttall_4")]
    Nuttall4,
    #[serde(rename = "kaiser")]
    Kaiser,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectionParams {
    pub algorithm_version: u32,
    pub input: InputParams,
    pub fft: FftParams,
    pub bands: BandParams,
    pub extent: ExtentParams,
    pub step: StepParams,
    pub gaps: GapParams,
    /// Full-band guard (v3, ADR-0038). `None` disables it (v2 behaviour).
    #[serde(default)]
    pub full_band: Option<FullBandParams>,
    pub verdict: VerdictParams,
    pub spectrogram: SpectrogramParams,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputParams {
    pub min_duration_s: f64,
    pub edge_skip_percent: f64,
    pub segments: u32,
    pub frames_per_segment: u32,
    pub silence_dbfs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FftParams {
    pub target_bin_hz: f64,
    pub window: WindowKind,
    pub kaiser_beta: f64,
    pub overlap: f64,
    /// Width of the bands of the mean and median spectra.
    pub band_hz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandParams {
    pub ref_band_low_hz: f64,
    pub ref_band_high_hz: f64,
    pub hf_band_low_hz: f64,
    pub hf_band_high_hz: f64,
    pub nyquist_guard_hz: f64,
    pub noise_floor_percentile: f64,
}

/// Content extent `fe` (rules.md section 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtentParams {
    pub content_drop_db: f64,
    pub content_min_width_hz: f64,
}

/// Cutoff step `D` at `fs` (rules.md section 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepParams {
    pub search_low_hz: f64,
    pub search_top_margin_hz: f64,
    pub window_bands: u32,
    pub rebound_db: f64,
}

/// Gap step `G` (rules.md section 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapParams {
    pub edge_hz: f64,
    pub band_hz: f64,
}

/// Upper window and thresholds of the full-band guard (rules.md section 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullBandParams {
    pub low_hz: f64,
    pub high_hz: f64,
    pub min_level_db: f64,
    pub min_burst_db: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictParams {
    pub low_hf_threshold_db: f64,
    pub gap_fake_db: f64,
    pub gap_suspicious_db: f64,
    pub step_fake_db: f64,
    pub step_suspicious_db: f64,
    pub step_low_suspicious_db: f64,
    pub step_high_suspicious_db: f64,
    pub lossy_min_cutoff_hz: f64,
    pub fake_max_cutoff_hz: f64,
    pub ambiguous_max_cutoff_hz: f64,
    pub gradual_min_cutoff_hz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectrogramParams {
    pub window: WindowKind,
    pub fft_size: u32,
    pub overlap: f64,
    pub max_width: u32,
    pub height: u32,
    pub db_floor: f64,
    pub db_ceiling: f64,
}

impl Default for DetectionParams {
    /// DetectionParams v3: the values of `specs/detection/detection-params.v3.toml`
    /// (a test checks the equality).
    fn default() -> Self {
        DetectionParams {
            algorithm_version: 3,
            input: InputParams {
                min_duration_s: 5.0,
                edge_skip_percent: 5.0,
                segments: 64,
                frames_per_segment: 4,
                silence_dbfs: -60.0,
            },
            fft: FftParams {
                target_bin_hz: 10.8,
                window: WindowKind::Hann,
                kaiser_beta: 10.0,
                overlap: 0.5,
                band_hz: 100.0,
            },
            bands: BandParams {
                ref_band_low_hz: 1000.0,
                ref_band_high_hz: 10000.0,
                hf_band_low_hz: 10000.0,
                hf_band_high_hz: 16000.0,
                nyquist_guard_hz: 100.0,
                noise_floor_percentile: 5.0,
            },
            extent: ExtentParams {
                content_drop_db: 60.0,
                content_min_width_hz: 200.0,
            },
            step: StepParams {
                search_low_hz: 11000.0,
                search_top_margin_hz: 400.0,
                window_bands: 3,
                rebound_db: 6.0,
            },
            gaps: GapParams {
                edge_hz: 16000.0,
                band_hz: 700.0,
            },
            full_band: Some(FullBandParams {
                low_hz: 20700.0,
                high_hz: 21000.0,
                min_level_db: -70.0,
                min_burst_db: 3.0,
            }),
            verdict: VerdictParams {
                low_hf_threshold_db: -60.0,
                gap_fake_db: 6.0,
                gap_suspicious_db: 4.0,
                step_fake_db: 10.0,
                step_suspicious_db: 7.0,
                step_low_suspicious_db: 10.0,
                step_high_suspicious_db: 12.0,
                lossy_min_cutoff_hz: 14500.0,
                fake_max_cutoff_hz: 20300.0,
                ambiguous_max_cutoff_hz: 20800.0,
                gradual_min_cutoff_hz: 19000.0,
            },
            spectrogram: SpectrogramParams {
                window: WindowKind::Hann,
                fft_size: 2048,
                overlap: 0.75,
                max_width: 1200,
                height: 512,
                db_floor: -120.0,
                db_ceiling: 0.0,
            },
        }
    }
}

/// A parameter set whose values break an invariant that the verdict rules rely on
/// (rules.md section 4).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParamsError {
    #[error(
        "invalid parameters: gap_suspicious_db ≤ gap_fake_db does not hold ({suspicious} > {fake})"
    )]
    GapThresholds { suspicious: f64, fake: f64 },
    #[error("invalid parameters: step_suspicious_db ≤ step_fake_db does not hold ({suspicious} > {fake})")]
    StepThresholds { suspicious: f64, fake: f64 },
    #[error("invalid parameters: fake_max_cutoff_hz < full_band.low_hz does not hold ({fake_max} >= {low})")]
    FullBandBelowLossyRange { fake_max: f64, low: f64 },
    #[error(
        "invalid parameters: full_band.low_hz < full_band.high_hz does not hold ({low} >= {high})"
    )]
    FullBandWindowEmpty { low: f64, high: f64 },
}

impl DetectionParams {
    /// Checks the value invariants of rules.md section 4. Every loader calls it after
    /// deserialising and refuses an invalid file.
    pub fn validate(&self) -> Result<(), ParamsError> {
        let v = &self.verdict;
        if v.gap_suspicious_db > v.gap_fake_db {
            return Err(ParamsError::GapThresholds {
                suspicious: v.gap_suspicious_db,
                fake: v.gap_fake_db,
            });
        }
        if v.step_suspicious_db > v.step_fake_db {
            return Err(ParamsError::StepThresholds {
                suspicious: v.step_suspicious_db,
                fake: v.step_fake_db,
            });
        }
        if let Some(fb) = &self.full_band {
            if v.fake_max_cutoff_hz >= fb.low_hz {
                return Err(ParamsError::FullBandBelowLossyRange {
                    fake_max: v.fake_max_cutoff_hz,
                    low: fb.low_hz,
                });
            }
            if fb.low_hz >= fb.high_hz {
                return Err(ParamsError::FullBandWindowEmpty {
                    low: fb.low_hz,
                    high: fb.high_hz,
                });
            }
        }
        Ok(())
    }
}

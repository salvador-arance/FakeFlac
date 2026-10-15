//! `DetectionParams` (S4, ADR-0011). Mirrors `specs/detection/detection-params.v1.toml`;
//! a test checks that `Default` equals that file. Values are data, never constants in the algorithm.

use serde::{Deserialize, Serialize};

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
    pub cutoff: CutoffParams,
    pub verdict: VerdictParams,
    pub spectrogram: SpectrogramParams,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputParams {
    pub min_duration_s: f64,
    pub edge_skip_percent: f64,
    pub segments: u32,
    pub silence_dbfs: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FftParams {
    pub target_bin_hz: f64,
    pub window: WindowKind,
    pub kaiser_beta: f64,
    pub overlap: f64,
    pub smoothing_hz: f64,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutoffParams {
    pub content_drop_db: f64,
    pub content_min_width_hz: f64,
    pub steepness_span_hz: f64,
    pub full_band_margin_hz: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictParams {
    pub low_hf_threshold_db: f64,
    pub cliff_min_steepness_db_per_khz: f64,
    pub fake_max_cutoff_hz: f64,
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
    /// DetectionParams v1.
    fn default() -> Self {
        DetectionParams {
            algorithm_version: 1,
            input: InputParams {
                min_duration_s: 5.0,
                edge_skip_percent: 5.0,
                segments: 32,
                silence_dbfs: -70.0,
            },
            fft: FftParams {
                target_bin_hz: 5.4,
                window: WindowKind::BlackmanHarris4,
                kaiser_beta: 10.0,
                overlap: 0.75,
                smoothing_hz: 100.0,
            },
            bands: BandParams {
                ref_band_low_hz: 1000.0,
                ref_band_high_hz: 10000.0,
                hf_band_low_hz: 10000.0,
                hf_band_high_hz: 16000.0,
                nyquist_guard_hz: 100.0,
                noise_floor_percentile: 5.0,
            },
            cutoff: CutoffParams {
                content_drop_db: 60.0,
                content_min_width_hz: 200.0,
                steepness_span_hz: 500.0,
                full_band_margin_hz: 1000.0,
            },
            verdict: VerdictParams {
                low_hf_threshold_db: -60.0,
                cliff_min_steepness_db_per_khz: 30.0,
                fake_max_cutoff_hz: 20500.0,
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

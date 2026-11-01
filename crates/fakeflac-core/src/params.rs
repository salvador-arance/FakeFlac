//! `DetectionParams` (S4, ADR-0011). Mirrors `specs/detection/detection-params.v8.toml`;
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
    /// Lossy-file rules (v5, `specs/detection/lossy-source.md`). `None` (a v2-v4 file): lossy files
    /// are not judged.
    #[serde(default)]
    pub lossy: Option<LossyParams>,
    /// Edge step on V4 (v6, ADR-0042). `None` (a v2-v5 file): `EdgeStep` never holds.
    #[serde(default)]
    pub edge: Option<EdgeParams>,
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
    /// v4 (ADR-0039): a cutoff step `D` at or above this with `fs` in the lossy range means
    /// not full band. `None` (a v3 file) disables the condition.
    #[serde(default)]
    pub max_lossy_step_db: Option<f64>,
}

/// The `[edge]` section (rules.md sections 2 and 4, ADR-0042).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeParams {
    pub floor_low_hz: f64,
    pub floor_high_hz: f64,
    pub floor_min_db: f64,
    pub max_median_step_db: f64,
    pub min_residual_db: f64,
    pub max_step_db: f64,
}

/// The `[lossy]` section (lossy-source.md section 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LossyParams {
    pub suspicious_shortfall_hz: f64,
    pub fake_shortfall_hz: f64,
    pub fake_min_step_db: f64,
    pub cutoffs: LossyCutoffs,
    /// Rule L6a (v7, lossy-source.md section 11). `None` (a v5 or v6 file): L6a never fires.
    #[serde(default)]
    pub streaming: Option<StreamingParams>,
}

/// A codec that rule L6a may judge (lossy-source.md section 11.3). Only `"mp3"` and `"aac"`
/// deserialise; any other value, `"opus"` included, is an error naming it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamingCodec {
    #[serde(rename = "mp3")]
    Mp3,
    #[serde(rename = "aac")]
    Aac,
}

/// The `[lossy.streaming]` section (lossy-source.md section 11.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamingParams {
    /// Decoded sample rates that count, compared exactly.
    pub sample_rates_hz: Vec<u32>,
    pub codecs: Vec<StreamingCodec>,
    /// The declared bitrate `b` must be at or above this.
    pub min_declared_bitrate_kbps: u32,
    /// The reported cutoff `fc` must lie in `[min_cutoff_hz, max_cutoff_hz)`.
    pub min_cutoff_hz: f64,
    pub max_cutoff_hz: f64,
}

/// The `[lossy.cutoffs]` tables: expected low-pass per declared bitrate, per codec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LossyCutoffs {
    pub mp3: Vec<CutoffRow>,
    pub aac: Vec<CutoffRow>,
    #[serde(default)]
    pub opus: Option<Vec<CutoffRow>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutoffRow {
    pub bitrate_kbps: u32,
    pub cutoff_hz: f64,
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
    /// DetectionParams v8: the values of `specs/detection/detection-params.v8.toml`
    /// (a test checks the equality).
    fn default() -> Self {
        DetectionParams {
            algorithm_version: 8,
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
                max_lossy_step_db: Some(3.0),
            }),
            lossy: Some(LossyParams {
                suspicious_shortfall_hz: 1000.0,
                fake_shortfall_hz: 2500.0,
                fake_min_step_db: 10.0,
                cutoffs: LossyCutoffs {
                    mp3: vec![
                        CutoffRow {
                            bitrate_kbps: 32,
                            cutoff_hz: 5500.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 40,
                            cutoff_hz: 7000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 48,
                            cutoff_hz: 7500.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 56,
                            cutoff_hz: 10000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 64,
                            cutoff_hz: 11000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 80,
                            cutoff_hz: 13500.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 96,
                            cutoff_hz: 15100.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 112,
                            cutoff_hz: 15600.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 128,
                            cutoff_hz: 17000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 160,
                            cutoff_hz: 17500.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 192,
                            cutoff_hz: 18600.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 224,
                            cutoff_hz: 19400.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 256,
                            cutoff_hz: 19700.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 320,
                            cutoff_hz: 20500.0,
                        },
                    ],
                    aac: vec![
                        CutoffRow {
                            bitrate_kbps: 64,
                            cutoff_hz: 11000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 96,
                            cutoff_hz: 13000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 128,
                            cutoff_hz: 15500.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 160,
                            cutoff_hz: 16000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 192,
                            cutoff_hz: 17000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 256,
                            cutoff_hz: 19000.0,
                        },
                        CutoffRow {
                            bitrate_kbps: 320,
                            cutoff_hz: 19500.0,
                        },
                    ],
                    opus: Some(
                        [48, 64, 96, 128, 160, 192]
                            .into_iter()
                            .map(|bitrate_kbps| CutoffRow {
                                bitrate_kbps,
                                cutoff_hz: 20000.0,
                            })
                            .collect(),
                    ),
                },
                streaming: Some(StreamingParams {
                    sample_rates_hz: vec![48_000],
                    codecs: vec![StreamingCodec::Mp3, StreamingCodec::Aac],
                    min_declared_bitrate_kbps: 192,
                    min_cutoff_hz: 19_800.0,
                    max_cutoff_hz: 20_200.0,
                }),
            }),
            edge: Some(EdgeParams {
                floor_low_hz: 20900.0,
                floor_high_hz: 22000.0,
                floor_min_db: -104.0,
                max_median_step_db: 3.0,
                min_residual_db: 15.0,
                max_step_db: 16.0,
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
    #[error("invalid parameters: full_band.max_lossy_step_db ≥ 0 does not hold ({value})")]
    MaxLossyStepNegative { value: f64 },
    #[error(
        "invalid parameters: full_band.max_lossy_step_db ≤ step_fake_db does not hold ({value} > {fake})"
    )]
    MaxLossyStepAboveStepFake { value: f64, fake: f64 },
    /// A `[lossy]` check failed (lossy-source.md section 6); the text names the inequality.
    #[error("invalid parameters: {check} does not hold")]
    Lossy { check: String },
    /// An `[edge]` check failed (rules.md section 4); the text names the inequality.
    #[error("invalid parameters: {check} does not hold")]
    Edge { check: String },
}

impl DetectionParams {
    /// Checks the value invariants of rules.md section 4. Every loader calls it after
    /// deserialising and refuses an invalid file.
    pub fn validate(&self) -> Result<(), ParamsError> {
        let v = &self.verdict;
        if !le(v.gap_suspicious_db, v.gap_fake_db) {
            return Err(ParamsError::GapThresholds {
                suspicious: v.gap_suspicious_db,
                fake: v.gap_fake_db,
            });
        }
        if !le(v.step_suspicious_db, v.step_fake_db) {
            return Err(ParamsError::StepThresholds {
                suspicious: v.step_suspicious_db,
                fake: v.step_fake_db,
            });
        }
        if let Some(fb) = &self.full_band {
            if !lt(v.fake_max_cutoff_hz, fb.low_hz) {
                return Err(ParamsError::FullBandBelowLossyRange {
                    fake_max: v.fake_max_cutoff_hz,
                    low: fb.low_hz,
                });
            }
            if !lt(fb.low_hz, fb.high_hz) {
                return Err(ParamsError::FullBandWindowEmpty {
                    low: fb.low_hz,
                    high: fb.high_hz,
                });
            }
            if let Some(max) = fb.max_lossy_step_db {
                if !le(0.0, max) {
                    return Err(ParamsError::MaxLossyStepNegative { value: max });
                }
                if !le(max, v.step_fake_db) {
                    return Err(ParamsError::MaxLossyStepAboveStepFake {
                        value: max,
                        fake: v.step_fake_db,
                    });
                }
            }
        }
        if let Some(l) = &self.lossy {
            validate_lossy(l, v.step_suspicious_db)?;
        }
        if let Some(e) = &self.edge {
            validate_edge(
                e,
                v.fake_max_cutoff_hz,
                f64::from(self.step.window_bands) * self.fft.band_hz,
                v.step_fake_db,
            )?;
        }
        Ok(())
    }
}

/// The `[edge]` checks of rules.md section 4, in the order listed there.
fn validate_edge(
    e: &EdgeParams,
    fake_max_cutoff_hz: f64,
    step_window_hz: f64,
    step_fake_db: f64,
) -> Result<(), ParamsError> {
    let fail = |check: &str| {
        Err(ParamsError::Edge {
            check: check.to_string(),
        })
    };
    if !le(fake_max_cutoff_hz + step_window_hz, e.floor_low_hz) {
        return fail("edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz");
    }
    if !lt(e.floor_low_hz, e.floor_high_hz) {
        return fail("edge.floor_low_hz < edge.floor_high_hz");
    }
    if !lt(e.floor_min_db, 0.0) {
        return fail("edge.floor_min_db < 0");
    }
    if !e.max_median_step_db.is_finite() {
        return fail("edge.max_median_step_db is finite");
    }
    if !lt(0.0, e.min_residual_db) {
        return fail("edge.min_residual_db > 0");
    }
    if !lt(step_fake_db, e.max_step_db) {
        return fail("edge.max_step_db > step_fake_db");
    }
    Ok(())
}

fn lossy_error(check: &str) -> ParamsError {
    ParamsError::Lossy {
        check: check.to_string(),
    }
}

/// The checks of lossy-source.md section 6, in the order listed there; the first failure is reported.
fn validate_lossy(l: &LossyParams, step_suspicious_db: f64) -> Result<(), ParamsError> {
    if !lt(0.0, l.suspicious_shortfall_hz) {
        return Err(lossy_error("lossy.suspicious_shortfall_hz > 0"));
    }
    if !le(l.suspicious_shortfall_hz, l.fake_shortfall_hz) {
        return Err(lossy_error(
            "lossy.suspicious_shortfall_hz ≤ lossy.fake_shortfall_hz",
        ));
    }
    if !le(step_suspicious_db, l.fake_min_step_db) {
        return Err(lossy_error("step_suspicious_db ≤ lossy.fake_min_step_db"));
    }
    let tables = [
        ("mp3", Some(&l.cutoffs.mp3)),
        ("aac", Some(&l.cutoffs.aac)),
        ("opus", l.cutoffs.opus.as_ref()),
    ];
    for (name, table) in tables {
        let Some(rows) = table else { continue };
        if rows.is_empty() {
            return Err(lossy_error(&format!("lossy.cutoffs.{name} is not empty")));
        }
        if rows.iter().any(|r| r.bitrate_kbps == 0) {
            return Err(lossy_error(&format!(
                "lossy.cutoffs.{name}: bitrate_kbps > 0"
            )));
        }
        if rows.iter().any(|r| !lt(0.0, r.cutoff_hz)) {
            return Err(lossy_error(&format!("lossy.cutoffs.{name}: cutoff_hz > 0")));
        }
        if rows
            .windows(2)
            .any(|w| w[0].bitrate_kbps >= w[1].bitrate_kbps)
        {
            return Err(lossy_error(&format!(
                "lossy.cutoffs.{name}: bitrate_kbps increases"
            )));
        }
        if rows.windows(2).any(|w| !le(w[0].cutoff_hz, w[1].cutoff_hz)) {
            return Err(lossy_error(&format!(
                "lossy.cutoffs.{name}: cutoff_hz does not decrease"
            )));
        }
    }
    if let Some(st) = &l.streaming {
        validate_streaming(st)?;
    }
    Ok(())
}

/// The `[lossy.streaming]` checks of lossy-source.md section 11.3, in the order listed there.
fn validate_streaming(s: &StreamingParams) -> Result<(), ParamsError> {
    if s.sample_rates_hz.is_empty() {
        return Err(lossy_error("lossy.streaming.sample_rates_hz is not empty"));
    }
    if s.sample_rates_hz.contains(&0) {
        return Err(lossy_error(
            "lossy.streaming.sample_rates_hz: sample rate > 0",
        ));
    }
    if s.sample_rates_hz.windows(2).any(|w| w[0] >= w[1]) {
        return Err(lossy_error("lossy.streaming.sample_rates_hz increases"));
    }
    if s.codecs.is_empty() {
        return Err(lossy_error("lossy.streaming.codecs is not empty"));
    }
    if s.codecs
        .iter()
        .enumerate()
        .any(|(i, c)| s.codecs[..i].contains(c))
    {
        return Err(lossy_error("lossy.streaming.codecs has no duplicates"));
    }
    if s.min_declared_bitrate_kbps == 0 {
        return Err(lossy_error("lossy.streaming.min_declared_bitrate_kbps > 0"));
    }
    if !lt(0.0, s.min_cutoff_hz) {
        return Err(lossy_error("lossy.streaming.min_cutoff_hz > 0"));
    }
    if !lt(s.min_cutoff_hz, s.max_cutoff_hz) {
        return Err(lossy_error(
            "lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz",
        ));
    }
    Ok(())
}

/// `a <= b`; false when either is NaN, so a NaN ordering is reported as not holding.
fn le(a: f64, b: f64) -> bool {
    matches!(
        a.partial_cmp(&b),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// `a < b`; false when either is NaN.
fn lt(a: f64, b: f64) -> bool {
    matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Less))
}

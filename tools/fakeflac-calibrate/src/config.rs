//! `calibration.v1.toml` (data, not code): split, generation, report and targets.

use crate::error::ToolError;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub config_version: u32,
    pub split: SplitConfig,
    pub generate: GenerateConfig,
    pub report: ReportConfig,
    #[serde(default)]
    pub targets: Vec<Target>,
}

/// Inputs.md section 4.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SplitConfig {
    pub seed: String,
    pub holdout_fraction: f64,
    pub min_holdout_albums: u32,
    pub disc_folder_prefixes: Vec<String>,
}

/// Generation.md sections 1 to 3. `fragment_s` omitted = full track.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateConfig {
    pub fragment_s: Option<f64>,
    pub keep_audio: bool,
    pub keep_misclassified: bool,
    pub decode_args: Vec<String>,
    pub settings: Vec<Setting>,
}

/// One `[[generate.settings]]`. `encoder` is `lame`, `aac`, `opus`, or `fixture` (tests only).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    pub id: String,
    pub encoder: String,
    pub ext: String,
    pub bitrate_kbps: Option<u32>,
    pub args: Vec<String>,
}

/// Evaluation.md sections 3 and 8.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportConfig {
    pub z: f64,
    pub small_sample_tracks: u32,
    pub small_sample_albums: u32,
    pub near_db: f64,
    pub near_hz: f64,
}

/// Evaluation.md section 6: rows selected by `classes` or by `settings`, with a verdict and a maximum.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub id: String,
    pub description: Option<String>,
    pub classes: Option<Vec<String>>,
    pub settings: Option<Vec<String>>,
    pub verdict: String,
    pub max_count: u32,
}

impl Config {
    /// Parses the text of a config file. Errors are exit 2.
    pub fn parse(_text: &str) -> Result<Config, ToolError> {
        todo!("RP-11: calibration.md section 2")
    }
}

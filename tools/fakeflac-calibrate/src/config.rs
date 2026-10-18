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

/// Encoders a setting may name (`fixture` is for tests: the binary rejects it).
pub const ENCODERS: [&str; 4] = ["lame", "aac", "opus", "fixture"];

impl Config {
    /// Parses the text of a config file. Errors are exit 2.
    pub fn parse(text: &str) -> Result<Config, ToolError> {
        let config: Config = toml::from_str(text).map_err(|e| {
            ToolError::input_lines(e.to_string().lines().map(str::to_string).collect())
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ToolError> {
        let mut errors: Vec<String> = Vec::new();
        if self.config_version != 1 {
            errors.push(format!(
                "config_version {} is not supported (expected 1)",
                self.config_version
            ));
        }
        if !(0.0..=1.0).contains(&self.split.holdout_fraction) {
            errors.push("split.holdout_fraction must be between 0 and 1".to_string());
        }
        if let Some(f) = self.generate.fragment_s {
            if f.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                errors.push("generate.fragment_s must be greater than 0".to_string());
            }
        }
        if self.report.z.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            errors.push("report.z must be greater than 0".to_string());
        }
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.generate.settings {
            if s.id.is_empty() || s.id == "none" || s.id == "confirmed_fake" {
                errors.push(format!("generate.settings: invalid id '{}'", s.id));
            }
            if !ids.insert(s.id.as_str()) {
                errors.push(format!("generate.settings: id '{}' is repeated", s.id));
            }
            if !ENCODERS.contains(&s.encoder.as_str()) {
                errors.push(format!(
                    "generate.settings '{}': unknown encoder '{}' (expected one of: {})",
                    s.id,
                    s.encoder,
                    ENCODERS.join(", ")
                ));
            }
            if s.ext.is_empty()
                || s.ext.eq_ignore_ascii_case("flac")
                || s.ext.contains(['/', '\\', '.'])
            {
                errors.push(format!(
                    "generate.settings '{}': invalid ext '{}'",
                    s.id, s.ext
                ));
            }
        }
        for t in &self.targets {
            if !["real", "suspicious", "fake"].contains(&t.verdict.as_str()) {
                errors.push(format!(
                    "targets '{}': verdict must be real, suspicious or fake",
                    t.id
                ));
            }
            if t.classes.is_none() && t.settings.is_none() {
                errors.push(format!("targets '{}': needs classes or settings", t.id));
            }
            for s in t.settings.iter().flatten() {
                if !ids.contains(s.as_str()) {
                    errors.push(format!("targets '{}': unknown setting '{s}'", t.id));
                }
            }
            for c in t.classes.iter().flatten() {
                if !["primary", "reviewed", "clean", "reference", "positive"].contains(&c.as_str())
                {
                    errors.push(format!("targets '{}': unknown class '{c}'", t.id));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ToolError::input_lines(errors))
        }
    }
}

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
    /// `[labels.upstream_by_rate]` (lossy.md section 1.5); absent in v1.
    #[serde(default)]
    pub labels: Labels,
}

/// `[labels]` of `calibration.v2.toml`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Labels {
    /// Decoded sample rate (as text, `"44100"`) to upstream codec and optional source bound.
    #[serde(default)]
    pub upstream_by_rate: std::collections::BTreeMap<String, UpstreamRate>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "RawUpstreamRate")]
pub struct UpstreamRate {
    pub upstream: String,
    /// `None` when absent or out of range (the latter is reported by validation).
    pub source_max_kbps: Option<u32>,
    /// `source_max_kbps` was present but is not an integer from 8 to 640.
    pub source_max_kbps_invalid: bool,
}

/// `source_max_kbps` is read as any TOML value so that a bad one is a validation error with the
/// documented message rather than a TOML type error.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUpstreamRate {
    upstream: String,
    source_max_kbps: Option<toml::Value>,
}

impl From<RawUpstreamRate> for UpstreamRate {
    fn from(raw: RawUpstreamRate) -> Self {
        let parsed = raw.source_max_kbps.as_ref().map(|v| match v {
            toml::Value::Integer(n) if (8..=640).contains(n) => Some(*n as u32),
            _ => None,
        });
        UpstreamRate {
            upstream: raw.upstream,
            source_max_kbps: parsed.flatten(),
            source_max_kbps_invalid: parsed == Some(None),
        }
    }
}

/// One `[[generate.lossy]]` (lossy.md section 4.1): setting ids applied in order.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LossyEntry {
    pub id: String,
    pub stages: Vec<String>,
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
    #[serde(default)]
    pub min_removed_hf_db: Option<f64>,
    pub settings: Vec<Setting>,
    /// Lossy entries (lossy.md section 4); empty in v1.
    #[serde(default)]
    pub lossy: Vec<LossyEntry>,
}

/// One `[[generate.settings]]`. `encoder` is `lame`, `aac`, `opus`, or `fixture` (tests only).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setting {
    pub id: String,
    pub encoder: String,
    pub ext: String,
    pub bitrate_kbps: Option<u32>,
    #[serde(default)]
    pub lowpass_hz: Option<u32>,
    /// `false`: the setting is only a stage of lossy entries (lossy.md section 4.1).
    #[serde(default = "default_true")]
    pub lossless_transcode: bool,
    pub args: Vec<String>,
    /// Decode arguments of this setting instead of `generate.decode_args` (config v3, generation.md
    /// section 2). `None` uses `generate.decode_args`.
    #[serde(default)]
    pub decode_args: Option<Vec<String>>,
}

impl GenerateConfig {
    /// The decode arguments used for `setting`: its own, or `generate.decode_args`.
    pub fn decode_args_for<'a>(&'a self, setting: &'a Setting) -> &'a [String] {
        setting.decode_args.as_deref().unwrap_or(&self.decode_args)
    }
}

fn default_true() -> bool {
    true
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
    /// The settings that get a FLAC transcode (`lossless_transcode`, lossy.md section 4.1): the
    /// rows of every lossless table.
    pub fn lossless_settings(&self) -> impl Iterator<Item = &Setting> {
        self.generate
            .settings
            .iter()
            .filter(|s| s.lossless_transcode)
    }

    /// The settings of a lossy entry, in stage order (a stage id that does not exist is skipped).
    pub fn stages_of(&self, entry: &LossyEntry) -> Vec<&Setting> {
        entry
            .stages
            .iter()
            .filter_map(|id| self.generate.settings.iter().find(|s| &s.id == id))
            .collect()
    }

    /// Lossy.md section 4.1: checks on `[[generate.lossy]]`, before any ffmpeg call.
    fn validate_lossy(&self, errors: &mut Vec<String>) {
        let mut seen = std::collections::BTreeSet::new();
        for entry in &self.generate.lossy {
            let id = &entry.id;
            if !seen.insert(id.as_str()) {
                errors.push(format!("lossy entry {id} is defined twice"));
            }
            if entry.stages.is_empty() {
                errors.push(format!("lossy entry {id}: no stages"));
                continue;
            }
            let mut stages: Vec<&Setting> = Vec::new();
            for stage in &entry.stages {
                match self.generate.settings.iter().find(|s| &s.id == stage) {
                    Some(s) => stages.push(s),
                    None => errors.push(format!("lossy entry {id}: unknown stage {stage}")),
                }
            }
            if stages.len() != entry.stages.len() {
                continue;
            }
            if stages.len() >= 2 {
                for s in &stages {
                    if s.bitrate_kbps.is_none() {
                        errors.push(format!(
                            "lossy entry {id}: stage {} has no bitrate_kbps",
                            s.id
                        ));
                    }
                }
            }
            if let Some(last) = stages.last() {
                let ext = last.ext.to_ascii_lowercase();
                if ext == "opus" {
                    errors.push(format!(
                        "lossy entry {id}: Opus output is not supported yet"
                    ));
                } else if ext != "mp3" && ext != "m4a" {
                    errors.push(format!(
                        "lossy entry {id}: the last stage's ext must be mp3 or m4a"
                    ));
                }
            }
        }
    }

    /// Lossy.md section 1.5: checks on `[labels.upstream_by_rate]`.
    fn validate_upstream_by_rate(&self, errors: &mut Vec<String>) {
        for (key, entry) in &self.labels.upstream_by_rate {
            let positive_integer =
                !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()) && !key.starts_with('0');
            if !positive_integer {
                errors.push(format!(
                    "labels.upstream_by_rate: invalid sample rate '{key}' (expected an integer in Hz, for example 44100)"
                ));
            }
            if !["aac", "opus", "mp3", "unknown"].contains(&entry.upstream.as_str()) {
                errors.push(format!(
                    "labels.upstream_by_rate '{key}': invalid upstream '{}' (expected aac, opus, mp3 or unknown)",
                    entry.upstream
                ));
            }
            if entry.source_max_kbps_invalid {
                errors.push(format!(
                    "labels.upstream_by_rate '{key}': source_max_kbps must be an integer from 8 to 640"
                ));
            }
        }
    }

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
        if !(1..=3).contains(&self.config_version) {
            errors.push(format!(
                "config_version {} is not supported (expected 1, 2 or 3)",
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
        self.validate_lossy(&mut errors);
        self.validate_upstream_by_rate(&mut errors);
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

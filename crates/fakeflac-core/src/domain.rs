//! Domain model. Mirrors `docs/architecture/backend.md` (section "Domain model").
//! Behaviour is specified in `specs/`; nothing here may be implemented ahead of an approved spec.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Result of analysing a track, relative to the quality the file *declares* (ADR-031).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Real,
    Suspicious,
    Fake,
    Error,
}

/// Supported input formats (RF-14, ADR-030).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
    M4a,
    Opus,
}

impl AudioFormat {
    /// Format for a file extension (case-insensitive, without the dot). H1 supports WAV and FLAC (RF-14).
    pub fn from_extension(ext: &str) -> Option<AudioFormat> {
        let _ = ext;
        todo!("RF-14")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuspicionReason {
    /// Little high-frequency content (a cappella, soft instrumentals): R1.
    LowHighFrequencyContent,
    /// Roll-off without a sharp cliff.
    GradualRolloff,
    AmbiguousCutoff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutoffEstimate {
    pub frequency_hz: f32,
    pub steepness_db_per_khz: f32,
    pub noise_floor_db: f32,
}

/// Technical metadata of a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub path: PathBuf,
    pub folder: PathBuf,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    /// Declared bitrate in kbps (lossy formats only).
    pub declared_bitrate_kbps: Option<u32>,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub modified_unix: i64,
}

/// Per-track error. Travels as data, never as a panic (ADR-013, RNF-04).
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "detail")]
pub enum TrackError {
    #[error("unreadable file: {0}")]
    Unreadable(String),
    #[error("corrupt stream: {0}")]
    CorruptStream(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("track too short to analyse")]
    TooShort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackAnalysis {
    pub info: TrackInfo,
    pub verdict: Verdict,
    pub cutoff: Option<CutoffEstimate>,
    /// Estimated source bitrate in kbps (RF-15, lossy formats).
    pub estimated_source_bitrate_kbps: Option<u32>,
    pub reasons: Vec<SuspicionReason>,
    pub error: Option<TrackError>,
    pub algorithm_version: u32,
}

/// Per-folder summary (RF-10). The summary verdict rule lives in `specs/detection/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSummary {
    pub folder: PathBuf,
    pub total: u32,
    pub real: u32,
    pub suspicious: u32,
    pub fake: u32,
    pub errors: u32,
    pub verdict: Verdict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_serializes_in_camel_case() {
        let json = serde_json::to_string(&Verdict::Suspicious).expect("serialize");
        assert_eq!(json, "\"suspicious\"");
    }
}

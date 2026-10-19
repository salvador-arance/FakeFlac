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
    /// Format for a file extension (case-insensitive, without the dot). WAV and FLAC (H1), MP3 and
    /// M4A (H2, ADR-0040); Opus is still skipped (RF-14).
    pub fn from_extension(ext: &str) -> Option<AudioFormat> {
        match ext.to_ascii_lowercase().as_str() {
            "wav" => Some(AudioFormat::Wav),
            "flac" => Some(AudioFormat::Flac),
            "mp3" => Some(AudioFormat::Mp3),
            "m4a" => Some(AudioFormat::M4a),
            _ => None,
        }
    }
}

impl AudioCodec {
    /// Codec implied by the container alone, for files that were not probed (WAV, FLAC, MP3).
    pub fn from_format(format: AudioFormat) -> Option<AudioCodec> {
        match format {
            AudioFormat::Wav => Some(AudioCodec::Pcm),
            AudioFormat::Flac => Some(AudioCodec::Flac),
            AudioFormat::Mp3 => Some(AudioCodec::Mp3),
            AudioFormat::M4a | AudioFormat::Opus => None,
        }
    }

    /// Lossy codecs have a declared bitrate (`specs/detection/lossy-source.md` section 2.3).
    pub fn is_lossy(self) -> bool {
        matches!(self, AudioCodec::Mp3 | AudioCodec::Aac | AudioCodec::Opus)
    }
}

/// The codec inside the container (contract version 3, `specs/detection/lossy-source.md` section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioCodec {
    Pcm,
    Flac,
    Alac,
    Mp3,
    Aac,
    Opus,
}

/// Bitrate mode of an MP3 stream (contract version 3, `specs/detection/lossy-source.md` section 2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BitrateMode {
    Cbr,
    Vbr,
    Abr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuspicionReason {
    /// Little high-frequency content (a cappella, soft instrumentals): R1.
    LowHighFrequencyContent,
    /// Roll-off without a sharp cliff.
    GradualRolloff,
    AmbiguousCutoff,
    /// Intermittent empty band above 16 kHz, as left by MP3 encoders (algorithm_version 2).
    HighBandGaps,
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
    /// The codec inside the container; `None` only when an M4A could not be probed.
    pub codec: Option<AudioCodec>,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    /// Declared bitrate in kbps (lossy formats only).
    pub declared_bitrate_kbps: Option<u32>,
    /// MP3 only (AAC does not signal it).
    pub bitrate_mode: Option<BitrateMode>,
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

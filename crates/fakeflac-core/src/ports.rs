//! Ports (traits) implemented by the adapter crates (`docs/architecture/backend.md`).
//! Only the ports listed in the architecture document exist; adding one requires an ADR (D3).

use crate::domain::{AudioFormat, TrackError};
use crate::params::DetectionParams;
use crate::spectrum::AverageSpectrum;
use std::path::PathBuf;

/// A supported file found by the scanner, with its file-system metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub folder: PathBuf,
    pub format: AudioFormat,
    pub size_bytes: u64,
    pub modified_unix: i64,
}

/// What the audio adapter learns from decoding a file.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioProbe {
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    pub duration_ms: u64,
    /// `None` when every analysed frame was silent.
    pub spectrum: Option<AverageSpectrum>,
}

/// Decodes a file and computes its average spectrum (RF-03). Read-only (RNF-05).
pub trait SpectrumAnalyzer: Send + Sync {
    fn analyze(
        &self,
        path: &std::path::Path,
        params: &DetectionParams,
    ) -> Result<AudioProbe, TrackError>;
}

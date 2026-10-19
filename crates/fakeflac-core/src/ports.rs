//! Ports (traits) implemented by the adapter crates (`docs/architecture/backend.md`).
//! Only the ports listed in the architecture document exist; adding one requires an ADR (D3).

use crate::domain::{AudioCodec, AudioFormat, BitrateMode, TrackError};
use crate::error::AppError;
use crate::params::DetectionParams;
use crate::spectrum::BandSpectra;
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
    /// The codec inside the container (`specs/detection/lossy-source.md` section 2).
    pub codec: Option<AudioCodec>,
    /// Declared bitrate in kbps (lossy codecs only, section 2.1 and 2.2).
    pub declared_bitrate_kbps: Option<u32>,
    pub bitrate_mode: Option<BitrateMode>,
    /// Encoder string of the LAME tag (backend only, not in the contract).
    pub encoder: Option<String>,
    /// Low-pass of the LAME tag in Hz; absent when the byte is 0 (backend only).
    pub encoder_lowpass_hz: Option<u32>,
    /// `None` when every analysed frame was silent.
    pub spectrum: Option<BandSpectra>,
}

/// Decodes a file and computes its band spectra (RF-03). Read-only (RNF-05).
pub trait SpectrumAnalyzer: Send + Sync {
    fn analyze(
        &self,
        path: &std::path::Path,
        params: &DetectionParams,
    ) -> Result<AudioProbe, TrackError>;
}

/// Lists the supported files under a folder (RF-02, RF-14). Read-only (RNF-05).
pub trait FileScanner: Send + Sync {
    /// Every supported file under `root`, recursively. Hidden files and folders (name starting
    /// with `.`) are skipped and symbolic links are not followed. Order is unspecified.
    /// Fails with `NOT_A_FOLDER` when `root` is a file, and with `FOLDER_UNREADABLE` when it is
    /// not an absolute path, does not exist or cannot be listed (`specs/contracts/commands.md`).
    fn scan(&self, root: &std::path::Path) -> Result<Vec<FileEntry>, AppError>;
}

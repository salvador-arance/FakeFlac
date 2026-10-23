//! The `Transcoder` trait internal to the tool (ADR-0036, not a core port). The binary uses
//! `ffmpeg::FfmpegTranscoder`; tests use a fixture transcoder (sample-corpus.md section 1).

use crate::config::Setting;
use crate::error::ToolError;
use std::path::{Path, PathBuf};

/// A fragment of the source: `-ss start_s -t duration_s`. `None` in a request means the full track.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fragment {
    pub start_s: f64,
    pub duration_s: f64,
}

/// What to transcode: `source` (read-only) through `setting` into `generated_dir/<id>.flac`
/// (16-bit FLAC after `decode_args`). The lossy intermediate `<id>.<ext>` is deleted by the transcoder.
#[derive(Debug, Clone)]
pub struct TranscodeRequest<'a> {
    pub source: &'a Path,
    pub setting: &'a Setting,
    pub fragment: Option<Fragment>,
    pub decode_args: &'a [String],
    pub generated_dir: &'a Path,
    pub id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscodeOutput {
    /// `generated_dir/<id>.flac`
    pub flac: PathBuf,
    /// Exact encoder version (generation.md section 2); `fixture` for the fixture transcoder.
    pub encoder_version: String,
}

/// A lossy entry (lossy.md section 4.3): `source` through `stages` in order (at least one setting).
/// The last stage writes `generated_dir/<id>.<ext of the last stage>`; intermediates are deleted.
#[derive(Debug, Clone)]
pub struct LossyRequest<'a> {
    pub source: &'a Path,
    pub stages: &'a [&'a Setting],
    pub fragment: Option<Fragment>,
    pub generated_dir: &'a Path,
    pub id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossyOutput {
    /// `generated_dir/<id>.<ext>`
    pub file: PathBuf,
    /// The version string of each stage, joined with ` > `.
    pub encoder_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscodeError {
    /// The program could not be started (exit 3).
    NotFound,
    /// One call failed: first line of its error output. That entry gets no row and is retried next run.
    Failed(String),
}

pub trait Transcoder: Send + Sync {
    /// Whether `setting.encoder` is accepted. The ffmpeg transcoder rejects `fixture` (the run ends with exit 2).
    fn supports(&self, encoder: &str) -> bool;
    /// `ffmpeg_version` (first line of `ffmpeg -version` cut before ` Copyright`). Called only when a
    /// transcode is needed or to check staleness; `Err` has kind `ExternalTool`.
    fn version(&self) -> Result<String, ToolError>;
    fn transcode(&self, request: &TranscodeRequest<'_>) -> Result<TranscodeOutput, TranscodeError>;
    /// Generates the lossy file of a `[[generate.lossy]]` entry (RF-15, lossy.md section 4.3).
    fn transcode_lossy(&self, request: &LossyRequest<'_>) -> Result<LossyOutput, TranscodeError>;
}

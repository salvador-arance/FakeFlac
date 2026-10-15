//! Contract types sent to the frontend (`specs/contracts/commands.md`). They mirror the core
//! types so the core stays free of `specta`; the TS bindings are generated from these (S3).
//! Paths become strings (lossy for the rare non-UTF-8 Windows path).

use fakeflac_core as core;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    Real,
    Suspicious,
    Fake,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Wav,
    Flac,
    Mp3,
    M4a,
    Opus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SuspicionReason {
    LowHighFrequencyContent,
    GradualRolloff,
    AmbiguousCutoff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub path: String,
    pub folder: String,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    pub declared_bitrate_kbps: Option<u32>,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub modified_unix: i64,
}

/// Exported as plain `number` (specta would add `| null` for floats, because JSON has no NaN):
/// the conversion only produces finite values.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CutoffEstimate {
    #[specta(type = specta_typescript::Number)]
    pub frequency_hz: f32,
    #[specta(type = specta_typescript::Number)]
    pub steepness_db_per_khz: f32,
    #[specta(type = specta_typescript::Number)]
    pub noise_floor_db: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TrackError {
    Unreadable { detail: String },
    CorruptStream { detail: String },
    UnsupportedFormat { detail: String },
    TooShort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackAnalysis {
    pub info: TrackInfo,
    pub verdict: Verdict,
    pub cutoff: Option<CutoffEstimate>,
    pub estimated_source_bitrate_kbps: Option<u32>,
    pub reasons: Vec<SuspicionReason>,
    pub error: Option<TrackError>,
    pub algorithm_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub root: String,
    pub total: u32,
    pub done: u32,
    pub real: u32,
    pub suspicious: u32,
    pub fake: u32,
    pub errors: u32,
    pub cancelled: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProgressEvent {
    Scanned { total: u32 },
    TrackStarted { index: u32, path: String },
    TrackDone { index: u32, analysis: TrackAnalysis },
    Finished { summary: AnalysisSummary },
    Cancelled { summary: AnalysisSummary },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    NoAudioFound,
    FolderUnreadable,
    AnalysisRunning,
    NotAnalyzed,
    ExportFailed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
}

impl From<&core::TrackAnalysis> for TrackAnalysis {
    fn from(a: &core::TrackAnalysis) -> Self {
        let _ = a;
        todo!("RF-01: TrackAnalysis DTO")
    }
}

impl From<&core::AnalysisSummary> for AnalysisSummary {
    fn from(s: &core::AnalysisSummary) -> Self {
        let _ = s;
        todo!("RF-01: AnalysisSummary DTO")
    }
}

impl From<&core::ProgressEvent> for ProgressEvent {
    fn from(e: &core::ProgressEvent) -> Self {
        let _ = e;
        todo!("RF-01: ProgressEvent DTO")
    }
}

impl From<core::AppError> for AppError {
    fn from(e: core::AppError) -> Self {
        let _ = e;
        todo!("RF-01: AppError DTO")
    }
}

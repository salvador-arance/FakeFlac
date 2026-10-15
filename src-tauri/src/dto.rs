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

fn path_string(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

/// The contract type is a plain `number`, which cannot carry NaN or infinity: those become 0.0.
fn finite(v: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

impl From<core::Verdict> for Verdict {
    fn from(v: core::Verdict) -> Self {
        match v {
            core::Verdict::Real => Verdict::Real,
            core::Verdict::Suspicious => Verdict::Suspicious,
            core::Verdict::Fake => Verdict::Fake,
            core::Verdict::Error => Verdict::Error,
        }
    }
}

impl From<core::AudioFormat> for AudioFormat {
    fn from(f: core::AudioFormat) -> Self {
        match f {
            core::AudioFormat::Wav => AudioFormat::Wav,
            core::AudioFormat::Flac => AudioFormat::Flac,
            core::AudioFormat::Mp3 => AudioFormat::Mp3,
            core::AudioFormat::M4a => AudioFormat::M4a,
            core::AudioFormat::Opus => AudioFormat::Opus,
        }
    }
}

impl From<core::SuspicionReason> for SuspicionReason {
    fn from(r: core::SuspicionReason) -> Self {
        match r {
            core::SuspicionReason::LowHighFrequencyContent => {
                SuspicionReason::LowHighFrequencyContent
            }
            core::SuspicionReason::GradualRolloff => SuspicionReason::GradualRolloff,
            core::SuspicionReason::AmbiguousCutoff => SuspicionReason::AmbiguousCutoff,
        }
    }
}

impl From<&core::TrackInfo> for TrackInfo {
    fn from(i: &core::TrackInfo) -> Self {
        TrackInfo {
            path: path_string(&i.path),
            folder: path_string(&i.folder),
            format: i.format.into(),
            sample_rate: i.sample_rate,
            bit_depth: i.bit_depth,
            channels: i.channels,
            declared_bitrate_kbps: i.declared_bitrate_kbps,
            duration_ms: i.duration_ms,
            size_bytes: i.size_bytes,
            modified_unix: i.modified_unix,
        }
    }
}

impl From<&core::CutoffEstimate> for CutoffEstimate {
    fn from(c: &core::CutoffEstimate) -> Self {
        CutoffEstimate {
            frequency_hz: finite(c.frequency_hz),
            steepness_db_per_khz: finite(c.steepness_db_per_khz),
            noise_floor_db: finite(c.noise_floor_db),
        }
    }
}

impl From<&core::TrackError> for TrackError {
    fn from(e: &core::TrackError) -> Self {
        match e {
            core::TrackError::Unreadable(d) => TrackError::Unreadable { detail: d.clone() },
            core::TrackError::CorruptStream(d) => TrackError::CorruptStream { detail: d.clone() },
            core::TrackError::UnsupportedFormat(d) => {
                TrackError::UnsupportedFormat { detail: d.clone() }
            }
            core::TrackError::TooShort => TrackError::TooShort,
        }
    }
}

impl From<&core::TrackAnalysis> for TrackAnalysis {
    fn from(a: &core::TrackAnalysis) -> Self {
        TrackAnalysis {
            info: (&a.info).into(),
            verdict: a.verdict.into(),
            cutoff: a.cutoff.as_ref().map(Into::into),
            estimated_source_bitrate_kbps: a.estimated_source_bitrate_kbps,
            reasons: a.reasons.iter().copied().map(Into::into).collect(),
            error: a.error.as_ref().map(Into::into),
            algorithm_version: a.algorithm_version,
        }
    }
}

impl From<&core::AnalysisSummary> for AnalysisSummary {
    fn from(s: &core::AnalysisSummary) -> Self {
        AnalysisSummary {
            root: path_string(&s.root),
            total: s.total,
            done: s.done,
            real: s.real,
            suspicious: s.suspicious,
            fake: s.fake,
            errors: s.errors,
            cancelled: s.cancelled,
            elapsed_ms: s.elapsed_ms,
        }
    }
}

impl From<&core::ProgressEvent> for ProgressEvent {
    fn from(e: &core::ProgressEvent) -> Self {
        match e {
            core::ProgressEvent::Scanned { total } => ProgressEvent::Scanned { total: *total },
            core::ProgressEvent::TrackStarted { index, path } => ProgressEvent::TrackStarted {
                index: *index,
                path: path_string(path),
            },
            core::ProgressEvent::TrackDone { index, analysis } => ProgressEvent::TrackDone {
                index: *index,
                analysis: analysis.into(),
            },
            core::ProgressEvent::Finished { summary } => ProgressEvent::Finished {
                summary: summary.into(),
            },
            core::ProgressEvent::Cancelled { summary } => ProgressEvent::Cancelled {
                summary: summary.into(),
            },
        }
    }
}

impl From<core::AppErrorCode> for AppErrorCode {
    fn from(c: core::AppErrorCode) -> Self {
        match c {
            core::AppErrorCode::NoAudioFound => AppErrorCode::NoAudioFound,
            core::AppErrorCode::FolderUnreadable => AppErrorCode::FolderUnreadable,
            core::AppErrorCode::AnalysisRunning => AppErrorCode::AnalysisRunning,
            core::AppErrorCode::NotAnalyzed => AppErrorCode::NotAnalyzed,
            core::AppErrorCode::ExportFailed => AppErrorCode::ExportFailed,
            core::AppErrorCode::Internal => AppErrorCode::Internal,
        }
    }
}

impl From<core::AppError> for AppError {
    fn from(e: core::AppError) -> Self {
        AppError {
            code: e.code.into(),
            message: e.message,
        }
    }
}

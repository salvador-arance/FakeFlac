//! Contract types sent to the frontend (`specs/contracts/commands.md`). They mirror the core
//! types so the core stays free of `specta`; the TS bindings are generated from these (S3).
//! Paths become strings (lossy for the rare non-UTF-8 Windows path).

use fakeflac_core as core;
use serde::{Deserialize, Serialize};
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
pub enum AudioCodec {
    Pcm,
    Flac,
    Alac,
    Mp3,
    Aac,
    Opus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BitrateMode {
    Cbr,
    Vbr,
    Abr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SuspicionReason {
    LowHighFrequencyContent,
    GradualRolloff,
    AmbiguousCutoff,
    HighBandGaps,
    CutoffBelowDeclared,
    // Contract version 6 (RF-16, rule L6a): a streaming sample rate with Opus's 20 kHz band edge.
    StreamingSampleRate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub path: String,
    pub folder: String,
    pub format: AudioFormat,
    pub codec: Option<AudioCodec>,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    pub declared_bitrate_kbps: Option<u32>,
    pub bitrate_mode: Option<BitrateMode>,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub modified_unix: i64,
}

/// Exported as plain `number` (specta would add `| null` for floats, because JSON has no NaN):
/// a non-finite estimate is sent as no cutoff (`finite_cutoff`).
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
#[serde(rename_all = "camelCase")]
pub struct AlbumSummary {
    pub folder: String,
    pub total: u32,
    pub real: u32,
    pub suspicious: u32,
    pub fake: u32,
    pub errors: u32,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AlbumUpdate {
    pub index: u32,
    pub name: String,
    pub files: u32,
    pub summary: AlbumSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
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
    /// Contract version 7 (RF-11): tracks whose `trackDone` carried a cached result.
    pub from_cache: u32,
    /// Contract version 7 (RF-11): how the run used the result cache.
    pub cache: CacheStatus,
}

/// Contract version 7 (RF-11, `specs/contracts/result-cache.md` section 8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CacheStatus {
    Off,
    Ok,
    Degraded,
    Unavailable,
}

#[allow(clippy::large_enum_variant)] // `TrackDone` carries the track and its album (RF-10)
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProgressEvent {
    Scanned {
        total: u32,
    },
    TrackStarted {
        index: u32,
        path: String,
    },
    TrackDone {
        index: u32,
        analysis: TrackAnalysis,
        album: AlbumUpdate,
    },
    Finished {
        summary: AnalysisSummary,
    },
    Cancelled {
        summary: AnalysisSummary,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    NoAudioFound,
    NotAFolder,
    FolderUnreadable,
    AnalysisRunning,
    NotAnalyzed,
    ExportFailed,
    // Contract version 7 (RF-11): `clear_cache` could not delete the cache files.
    CacheFailed,
    Internal,
    // Contract version 8 (RF-13): only from `set_settings` (`specs/contracts/settings.md`).
    SettingsWriteFailed,
    SettingsReadOnly,
}

/// Contract version 8 (RF-13): an interface language. Exactly `en` or `es`; case variants and
/// region tags are rejected by deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Es,
}

/// Contract version 8 (RF-13): the language-only application settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub language: Language,
}

/// Contract version 8 (RF-13): how the settings of this process were loaded
/// (`specs/contracts/settings.md` section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SettingsLoadStatus {
    Default,
    Loaded,
    Invalid,
    Unavailable,
    NewerVersion,
}

/// Contract version 8 (RF-13): the active settings and how they were loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub settings: Settings,
    pub load_status: SettingsLoadStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
}

fn path_string(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

/// The contract's `number` cannot carry NaN or infinity. An estimate with such a value is not
/// shown as a plausible-looking number: the track is sent without a cutoff instead.
fn finite_cutoff(c: &core::CutoffEstimate) -> Option<CutoffEstimate> {
    let values = [c.frequency_hz, c.steepness_db_per_khz, c.noise_floor_db];
    values.iter().all(|v| v.is_finite()).then(|| c.into())
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

impl From<core::AudioCodec> for AudioCodec {
    fn from(c: core::AudioCodec) -> Self {
        match c {
            core::AudioCodec::Pcm => AudioCodec::Pcm,
            core::AudioCodec::Flac => AudioCodec::Flac,
            core::AudioCodec::Alac => AudioCodec::Alac,
            core::AudioCodec::Mp3 => AudioCodec::Mp3,
            core::AudioCodec::Aac => AudioCodec::Aac,
            core::AudioCodec::Opus => AudioCodec::Opus,
        }
    }
}

impl From<core::BitrateMode> for BitrateMode {
    fn from(m: core::BitrateMode) -> Self {
        match m {
            core::BitrateMode::Cbr => BitrateMode::Cbr,
            core::BitrateMode::Vbr => BitrateMode::Vbr,
            core::BitrateMode::Abr => BitrateMode::Abr,
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
            core::SuspicionReason::HighBandGaps => SuspicionReason::HighBandGaps,
            core::SuspicionReason::CutoffBelowDeclared => SuspicionReason::CutoffBelowDeclared,
            core::SuspicionReason::StreamingSampleRate => SuspicionReason::StreamingSampleRate,
        }
    }
}

impl From<&core::TrackInfo> for TrackInfo {
    fn from(i: &core::TrackInfo) -> Self {
        TrackInfo {
            path: path_string(&i.path),
            folder: path_string(&i.folder),
            format: i.format.into(),
            codec: i.codec.map(Into::into),
            sample_rate: i.sample_rate,
            bit_depth: i.bit_depth,
            channels: i.channels,
            declared_bitrate_kbps: i.declared_bitrate_kbps,
            bitrate_mode: i.bitrate_mode.map(Into::into),
            duration_ms: i.duration_ms,
            size_bytes: i.size_bytes,
            modified_unix: i.modified_unix,
        }
    }
}

impl From<&core::CutoffEstimate> for CutoffEstimate {
    fn from(c: &core::CutoffEstimate) -> Self {
        CutoffEstimate {
            frequency_hz: c.frequency_hz,
            steepness_db_per_khz: c.steepness_db_per_khz,
            noise_floor_db: c.noise_floor_db,
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
            cutoff: a.cutoff.as_ref().and_then(finite_cutoff),
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
            from_cache: s.from_cache,
            cache: s.cache.into(),
        }
    }
}

impl From<&core::AlbumSummary> for AlbumSummary {
    fn from(s: &core::AlbumSummary) -> Self {
        AlbumSummary {
            folder: path_string(&s.folder),
            total: s.total,
            real: s.real,
            suspicious: s.suspicious,
            fake: s.fake,
            errors: s.errors,
            verdict: s.verdict.into(),
        }
    }
}

impl From<&core::AlbumUpdate> for AlbumUpdate {
    fn from(a: &core::AlbumUpdate) -> Self {
        AlbumUpdate {
            index: a.index,
            name: path_string(&a.name),
            files: a.files,
            summary: (&a.summary).into(),
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
            core::ProgressEvent::TrackDone {
                index,
                analysis,
                album,
            } => ProgressEvent::TrackDone {
                index: *index,
                analysis: analysis.into(),
                album: album.into(),
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

impl From<core::CacheStatus> for CacheStatus {
    fn from(c: core::CacheStatus) -> Self {
        match c {
            core::CacheStatus::Off => CacheStatus::Off,
            core::CacheStatus::Ok => CacheStatus::Ok,
            core::CacheStatus::Degraded => CacheStatus::Degraded,
            core::CacheStatus::Unavailable => CacheStatus::Unavailable,
        }
    }
}

impl From<core::AppErrorCode> for AppErrorCode {
    fn from(c: core::AppErrorCode) -> Self {
        match c {
            core::AppErrorCode::NoAudioFound => AppErrorCode::NoAudioFound,
            core::AppErrorCode::NotAFolder => AppErrorCode::NotAFolder,
            core::AppErrorCode::FolderUnreadable => AppErrorCode::FolderUnreadable,
            core::AppErrorCode::AnalysisRunning => AppErrorCode::AnalysisRunning,
            core::AppErrorCode::NotAnalyzed => AppErrorCode::NotAnalyzed,
            core::AppErrorCode::ExportFailed => AppErrorCode::ExportFailed,
            core::AppErrorCode::CacheFailed => AppErrorCode::CacheFailed,
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

/// RF-07. `data` is serialised as a JSON number array (`number[]` in the contract).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Spectrogram {
    pub width: u32,
    pub height: u32,
    pub sample_rate: u32,
    #[specta(type = specta_typescript::Number)]
    pub max_frequency_hz: f64,
    #[specta(type = (specta_typescript::Number, specta_typescript::Number))]
    pub db_range: (f64, f64),
    pub data: Vec<u8>,
}

impl From<&core::Spectrogram> for Spectrogram {
    fn from(s: &core::Spectrogram) -> Self {
        Spectrogram {
            width: s.width,
            height: s.height,
            sample_rate: s.sample_rate,
            max_frequency_hz: s.max_frequency_hz,
            db_range: (s.db_range[0], s.db_range[1]),
            data: s.data.clone(),
        }
    }
}

/// Result of `export_csv` (contract version 4, RF-09).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    /// Absolute path of the CSV written.
    pub path: String,
    /// Tracks written (header not counted).
    pub rows: u32,
}

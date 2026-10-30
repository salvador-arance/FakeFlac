//! Contract-level errors (`AppError`, `specs/contracts/commands.md`). Per-track problems are
//! not errors at this level: they travel inside `TrackAnalysis` (ADR-0013).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    NoAudioFound,
    NotAFolder,
    FolderUnreadable,
    AnalysisRunning,
    NotAnalyzed,
    ExportFailed,
    /// `clear_cache` could not delete the cache files (RF-11, commands.md version 7).
    CacheFailed,
    Internal,
}

impl AppErrorCode {
    /// The code as it appears in the contract (`NO_AUDIO_FOUND`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            AppErrorCode::NoAudioFound => "NO_AUDIO_FOUND",
            AppErrorCode::NotAFolder => "NOT_A_FOLDER",
            AppErrorCode::FolderUnreadable => "FOLDER_UNREADABLE",
            AppErrorCode::AnalysisRunning => "ANALYSIS_RUNNING",
            AppErrorCode::NotAnalyzed => "NOT_ANALYZED",
            AppErrorCode::ExportFailed => "EXPORT_FAILED",
            AppErrorCode::CacheFailed => "CACHE_FAILED",
            AppErrorCode::Internal => "INTERNAL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("{}: {message}", code.as_str())]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: AppErrorCode, message: impl Into<String>) -> AppError {
        AppError {
            code,
            message: message.into(),
        }
    }
}

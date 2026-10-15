//! Contract-level errors (`AppError`, `specs/contracts/commands.md`). Per-track problems are
//! not errors at this level: they travel inside `TrackAnalysis` (ADR-0013).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    NoAudioFound,
    FolderUnreadable,
    AnalysisRunning,
    NotAnalyzed,
    ExportFailed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("{code:?}: {message}")]
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

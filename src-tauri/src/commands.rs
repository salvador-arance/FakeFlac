//! Tauri commands (`specs/contracts/commands.md`). Thin: each one delegates to `run` or a plugin.

use crate::dto;
use fakeflac_core::AnalysisSlot;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

/// RF-01: native folder dialog. `None` when the user cancels it.
#[tauri::command]
#[specta::specta]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    let _ = app;
    todo!("RF-01: pick_folder")
}

/// RF-01..06: analyses `root`, streaming `ProgressEvent`s through `on_progress`.
#[tauri::command]
#[specta::specta]
pub async fn start_analysis(
    slot: State<'_, std::sync::Arc<AnalysisSlot>>,
    root: String,
    on_progress: Channel<dto::ProgressEvent>,
) -> Result<dto::AnalysisSummary, dto::AppError> {
    let _ = (slot, root, on_progress);
    todo!("RF-01: start_analysis")
}

/// RF-05: cancels the running analysis; no-op if none is running.
#[tauri::command]
#[specta::specta]
pub fn cancel_analysis(slot: State<'_, std::sync::Arc<AnalysisSlot>>) {
    let _ = slot;
    todo!("RF-05: cancel_analysis")
}

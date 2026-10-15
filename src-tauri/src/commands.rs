//! Tauri commands (`specs/contracts/commands.md`). Thin: each one delegates to `run` or a plugin.

use crate::{dto, run};
use fakeflac_core::AnalysisSlot;
use std::path::Path;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

/// RF-01: native folder dialog. `None` when the user cancels it.
#[tauri::command]
#[specta::specta]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    let picked = app.dialog().file().blocking_pick_folder()?;
    Some(match picked.into_path() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(other) => other.to_string(),
    })
}

/// RF-01..06: analyses `root`, streaming `ProgressEvent`s through `on_progress`.
#[tauri::command]
#[specta::specta]
pub async fn start_analysis(
    slot: State<'_, std::sync::Arc<AnalysisSlot>>,
    root: String,
    on_progress: Channel<dto::ProgressEvent>,
) -> Result<dto::AnalysisSummary, dto::AppError> {
    let slot = std::sync::Arc::clone(&slot);
    tauri::async_runtime::spawn_blocking(move || {
        run::run_analysis(&slot, Path::new(&root), &|e| on_progress.send(e).is_ok())
    })
    .await
    .map_err(|e| dto::AppError {
        code: dto::AppErrorCode::Internal,
        message: format!("analysis task failed: {e}"),
    })?
}

/// RF-05: cancels the running analysis; no-op if none is running.
#[tauri::command]
#[specta::specta]
pub fn cancel_analysis(slot: State<'_, std::sync::Arc<AnalysisSlot>>) {
    slot.cancel();
}

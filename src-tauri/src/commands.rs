//! Tauri commands (`specs/contracts/commands.md`). Thin: each one delegates to `run` or a plugin.

use crate::{dto, run};
use fakeflac_core::AnalysisSlot;
use std::path::Path;
use tauri::ipc::Channel;
use tauri::{State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// RF-01: native folder dialog. `None` when the user cancels it.
#[tauri::command]
#[specta::specta]
pub async fn pick_folder(window: WebviewWindow) -> Option<String> {
    // Modal to the window (one dialog at a time); waits on a blocking thread, not on an async
    // runtime worker.
    let picked = tauri::async_runtime::spawn_blocking(move || {
        window
            .dialog()
            .file()
            .set_parent(&window)
            .blocking_pick_folder()
    })
    .await
    .ok()
    .flatten()?;
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
    export: State<'_, std::sync::Arc<run::ExportState>>,
    root: String,
    on_progress: Channel<dto::ProgressEvent>,
) -> Result<dto::AnalysisSummary, dto::AppError> {
    let slot = std::sync::Arc::clone(&slot);
    let export = std::sync::Arc::clone(&export);
    tauri::async_runtime::spawn_blocking(move || {
        run::run_analysis_kept(&slot, &export, Path::new(&root), &|e| {
            on_progress.send(e).is_ok()
        })
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

/// RF-07: spectrogram of one file for display. Decoding runs on a blocking thread.
#[tauri::command]
#[specta::specta]
pub async fn get_spectrogram(path: String) -> Result<dto::Spectrogram, dto::AppError> {
    tauri::async_runtime::spawn_blocking(move || run::get_spectrogram(Path::new(&path)))
        .await
        .map_err(|e| dto::AppError {
            code: dto::AppErrorCode::Internal,
            message: format!("spectrogram task failed: {e}"),
        })?
}

/// RF-09: saves the results of the latest run as CSV. The destination is chosen in the native
/// save dialog (never by the page). `None` when the user cancels it.
#[tauri::command]
#[specta::specta]
pub async fn export_csv(
    window: WebviewWindow,
    slot: State<'_, std::sync::Arc<AnalysisSlot>>,
    export: State<'_, std::sync::Arc<run::ExportState>>,
) -> Result<Option<dto::ExportResult>, dto::AppError> {
    let (snapshot, guard) = run::begin_export(&slot, &export)?;
    let file_name = fakeflac_export::default_file_name(&snapshot.root);
    // The guard lives until the export ends, whatever the outcome.
    let result = tauri::async_runtime::spawn_blocking(move || {
        let picked = window
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter("CSV (*.csv)", &["csv"])
            .set_file_name(file_name)
            .blocking_save_file();
        let Some(picked) = picked else {
            return Ok(None);
        };
        let dest = picked.into_path().map_err(|e| dto::AppError {
            code: dto::AppErrorCode::ExportFailed,
            message: format!("the chosen destination is not a file path: {e}"),
        })?;
        run::export_csv(&snapshot, &dest).map(Some)
    })
    .await
    .map_err(|e| dto::AppError {
        code: dto::AppErrorCode::Internal,
        message: format!("export task failed: {e}"),
    });
    drop(guard);
    result?
}

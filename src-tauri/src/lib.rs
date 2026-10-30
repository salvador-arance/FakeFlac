//! Tauri adapter and composition root (ADR-001). Wiring and commands only:
//! no domain logic lives here (see `src-tauri/CLAUDE.md`).

pub mod commands;
pub mod dto;
pub mod run;

use fakeflac_core::AnalysisSlot;
use std::path::Path;
use std::sync::Arc;
use tauri::Manager;

/// Commands and types of the contract, shared by the app and the bindings export.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::pick_folder,
            commands::start_analysis,
            commands::cancel_analysis,
            commands::get_spectrogram,
            commands::export_csv,
            commands::clear_cache,
        ])
        // u64/i64 fields (durations, sizes, times) stay far below 2^53: the contract says `number`.
        .dangerously_cast_bigints_to_number()
}

/// Writes the generated TS bindings to `path` (`just gen` → `src/lib/api/bindings.ts`).
pub fn export_bindings(path: &Path) -> Result<(), String> {
    specta_builder()
        .export(specta_typescript::Typescript::default(), path)
        .map_err(|e| e.to_string())
}

pub fn run() {
    let builder = specta_builder();
    // Failing to start the shell is unrecoverable; this is the one place where `expect` is the Tauri idiom.
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AnalysisSlot::new()))
        .manage(Arc::new(run::ExportState::new()))
        .invoke_handler(builder.invoke_handler())
        // A reload (F5, dev server) leaves no listener for the running analysis: cancel it so the
        // new page is not told `ANALYSIS_RUNNING` until the old run ends.
        .on_page_load(|webview, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Started {
                webview.state::<Arc<AnalysisSlot>>().cancel();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running the FakeFlac application");
}

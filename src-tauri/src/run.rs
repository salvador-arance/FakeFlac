//! Wiring of the analyze-folder use case to the real adapters (RF-01, RF-02, RF-05).
//! Kept apart from the Tauri commands so it can be tested without a Tauri runtime.

use crate::dto;
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::{analyze_folder, AnalysisSlot, DetectionParams};
use fakeflac_fs::WalkDirScanner;
use std::path::Path;

/// Runs one analysis of `root` with the file-system scanner and the symphonia analyzer.
///
/// Takes the `slot` for the whole run (`ANALYSIS_RUNNING` if another run holds it), converts
/// every event to its contract type and hands it to `send`. When `send` returns `false` the run
/// is cancelled (Tauri rarely reports a closed page this way; a page reload cancels the run in
/// `lib.rs`). Uses `cores − 1` workers (at least 1).
pub fn run_analysis(
    slot: &AnalysisSlot,
    root: &Path,
    send: &(dyn Fn(dto::ProgressEvent) -> bool + Sync),
) -> Result<dto::AnalysisSummary, dto::AppError> {
    // Held until the end of the function: dropping it frees the slot.
    let run = slot.begin()?;
    let summary = analyze_folder(
        &WalkDirScanner,
        &SymphoniaAnalyzer,
        &DetectionParams::default(),
        root,
        worker_count(),
        run.token(),
        &|e| {
            if !send((&e).into()) {
                run.token().cancel();
            }
        },
    )?;
    Ok((&summary).into())
}

/// Worker threads for a run: available cores − 1, at least 1 (ADR-0034).
pub fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .max(1)
}

/// RF-07: spectrogram of `path` with the default `SpectrogramParams`. A file that cannot be
/// decoded fails with `INTERNAL` (`specs/contracts/commands.md`).
pub fn get_spectrogram(_path: &Path) -> Result<dto::Spectrogram, dto::AppError> {
    todo!("RF-07")
}

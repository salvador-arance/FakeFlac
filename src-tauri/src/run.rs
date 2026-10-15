//! Wiring of the analyze-folder use case to the real adapters (RF-01, RF-02, RF-05).
//! Kept apart from the Tauri commands so it can be tested without a Tauri runtime.

use crate::dto;
use fakeflac_core::AnalysisSlot;
use std::path::Path;

/// Runs one analysis of `root` with the file-system scanner and the symphonia analyzer.
///
/// Takes the `slot` for the whole run (`ANALYSIS_RUNNING` if another run holds it), converts
/// every event to its contract type and hands it to `send`. When `send` returns `false` (the
/// window is gone) the run is cancelled. Uses `cores − 1` workers (at least 1).
pub fn run_analysis(
    slot: &AnalysisSlot,
    root: &Path,
    send: &(dyn Fn(dto::ProgressEvent) -> bool + Sync),
) -> Result<dto::AnalysisSummary, dto::AppError> {
    let _ = (slot, root, send);
    todo!("RF-01/RF-05: run_analysis")
}

/// Worker threads for a run: available cores − 1, at least 1 (ADR-0034).
pub fn worker_count() -> usize {
    todo!("RF-05: worker_count")
}

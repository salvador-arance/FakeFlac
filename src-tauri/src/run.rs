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
pub fn get_spectrogram(path: &Path) -> Result<dto::Spectrogram, dto::AppError> {
    let params = DetectionParams::default();
    SymphoniaAnalyzer
        .spectrogram(path, &params.spectrogram)
        .map(|s| (&s).into())
        .map_err(|e| {
            dto::AppError::from(fakeflac_core::AppError::new(
                fakeflac_core::AppErrorCode::Internal,
                format!("cannot decode {}: {e:?}", path.display()),
            ))
        })
}

// --- RF-09: CSV export (specs/contracts/csv-export.md section 5, commands.md "CSV export") ---

/// The results of the latest run, kept in memory for the CSV export, and the "export in
/// progress" flag (csv-export.md section 5.2).
#[derive(Debug, Default)]
pub struct ExportState {}

/// The results taken when the export starts, before the dialog opens.
#[derive(Debug, Clone)]
pub struct ExportSnapshot {
    /// The `root` of the run the tracks come from (the dialog proposes `default_file_name(root)`).
    pub root: std::path::PathBuf,
    /// The tracks of that run in `index` (scan) order.
    pub tracks: Vec<fakeflac_core::TrackAnalysis>,
}

/// Proof that this export is the one in progress; dropping it frees the export slot.
#[derive(Debug)]
pub struct ExportGuard {}

impl Drop for ExportGuard {
    fn drop(&mut self) {}
}

impl ExportState {
    pub fn new() -> ExportState {
        ExportState::default()
    }

    /// Records one event of the run of `root`: `Scanned` replaces the kept results, `TrackDone`
    /// adds a track at its `index`; other events change nothing.
    pub fn record(&self, _root: &Path, _event: &fakeflac_core::ProgressEvent) {
        todo!("RF-09: keep the results of the latest run")
    }
}

/// Like `run_analysis`, and records every event in `state` before passing it to `send`.
pub fn run_analysis_kept(
    _slot: &AnalysisSlot,
    _state: &ExportState,
    _root: &Path,
    _send: &(dyn Fn(dto::ProgressEvent) -> bool + Sync),
) -> Result<dto::AnalysisSummary, dto::AppError> {
    todo!("RF-09: run an analysis and keep its results")
}

/// Step before the dialog. Checks, in this order: an export in progress (`INTERNAL`, "an export
/// is already in progress"), a run holding `slot` (`ANALYSIS_RUNNING`), no results
/// (`NOT_ANALYZED`). Returns the snapshot and the guard that lives until the export ends.
pub fn begin_export(
    _slot: &AnalysisSlot,
    _state: &ExportState,
) -> Result<(ExportSnapshot, ExportGuard), dto::AppError> {
    todo!("RF-09: begin_export")
}

/// Step after the dialog: writes the snapshot to `dest` (`EXPORT_FAILED` unless `dest` is an
/// absolute `.csv` path that can be written) and returns the rows written and `dest`.
pub fn export_csv(
    _snapshot: &ExportSnapshot,
    _dest: &Path,
) -> Result<dto::ExportResult, dto::AppError> {
    todo!("RF-09: export_csv")
}

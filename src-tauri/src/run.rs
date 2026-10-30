//! Wiring of the analyze-folder use case to the real adapters (RF-01, RF-02, RF-05).
//! Kept apart from the Tauri commands so it can be tested without a Tauri runtime.

use crate::dto;
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::{analyze_folder, AnalysisSlot, DetectionParams};
use fakeflac_fs::WalkDirScanner;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    run_with(slot, None, root, send)
}

/// Shared body of `run_analysis` and `run_analysis_kept`.
fn run_with(
    slot: &AnalysisSlot,
    state: Option<&ExportState>,
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
            if let Some(state) = state {
                state.record(root, &e);
            }
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

#[derive(Debug, Default)]
struct Kept {
    root: Option<PathBuf>,
    tracks: BTreeMap<u32, fakeflac_core::TrackAnalysis>,
}

/// The results of the latest run, kept in memory for the CSV export, and the "export in
/// progress" flag (csv-export.md section 5.2).
#[derive(Debug, Default)]
pub struct ExportState {
    kept: Mutex<Kept>,
    exporting: Arc<AtomicBool>,
}

/// The results taken when the export starts, before the dialog opens.
#[derive(Debug, Clone)]
pub struct ExportSnapshot {
    /// The `root` of the run the tracks come from (the dialog proposes `default_file_name(root)`).
    pub root: PathBuf,
    /// The tracks of that run in `index` (scan) order.
    pub tracks: Vec<fakeflac_core::TrackAnalysis>,
}

/// Proof that this export is the one in progress; dropping it frees the export slot.
#[derive(Debug)]
pub struct ExportGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for ExportGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

fn internal(message: &str) -> dto::AppError {
    fakeflac_core::AppError::new(fakeflac_core::AppErrorCode::Internal, message).into()
}

impl ExportState {
    pub fn new() -> ExportState {
        ExportState::default()
    }

    /// Records one event of the run of `root`: `Scanned` replaces the kept results, `TrackDone`
    /// adds a track at its `index`; other events change nothing.
    pub fn record(&self, root: &Path, event: &fakeflac_core::ProgressEvent) {
        use fakeflac_core::ProgressEvent;
        let mut kept = self
            .kept
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match event {
            ProgressEvent::Scanned { .. } => {
                kept.root = Some(root.to_path_buf());
                kept.tracks.clear();
            }
            ProgressEvent::TrackDone {
                index, analysis, ..
            } => {
                if kept.root.is_none() {
                    kept.root = Some(root.to_path_buf());
                }
                kept.tracks.insert(*index, analysis.clone());
            }
            _ => {}
        }
    }
}

/// Like `run_analysis`, and records every event in `state` before passing it to `send`.
pub fn run_analysis_kept(
    slot: &AnalysisSlot,
    state: &ExportState,
    root: &Path,
    send: &(dyn Fn(dto::ProgressEvent) -> bool + Sync),
) -> Result<dto::AnalysisSummary, dto::AppError> {
    run_with(slot, Some(state), root, send)
}

/// Step before the dialog. Checks, in this order: an export in progress (`INTERNAL`, "an export
/// is already in progress"), a run holding `slot` (`ANALYSIS_RUNNING`), no results
/// (`NOT_ANALYZED`). Returns the snapshot and the guard that lives until the export ends.
pub fn begin_export(
    slot: &AnalysisSlot,
    state: &ExportState,
) -> Result<(ExportSnapshot, ExportGuard), dto::AppError> {
    if state
        .exporting
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(internal("an export is already in progress"));
    }
    // From here on the guard frees the flag on any early return.
    let guard = ExportGuard {
        flag: Arc::clone(&state.exporting),
    };
    // A run holds the slot while it works; taking and dropping it is the check.
    drop(slot.begin()?);
    let kept = state
        .kept
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = match (&kept.root, kept.tracks.is_empty()) {
        (Some(root), false) => root.clone(),
        _ => {
            return Err(fakeflac_core::AppError::new(
                fakeflac_core::AppErrorCode::NotAnalyzed,
                "there are no results to export",
            )
            .into())
        }
    };
    let tracks = kept.tracks.values().cloned().collect();
    Ok((ExportSnapshot { root, tracks }, guard))
}

/// Step after the dialog: writes the snapshot to `dest` (`EXPORT_FAILED` unless `dest` is an
/// absolute `.csv` path that can be written) and returns the rows written and `dest`.
pub fn export_csv(
    snapshot: &ExportSnapshot,
    dest: &Path,
) -> Result<dto::ExportResult, dto::AppError> {
    let rows =
        fakeflac_core::export_csv(&fakeflac_export::CsvFileExporter, &snapshot.tracks, dest)?;
    Ok(dto::ExportResult {
        path: dest.to_string_lossy().into_owned(),
        rows,
    })
}

// --- RF-11: result cache (specs/contracts/result-cache.md sections 9 and 10.3) ---------------

/// Where the result cache lives: `<app_cache_dir>/results-cache.sqlite3`, or `None` when the
/// folder cannot be resolved (result-cache.md section 5).
#[derive(Debug, Clone, Default)]
pub struct CacheConfig {
    pub file: Option<PathBuf>,
}

/// Like `run_analysis_kept`, with the result cache at `cache_file` (result-cache.md 10.3).
pub fn run_analysis_cached(
    slot: &AnalysisSlot,
    export: &ExportState,
    cache_file: Option<&Path>,
    root: &Path,
    send: &(dyn Fn(dto::ProgressEvent) -> bool + Sync),
) -> Result<dto::AnalysisSummary, dto::AppError> {
    let _ = (slot, export, cache_file, root, send);
    todo!("RF-11: run_analysis_cached (result-cache.md 10.3)")
}

/// Deletes the cache files while holding the analysis slot (result-cache.md section 9).
pub fn clear_cache(slot: &AnalysisSlot, cache_file: Option<&Path>) -> Result<(), dto::AppError> {
    let _ = (slot, cache_file);
    todo!("RF-11: clear_cache (result-cache.md 9)")
}

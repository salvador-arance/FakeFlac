//! Use case: analyse a folder (RF-02, RF-05, RNF-04). Scan → per track in parallel → progress
//! events → summary. Contract: `specs/contracts/commands.md`; scenarios: `specs/features/analyze-folder.feature`.

use crate::analysis::analyze_track;
use crate::domain::{TrackAnalysis, Verdict};
use crate::error::{AppError, AppErrorCode};
use crate::params::DetectionParams;
use crate::ports::{FileEntry, FileScanner, SpectrumAnalyzer};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

/// Cooperative cancellation token (RF-05). Cloning shares the same flag.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> CancelToken {
        CancelToken::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Totals of one run, sent with the last event and returned by `analyze_folder`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub root: PathBuf,
    /// Supported files found by the scan.
    pub total: u32,
    /// Tracks with a `TrackDone` event (errors included).
    pub done: u32,
    pub real: u32,
    pub suspicious: u32,
    pub fake: u32,
    pub errors: u32,
    pub cancelled: bool,
    pub elapsed_ms: u64,
}

/// Progress of one run, in order: `Scanned`, then `TrackStarted`/`TrackDone` per track
/// (`TrackDone` may arrive out of index order), then exactly one of `Finished` / `Cancelled`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProgressEvent {
    Scanned { total: u32 },
    TrackStarted { index: u32, path: PathBuf },
    TrackDone { index: u32, analysis: TrackAnalysis },
    Finished { summary: AnalysisSummary },
    Cancelled { summary: AnalysisSummary },
}

/// Scans `root`, analyses every supported file on `workers` threads and reports progress.
///
/// Files are indexed in path order (case-insensitive). A per-track failure becomes a
/// `Verdict::Error` track and the run continues (RNF-04). When `cancel` fires, no new track
/// starts and the run ends with `Cancelled`. Errors (`FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`)
/// are returned before any event is sent. `workers` is chosen by the composition root
/// (cores − 1); 0 is treated as 1. Threads come from `std` so the core keeps no dependencies.
#[allow(clippy::too_many_arguments)]
pub fn analyze_folder(
    scanner: &dyn FileScanner,
    analyzer: &dyn SpectrumAnalyzer,
    params: &DetectionParams,
    root: &Path,
    workers: usize,
    cancel: &CancelToken,
    on_progress: &(dyn Fn(ProgressEvent) + Sync),
) -> Result<AnalysisSummary, AppError> {
    let started = Instant::now();
    let mut entries = scanner.scan(root)?;
    if entries.is_empty() {
        return Err(AppError::new(
            AppErrorCode::NoAudioFound,
            format!("no supported audio files in {}", root.display()),
        ));
    }
    entries.sort_by_cached_key(|e| {
        (
            e.path.to_string_lossy().to_lowercase(),
            e.path.to_string_lossy().into_owned(),
        )
    });
    let total = entries.len();
    on_progress(ProgressEvent::Scanned {
        total: total as u32,
    });

    let counts = Counts::default();
    let next = AtomicUsize::new(0);
    let worker_count = workers.max(1).min(total);
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| {
                run_worker(
                    &entries,
                    &next,
                    &counts,
                    analyzer,
                    params,
                    cancel,
                    on_progress,
                )
            });
        }
    });

    let done = counts.done.load(Ordering::SeqCst);
    let summary = AnalysisSummary {
        root: root.to_path_buf(),
        total: total as u32,
        done,
        real: counts.real.load(Ordering::SeqCst),
        suspicious: counts.suspicious.load(Ordering::SeqCst),
        fake: counts.fake.load(Ordering::SeqCst),
        errors: counts.errors.load(Ordering::SeqCst),
        cancelled: (done as usize) < total,
        elapsed_ms: started.elapsed().as_millis() as u64,
    };
    if summary.cancelled {
        on_progress(ProgressEvent::Cancelled {
            summary: summary.clone(),
        });
    } else {
        on_progress(ProgressEvent::Finished {
            summary: summary.clone(),
        });
    }
    Ok(summary)
}

/// Verdict tallies shared by the workers.
#[derive(Default)]
struct Counts {
    done: AtomicU32,
    real: AtomicU32,
    suspicious: AtomicU32,
    fake: AtomicU32,
    errors: AtomicU32,
}

impl Counts {
    fn record(&self, verdict: Verdict) {
        let counter = match verdict {
            Verdict::Real => &self.real,
            Verdict::Suspicious => &self.suspicious,
            Verdict::Fake => &self.fake,
            Verdict::Error => &self.errors,
        };
        counter.fetch_add(1, Ordering::SeqCst);
        self.done.fetch_add(1, Ordering::SeqCst);
    }
}

/// Pulls track indexes until none are left or the run is cancelled.
fn run_worker(
    entries: &[FileEntry],
    next: &AtomicUsize,
    counts: &Counts,
    analyzer: &dyn SpectrumAnalyzer,
    params: &DetectionParams,
    cancel: &CancelToken,
    on_progress: &(dyn Fn(ProgressEvent) + Sync),
) {
    loop {
        if cancel.is_cancelled() {
            return;
        }
        let index = next.fetch_add(1, Ordering::SeqCst);
        let Some(entry) = entries.get(index) else {
            return;
        };
        on_progress(ProgressEvent::TrackStarted {
            index: index as u32,
            path: entry.path.clone(),
        });
        let analysis = analyze_track(analyzer, entry, params);
        counts.record(analysis.verdict);
        on_progress(ProgressEvent::TrackDone {
            index: index as u32,
            analysis,
        });
    }
}

/// Holds the running analysis, so only one runs at a time (RF-05, `ANALYSIS_RUNNING`).
#[derive(Debug, Default)]
pub struct AnalysisSlot {
    current: Arc<Mutex<Option<CancelToken>>>,
}

/// The running analysis. Dropping it frees the slot.
#[derive(Debug)]
pub struct ActiveRun {
    token: CancelToken,
    slot: Arc<Mutex<Option<CancelToken>>>,
}

impl AnalysisSlot {
    pub fn new() -> AnalysisSlot {
        AnalysisSlot::default()
    }

    /// Takes the slot, or fails with `ANALYSIS_RUNNING` if a run holds it.
    pub fn begin(&self) -> Result<ActiveRun, AppError> {
        let mut current = lock(&self.current);
        if current.is_some() {
            return Err(AppError::new(
                AppErrorCode::AnalysisRunning,
                "an analysis is already running",
            ));
        }
        let token = CancelToken::new();
        *current = Some(token.clone());
        Ok(ActiveRun {
            token,
            slot: Arc::clone(&self.current),
        })
    }

    /// Cancels the running analysis; no-op if nothing is running.
    pub fn cancel(&self) {
        if let Some(token) = lock(&self.current).as_ref() {
            token.cancel();
        }
    }
}

impl ActiveRun {
    pub fn token(&self) -> &CancelToken {
        &self.token
    }
}

impl Drop for ActiveRun {
    fn drop(&mut self) {
        *lock(&self.slot) = None;
    }
}

/// Locks the slot, recovering the value if a thread panicked while holding it.
fn lock(slot: &Mutex<Option<CancelToken>>) -> MutexGuard<'_, Option<CancelToken>> {
    slot.lock().unwrap_or_else(|e| e.into_inner())
}

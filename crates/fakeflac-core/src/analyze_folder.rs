//! Use case: analyse a folder (RF-02, RF-05, RNF-04). Scan → per track in parallel → progress
//! events → summary. Contract: `specs/contracts/commands.md`; scenarios: `specs/features/analyze-folder.feature`.

use crate::analysis::analyze_track;
use crate::domain::{AlbumSummary, TrackAnalysis, TrackError, Verdict};
use crate::error::{AppError, AppErrorCode};
use crate::params::DetectionParams;
use crate::ports::{AudioProbe, FileEntry, FileScanner, SpectrumAnalyzer};
use crate::summary::AlbumTally;
use serde::{Deserialize, Serialize};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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

/// The album of a `TrackDone` after counting that track (RF-10, commands.md version 5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumUpdate {
    /// Key of the album within the run (path order of the album folders).
    pub index: u32,
    /// Display name (see `album_display_name`).
    pub name: PathBuf,
    /// Supported files the scan found directly in the album folder.
    pub files: u32,
    pub summary: AlbumSummary,
}

/// Display name of the album `folder` of a run on `root` (commands.md, "Name rule").
///
/// When `root` has a parent and `folder` lies under it (component-wise), the name is `folder`
/// without that parent, so it starts with the root's own name (`C:\Music\Artist` and
/// `C:\Music\Artist\Album 1` give `Artist\Album 1`). A drive root (`D:\`, `\\?\C:\`) or a UNC
/// share root (`\\nas\music`) has no parent: the name is `folder` in full. Pure path operations
/// over `Path` components; no I/O and no lossy conversion.
pub fn album_display_name(root: &Path, folder: &Path) -> PathBuf {
    root.parent()
        .and_then(|parent| folder.strip_prefix(parent).ok())
        .filter(|name| !name.as_os_str().is_empty())
        .map_or_else(|| folder.to_path_buf(), Path::to_path_buf)
}

/// Progress of one run, in order: `Scanned`, then `TrackStarted`/`TrackDone` per track
/// (`TrackDone` may arrive out of index order), then exactly one of `Finished` / `Cancelled`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)] // `TrackDone` carries the track and its album (RF-10)
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProgressEvent {
    Scanned {
        total: u32,
    },
    TrackStarted {
        index: u32,
        path: PathBuf,
    },
    TrackDone {
        index: u32,
        analysis: TrackAnalysis,
        album: AlbumUpdate,
    },
    Finished {
        summary: AnalysisSummary,
    },
    Cancelled {
        summary: AnalysisSummary,
    },
}

/// Scans `root`, analyses every supported file on `workers` threads and reports progress.
///
/// Files are indexed in path order (case-insensitive). A per-track failure becomes a
/// `Verdict::Error` track and the run continues (RNF-04). When `cancel` fires, no new track
/// starts and the run ends with `Cancelled`. A panic while analysing one track (a decoder bug)
/// is caught and becomes a `CorruptStream` error track (ADR-0013). Errors (`FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`)
/// are returned before any event is sent. `workers` is chosen by the composition root
/// (cores − 1); 0 is treated as 1. Threads come from `std` so the core keeps no dependencies
/// (ADR-0034).
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
    // Component by component, so a folder's files stay together ("Album\x" before "Album (Deluxe)\y").
    entries.sort_by_cached_key(|e| (path_key(&e.path), e.path.clone()));
    let (albums, tracks) = group_albums(root, &entries);
    let total = tracks.len();
    on_progress(ProgressEvent::Scanned {
        total: total as u32,
    });

    let counts = Counts::default();
    let next = Mutex::new(0usize);
    // The album lock (commands.md, "Album lock"): a mutex of its own, not the `next` one.
    let albums = Mutex::new(albums);
    let analyzer = PanicSafe(analyzer);
    let worker = || {
        run_worker(
            &tracks,
            &next,
            &albums,
            &counts,
            &analyzer,
            params,
            cancel,
            on_progress,
        )
    };
    std::thread::scope(|scope| {
        let spawned = (0..workers.max(1).min(total))
            .filter(|_| {
                std::thread::Builder::new()
                    .spawn_scoped(scope, worker)
                    .is_ok()
            })
            .count();
        // The OS refused every thread: analyse on the calling thread instead.
        if spawned == 0 {
            worker();
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

/// Sort key of a path: its components, lower-cased (the scan order and the album order).
fn path_key(path: &Path) -> Vec<String> {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

/// One album of the run (RF-10): the supported files the scan found directly in `folder`.
struct Album {
    folder: PathBuf,
    name: PathBuf,
    files: u32,
    tally: AlbumTally,
}

/// One file to analyse and the position of its album in the album table (= `AlbumUpdate.index`).
struct Track<'a> {
    entry: &'a FileEntry,
    album: usize,
}

/// Groups the sorted `entries` by direct parent folder (`FileEntry.folder`). The albums are
/// numbered by sorting their folders component-wise, case-insensitively, then by exact path, so
/// the numbering is fixed before any track starts. Returns the album table and the tracks in
/// `entries` order.
fn group_albums<'a>(root: &Path, entries: &'a [FileEntry]) -> (Vec<Album>, Vec<Track<'a>>) {
    let mut by_folder: Vec<(Vec<String>, &Path, usize)> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (path_key(&e.folder), e.folder.as_path(), i))
        .collect();
    by_folder.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));

    let mut albums: Vec<Album> = Vec::new();
    let mut album_of = vec![0usize; entries.len()];
    for (_, folder, entry) in by_folder {
        match albums.last_mut() {
            Some(last) if last.folder == folder => last.files += 1,
            _ => albums.push(Album {
                folder: folder.to_path_buf(),
                name: album_display_name(root, folder),
                files: 1,
                tally: AlbumTally::default(),
            }),
        }
        if let Some(slot) = album_of.get_mut(entry) {
            *slot = albums.len().saturating_sub(1);
        }
    }
    let tracks = entries
        .iter()
        .zip(album_of)
        .map(|(entry, album)| Track { entry, album })
        .collect();
    (albums, tracks)
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

/// Turns a panic inside the analyzer into a per-track error (ADR-0013, RNF-04).
struct PanicSafe<'a>(&'a dyn SpectrumAnalyzer);

impl SpectrumAnalyzer for PanicSafe<'_> {
    fn analyze(&self, path: &Path, params: &DetectionParams) -> Result<AudioProbe, TrackError> {
        catch_unwind(AssertUnwindSafe(|| self.0.analyze(path, params))).unwrap_or_else(|_| {
            Err(TrackError::CorruptStream(
                "the decoder failed unexpectedly".into(),
            ))
        })
    }
}

/// Pulls track indexes until none are left or the run is cancelled.
#[allow(clippy::too_many_arguments)] // the run's shared state, borrowed by every worker
fn run_worker(
    tracks: &[Track<'_>],
    next: &Mutex<usize>,
    albums: &Mutex<Vec<Album>>,
    counts: &Counts,
    analyzer: &dyn SpectrumAnalyzer,
    params: &DetectionParams,
    cancel: &CancelToken,
    on_progress: &(dyn Fn(ProgressEvent) + Sync),
) {
    loop {
        // Taking the index and reporting `TrackStarted` under one lock keeps those events
        // in index order (only `TrackDone` may arrive out of order, per the contract).
        let (index, track) = {
            let mut next = lock(next);
            if cancel.is_cancelled() {
                return;
            }
            let index = *next;
            let Some(track) = tracks.get(index) else {
                return;
            };
            *next += 1;
            on_progress(ProgressEvent::TrackStarted {
                index: index as u32,
                path: track.entry.path.clone(),
            });
            (index, track)
        };
        // No lock is held while the track is analysed.
        let analysis = analyze_track(analyzer, track.entry, params);
        counts.record(analysis.verdict);
        // The album lock covers exactly counting the track in its album and sending its
        // `TrackDone`, so the n-th `TrackDone` of an album carries `summary.total == n` in
        // channel order (commands.md, "Order").
        let mut albums = lock(albums);
        let album = albums.get_mut(track.album).map(|album| {
            album.tally.add(analysis.verdict);
            AlbumUpdate {
                index: track.album as u32,
                name: album.name.clone(),
                files: album.files,
                summary: album.tally.summary(&album.folder),
            }
        });
        // `group_albums` gives every track an album of the table, so `album` is always `Some`.
        debug_assert!(album.is_some(), "track {index} has no album");
        if let Some(album) = album {
            on_progress(ProgressEvent::TrackDone {
                index: index as u32,
                analysis,
                album,
            });
        }
        drop(albums);
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

/// Locks a mutex, recovering the value if a thread panicked while holding it.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

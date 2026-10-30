//! Fakes shared by the result-cache tests of `specs/features/result-cache.feature` (@RF-11):
//! the decoder spy (a counting fake of `SpectrumAnalyzer`), the in-memory store fake, a counting
//! decorator of the `ResultStore` port, and helpers over the progress events. Used by
//! `crates/fakeflac-core/tests/result_cache*.rs` and, through `#[path]`, by the SQLite adapter
//! tests of `crates/fakeflac-store/tests/`. No real audio. Protected after the `test(RF-11)` commit.

#![allow(dead_code, clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core::ports::{AudioProbe, FileEntry, SpectrumAnalyzer};
use fakeflac_core::{
    AudioCodec, BandSpectra, CacheError, DetectionParams, ProgressEvent, ResultStore,
    TrackAnalysis, TrackError,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

// --- A clock shared by the store fakes and the event recorder ------------------------------

static CLOCK: AtomicUsize = AtomicUsize::new(1);

/// A strictly increasing stamp, shared by every thread of the test binary.
pub fn tick() -> usize {
    CLOCK.fetch_add(1, Ordering::SeqCst)
}

// --- The decoder spy ------------------------------------------------------------------------

/// What the decoder spy does for one file.
#[derive(Debug, Clone)]
pub enum Behaviour {
    /// Flat band spectra: `real`.
    Flat,
    /// A cliff at 16 kHz: `fake`.
    Cliff16k,
    /// A cliff at 20.5 kHz: `suspicious`.
    Cliff20k5,
    /// A probe lasting 500 ms (shorter than `min_duration_s`): the error `tooShort`.
    Short,
    /// The given `TrackError`.
    Fail(TrackError),
    /// Panics (a decoder bug, ADR-0013).
    Panic,
}

/// Band spectra at 44.1 kHz (220 bands of 100 Hz), mean and median equal, level `db(centre)`
/// (the fakes of `crates/fakeflac-core/tests/analyze_folder.rs`).
pub fn spectrum(db: impl Fn(f64) -> f64) -> BandSpectra {
    let band_hz = 100.0;
    let levels: Vec<f64> = (0..220)
        .map(|b| db((f64::from(b) + 0.5) * band_hz))
        .collect();
    BandSpectra {
        sample_rate: 44_100,
        band_hz,
        mean_db: levels.clone(),
        median_db: levels,
    }
}

/// The probe of a 180 s, 44.1 kHz, 16-bit stereo FLAC with the given spectra.
pub fn probe(spectrum: BandSpectra, duration_ms: u64) -> AudioProbe {
    AudioProbe {
        sample_rate: 44_100,
        bit_depth: Some(16),
        channels: 2,
        duration_ms,
        codec: Some(AudioCodec::Flac),
        declared_bitrate_kbps: None,
        bitrate_mode: None,
        encoder: None,
        encoder_lowpass_hz: None,
        spectrum: Some(spectrum),
    }
}

/// "The decoder spy": a counting fake of the `SpectrumAnalyzer` port. By file name: `fake*`
/// gives the cliff at 16 kHz, `susp*` the cliff at 20.5 kHz, anything else the flat spectrum,
/// unless `on` chose another behaviour for that file name.
#[derive(Default)]
pub struct DecoderSpy {
    calls: Mutex<Vec<PathBuf>>,
    behaviours: Mutex<HashMap<String, Behaviour>>,
    delay: Duration,
}

impl DecoderSpy {
    pub fn new() -> DecoderSpy {
        DecoderSpy::default()
    }

    /// A spy that sleeps `ms` in every call (so that a cancel can land between tracks).
    pub fn with_delay(ms: u64) -> DecoderSpy {
        DecoderSpy {
            delay: Duration::from_millis(ms),
            ..DecoderSpy::default()
        }
    }

    /// Chooses the behaviour for the file called `file_name`.
    pub fn on(&self, file_name: &str, behaviour: Behaviour) {
        self.behaviours
            .lock()
            .unwrap()
            .insert(file_name.to_string(), behaviour);
    }

    /// Every analyzed path, in call order.
    pub fn calls(&self) -> Vec<PathBuf> {
        self.calls.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// The calls whose file name is `file_name`.
    pub fn count_of(&self, file_name: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|p| file_name_of(p) == file_name)
            .count()
    }

    /// The file names analyzed, sorted.
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.calls().iter().map(|p| file_name_of(p)).collect();
        v.sort();
        v
    }

    fn behaviour(&self, path: &Path) -> Behaviour {
        let name = file_name_of(path);
        if let Some(b) = self.behaviours.lock().unwrap().get(&name) {
            return b.clone();
        }
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("fake") {
            Behaviour::Cliff16k
        } else if lower.starts_with("susp") {
            Behaviour::Cliff20k5
        } else {
            Behaviour::Flat
        }
    }
}

impl SpectrumAnalyzer for DecoderSpy {
    fn analyze(&self, path: &Path, _params: &DetectionParams) -> Result<AudioProbe, TrackError> {
        self.calls.lock().unwrap().push(path.to_path_buf());
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        match self.behaviour(path) {
            Behaviour::Flat => Ok(probe(spectrum(|_| 0.0), 180_000)),
            Behaviour::Cliff16k => Ok(probe(
                spectrum(|f| if f < 16_000.0 { 0.0 } else { -100.0 }),
                180_000,
            )),
            Behaviour::Cliff20k5 => Ok(probe(
                spectrum(|f| if f < 20_500.0 { 0.0 } else { -100.0 }),
                180_000,
            )),
            Behaviour::Short => Ok(probe(spectrum(|_| 0.0), 500)),
            Behaviour::Fail(e) => Err(e),
            Behaviour::Panic => panic!("decoder bug (expected by the test)"),
        }
    }
}

pub fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// --- Store calls ------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Get,
    Put,
}

/// One call of the `ResultStore` port, as a fake or the decorator saw it.
#[derive(Debug, Clone)]
pub struct Call {
    pub op: Op,
    pub path: PathBuf,
    pub thread: ThreadId,
    pub started: Instant,
    pub ended: Instant,
    /// `tick()` when the call returned (or was about to panic).
    pub stamp: usize,
    /// `false` for an `Err` or a panic.
    pub ok: bool,
    /// The analysis of a put.
    pub analysis: Option<TrackAnalysis>,
}

// --- The store fake -------------------------------------------------------------------------

type Key = (PathBuf, u64, i64, u32);

/// The in-memory store fake: results keyed by (path, size, mtime, algorithm_version). It can be
/// told to fail on its n-th call (gets and puts counted together, 1-based), to panic on its
/// n-th get, or to return a given analysis for a path whatever the key.
#[derive(Default)]
pub struct StoreFake {
    map: Mutex<HashMap<Key, TrackAnalysis>>,
    forced: Mutex<HashMap<PathBuf, TrackAnalysis>>,
    log: Mutex<Vec<Call>>,
    calls: AtomicUsize,
    gets: AtomicUsize,
    fail_on_call: Option<usize>,
    panic_on_get: Option<usize>,
}

impl StoreFake {
    pub fn new() -> StoreFake {
        StoreFake::default()
    }

    pub fn failing_on_call(n: usize) -> StoreFake {
        StoreFake {
            fail_on_call: Some(n),
            ..StoreFake::default()
        }
    }

    pub fn panicking_on_get(n: usize) -> StoreFake {
        StoreFake {
            panic_on_get: Some(n),
            ..StoreFake::default()
        }
    }

    /// Stores `analysis` under the key of `entry` and `algorithm_version`.
    pub fn hold(&self, entry: &FileEntry, algorithm_version: u32, analysis: TrackAnalysis) {
        self.map.lock().unwrap().insert(
            (
                entry.path.clone(),
                entry.size_bytes,
                entry.modified_unix,
                algorithm_version,
            ),
            analysis,
        );
    }

    /// Every `get` of `path` returns `analysis`, whatever the size, mtime or version.
    pub fn force(&self, path: &Path, analysis: TrackAnalysis) {
        self.forced
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), analysis);
    }

    pub fn log(&self) -> Vec<Call> {
        self.log.lock().unwrap().clone()
    }

    pub fn puts(&self) -> Vec<Call> {
        self.log().into_iter().filter(|c| c.op == Op::Put).collect()
    }

    pub fn gets(&self) -> Vec<Call> {
        self.log().into_iter().filter(|c| c.op == Op::Get).collect()
    }

    /// Calls (gets and puts) for `path`.
    pub fn calls_for(&self, path: &Path) -> Vec<Call> {
        self.log().into_iter().filter(|c| c.path == path).collect()
    }

    fn record(
        &self,
        op: Op,
        entry: &FileEntry,
        started: Instant,
        ok: bool,
        analysis: Option<&TrackAnalysis>,
    ) {
        self.log.lock().unwrap().push(Call {
            op,
            path: entry.path.clone(),
            thread: std::thread::current().id(),
            started,
            ended: Instant::now(),
            stamp: tick(),
            ok,
            analysis: analysis.cloned(),
        });
    }

    /// Counts this call; `true` if it is the one that must fail.
    fn must_fail(&self) -> bool {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.fail_on_call == Some(n)
    }
}

impl ResultStore for StoreFake {
    fn get(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
    ) -> Result<Option<TrackAnalysis>, CacheError> {
        let started = Instant::now();
        let fail = self.must_fail();
        let get_n = self.gets.fetch_add(1, Ordering::SeqCst) + 1;
        if self.panic_on_get == Some(get_n) {
            self.record(Op::Get, entry, started, false, None);
            panic!("store fake panics on get {get_n} (expected by the test)");
        }
        if fail {
            self.record(Op::Get, entry, started, false, None);
            return Err(CacheError("store fake fails (expected by the test)".into()));
        }
        let forced = self.forced.lock().unwrap().get(&entry.path).cloned();
        let found = forced.or_else(|| {
            self.map
                .lock()
                .unwrap()
                .get(&(
                    entry.path.clone(),
                    entry.size_bytes,
                    entry.modified_unix,
                    params.algorithm_version,
                ))
                .cloned()
        });
        self.record(Op::Get, entry, started, true, None);
        Ok(found)
    }

    fn put(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
        analysis: &TrackAnalysis,
    ) -> Result<(), CacheError> {
        let started = Instant::now();
        if self.must_fail() {
            self.record(Op::Put, entry, started, false, Some(analysis));
            return Err(CacheError("store fake fails (expected by the test)".into()));
        }
        self.map.lock().unwrap().insert(
            (
                entry.path.clone(),
                entry.size_bytes,
                entry.modified_unix,
                params.algorithm_version,
            ),
            analysis.clone(),
        );
        self.record(Op::Put, entry, started, true, Some(analysis));
        Ok(())
    }
}

// --- The counting decorator -----------------------------------------------------------------

/// A counting decorator of the `ResultStore` port: records, for every call, the calling thread,
/// its start and end times and its result, and forwards it to `inner`.
pub struct Counting<'a> {
    inner: &'a dyn ResultStore,
    log: Mutex<Vec<Call>>,
}

impl<'a> Counting<'a> {
    pub fn new(inner: &'a dyn ResultStore) -> Counting<'a> {
        Counting {
            inner,
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn log(&self) -> Vec<Call> {
        self.log.lock().unwrap().clone()
    }

    fn push(&self, op: Op, entry: &FileEntry, started: Instant, ok: bool) {
        self.log.lock().unwrap().push(Call {
            op,
            path: entry.path.clone(),
            thread: std::thread::current().id(),
            started,
            ended: Instant::now(),
            stamp: tick(),
            ok,
            analysis: None,
        });
    }
}

impl ResultStore for Counting<'_> {
    fn get(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
    ) -> Result<Option<TrackAnalysis>, CacheError> {
        let started = Instant::now();
        let r = self.inner.get(entry, params);
        self.push(Op::Get, entry, started, r.is_ok());
        r
    }

    fn put(
        &self,
        entry: &FileEntry,
        params: &DetectionParams,
        analysis: &TrackAnalysis,
    ) -> Result<(), CacheError> {
        let started = Instant::now();
        let r = self.inner.put(entry, params, analysis);
        self.push(Op::Put, entry, started, r.is_ok());
        r
    }
}

/// Bound F1 of `result-cache.md` section 7: no thread made a store call after one of its own
/// calls had failed. `log` is in the order the calls ended; a thread's calls are sequential.
pub fn assert_no_call_after_own_failure(log: &[Call]) {
    let mut failed: HashMap<ThreadId, usize> = HashMap::new();
    for (i, c) in log.iter().enumerate() {
        if let Some(at) = failed.get(&c.thread) {
            panic!(
                "thread {:?} made call {i} ({:?} {}) after its failed call {at}",
                c.thread,
                c.op,
                c.path.display()
            );
        }
        if !c.ok {
            failed.insert(c.thread, i);
        }
    }
}

// --- Events -----------------------------------------------------------------------------------

/// Collects the events of a run, each with the `tick()` at which it was received.
#[derive(Default)]
pub struct Recorder {
    events: Mutex<Vec<(usize, ProgressEvent)>>,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder::default()
    }

    pub fn push(&self, e: ProgressEvent) {
        self.events.lock().unwrap().push((tick(), e));
    }

    pub fn stamped(&self) -> Vec<(usize, ProgressEvent)> {
        self.events.lock().unwrap().clone()
    }

    pub fn events(&self) -> Vec<ProgressEvent> {
        self.stamped().into_iter().map(|(_, e)| e).collect()
    }
}

/// `(index, analysis)` of every `TrackDone`, sorted by index.
pub fn done_by_index(events: &[ProgressEvent]) -> Vec<(u32, TrackAnalysis)> {
    let mut v: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone {
                index, analysis, ..
            } => Some((*index, analysis.clone())),
            _ => None,
        })
        .collect();
    v.sort_by_key(|(i, _)| *i);
    v
}

/// The analysis of the `TrackDone` whose path has the file name `file_name`.
pub fn done_of(events: &[ProgressEvent], file_name: &str) -> TrackAnalysis {
    done_by_index(events)
        .into_iter()
        .map(|(_, a)| a)
        .find(|a| file_name_of(&a.info.path) == file_name)
        .unwrap_or_else(|| panic!("no trackDone for {file_name}"))
}

/// Every file got `TrackStarted` and then `TrackDone`, once each.
pub fn assert_started_then_done(events: &[ProgressEvent], total: u32) {
    for i in 0..total {
        let started: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, ProgressEvent::TrackStarted { index, .. } if *index == i))
            .map(|(p, _)| p)
            .collect();
        let done: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, ProgressEvent::TrackDone { index, .. } if *index == i))
            .map(|(p, _)| p)
            .collect();
        assert_eq!(started.len(), 1, "trackStarted of {i}");
        assert_eq!(done.len(), 1, "trackDone of {i}");
        assert!(started[0] < done[0], "track {i}: started before done");
    }
}

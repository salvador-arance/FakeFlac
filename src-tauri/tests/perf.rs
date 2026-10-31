//! RNF-01 performance harness and its CI smoke test (@RNF-01, with @RNF-04, @RNF-05 and @RNF-09
//! where they apply). Spec: `specs/contracts/performance.md` (sections 2 to 6 and 9); scenarios:
//! `specs/features/performance.feature`; decision: ADR-0046.
//!
//! - The timed suite is the ignored test `timed_suite`. It runs only through `just perf`
//!   (`cargo test --release -p fakeflac --test perf -- --ignored --nocapture --test-threads=1`),
//!   never in CI, and prints the report of section 6. With `FAKEFLAC_PERF_FOLDER` set it times that
//!   folder instead (section 5.3) and prints aggregates only (ADR-0022, D8).
//! - The other tests are the smoke test (T0, T5, T12, T13, T14). `just check-app` runs them; they
//!   assert nothing about any time.
//!
//! Fixtures are generated here (section 2, D4) with a fixed seed, in a new folder under
//! `std::env::temp_dir()` that a drop guard removes, also when a test fails. Never real audio,
//! never the corpus. `std::time::Instant` only (no new dependency besides `flacenc`, Q9).
//!
//! Protected after the `test(RNF-01)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::{
    analyze_folder_cached, AnalysisSlot, CancelToken, DetectionParams, ResultCache,
};
use fakeflac_fs::WalkDirScanner;
use fakeflac_lib::dto::{self, CacheStatus, ProgressEvent, TrackError, Verdict};
use fakeflac_lib::run::{
    get_spectrogram, run_analysis, run_analysis_cached, worker_count, ExportState,
};
use fakeflac_store::{clear_cache_files, EngineTag, SqliteResultStore};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

// --- Fixture parameters (section 2) -----------------------------------------------------------

/// FLAC block length: every FLAC fixture is a whole number of these frames (`fixtures.md`).
const BLOCK: u64 = 4096;
/// `typical` (2.1): 3875 blocks, 359.909 s at 44.1 kHz.
const TYPICAL_FRAMES: u64 = 15_872_000;
/// RMS level of `typical`'s Gaussian noise (dBFS, full scale = 32767). Picked once by the test
/// stage: with `flacenc` 0.5.1 it gives about 40.0 MB (section 2.1 allows 38 to 42 MB, and asks
/// for a level at or above −45 dBFS).
const TYPICAL_LEVEL_DBFS: f64 = -42.5;
/// `hires` (2.3): 2109 blocks, 89.984 s at 96 kHz.
const HIRES_FRAMES: u64 = 8_638_464;
/// RMS level of `hires`' Gaussian noise (dBFS, full scale = 8388607). Picked once by the test
/// stage: with `flacenc` 0.5.1, −45 dBFS gives 41 289 467 bytes, and only about −45 to
/// −44.6 dBFS keeps the size in the 38–42 MB window (−39 dBFS gives about 51 MB). Section 2.3
/// allows any level at or above −50 dBFS (10 dB above `silence_dbfs`, −60), so −45 is inside
/// the range. After a `flacenc` upgrade that moves the size out of the window, T5 stops before
/// any timing; then re-pick the level at or above −50 dBFS (about 2 MB per 6 dB on this
/// fixture) so the size is back inside, or, if no such level exists, change the spec at G1.
const HIRES_LEVEL_DBFS: f64 = -45.0;
/// `wav` (2.3): 226.757 s at 44.1 kHz, 40 000 000 bytes of samples.
const WAV_FRAMES: u64 = 10_000_000;
const WAV_LEVEL_DBFS: f64 = -12.0;
/// The smoke test's tiny files (2.3): 107 blocks, 9.938 s.
const TINY_FRAMES: u64 = 438_272;
/// Size window of `typical`, `hires` and `wav` (40 MB ± 5 %, D1: 1 MB = 1 000 000 bytes).
const SIZE_MIN: u64 = 38_000_000;
const SIZE_MAX: u64 = 42_000_000;
const ALBUM_TRACKS: usize = 15;
/// File `08` of `album_corrupt` is truncated (2.2).
const CORRUPT_TRACK: usize = 8;

const SEED_TYPICAL: u64 = 1;
const SEED_HIRES: u64 = 2;
const SEED_WAV: u64 = 3;
const SEED_TINY: u64 = 4;

const _: () = assert!(TYPICAL_FRAMES % BLOCK == 0);
const _: () = assert!(HIRES_FRAMES % BLOCK == 0);
const _: () = assert!(TINY_FRAMES % BLOCK == 0);

/// D5 (RNF-09): every timed folder has this name.
const ALBUM_DIR: &str = "Álbum ″Ñandú″";

fn track_name(n: usize, ext: &str) -> String {
    format!("{n:02} – Canción ‘x’.{ext}")
}

// --- Timing parameters (sections 3 and 4) -----------------------------------------------------

/// Measured runs per figure after 1 warm-up run (3.3).
const MEASURED_RUNS: usize = 5;
/// `worker_count()` of the reference machine (section 4, Q7).
const REFERENCE_WORKERS: usize = 7;
/// Folder mode: runs after the first one (5.3 step 1).
const FOLDER_RUNS: usize = 3;

const BUILD_ERROR: &str = "perf: figures need a release build (run: just perf)";
const REPORT_TITLE: &str = "RNF-01 performance report (specs/contracts/performance.md)";
const FOLDER_TITLE: &str = "RNF-01 folder report (aggregates only, ADR-0022)";

// --- Pure functions of the harness (D2, section 2) ---------------------------------------------

/// D2: the figures come from a release build only.
fn check_release_build(debug_assertions: bool) -> Result<(), String> {
    if debug_assertions {
        Err(BUILD_ERROR.to_string())
    } else {
        Ok(())
    }
}

/// Section 2: the size check of a fixture (the window is inclusive).
fn check_fixture(id: &str, bytes: u64, min: u64, max: u64) -> Result<(), String> {
    if (min..=max).contains(&bytes) {
        Ok(())
    } else {
        Err(format!(
            "perf: fixture {id} is {bytes} bytes, outside [{min}, {max}]"
        ))
    }
}

fn build_line() -> String {
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!("build: {build}")
}

// --- Temporary folder (D4) ---------------------------------------------------------------------

/// The test's own folder under `std::env::temp_dir()`, removed when dropped (also on a panic).
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(tag: &str) -> TempRoot {
        let dir = std::env::temp_dir().join(format!("fakeflac-perf-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the temporary folder");
        TempRoot(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// `<root>/<parent>/Álbum ″Ñandú″`, created (2.2 "Layout").
    fn album_dir(&self, parent: &str) -> PathBuf {
        let dir = self.0.join(parent).join(ALBUM_DIR);
        std::fs::create_dir_all(&dir).expect("create a fixture folder");
        dir
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// --- Fixture writers (section 2) ----------------------------------------------------------------

/// Independent white Gaussian noise per channel, stereo interleaved, at `level_dbfs` RMS, quantised
/// with plain rounding and clamped to the `bits` range (`fixtures.md` "Levels").
fn gaussian_stereo(frames: u64, bits: u32, level_dbfs: f64, seed: u64) -> Vec<i32> {
    let full = f64::from((1i32 << (bits - 1)) - 1);
    let sigma = 10f64.powf(level_dbfs / 20.0) * full;
    let n = usize::try_from(frames * 2).expect("sample count");
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        // Box-Muller: two independent standard normals per pair of uniforms.
        let u1 = 1.0 - rng.random::<f64>();
        let u2 = rng.random::<f64>();
        let r = (-2.0 * u1.ln()).sqrt();
        let t = std::f64::consts::TAU * u2;
        for g in [r * t.cos(), r * t.sin()] {
            if out.len() < n {
                out.push((sigma * g).round().clamp(-full - 1.0, full) as i32);
            }
        }
    }
    out
}

fn write_flac(path: &Path, samples: &[i32], rate: u32, bits: u32) {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .expect("flacenc config");
    let source = flacenc::source::MemSource::from_samples(samples, 2, bits as usize, rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .expect("flacenc encode");
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).expect("flacenc write");
    std::fs::write(path, sink.as_slice()).expect("write a FLAC fixture");
}

fn write_wav(path: &Path, samples: &[i32], rate: u32) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("create a WAV fixture");
    for s in samples {
        w.write_sample(*s as i16).expect("write a WAV sample");
    }
    w.finalize().expect("finalize a WAV fixture");
}

/// `dst` as a hard link to `src`, or a copy where a hard link fails (D4).
fn link_or_copy(src: &Path, dst: &Path) {
    if std::fs::hard_link(src, dst).is_err() {
        std::fs::copy(src, dst).expect("copy a fixture");
    }
}

/// The first `floor(size / 2)` bytes of `src`, written to `dst` (2.2 `album_corrupt`).
fn write_first_half(src: &Path, dst: &Path) {
    let bytes = std::fs::read(src).expect("read a fixture");
    std::fs::write(dst, &bytes[..bytes.len() / 2]).expect("write the truncated fixture");
}

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).expect("fixture metadata").len()
}

// --- Fixture checks (section 2, T5) -------------------------------------------------------------

fn read_head(path: &Path, n: usize) -> Vec<u8> {
    let mut head = Vec::with_capacity(n);
    std::fs::File::open(path)
        .expect("open a fixture")
        .take(n as u64)
        .read_to_end(&mut head)
        .expect("read a fixture");
    head
}

fn format_name(head: &[u8]) -> &'static str {
    if head.starts_with(b"fLaC") {
        "flac"
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WAVE" {
        "wav"
    } else {
        "unknown"
    }
}

struct StreamInfo {
    rate: u32,
    channels: u32,
    bits: u32,
    total_samples: u64,
}

/// The STREAMINFO block of a FLAC file (the first metadata block, data from byte 8).
/// Bits: 16+16 block sizes, 24+24 frame sizes, 20 rate, 3 channels − 1, 5 depth − 1, 36 total.
fn stream_info(head: &[u8]) -> Option<StreamInfo> {
    if head.len() < 42 || !head.starts_with(b"fLaC") || head[4] & 0x7F != 0 {
        return None;
    }
    let d = &head[8..42];
    let rate = (u32::from(d[10]) << 12) | (u32::from(d[11]) << 4) | (u32::from(d[12]) >> 4);
    let channels = u32::from((d[12] >> 1) & 0x07) + 1;
    let bits = u32::from(((d[12] & 0x01) << 4) | (d[13] >> 4)) + 1;
    let mut total_samples = u64::from(d[13] & 0x0F);
    for b in &d[14..18] {
        total_samples = (total_samples << 8) | u64::from(*b);
    }
    Some(StreamInfo {
        rate,
        channels,
        bits,
        total_samples,
    })
}

/// Format, sample rate, bit depth, channels and STREAMINFO total samples of a FLAC fixture.
/// Returns its frames.
fn check_flac(id: &str, path: &Path, rate: u32, bits: u32, frames: u64) -> Result<u64, String> {
    let head = read_head(path, 42);
    let info = stream_info(&head).ok_or_else(|| {
        format!(
            "perf: fixture {id} has format {}, expected flac",
            format_name(&head)
        )
    })?;
    let has = |what: &str, value: u64, expected: u64| {
        if value == expected {
            Ok(())
        } else {
            Err(format!(
                "perf: fixture {id} has {what} {value}, expected {expected}"
            ))
        }
    };
    has("sample rate", info.rate.into(), rate.into())?;
    has("bit depth", info.bits.into(), bits.into())?;
    has("channels", info.channels.into(), 2)?;
    has("STREAMINFO total samples", info.total_samples, frames)?;
    Ok(info.total_samples)
}

/// Format, sample rate, bit depth, channels and frames of the WAV fixture. Returns its frames.
fn check_wav(id: &str, path: &Path, rate: u32, frames: u64) -> Result<u64, String> {
    let head = read_head(path, 12);
    if format_name(&head) != "wav" {
        return Err(format!(
            "perf: fixture {id} has format {}, expected wav",
            format_name(&head)
        ));
    }
    let reader = hound::WavReader::open(path)
        .map_err(|e| format!("perf: fixture {id} has format unreadable ({e}), expected wav"))?;
    let spec = reader.spec();
    let has = |what: &str, value: u64, expected: u64| {
        if value == expected {
            Ok(())
        } else {
            Err(format!(
                "perf: fixture {id} has {what} {value}, expected {expected}"
            ))
        }
    };
    has("sample rate", spec.sample_rate.into(), rate.into())?;
    has("bit depth", spec.bits_per_sample.into(), 16)?;
    has("channels", spec.channels.into(), 2)?;
    has("frames", reader.duration().into(), frames)?;
    Ok(reader.duration().into())
}

// --- One run (section 3.1) ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    Finished,
    Cancelled,
}

/// What a run's checks need from one `trackDone`; the DTO itself is dropped (3.1).
#[derive(Debug, Clone)]
struct DoneRec {
    index: u32,
    at: Instant,
    name: String,
    format: dto::AudioFormat,
    size_bytes: u64,
    verdict: Verdict,
    error: Option<&'static str>,
    duration_ms: u64,
    sample_rate: u32,
}

fn error_kind(e: &TrackError) -> &'static str {
    match e {
        TrackError::Unreadable { .. } => "unreadable",
        TrackError::CorruptStream { .. } => "corruptStream",
        TrackError::UnsupportedFormat { .. } => "unsupportedFormat",
        TrackError::TooShort => "tooShort",
    }
}

#[derive(Debug, Default)]
struct Rec {
    started: Vec<(u32, Instant)>,
    done: Vec<DoneRec>,
    last: Option<Last>,
    cancel_at: Option<Instant>,
}

/// The `send` of section 3.1: takes each event as its contract DTO, records when `trackStarted`
/// and `trackDone` arrive, and drops the event. With a token, cancels on the first `trackDone`
/// (3.5).
struct Recorder<'a> {
    cancel_on_first_done: Option<&'a CancelToken>,
    rec: Mutex<Rec>,
}

impl<'a> Recorder<'a> {
    fn new(cancel_on_first_done: Option<&'a CancelToken>) -> Recorder<'a> {
        Recorder {
            cancel_on_first_done,
            rec: Mutex::new(Rec::default()),
        }
    }

    fn record(&self, event: ProgressEvent) {
        let at = Instant::now();
        let mut rec = self.rec.lock().unwrap_or_else(PoisonError::into_inner);
        match event {
            ProgressEvent::TrackStarted { index, .. } => rec.started.push((index, at)),
            ProgressEvent::TrackDone {
                index, analysis, ..
            } => {
                let path = analysis.info.path.as_str();
                let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
                rec.done.push(DoneRec {
                    index,
                    at,
                    name,
                    format: analysis.info.format,
                    size_bytes: analysis.info.size_bytes,
                    verdict: analysis.verdict,
                    error: analysis.error.as_ref().map(error_kind),
                    duration_ms: analysis.info.duration_ms,
                    sample_rate: analysis.info.sample_rate,
                });
                if let Some(token) = self.cancel_on_first_done {
                    if rec.cancel_at.is_none() {
                        rec.cancel_at = Some(Instant::now());
                        token.cancel();
                    }
                }
            }
            ProgressEvent::Finished { .. } => rec.last = Some(Last::Finished),
            ProgressEvent::Cancelled { .. } => rec.last = Some(Last::Cancelled),
            ProgressEvent::Scanned { .. } => {}
        }
    }

    fn into_rec(self) -> Rec {
        self.rec
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

struct RunOutcome {
    elapsed: Duration,
    end: Instant,
    summary: dto::AnalysisSummary,
    rec: Rec,
}

impl RunOutcome {
    /// 3.5: from `cancel()` to the return of the run.
    fn cancel_latency(&self) -> Option<Duration> {
        self.rec
            .cancel_at
            .map(|at| self.end.saturating_duration_since(at))
    }

    /// 3.4: each track from its `trackStarted` to its `trackDone`, with its `trackDone`.
    fn per_track(&self) -> Vec<(&DoneRec, Duration)> {
        self.rec
            .done
            .iter()
            .filter_map(|d| {
                self.rec
                    .started
                    .iter()
                    .find(|(i, _)| *i == d.index)
                    .map(|(_, at)| (d, d.at.saturating_duration_since(*at)))
            })
            .collect()
    }
}

/// A run that did not return a summary: the `AppError` code only (D8), or a harness failure.
#[derive(Debug)]
enum RunFail {
    App(String),
    Other(String),
}

fn code_name(code: dto::AppErrorCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| format!("{code:?}"))
}

fn fail_in(row: &str) -> impl Fn(RunFail) -> String + '_ {
    move |f| match f {
        RunFail::App(code) => format!("perf: {row} failed: {code}"),
        RunFail::Other(message) => format!("perf: {row} failed: {message}"),
    }
}

/// Cache mode of a run (3.2).
#[derive(Debug, Clone, Copy)]
enum Mode<'a> {
    Off,
    Cold(&'a Path),
    Warm(&'a Path),
}

/// Before a `cold` run (the warm-up included) the cache files are deleted (3.2).
fn prepare(mode: Mode<'_>) -> Result<(), RunFail> {
    if let Mode::Cold(file) = mode {
        clear_cache_files(file)
            .map_err(|e| RunFail::Other(format!("cannot delete the cache files: {e}")))?;
    }
    Ok(())
}

/// The value of `run::engine_tag()` (private), built as 3.1 says.
fn engine_tag() -> EngineTag {
    EngineTag {
        app_version: env!("CARGO_PKG_VERSION").into(),
        extraction_version: fakeflac_audio::EXTRACTION_VERSION,
    }
}

/// App path (3.1): `run::run_analysis` (off) or `run::run_analysis_cached` (cold, warm).
fn run_app(root: &Path, mode: Mode<'_>) -> Result<RunOutcome, RunFail> {
    prepare(mode)?;
    let slot = AnalysisSlot::new();
    let export = ExportState::new();
    let recorder = Recorder::new(None);
    let send = |e: ProgressEvent| {
        recorder.record(e);
        true
    };
    let start = Instant::now();
    let result = match mode {
        Mode::Off => run_analysis(&slot, root, &send),
        Mode::Cold(file) | Mode::Warm(file) => {
            run_analysis_cached(&slot, &export, Some(file), root, &send)
        }
    };
    let end = Instant::now();
    let summary = result.map_err(|e| RunFail::App(code_name(e.code)))?;
    Ok(RunOutcome {
        elapsed: end - start,
        end,
        summary,
        rec: recorder.into_rec(),
    })
}

/// Reference path (3.1): `analyze_folder_cached` with the adapters and params of `run.rs`,
/// `workers` workers; in a cached mode the timed span includes opening the store.
fn run_reference(
    root: &Path,
    workers: usize,
    mode: Mode<'_>,
    cancel_on_first_done: bool,
) -> Result<RunOutcome, RunFail> {
    prepare(mode)?;
    let token = CancelToken::new();
    let recorder = Recorder::new(cancel_on_first_done.then_some(&token));
    let sink = |e: fakeflac_core::ProgressEvent| recorder.record(ProgressEvent::from(&e));
    let start = Instant::now();
    let store = match mode {
        Mode::Off => None,
        Mode::Cold(file) | Mode::Warm(file) => Some(
            SqliteResultStore::open(file, engine_tag())
                .map_err(|e| RunFail::Other(format!("cannot open the cache: {e}")))?,
        ),
    };
    let cache = match &store {
        Some(store) => ResultCache::Store(store),
        None => ResultCache::Off,
    };
    let result = analyze_folder_cached(
        &WalkDirScanner,
        &SymphoniaAnalyzer,
        cache,
        &DetectionParams::default(),
        root,
        workers,
        &token,
        &sink,
    );
    drop(store);
    let end = Instant::now();
    let summary = result.map_err(|e| RunFail::App(code_name(dto::AppError::from(e).code)))?;
    Ok(RunOutcome {
        elapsed: end - start,
        end,
        summary: (&summary).into(),
        rec: recorder.into_rec(),
    })
}

// --- Run checks (T3, T4, T6, T8, T9) ------------------------------------------------------------

/// A timed folder and what its tracks must look like.
struct Folder {
    dir: PathBuf,
    names: Vec<String>,
    frames: u64,
    rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheCheck {
    Off,
    Cold,
    Warm,
}

struct Expect<'a> {
    row: &'a str,
    folder: &'a Folder,
    /// Every track `real` (T3) instead of only "not `error`" (T8, the smoke test's tiny set).
    strict_real: bool,
    /// The one file that must be `error` with `corruptStream` (T4).
    corrupt: Option<&'a str>,
    cache: CacheCheck,
}

/// T3 (and T4, T6, T8): the run finished, every file of the folder has its `trackDone` with the
/// expected verdict, duration and sample rate, and the cache status matches the mode.
fn check_finished(out: &RunOutcome, x: &Expect<'_>) -> Result<(), String> {
    let fail = |what: String| Err(format!("perf: {} check failed: {what}", x.row));
    let s = &out.summary;
    if out.rec.last != Some(Last::Finished) || s.cancelled {
        return fail(format!(
            "the run did not finish (last event {:?}, cancelled {})",
            out.rec.last, s.cancelled
        ));
    }
    let n = x.folder.names.len();
    if s.total as usize != n || s.done as usize != n || out.rec.done.len() != n {
        return fail(format!(
            "done {} total {} trackDone {}, expected {n}",
            s.done,
            s.total,
            out.rec.done.len()
        ));
    }
    // RNF-09: every trackDone path ends with its file name exactly.
    let got: BTreeSet<&str> = out.rec.done.iter().map(|d| d.name.as_str()).collect();
    let want: BTreeSet<&str> = x.folder.names.iter().map(String::as_str).collect();
    if got != want {
        return fail(format!("track names {got:?}, expected {want:?}"));
    }
    let expected_ms = x.folder.frames as f64 * 1000.0 / f64::from(x.folder.rate);
    let mut errors = 0u32;
    for d in &out.rec.done {
        if Some(d.name.as_str()) == x.corrupt {
            if d.verdict != Verdict::Error || d.error != Some("corruptStream") {
                return fail(format!(
                    "{} has verdict {:?} and error {:?}, expected error corruptStream",
                    d.name, d.verdict, d.error
                ));
            }
            errors += 1;
            continue;
        }
        if d.verdict == Verdict::Error || d.error.is_some() {
            return fail(format!(
                "{} has verdict {:?} and error {:?}",
                d.name, d.verdict, d.error
            ));
        }
        if x.strict_real && d.verdict != Verdict::Real {
            return fail(format!(
                "{} has verdict {:?}, expected real",
                d.name, d.verdict
            ));
        }
        if (d.duration_ms as f64 - expected_ms).abs() > 1.0 {
            return fail(format!(
                "{} has duration {} ms, expected {expected_ms:.0} ms",
                d.name, d.duration_ms
            ));
        }
        if d.sample_rate != x.folder.rate {
            return fail(format!(
                "{} has sample rate {}, expected {}",
                d.name, d.sample_rate, x.folder.rate
            ));
        }
    }
    if x.corrupt.is_some() && errors != 1 {
        return fail("the truncated file has no trackDone".into());
    }
    if s.errors != errors {
        return fail(format!("errors {}, expected {errors}", s.errors));
    }
    let (status, from_cache) = match x.cache {
        CacheCheck::Off => (CacheStatus::Off, 0),
        CacheCheck::Cold => (CacheStatus::Ok, 0),
        CacheCheck::Warm => (CacheStatus::Ok, s.total),
    };
    if s.cache != status || s.from_cache != from_cache {
        return fail(format!(
            "cache {:?} fromCache {}, expected {status:?} and {from_cache}",
            s.cache, s.from_cache
        ));
    }
    Ok(())
}

/// T9 (3.5): the run ended `cancelled: true` with fewer tracks done than found, and every
/// `trackStarted` has its `trackDone` (RF-05).
fn check_cancelled(out: &RunOutcome, files: usize, done: Option<u32>) -> Result<(), String> {
    let fail = |what: String| Err(format!("perf: T9 check failed: {what}"));
    let s = &out.summary;
    if out.rec.last != Some(Last::Cancelled) || !s.cancelled || out.rec.cancel_at.is_none() {
        return fail(format!(
            "the run was not cancelled (last event {:?}, cancelled {})",
            out.rec.last, s.cancelled
        ));
    }
    if s.total as usize != files || s.done >= s.total || s.done as usize != out.rec.done.len() {
        return fail(format!(
            "done {} total {} trackDone {}",
            s.done,
            s.total,
            out.rec.done.len()
        ));
    }
    if let Some(expected) = done {
        if s.done != expected {
            return fail(format!("done {}, expected {expected}", s.done));
        }
    }
    let started: BTreeSet<u32> = out.rec.started.iter().map(|(i, _)| *i).collect();
    let finished: BTreeSet<u32> = out.rec.done.iter().map(|d| d.index).collect();
    if started != finished || started.len() != out.rec.started.len() {
        return fail(format!(
            "trackStarted {started:?} and trackDone {finished:?} do not match"
        ));
    }
    Ok(())
}

// --- Figures (3.3) ------------------------------------------------------------------------------

/// Whole milliseconds, rounded to the nearest.
fn ms(d: Duration) -> u64 {
    u64::try_from((d.as_nanos() + 500_000) / 1_000_000).unwrap_or(u64::MAX)
}

/// The median of a list (the middle value; for an even count the upper of the two middles).
fn median_of(values: &[u64]) -> u64 {
    let mut v = values.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

/// One row's figures: the first measured run, the median and the maximum, in whole ms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fig {
    workers: usize,
    runs: usize,
    first_ms: u64,
    median_ms: u64,
    max_ms: u64,
}

fn fig(times: &[Duration], workers: usize) -> Fig {
    let v: Vec<u64> = times.iter().map(|d| ms(*d)).collect();
    Fig {
        workers,
        runs: v.len(),
        first_ms: v[0],
        median_ms: median_of(&v),
        max_ms: v.iter().copied().max().unwrap_or(0),
    }
}

/// The run whose time is the median (for `elapsedMs (median run)`).
fn median_run(outs: &[RunOutcome]) -> &RunOutcome {
    let mut order: Vec<usize> = (0..outs.len()).collect();
    order.sort_by_key(|&i| outs[i].elapsed);
    &outs[order[outs.len() / 2]]
}

fn times(outs: &[RunOutcome]) -> Vec<Duration> {
    outs.iter().map(|o| o.elapsed).collect()
}

/// 1 warm-up run (not measured) if `warm_up`, then `runs` measured runs; every run is checked
/// inside `f`, the warm-up included (T3).
fn repeat<T>(
    warm_up: bool,
    runs: usize,
    mut f: impl FnMut() -> Result<T, String>,
) -> Result<Vec<T>, String> {
    if warm_up {
        f()?;
    }
    (0..runs).map(|_| f()).collect()
}

// --- The report (section 6.1, 6.3) --------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FixtureFig {
    id: &'static str,
    bytes: u64,
    frames: u64,
}

/// Everything the report shows. `build_report` turns it into lines (6.3), so the smoke test can
/// check the result logic with given figures.
#[derive(Debug, Clone)]
struct Figures {
    release: bool,
    app_workers: usize,
    gated_workers: usize,
    fixtures: [FixtureFig; 3],
    t1_off: Fig,
    t1_cold: Fig,
    t2_off: Fig,
    t2_cold: Fig,
    t4: Fig,
    t6: Fig,
    t7: Fig,
    t8_hires: Fig,
    t8_wav: Fig,
    t9: Fig,
    t10: Fig,
    t11: Fig,
    elapsed_t1_off: u64,
    elapsed_t2_off: u64,
    per_track_median: u64,
    per_track_max: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Gate,
    Indicative,
    Report,
}

struct RowSpec<'a> {
    id: &'static str,
    case: &'static str,
    cache: &'static str,
    /// `None` shows `-` (T10).
    workers: Option<usize>,
    fig: &'a Fig,
    /// `None` shows `-` (T11).
    target: Option<u64>,
    kind: Kind,
}

struct Report {
    lines: Vec<String>,
    /// Failed gate rows as `<id> <cache>`, in table order.
    failed: Vec<String>,
}

impl Report {
    fn passed(&self) -> bool {
        self.failed.is_empty()
    }
}

const TABLE_HEADER: &str =
    "| id | case | cache | workers | runs | first ms | median ms | max ms | target ms | kind | result |";
const TABLE_RULE: &str = "|---|---|---|---|---|---|---|---|---|---|---|";

fn build_report(f: &Figures) -> Report {
    let build = if f.release { "release" } else { "debug" };
    let below = if f.gated_workers < REFERENCE_WORKERS {
        ", host below reference"
    } else {
        ""
    };
    let fixtures: Vec<String> = f
        .fixtures
        .iter()
        .map(|x| format!("{} {} B {} frames", x.id, x.bytes, x.frames))
        .collect();
    let mut lines = vec![
        REPORT_TITLE.to_string(),
        format!("build: {build}"),
        format!(
            "workers: app {}, reference {REFERENCE_WORKERS}, gated album {}{below}",
            f.app_workers, f.gated_workers
        ),
        format!("fixtures: {}", fixtures.join(", ")),
        TABLE_HEADER.to_string(),
        TABLE_RULE.to_string(),
    ];
    // T6: the smaller of 1000 and 10 % of the T2 `off` median, rounded to the nearest ms (D9).
    let t6_target = ((f.t2_off.median_ms + 5) / 10).min(1000);
    fn row<'a>(
        id: &'static str,
        case: &'static str,
        cache: &'static str,
        fig: &'a Fig,
        target: u64,
        kind: Kind,
    ) -> RowSpec<'a> {
        RowSpec {
            id,
            case,
            cache,
            workers: Some(fig.workers),
            fig,
            target: Some(target),
            kind,
        }
    }
    let rows = [
        row("T1", "typical track", "off", &f.t1_off, 3000, Kind::Gate),
        row("T1", "typical track", "cold", &f.t1_cold, 3000, Kind::Gate),
        row("T2", "album", "off", &f.t2_off, 20000, Kind::Gate),
        row("T2", "album", "cold", &f.t2_cold, 20000, Kind::Gate),
        row(
            "T4",
            "album, one corrupt track",
            "off",
            &f.t4,
            20000,
            Kind::Gate,
        ),
        row("T6", "album", "warm", &f.t6, t6_target, Kind::Indicative),
        row("T7", "album", "off", &f.t7, 20000, Kind::Indicative),
        row(
            "T8",
            "hires track",
            "off",
            &f.t8_hires,
            3000,
            Kind::Indicative,
        ),
        row("T8", "wav track", "off", &f.t8_wav, 3000, Kind::Indicative),
        row(
            "T9",
            "cancel latency, album",
            "off",
            &f.t9,
            500,
            Kind::Indicative,
        ),
        RowSpec {
            cache: "-",
            workers: None,
            ..row(
                "T10",
                "spectrogram, typical track",
                "-",
                &f.t10,
                2000,
                Kind::Indicative,
            )
        },
        RowSpec {
            target: None,
            ..row("T11", "album, sequential", "off", &f.t11, 0, Kind::Report)
        },
    ];
    let mut failed = Vec::new();
    for r in &rows {
        // D1, D9: a figure meets its target only when strictly below it.
        let below_target = r.target.is_some_and(|t| r.fig.median_ms < t);
        let (kind, result) = match r.kind {
            Kind::Gate if below_target => ("gate", "pass"),
            Kind::Gate => {
                failed.push(format!("{} {}", r.id, r.cache));
                ("gate", "FAIL")
            }
            Kind::Indicative if below_target => ("indicative", "met"),
            Kind::Indicative => ("indicative", "missed"),
            Kind::Report => ("report", "-"),
        };
        let workers = r.workers.map_or("-".to_string(), |w| w.to_string());
        let target = r.target.map_or("-".to_string(), |t| t.to_string());
        lines.push(format!(
            "| {} | {} | {} | {workers} | {} | {} | {} | {} | {target} | {kind} | {result} |",
            r.id, r.case, r.cache, r.fig.runs, r.fig.first_ms, r.fig.median_ms, r.fig.max_ms
        ));
    }
    lines.push(format!(
        "elapsedMs (median run): T1 off {}, T2 off {}",
        f.elapsed_t1_off, f.elapsed_t2_off
    ));
    lines.push(format!(
        "per track (T11, sequential): median {}, max {}",
        f.per_track_median, f.per_track_max
    ));
    lines.push(if failed.is_empty() {
        "result: pass".to_string()
    } else {
        format!("result: FAIL ({})", failed.join(", "))
    });
    Report { lines, failed }
}

// --- The synthetic suite (sections 3 and 6.1) ---------------------------------------------------

/// Everything one pass of the synthetic suite needs: the `just perf` fixtures with 5 measured
/// runs, or the smoke test's tiny set with 1 (T12).
struct Suite {
    runs: usize,
    gated_workers: usize,
    t9_workers: usize,
    /// The smoke test's T9 must end with exactly this many tracks done (3.5).
    t9_done: Option<u32>,
    strict_real: bool,
    track: Folder,
    album: Folder,
    album_corrupt: Folder,
    corrupt_name: String,
    hires: Folder,
    wav: Folder,
    typical_file: PathBuf,
    cache_file: PathBuf,
    fixtures: [FixtureFig; 3],
}

fn run_suite(s: &Suite) -> Result<Report, String> {
    let n = s.runs;
    let app = worker_count();
    let gated = s.gated_workers;
    let finished = |out: RunOutcome, x: Expect<'_>| -> Result<RunOutcome, String> {
        check_finished(&out, &x)?;
        Ok(out)
    };
    let expect = |row, folder, strict_real, cache| Expect {
        row,
        folder,
        strict_real,
        corrupt: None,
        cache,
    };

    // T1 (scenario "The typical track is analysed in under 3 seconds"), app path.
    let t1_off = repeat(true, n, || {
        let out = run_app(&s.track.dir, Mode::Off).map_err(fail_in("T1 off"))?;
        finished(
            out,
            expect("T1 off", &s.track, s.strict_real, CacheCheck::Off),
        )
    })?;
    // Scenario "The first run with an empty cache stays within the budget".
    let t1_cold = repeat(true, n, || {
        let out = run_app(&s.track.dir, Mode::Cold(&s.cache_file)).map_err(fail_in("T1 cold"))?;
        finished(
            out,
            expect("T1 cold", &s.track, s.strict_real, CacheCheck::Cold),
        )
    })?;

    // T2 (scenario "A 15-track album is analysed in under 20 seconds"), reference path.
    let t2_off = repeat(true, n, || {
        let out =
            run_reference(&s.album.dir, gated, Mode::Off, false).map_err(fail_in("T2 off"))?;
        finished(
            out,
            expect("T2 off", &s.album, s.strict_real, CacheCheck::Off),
        )
    })?;
    let t2_cold = repeat(true, n, || {
        let out = run_reference(&s.album.dir, gated, Mode::Cold(&s.cache_file), false)
            .map_err(fail_in("T2 cold"))?;
        finished(
            out,
            expect("T2 cold", &s.album, s.strict_real, CacheCheck::Cold),
        )
    })?;

    // T4 (@RNF-04, scenario "A corrupt track in the album keeps the album within the budget").
    let t4 = repeat(true, n, || {
        let out = run_reference(&s.album_corrupt.dir, gated, Mode::Off, false)
            .map_err(fail_in("T4 off"))?;
        let x = Expect {
            corrupt: Some(&s.corrupt_name),
            ..expect("T4 off", &s.album_corrupt, s.strict_real, CacheCheck::Off)
        };
        finished(out, x)
    })?;

    // T6 (scenario "A warm run is reported with an indicative target"): one cold run fills the
    // cache, then the warm-up and the measured runs find every track in it.
    let fill = run_reference(&s.album.dir, gated, Mode::Cold(&s.cache_file), false)
        .map_err(fail_in("T6 fill"))?;
    finished(
        fill,
        expect("T6 fill", &s.album, s.strict_real, CacheCheck::Cold),
    )?;
    let t6 = repeat(true, n, || {
        let out = run_reference(&s.album.dir, gated, Mode::Warm(&s.cache_file), false)
            .map_err(fail_in("T6 warm"))?;
        finished(
            out,
            expect("T6 warm", &s.album, s.strict_real, CacheCheck::Warm),
        )
    })?;

    // T7: the album on the app path, one measured run (G1 Q4), no warm-up of its own.
    let t7 = repeat(false, 1, || {
        let out = run_app(&s.album.dir, Mode::Off).map_err(fail_in("T7 off"))?;
        finished(
            out,
            expect("T7 off", &s.album, s.strict_real, CacheCheck::Off),
        )
    })?;

    // T8: hires and wav, any verdict but error.
    let t8_hires = repeat(true, n, || {
        let out = run_app(&s.hires.dir, Mode::Off).map_err(fail_in("T8 hires"))?;
        finished(out, expect("T8 hires", &s.hires, false, CacheCheck::Off))
    })?;
    let t8_wav = repeat(true, n, || {
        let out = run_app(&s.wav.dir, Mode::Off).map_err(fail_in("T8 wav"))?;
        finished(out, expect("T8 wav", &s.wav, false, CacheCheck::Off))
    })?;

    // T9 (scenarios "Cancellation latency is reported" and "The smoke test's cancelled run leaves
    // a track unstarted"): cancel on the first trackDone.
    let t9 = repeat(true, n, || {
        let out =
            run_reference(&s.album.dir, s.t9_workers, Mode::Off, true).map_err(fail_in("T9"))?;
        check_cancelled(&out, s.album.names.len(), s.t9_done)?;
        out.cancel_latency()
            .ok_or_else(|| "perf: T9 check failed: no cancel".to_string())
    })?;

    // T10 (scenario "Spectrogram time is reported"): the DTO built, size above 0.
    let t10 = repeat(true, n, || {
        let start = Instant::now();
        let spectrogram = get_spectrogram(&s.typical_file)
            .map_err(|e| format!("perf: T10 failed: {}", code_name(e.code)))?;
        let took = start.elapsed();
        if spectrogram.width == 0 || spectrogram.height == 0 {
            return Err(format!(
                "perf: T10 check failed: spectrogram {} x {}",
                spectrogram.width, spectrogram.height
            ));
        }
        Ok(took)
    })?;

    // T11: the album with one worker, one measured run; its per-track times are clean (3.4).
    let t11 = repeat(false, 1, || {
        let out = run_reference(&s.album.dir, 1, Mode::Off, false).map_err(fail_in("T11"))?;
        finished(out, expect("T11", &s.album, s.strict_real, CacheCheck::Off))
    })?;
    let per_track: Vec<u64> = t11[0].per_track().iter().map(|(_, d)| ms(*d)).collect();

    let figures = Figures {
        release: !cfg!(debug_assertions),
        app_workers: app,
        gated_workers: gated,
        fixtures: s.fixtures,
        t1_off: fig(&times(&t1_off), app),
        t1_cold: fig(&times(&t1_cold), app),
        t2_off: fig(&times(&t2_off), gated),
        t2_cold: fig(&times(&t2_cold), gated),
        t4: fig(&times(&t4), gated),
        t6: fig(&times(&t6), gated),
        t7: fig(&times(&t7), app),
        t8_hires: fig(&times(&t8_hires), app),
        t8_wav: fig(&times(&t8_wav), app),
        t9: fig(&t9, s.t9_workers),
        t10: fig(&t10, 0),
        t11: fig(&times(&t11), 1),
        elapsed_t1_off: median_run(&t1_off).summary.elapsed_ms,
        elapsed_t2_off: median_run(&t2_off).summary.elapsed_ms,
        per_track_median: median_of(&per_track),
        per_track_max: per_track.iter().copied().max().unwrap_or(0),
    };
    Ok(build_report(&figures))
}

/// `just perf`'s fixtures (section 2), written once before the first run, then checked (T5).
fn synthetic_suite(tmp: &TempRoot) -> Result<Suite, String> {
    let flac_names: Vec<String> = (1..=ALBUM_TRACKS).map(|i| track_name(i, "flac")).collect();
    let first_flac = track_name(1, "flac");
    let track_dir = tmp.album_dir("track");
    let album_dir = tmp.album_dir("album");
    let corrupt_dir = tmp.album_dir("album_corrupt");
    let hires_dir = tmp.album_dir("hires");
    let wav_dir = tmp.album_dir("wav");

    // typical: written once into the album, copied 14 times, and once more for `track` (D4).
    let typical = album_dir.join(&first_flac);
    write_flac(
        &typical,
        &gaussian_stereo(TYPICAL_FRAMES, 16, TYPICAL_LEVEL_DBFS, SEED_TYPICAL),
        44_100,
        16,
    );
    for name in &flac_names[1..] {
        std::fs::copy(&typical, album_dir.join(name)).expect("copy typical");
    }
    std::fs::copy(&typical, track_dir.join(&first_flac)).expect("copy typical");
    // album_corrupt: 08 is the first half of typical; the 14 others link to the album's.
    for (i, name) in flac_names.iter().enumerate() {
        if i + 1 == CORRUPT_TRACK {
            write_first_half(&typical, &corrupt_dir.join(name));
        } else {
            link_or_copy(&album_dir.join(name), &corrupt_dir.join(name));
        }
    }
    let hires = hires_dir.join(&first_flac);
    write_flac(
        &hires,
        &gaussian_stereo(HIRES_FRAMES, 24, HIRES_LEVEL_DBFS, SEED_HIRES),
        96_000,
        24,
    );
    let wav_name = track_name(1, "wav");
    let wav = wav_dir.join(&wav_name);
    write_wav(
        &wav,
        &gaussian_stereo(WAV_FRAMES, 16, WAV_LEVEL_DBFS, SEED_WAV),
        44_100,
    );

    // T5: every fixture against its row before anything is timed.
    let typical_bytes = file_len(&typical);
    check_fixture("typical", typical_bytes, SIZE_MIN, SIZE_MAX)?;
    let typical_frames = check_flac("typical", &typical, 44_100, 16, TYPICAL_FRAMES)?;
    let hires_bytes = file_len(&hires);
    check_fixture("hires", hires_bytes, SIZE_MIN, SIZE_MAX)?;
    let hires_frames = check_flac("hires", &hires, 96_000, 24, HIRES_FRAMES)?;
    let wav_bytes = file_len(&wav);
    check_fixture("wav", wav_bytes, SIZE_MIN, SIZE_MAX)?;
    let wav_frames = check_wav("wav", &wav, 44_100, WAV_FRAMES)?;

    let gated = worker_count().min(REFERENCE_WORKERS);
    Ok(Suite {
        runs: MEASURED_RUNS,
        gated_workers: gated,
        t9_workers: gated,
        t9_done: None,
        strict_real: true,
        track: Folder {
            dir: track_dir,
            names: vec![first_flac.clone()],
            frames: TYPICAL_FRAMES,
            rate: 44_100,
        },
        album: Folder {
            dir: album_dir,
            names: flac_names.clone(),
            frames: TYPICAL_FRAMES,
            rate: 44_100,
        },
        album_corrupt: Folder {
            dir: corrupt_dir,
            names: flac_names.clone(),
            frames: TYPICAL_FRAMES,
            rate: 44_100,
        },
        corrupt_name: flac_names[CORRUPT_TRACK - 1].clone(),
        hires: Folder {
            dir: hires_dir,
            names: vec![first_flac],
            frames: HIRES_FRAMES,
            rate: 96_000,
        },
        wav: Folder {
            dir: wav_dir,
            names: vec![wav_name],
            frames: WAV_FRAMES,
            rate: 44_100,
        },
        typical_file: typical,
        cache_file: tmp.path().join("perf-cache.sqlite3"),
        fixtures: [
            FixtureFig {
                id: "typical",
                bytes: typical_bytes,
                frames: typical_frames,
            },
            FixtureFig {
                id: "hires",
                bytes: hires_bytes,
                frames: hires_frames,
            },
            FixtureFig {
                id: "wav",
                bytes: wav_bytes,
                frames: wav_frames,
            },
        ],
    })
}

/// The smoke test's tiny set (2.3, T12): two FLAC 16/44.1 stereo files of `TINY_FRAMES` in
/// `Álbum ″Ñandú″`. T1, T8 and T10 use the first one; T4 truncates the second one to half.
fn tiny_suite(tmp: &TempRoot) -> Result<Suite, String> {
    let names = vec![track_name(1, "flac"), track_name(2, "flac")];
    let track_dir = tmp.album_dir("track");
    let album_dir = tmp.album_dir("album");
    let corrupt_dir = tmp.album_dir("album_corrupt");
    let tiny = track_dir.join(&names[0]);
    write_tiny(&tiny);
    for name in &names {
        std::fs::copy(&tiny, album_dir.join(name)).expect("copy the tiny file");
    }
    link_or_copy(&album_dir.join(&names[0]), &corrupt_dir.join(&names[0]));
    write_first_half(&tiny, &corrupt_dir.join(&names[1]));
    // STREAMINFO total samples 438 272 on each tiny file (scenario "The fixtures declare their
    // length, so a truncated copy is a corrupt stream"); no size window (2.3).
    for name in &names {
        check_flac("tiny", &album_dir.join(name), 44_100, 16, TINY_FRAMES)?;
    }
    let bytes = file_len(&tiny);
    let tiny_fig = |id| FixtureFig {
        id,
        bytes,
        frames: TINY_FRAMES,
    };
    let folder = |dir: &Path, names: &[String]| Folder {
        dir: dir.to_path_buf(),
        names: names.to_vec(),
        frames: TINY_FRAMES,
        rate: 44_100,
    };
    Ok(Suite {
        runs: 1,
        gated_workers: worker_count().min(REFERENCE_WORKERS),
        t9_workers: 1,
        t9_done: Some(1),
        strict_real: false,
        track: folder(&track_dir, &names[..1]),
        album: folder(&album_dir, &names),
        album_corrupt: folder(&corrupt_dir, &names),
        corrupt_name: names[1].clone(),
        // The smoke test writes no WAV and no 24-bit file: T8 uses the first tiny FLAC (T12).
        hires: folder(&track_dir, &names[..1]),
        wav: folder(&track_dir, &names[..1]),
        typical_file: tiny,
        cache_file: tmp.path().join("perf-cache.sqlite3"),
        fixtures: [tiny_fig("typical"), tiny_fig("hires"), tiny_fig("wav")],
    })
}

fn write_tiny(path: &Path) {
    write_flac(
        path,
        &gaussian_stereo(TINY_FRAMES, 16, TYPICAL_LEVEL_DBFS, SEED_TINY),
        44_100,
        16,
    );
}

// --- Folder mode (sections 5.3 and 6.2) ---------------------------------------------------------

const FORMATS: [(dto::AudioFormat, &str); 5] = [
    (dto::AudioFormat::Flac, "flac"),
    (dto::AudioFormat::Wav, "wav"),
    (dto::AudioFormat::Mp3, "mp3"),
    (dto::AudioFormat::M4a, "m4a"),
    (dto::AudioFormat::Opus, "opus"),
];

fn mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_000_000.0)
}

/// Times `root` read-only, cache off, and returns the harness output (aggregates only: no path,
/// no name, D8) and whether it passed. A folder the run refuses gives `perf: <CODE>` only.
fn folder_report(root: &Path) -> (Vec<String>, Result<(), String>) {
    let mut lines = vec![FOLDER_TITLE.to_string(), build_line()];
    let refused = |mut lines: Vec<String>, f: RunFail| {
        let message = match f {
            RunFail::App(code) => format!("perf: {code}"),
            RunFail::Other(message) => format!("perf: {message}"),
        };
        lines.push(message.clone());
        (lines, Err(message))
    };
    // Step 1: total time on the app path, 1 first run and 3 more.
    let mut runs = Vec::new();
    for _ in 0..=FOLDER_RUNS {
        match run_app(root, Mode::Off) {
            Ok(out) => runs.push(ms(out.elapsed)),
            Err(f) => return refused(lines, f),
        }
    }
    // Step 2: one sequential pass on the reference path, per-track times by format.
    let pass = match run_reference(root, 1, Mode::Off, false) {
        Ok(out) => out,
        Err(f) => return refused(lines, f),
    };
    let tracks = pass.per_track();
    let errors = pass.rec.done.iter().filter(|d| d.error.is_some()).count();
    let total_bytes: u64 = pass.rec.done.iter().map(|d| d.size_bytes).sum();
    let counts: Vec<String> = FORMATS
        .iter()
        .map(|(format, name)| {
            let n = pass.rec.done.iter().filter(|d| d.format == *format).count();
            format!("{name} {n}")
        })
        .collect();
    lines.push(format!(
        "tracks {} | errors {errors} | {} MB | {}",
        pass.summary.total,
        mb(total_bytes),
        counts.join(", ")
    ));
    let next = &runs[1..];
    lines.push("| run | workers | runs | first ms | median ms | max ms |".to_string());
    lines.push("|---|---|---|---|---|---|".to_string());
    lines.push(format!(
        "| folder | {} | {FOLDER_RUNS} | {} | {} | {} |",
        worker_count(),
        runs[0],
        median_of(next),
        next.iter().copied().max().unwrap_or(0)
    ));
    lines.push(
        "| format | tracks | errors | MB | median ms per track | max ms per track |".to_string(),
    );
    lines.push("|---|---|---|---|---|---|".to_string());
    for (format, name) in FORMATS {
        let of_format: Vec<&(&DoneRec, Duration)> =
            tracks.iter().filter(|(d, _)| d.format == format).collect();
        if of_format.is_empty() {
            continue;
        }
        // Error tracks count in every cell; `errors` says which format they came from (6.2).
        let format_errors = of_format.iter().filter(|(d, _)| d.error.is_some()).count();
        let bytes: u64 = of_format.iter().map(|(d, _)| d.size_bytes).sum();
        let per: Vec<u64> = of_format.iter().map(|(_, t)| ms(*t)).collect();
        lines.push(format!(
            "| {name} | {} | {format_errors} | {} | {} | {} |",
            of_format.len(),
            mb(bytes),
            median_of(&per),
            per.iter().copied().max().unwrap_or(0)
        ));
    }
    (lines, Ok(()))
}

// --- The timed suite (ignored: `just perf` only) -------------------------------------------------

/// Scenarios "(timed)" of performance.feature: T1–T11 on the generated fixtures, or the folder
/// of `FAKEFLAC_PERF_FOLDER` (5.3). Fails after printing the whole report if a gate fails.
#[test]
#[ignore = "RNF-01 timed suite: run it through `just perf` (release build, never in CI)"]
fn timed_suite() {
    // Scenario "The timed suite in a debug build prints the error and times nothing" (D2).
    if let Err(e) = check_release_build(cfg!(debug_assertions)) {
        println!("{e}");
        panic!("{e}");
    }
    if let Some(folder) = std::env::var_os("FAKEFLAC_PERF_FOLDER") {
        let (lines, result) = folder_report(Path::new(&folder));
        for line in &lines {
            println!("{line}");
        }
        if let Err(e) = result {
            panic!("{e}");
        }
        return;
    }
    let tmp = TempRoot::new("suite");
    // Scenario "A fixture outside its size window stops the harness before timing".
    let suite = match synthetic_suite(&tmp) {
        Ok(suite) => suite,
        Err(e) => {
            println!("{e}");
            panic!("{e}");
        }
    };
    let report = match run_suite(&suite) {
        Ok(report) => report,
        Err(e) => {
            println!("{e}");
            panic!("{e}");
        }
    };
    for line in &report.lines {
        println!("{line}");
    }
    // Scenario "A missed gate fails "just perf" after the whole report is printed".
    assert!(
        report.passed(),
        "{}",
        report.lines.last().map_or("", String::as_str)
    );
}

// --- Smoke test (CI, `just check-app`; no assertion on any time) --------------------------------

/// The 12 rows of section 6.1: id, case, cache and kind, in order. Written out here on purpose
/// (not taken from `build_report`), so the smoke test checks the report against the spec.
const EXPECTED_ROWS: [(&str, &str, &str, &str); 12] = [
    ("T1", "typical track", "off", "gate"),
    ("T1", "typical track", "cold", "gate"),
    ("T2", "album", "off", "gate"),
    ("T2", "album", "cold", "gate"),
    ("T4", "album, one corrupt track", "off", "gate"),
    ("T6", "album", "warm", "indicative"),
    ("T7", "album", "off", "indicative"),
    ("T8", "hires track", "off", "indicative"),
    ("T8", "wav track", "off", "indicative"),
    ("T9", "cancel latency, album", "off", "indicative"),
    ("T10", "spectrogram, typical track", "-", "indicative"),
    ("T11", "album, sequential", "off", "report"),
];

fn cells(line: &str) -> Vec<&str> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(str::trim).collect()
}

fn table_rows(lines: &[String]) -> Vec<Vec<&str>> {
    lines
        .iter()
        .filter(|l| l.starts_with("| T"))
        .map(|l| cells(l))
        .collect()
}

fn row_of<'a>(rows: &'a [Vec<&'a str>], id: &str, cache: &str) -> &'a [&'a str] {
    rows.iter()
        .find(|r| r[0] == id && r[2] == cache)
        .unwrap_or_else(|| panic!("no row {id} {cache}"))
}

/// Scenario "The report has a fixed shape".
fn check_report_shape(lines: &[String]) {
    let build = build_line();
    assert!(lines.iter().any(|l| l == REPORT_TITLE), "{lines:#?}");
    assert!(lines.contains(&build), "{lines:#?}");
    assert!(
        lines.iter().any(|l| l.starts_with("workers: ")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("fixtures: ")),
        "{lines:#?}"
    );
    // Only the recipe prints the machine line (5.2).
    assert!(!lines.iter().any(|l| l.contains("machine:")), "{lines:#?}");
    let rows = table_rows(lines);
    assert_eq!(rows.len(), 12, "{lines:#?}");
    for (row, (id, case, cache, kind)) in rows.iter().zip(EXPECTED_ROWS) {
        assert_eq!(row.len(), 11, "{row:?}");
        assert_eq!(
            (row[0], row[1], row[2], row[9]),
            (id, case, cache, kind),
            "{row:?}"
        );
        for cell in &row[5..8] {
            assert!(cell.parse::<u64>().is_ok(), "time cell {cell:?} in {row:?}");
        }
        if id == "T11" {
            assert_eq!(row[8], "-", "{row:?}");
        } else {
            assert!(
                row[8].parse::<u64>().is_ok(),
                "target {:?} in {row:?}",
                row[8]
            );
        }
    }
    let t10 = row_of(&rows, "T10", "-");
    assert_eq!((t10[2], t10[3]), ("-", "-"), "{t10:?}");
    let last = lines.last().expect("a report line");
    assert!(
        last == "result: pass" || last.starts_with("result: FAIL ("),
        "{last}"
    );
}

/// T12: the whole synthetic suite's code path on the tiny set, 1 warm-up and 1 measured run per
/// row. Scenarios "The report has a fixed shape", "The smoke test's cancelled run leaves a track
/// unstarted" and "The fixtures declare their length, so a truncated copy is a corrupt stream".
#[test]
fn smoke_synthetic_suite_runs_on_the_tiny_set() {
    let tmp = TempRoot::new("smoke");
    let suite = tiny_suite(&tmp).unwrap_or_else(|e| panic!("{e}"));
    let report = run_suite(&suite).unwrap_or_else(|e| panic!("{e}"));
    for line in &report.lines {
        println!("{line}");
    }
    check_report_shape(&report.lines);
    // T9 ran with 1 worker (3.5).
    let rows = table_rows(&report.lines);
    assert_eq!(row_of(&rows, "T9", "off")[3], "1");
    assert_eq!(row_of(&rows, "T11", "off")[3], "1");
}

/// Figures below every target (the result logic's starting point, 6.3).
fn figures_below_targets() -> Figures {
    let f = |workers, median_ms| Fig {
        workers,
        runs: 5,
        first_ms: median_ms,
        median_ms,
        max_ms: median_ms,
    };
    let fixture = |id| FixtureFig {
        id,
        bytes: 40_000_000,
        frames: TYPICAL_FRAMES,
    };
    Figures {
        release: !cfg!(debug_assertions),
        app_workers: 15,
        gated_workers: 7,
        fixtures: [fixture("typical"), fixture("hires"), fixture("wav")],
        t1_off: f(15, 270),
        t1_cold: f(15, 281),
        t2_off: f(7, 861),
        t2_cold: f(7, 881),
        t4: f(7, 729),
        t6: f(7, 3),
        t7: Fig {
            runs: 1,
            ..f(15, 434)
        },
        t8_hires: f(15, 188),
        t8_wav: f(15, 48),
        t9: f(7, 25),
        t10: f(0, 445),
        t11: Fig {
            runs: 1,
            ..f(1, 3979)
        },
        elapsed_t1_off: 268,
        elapsed_t2_off: 859,
        per_track_median: 266,
        per_track_max: 270,
    }
}

fn last_line(report: &Report) -> &str {
    report.lines.last().map_or("", String::as_str)
}

/// Scenario Outline "Which figures fail the run" and "Indicative figures never fail the run"
/// (6.3, T12 "With given figures").
#[test]
fn report_result_logic_with_given_figures() {
    let base = figures_below_targets();
    let report = build_report(&base);
    check_report_shape(&report.lines);
    assert_eq!(last_line(&report), "result: pass");
    assert!(report.passed());
    // 6.3: the `build:` line names the build the figures come from.
    for (release, line) in [(true, "build: release"), (false, "build: debug")] {
        let report = build_report(&Figures {
            release,
            ..base.clone()
        });
        assert_eq!(report.lines[1], line);
    }

    // A T1 cold median of 3000 ms fails that row (D1: strictly below).
    let mut f = base.clone();
    f.t1_cold.median_ms = 3000;
    let report = build_report(&f);
    let rows = table_rows(&report.lines);
    assert_eq!(rows.len(), 12);
    assert_eq!(row_of(&rows, "T1", "cold")[10], "FAIL");
    assert_eq!(row_of(&rows, "T1", "off")[10], "pass");
    assert_eq!(last_line(&report), "result: FAIL (T1 cold)");
    assert!(!report.passed());

    // 2999 ms passes.
    let mut f = base.clone();
    f.t1_cold.median_ms = 2999;
    let report = build_report(&f);
    assert_eq!(row_of(&table_rows(&report.lines), "T1", "cold")[10], "pass");
    assert_eq!(last_line(&report), "result: pass");

    // Two failed gates, in table order.
    let mut f = base.clone();
    f.t1_cold.median_ms = 3000;
    f.t2_off.median_ms = 20000;
    let report = build_report(&f);
    assert_eq!(table_rows(&report.lines).len(), 12);
    assert_eq!(last_line(&report), "result: FAIL (T1 cold, T2 off)");

    // An indicative figure above its target is `missed` and never fails the run (D9).
    let mut f = base.clone();
    f.t9.median_ms = 501;
    let report = build_report(&f);
    let rows = table_rows(&report.lines);
    assert_eq!(row_of(&rows, "T9", "off")[10], "missed");
    assert_eq!(last_line(&report), "result: pass");

    // Targets and kinds of the indicative and report rows (scenario "Indicative figures never
    // fail the run"): T6 is the smaller of 1000 and a tenth of the T2 off median (861 -> 86).
    let report = build_report(&base);
    let rows = table_rows(&report.lines);
    let target = |id, cache| row_of(&rows, id, cache)[8].to_string();
    assert_eq!(target("T6", "warm"), "86");
    assert_eq!(row_of(&rows, "T6", "warm")[10], "met");
    assert_eq!(target("T7", "off"), "20000");
    assert_eq!(target("T9", "off"), "500");
    assert_eq!(target("T10", "-"), "2000");
    assert_eq!(target("T11", "off"), "-");
    assert_eq!(row_of(&rows, "T11", "off")[10], "-");
    let t8: Vec<&Vec<&str>> = rows.iter().filter(|r| r[0] == "T8").collect();
    assert_eq!((t8[0][1], t8[0][8]), ("hires track", "3000"));
    assert_eq!((t8[1][1], t8[1][8]), ("wav track", "3000"));
    let mut f = base.clone();
    f.t2_off.median_ms = 19_000;
    let report = build_report(&f);
    let rows = table_rows(&report.lines);
    assert_eq!(row_of(&rows, "T6", "warm")[8], "1000");

    // The workers line names a host below the reference (section 4).
    let mut f = base.clone();
    f.gated_workers = 3;
    let report = build_report(&f);
    assert!(report
        .lines
        .iter()
        .any(|l| l == "workers: app 15, reference 7, gated album 3, host below reference"));
    let report = build_report(&base);
    assert!(report
        .lines
        .iter()
        .any(|l| l == "workers: app 15, reference 7, gated album 7"));
}

/// Scenario Outline "A debug build gives no figures" (D2, T0).
#[test]
fn check_release_build_with_given_values() {
    assert_eq!(
        check_release_build(true),
        Err("perf: figures need a release build (run: just perf)".to_string())
    );
    assert_eq!(check_release_build(false), Ok(()));
}

/// Scenario Outline "A fixture outside its size window stops the harness before timing" (T5).
#[test]
fn check_fixture_with_given_values() {
    assert_eq!(
        check_fixture("typical", 43_000_000, 38_000_000, 42_000_000),
        Err("perf: fixture typical is 43000000 bytes, outside [38000000, 42000000]".to_string())
    );
    assert_eq!(
        check_fixture("typical", 38_000_000, 38_000_000, 42_000_000),
        Ok(())
    );
    assert_eq!(
        check_fixture("typical", 42_000_000, 38_000_000, 42_000_000),
        Ok(())
    );
}

/// Name, size, modification time and content hash of every entry of `dir` (RNF-05).
fn snapshot(dir: &Path) -> Vec<(String, u64, SystemTime, u64)> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read the folder")
        .map(|e| {
            let e = e.expect("folder entry");
            let meta = e.metadata().expect("metadata");
            let mut hasher = DefaultHasher::new();
            if meta.is_file() {
                std::fs::read(e.path()).expect("read").hash(&mut hasher);
            }
            (
                e.file_name().to_string_lossy().into_owned(),
                meta.len(),
                meta.modified().expect("mtime"),
                hasher.finish(),
            )
        })
        .collect();
    entries.sort();
    entries
}

/// T13 (a), scenario "The empty state: a folder without audio gives no figures" and @RNF-05.
#[test]
fn folder_mode_on_a_folder_without_audio_gives_no_figures() {
    let tmp = TempRoot::new("t13a");
    let dir = tmp.path().join("notes only");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("notes.txt"), "not audio").unwrap();
    let before = snapshot(&dir);
    let (lines, result) = folder_report(&dir);
    for line in &lines {
        println!("{line}");
    }
    assert_eq!(
        lines,
        vec![
            "RNF-01 folder report (aggregates only, ADR-0022)".to_string(),
            build_line(),
            "perf: NO_AUDIO_FOUND".to_string(),
        ]
    );
    assert!(result.is_err());
    assert_eq!(snapshot(&dir), before);
}

const FORMAT_TABLE_HEADER: &str =
    "| format | tracks | errors | MB | median ms per track | max ms per track |";

/// The format table of the folder report (section 6.2): checks its header and separator (6
/// cells) and returns the cells of each format row, each row having the header's 6 cells.
fn format_rows(lines: &[String]) -> Vec<Vec<&str>> {
    let output = lines.join("\n");
    let at = lines
        .iter()
        .position(|l| l.starts_with("| format |"))
        .unwrap_or_else(|| panic!("no format table:\n{output}"));
    assert_eq!(lines[at], FORMAT_TABLE_HEADER, "{output}");
    assert_eq!(
        lines.get(at + 1).map(String::as_str),
        Some("|---|---|---|---|---|---|"),
        "{output}"
    );
    let rows: Vec<Vec<&str>> = lines
        .iter()
        .filter(|l| {
            FORMATS
                .iter()
                .any(|(_, n)| l.starts_with(&format!("| {n} |")))
        })
        .map(|l| cells(l))
        .collect();
    for row in &rows {
        assert_eq!(row.len(), 6, "{output}");
    }
    rows
}

/// Section 6.2's one rule: every `trackDone` counts, so the format rows' `tracks` cells add up to
/// the header's `tracks <n>` and their `errors` cells to the header's `errors <n>`.
fn assert_rows_add_up(header: &str, rows: &[Vec<&str>], output: &str) {
    let count = |label: &str| -> u64 {
        header
            .split(" | ")
            .find_map(|part| part.strip_prefix(label)?.strip_prefix(' '))
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("no `{label} <n>` in the header {header:?}"))
    };
    let sum = |column: usize| -> u64 {
        rows.iter()
            .map(|row| {
                row[column]
                    .parse::<u64>()
                    .unwrap_or_else(|_| panic!("not a count: {:?}\n{output}", row[column]))
            })
            .sum()
    };
    assert_eq!(sum(1), count("tracks"), "rows' tracks vs header\n{output}");
    assert_eq!(sum(2), count("errors"), "rows' errors vs header\n{output}");
}

/// T13 (b), scenarios "A folder whose path has spaces, an ampersand and Unicode characters is
/// timed" (@RNF-09) and "The folder mode never modifies the timed files" (@RNF-05).
#[test]
fn folder_mode_reports_aggregates_only_and_writes_nothing() {
    let tmp = TempRoot::new("t13b");
    let folder_name = "Álbum ″Ñandú″ & co";
    let dir = tmp.path().join(folder_name);
    std::fs::create_dir_all(&dir).unwrap();
    let names = [track_name(1, "flac"), track_name(2, "flac")];
    write_tiny(&dir.join(&names[0]));
    std::fs::copy(dir.join(&names[0]), dir.join(&names[1])).unwrap();
    let before = snapshot(&dir);

    let (lines, result) = folder_report(&dir);
    for line in &lines {
        println!("{line}");
    }
    result.unwrap_or_else(|e| panic!("{e}"));
    let output = lines.join("\n");
    assert_eq!(lines[0], FOLDER_TITLE);
    assert_eq!(lines[1], build_line());
    assert!(!output.contains("machine:"), "{output}");
    let header = &lines[2];
    assert!(header.starts_with("tracks 2 | errors 0 | "), "{header}");
    assert!(
        header.ends_with(" MB | flac 2, wav 0, mp3 0, m4a 0, opus 0"),
        "{header}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("| folder | ")),
        "{output}"
    );
    // One format row, for flac, with tracks 2 and errors 0 (section 6.2).
    let rows = format_rows(&lines);
    assert_eq!(rows.len(), 1, "{output}");
    assert_eq!(&rows[0][..3], ["flac", "2", "0"], "{output}");
    assert_rows_add_up(header, &rows, &output);

    // D8: no path and no name anywhere.
    assert!(!output.contains(folder_name), "{output}");
    for name in &names {
        assert!(!output.contains(name.as_str()), "{output}");
    }
    let absolute = dir.to_string_lossy().into_owned();
    for component in absolute.split(['\\', '/']) {
        if component.ends_with(':') || component.chars().count() < 4 {
            continue;
        }
        assert!(
            !output.contains(component),
            "the output names the path component {component:?}:\n{output}"
        );
    }
    // RNF-05: the files are unchanged and nothing was created.
    assert_eq!(snapshot(&dir), before);
}

/// T13 (c), scenario "The folder report says which format an error came from" (@RNF-04), and
/// "The folder mode never modifies the timed files" (@RNF-05).
#[test]
fn folder_report_says_which_format_an_error_came_from() {
    let tmp = TempRoot::new("t13c");
    let source = tmp.path().join("source");
    std::fs::create_dir_all(&source).unwrap();
    let tiny = source.join(track_name(1, "flac"));
    write_tiny(&tiny);
    let dir = tmp.album_dir("one_truncated");
    let names = [track_name(1, "flac"), track_name(2, "flac")];
    std::fs::copy(&tiny, dir.join(&names[0])).unwrap();
    write_first_half(&tiny, &dir.join(&names[1]));
    let before = snapshot(&dir);

    let (lines, result) = folder_report(&dir);
    for line in &lines {
        println!("{line}");
    }
    result.unwrap_or_else(|e| panic!("{e}"));
    let output = lines.join("\n");
    let header = &lines[2];
    assert!(header.starts_with("tracks 2 | errors 1 | "), "{header}");
    assert!(
        header.ends_with(" MB | flac 2, wav 0, mp3 0, m4a 0, opus 0"),
        "{header}"
    );
    let rows = format_rows(&lines);
    assert_eq!(rows.len(), 1, "{output}");
    assert_eq!(&rows[0][..3], ["flac", "2", "1"], "{output}");
    assert_rows_add_up(header, &rows, &output);
    // RNF-05: the files are unchanged and nothing was created.
    assert_eq!(snapshot(&dir), before);
}

// --- T14: wiring of the recipe (scenario "The gate is opt-in and never runs in CI") --------------

struct Recipe {
    name: String,
    deps: String,
    body: Vec<String>,
}

/// A recipe header: `name [params]: deps` at column 0 (not `:=`, not a comment or attribute).
fn recipe_header(line: &str) -> Option<Recipe> {
    if line.starts_with('#') || line.starts_with('[') {
        return None;
    }
    let colon = line.find(':')?;
    if line[colon + 1..].starts_with('=') {
        return None;
    }
    let name = line[..colon]
        .split_whitespace()
        .next()?
        .trim_start_matches('@');
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    valid.then(|| Recipe {
        name: name.to_string(),
        deps: line[colon + 1..].to_string(),
        body: Vec::new(),
    })
}

/// The recipes of a justfile: header, dependency list, and body (the indented lines that follow,
/// up to the next non-empty line that is not indented; comment lines excluded).
fn parse_recipes(text: &str) -> Vec<Recipe> {
    let mut recipes = Vec::new();
    let mut current: Option<Recipe> = None;
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(recipe) = current.as_mut() {
                if !line.trim_start().starts_with('#') {
                    recipe.body.push(line.to_string());
                }
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(recipe) = current.take() {
            recipes.push(recipe);
        }
        current = recipe_header(line);
    }
    recipes.extend(current);
    recipes
}

/// `\bword\b`: `word` with no word character on either side.
fn names_word(text: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

fn repo_file(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn just_perf_is_wired_and_opt_in() {
    // The parser itself, on a small justfile.
    let sample = "set shell := [\"sh\"]\n\n# comment\n[positional-arguments]\nperf *args:\n    # perf in a comment\n    run perf\n\n    more\ncheck: check-fast test\n    cargo test\nother:\n  echo performance\n";
    let parsed = parse_recipes(sample);
    let names: Vec<&str> = parsed.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["perf", "check", "other"]);
    assert_eq!(parsed[0].body, ["    run perf", "    more"]);
    assert_eq!(parsed[1].deps.trim(), "check-fast test");
    assert!(!names_word(&parsed[2].body.join("\n"), "perf"));
    assert!(names_word("just perf FOLDER", "perf"));
    assert!(names_word("check-perf", "perf"));

    let justfile = repo_file("justfile");
    let recipes = parse_recipes(&justfile);
    let perf = recipes
        .iter()
        .find(|r| r.name == "perf")
        .expect("justfile has no `perf` recipe (performance.md 5.2, T14)");
    let body = perf.body.join("\n");
    for needle in [
        "--release",
        "--ignored",
        "--test-threads=1",
        "--test perf",
        "FAKEFLAC_PERF_FOLDER",
        "usage: just perf [FOLDER]",
    ] {
        assert!(
            body.contains(needle),
            "the `perf` recipe's body does not hold {needle:?}:\n{body}"
        );
    }
    for name in ["ci", "check", "check-fast", "test", "check-app"] {
        let recipe = recipes
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("justfile has no `{name}` recipe"));
        assert!(
            !names_word(&recipe.deps, "perf") && !names_word(&recipe.body.join("\n"), "perf"),
            "the `{name}` recipe names perf"
        );
    }
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        assert!(
            !repo_file(workflow).contains("just perf"),
            "{workflow} runs just perf"
        );
    }
}

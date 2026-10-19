//! Test support for the calibration tool (RP-11): the synthetic signals of `specs/detection/fixtures.md`
//! (ADR-0022), the sample corpus of `specs/calibration/sample-corpus.md`, a fixture transcoder and
//! small CSV and file-system helpers. Never real audio, never the real corpus, no ffmpeg.
//!
//! The signals follow `crates/fakeflac-audio/tests/common/mod.rs` (same seeds and recipes). That module
//! needs `realfft`, which ADR-0036 does not list as a dev-dependency of the tool, so the inverse FFT
//! is written here (`ifft`). Folders are created under the system temporary folder, NOT under
//! `CARGO_TARGET_TMPDIR`: that one is inside the repository, and the tool refuses to work there.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use fakeflac_calibrate::config::Setting;
use fakeflac_calibrate::error::{ExitKind, ToolError};
use fakeflac_calibrate::run::{AcceptArgs, CalibrateArgs, Command, Outcome, PrivacyArgs};
use fakeflac_calibrate::transcode::{
    TranscodeError, TranscodeOutput, TranscodeRequest, Transcoder,
};
use fakeflac_calibrate::{run, Env};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

// ---------------------------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------------------------

/// The repository root (two levels above `tools/fakeflac-calibrate`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

pub fn v2_params() -> PathBuf {
    repo_root().join("specs/detection/detection-params.v2.toml")
}

/// Detection v3: v2 plus the optional `[full_band]` section (rules.md, ADR-0038).
pub fn v3_params() -> PathBuf {
    repo_root().join("specs/detection/detection-params.v3.toml")
}

pub fn v1_params() -> PathBuf {
    repo_root().join("specs/detection/detection-params.v1.toml")
}

/// The next free `algorithm_version` as sample-corpus.md section 4 and evaluation.md section 7
/// define it: 1 + the largest `algorithm_version` of `specs/detection/detection-params.v*.toml`.
pub fn next_free_version() -> u32 {
    let dir = repo_root().join("specs/detection");
    let highest = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("detection-params.v") && name.ends_with(".toml")
        })
        .map(|e| {
            let table: toml::Table = toml::from_str(&read(&e.path())).unwrap();
            table["algorithm_version"].as_integer().unwrap()
        })
        .max()
        .expect("specs/detection has params files");
    u32::try_from(highest).unwrap() + 1
}

pub fn spec_config() -> PathBuf {
    repo_root().join("specs/calibration/calibration.v1.toml")
}

pub fn read(path: &Path) -> String {
    String::from_utf8(std::fs::read(path).unwrap()).unwrap()
}

/// Base folder of all test folders: the system temporary folder, outside the repository.
fn base_dir() -> PathBuf {
    std::env::temp_dir().join("fakeflac-calibrate-tests")
}

/// Removes test folders (`<pid>-<name>`, not the fixtures) older than two hours: shared runs are never
/// cleaned by their own process.
fn sweep_stale() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let Ok(read_dir) = std::fs::read_dir(base_dir()) else {
            return;
        };
        let limit = std::time::Duration::from_secs(2 * 3600);
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let old = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > limit);
            if old && name.starts_with(|c: char| c.is_ascii_digit()) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    });
}

/// A fresh empty folder for one test (`<temp>/fakeflac-calibrate-tests/<pid>-<name>`).
pub fn scratch_dir(name: &str) -> PathBuf {
    sweep_stale();
    let dir = base_dir().join(format!("{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every file and folder under `dir` (relative path with `/`), with size and mtime in seconds
/// (`0`, `0` for folders).
pub fn snapshot(dir: &Path) -> BTreeMap<String, (u64, i64)> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, (u64, i64)>) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                out.insert(rel, (0, 0));
                walk(root, &path, out);
            } else {
                let mtime = meta
                    .modified()
                    .unwrap()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                out.insert(rel, (meta.len(), mtime));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// Names only (no size, no mtime): "nothing was written" checks.
pub fn names(dir: &Path) -> Vec<String> {
    snapshot(dir).into_keys().collect()
}

/// Files (not folders) under `dir`, recursively.
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
    snapshot(dir)
        .into_iter()
        .filter(|(rel, (size, mtime))| !(*size == 0 && *mtime == 0 && dir.join(rel).is_dir()))
        .map(|(rel, _)| dir.join(rel))
        .collect()
}

/// `git status --porcelain` of the repository: compared before and after a run.
pub fn git_status() -> String {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root())
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ---------------------------------------------------------------------------------------------
// CSV (RFC 4180), read side only
// ---------------------------------------------------------------------------------------------

/// Parses RFC 4180 text: quoted fields, doubled quotes, line breaks inside quotes, CRLF.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' => quoted = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\r' => {}
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                other => field.push(other),
            }
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// A CSV file with a header row.
pub struct Table {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn read(path: &Path) -> Table {
        let text = read(path);
        let mut rows = parse_csv(&text);
        assert!(!rows.is_empty(), "{} is empty", path.display());
        let header = rows.remove(0);
        Table { header, rows }
    }

    pub fn has_column(&self, name: &str) -> bool {
        self.header.iter().any(|h| h == name)
    }

    pub fn col(&self, name: &str) -> usize {
        self.header
            .iter()
            .position(|h| h == name)
            .unwrap_or_else(|| panic!("column {name} missing in {:?}", self.header))
    }

    pub fn get<'a>(&self, row: &'a [String], name: &str) -> &'a str {
        &row[self.col(name)]
    }

    /// First row whose `column` equals `value`.
    pub fn find(&self, column: &str, value: &str) -> Option<&Vec<String>> {
        let c = self.col(column);
        self.rows.iter().find(|r| r[c] == value)
    }
}

pub fn labels(outcome: &Outcome, sample: &Sample) -> Table {
    let _ = outcome;
    Table::read(&sample.calibration.join("labels.csv"))
}

pub fn tracks(outcome: &Outcome) -> Table {
    Table::read(&outcome.report_dir.as_ref().unwrap().join("tracks.csv"))
}

pub fn report_text(outcome: &Outcome, file: &str) -> String {
    read(&outcome.report_dir.as_ref().unwrap().join(file))
}

/// Drops the lines that may differ between two runs with the same inputs: run id, date and cache
/// statistics (calibration.md section 4, "Determinism").
pub fn without_volatile(text: &str) -> String {
    const VOLATILE: [&str; 8] = [
        "run id",
        "run:",
        "date",
        "cache",
        "decoded",
        "transcoded",
        "hits",
        "skipped lines",
    ];
    text.lines()
        .filter(|l| {
            let l = l.to_lowercase();
            !VOLATILE.iter().any(|v| l.contains(v))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------------
// Synthetic signals (fixtures.md)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Container {
    Flac,
    Wav,
}

impl Container {
    fn ext(self) -> &'static str {
        match self {
            Container::Flac => "flac",
            Container::Wav => "wav",
        }
    }
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Fixture folder; its name depends on this file, so a change of the generator starts from scratch.
fn fixture_dir() -> PathBuf {
    base_dir().join(format!(
        "fixtures-{:016x}",
        fnv(include_str!("mod.rs").as_bytes())
    ))
}

/// Path to the fixture `name` (ids of fixtures.md) in the given container, generating it once.
/// The file is shared by every test process: it is linked into place and never replaced.
pub fn fixture(name: &str, container: Container) -> PathBuf {
    type Cells = Mutex<HashMap<(String, Container), Arc<OnceLock<PathBuf>>>>;
    static CACHE: OnceLock<Cells> = OnceLock::new();
    let cell = {
        let mut map = CACHE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        map.entry((name.to_string(), container))
            .or_default()
            .clone()
    };
    cell.get_or_init(|| {
        let dir = fixture_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.{}", container.ext()));
        if !path.exists() {
            let tmp = dir.join(format!(
                "{name}.{}.{}.tmp",
                std::process::id(),
                container.ext()
            ));
            build(name, container, &tmp);
            let _ = std::fs::hard_link(&tmp, &path);
            let _ = std::fs::remove_file(&tmp);
            assert!(path.exists(), "fixture {} missing", path.display());
        }
        path
    })
    .clone()
}

/// Copies fixture `name` to `dest` (new modification time), creating parent folders.
pub fn place(name: &str, container: Container, dest: &Path) {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::copy(fixture(name, container), dest).unwrap();
}

fn build(name: &str, container: Container, path: &Path) {
    if name == "corrupt" {
        let bytes = std::fs::read(fixture("full_band", Container::Flac)).unwrap();
        std::fs::write(path, &bytes[..16 * 1024]).unwrap();
        return;
    }
    let (samples, bits, dither) = signal(name);
    let ints = quantize(&samples, bits, dither);
    match container {
        Container::Wav => write_wav(path, &ints, 44_100, bits),
        Container::Flac => write_flac(path, &ints, 44_100, bits),
    }
}

/// Samples, bit depth and dither seed of a fixture. All are 44.1 kHz, 16-bit, mono.
fn signal(name: &str) -> (Vec<f64>, u16, Option<u64>) {
    let sr = 44_100;
    let secs = if name == "short" { 2.0 } else { 30.0 };
    let mut samples = match name {
        "full_band" | "short" => shaped_noise(sr, secs, |_| 1.0, 1),
        "low_hf" => shaped_noise(
            sr,
            secs,
            |f| {
                if (20.0..6_000.0).contains(&f) {
                    1.0 / f.sqrt()
                } else {
                    0.0
                }
            },
            6,
        ),
        "lp16k_dither" => shaped_noise(sr, secs, |f| brick(f, 16_000.0) * pink(f), 7),
        "lp20k_floor" => shaped_noise(
            sr,
            secs,
            |f| {
                if f < 20_000.0 {
                    pink(f)
                } else {
                    pink(20_000.0) * db_to_gain(-15.0)
                }
            },
            8,
        ),
        "sfb21_gaps" => gated_pink(sr, secs, 16_000.0, -20.0, 9),
        "sfb21_gaps_weak" => gated_pink(sr, secs, 16_000.0, -6.0, 10),
        "hf_bursts" => gated_pink(sr, secs, 8_000.0, -20.0, 11),
        other => panic!("unknown fixture {other}"),
    };
    let level = if name == "lp16k_dither" { -40.0 } else { -12.0 };
    set_rms_dbfs(&mut samples, level);
    limit_peak(&mut samples, -1.0);
    (samples, 16, (name == "lp16k_dither").then_some(70))
}

/// Frames for `secs` seconds, rounded down to a multiple of the FLAC block size (4096).
fn frames(sr: u32, secs: f64) -> usize {
    const FLAC_BLOCK: usize = 4096;
    (f64::from(sr) * secs) as usize / FLAC_BLOCK * FLAC_BLOCK
}

fn pink(f: f64) -> f64 {
    if f >= 20.0 {
        1.0 / f.sqrt()
    } else {
        0.0
    }
}

fn db_to_gain(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

fn brick(f: f64, fc: f64) -> f64 {
    if f < fc {
        1.0
    } else {
        0.0
    }
}

/// Pink noise split exactly in frequency at `edge_hz`; the part above is gated (gain 1 in the last
/// 0.5 s of every 2 s, `gate_db` otherwise, 5 ms raised-cosine ramps).
fn gated_pink(sr: u32, secs: f64, edge_hz: f64, gate_db: f64, seed: u64) -> Vec<f64> {
    let low = shaped_noise(sr, secs, |f| if f < edge_hz { pink(f) } else { 0.0 }, seed);
    let high = shaped_noise(sr, secs, |f| if f < edge_hz { 0.0 } else { pink(f) }, seed);
    let low_gain = db_to_gain(gate_db);
    low.iter()
        .zip(&high)
        .enumerate()
        .map(|(i, (l, h))| {
            let t = i as f64 / f64::from(sr);
            l + h * gate(t % 2.0, low_gain)
        })
        .collect()
}

fn gate(t: f64, low: f64) -> f64 {
    const RAMP: f64 = 0.005;
    let blend = |x: f64| 0.5 * (1.0 - (std::f64::consts::PI * x).cos());
    if t >= 1.5 {
        1.0
    } else if t >= 1.5 - RAMP {
        low + (1.0 - low) * blend((t - (1.5 - RAMP)) / RAMP)
    } else if t < RAMP {
        1.0 + (low - 1.0) * blend(t / RAMP)
    } else {
        low
    }
}

/// Noise with magnitude response `gain(f)` and random phases, built with one inverse FFT.
fn shaped_noise(sr: u32, secs: f64, gain: impl Fn(f64) -> f64, seed: u64) -> Vec<f64> {
    let len = frames(sr, secs);
    let n = len.next_power_of_two();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut re = vec![0.0f64; n];
    let mut im = vec![0.0f64; n];
    let bins = n / 2 + 1;
    for k in 1..bins - 1 {
        let f = k as f64 * f64::from(sr) / n as f64;
        let phase = rng.random::<f64>() * 2.0 * std::f64::consts::PI;
        let g = gain(f);
        let (s, c) = phase.sin_cos();
        re[k] = g * c;
        im[k] = g * s;
        re[n - k] = g * c;
        im[n - k] = -g * s;
    }
    ifft(&mut re, &mut im);
    re.truncate(len);
    re
}

/// Unnormalised inverse FFT (positive exponent), radix 2, in place. `re.len()` is a power of two.
fn ifft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = i.reverse_bits() >> (usize::BITS - bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let tw: Vec<(f64, f64)> = (0..n / 2)
        .map(|k| {
            let a = 2.0 * std::f64::consts::PI * k as f64 / n as f64;
            (a.cos(), a.sin())
        })
        .collect();
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let step = n / len;
        for start in (0..n).step_by(len) {
            for k in 0..half {
                let (wr, wi) = tw[k * step];
                let (a, b) = (start + k, start + k + half);
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len *= 2;
    }
}

fn set_rms_dbfs(x: &mut [f64], dbfs: f64) {
    let rms = (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
    if rms > 0.0 {
        let k = 10f64.powf(dbfs / 20.0) / rms;
        x.iter_mut().for_each(|v| *v *= k);
    }
}

fn limit_peak(x: &mut [f64], dbfs: f64) {
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let limit = 10f64.powf(dbfs / 20.0);
    if peak > limit {
        let k = limit / peak;
        x.iter_mut().for_each(|v| *v *= k);
    }
}

fn quantize(x: &[f64], bits: u16, dither_seed: Option<u64>) -> Vec<i32> {
    let full = f64::from((1i32 << (bits - 1)) - 1);
    match dither_seed {
        None => x.iter().map(|v| (v * full).round() as i32).collect(),
        Some(seed) => {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            x.iter()
                .map(|v| {
                    let tpdf = (rng.random::<f64>() - 0.5) + (rng.random::<f64>() - 0.5);
                    (v * full + tpdf).round() as i32
                })
                .collect()
        }
    }
}

fn write_wav(path: &Path, samples: &[i32], sr: u32, bits: u16) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for s in samples {
        w.write_sample(*s).unwrap();
    }
    w.finalize().unwrap();
}

fn write_flac(path: &Path, samples: &[i32], sr: u32, bits: u16) {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source = flacenc::source::MemSource::from_samples(samples, 1, bits as usize, sr as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size).unwrap();
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).unwrap();
    std::fs::write(path, sink.as_slice()).unwrap();
}

// ---------------------------------------------------------------------------------------------
// Fixture transcoder (sample-corpus.md section 1)
// ---------------------------------------------------------------------------------------------

/// Ignores the source audio and writes the fixture named in `args[0]` as `<generated_dir>/<id>.flac`.
pub struct FixtureTranscoder;

impl Transcoder for FixtureTranscoder {
    fn supports(&self, encoder: &str) -> bool {
        encoder == "fixture"
    }

    fn version(&self) -> Result<String, ToolError> {
        Ok("fixture".to_string())
    }

    fn transcode(&self, request: &TranscodeRequest<'_>) -> Result<TranscodeOutput, TranscodeError> {
        let name = request
            .setting
            .args
            .first()
            .ok_or_else(|| TranscodeError::Failed("no fixture named".into()))?;
        std::fs::create_dir_all(request.generated_dir).unwrap();
        let flac = request.generated_dir.join(format!("{}.flac", request.id));
        std::fs::copy(fixture(name, Container::Flac), &flac)
            .map_err(|e| TranscodeError::Failed(e.to_string()))?;
        Ok(TranscodeOutput {
            flac,
            encoder_version: "fixture".to_string(),
        })
    }
}

/// A transcoder that must never be called: runs where every entry is cached.
pub struct NoTranscoder;

impl Transcoder for NoTranscoder {
    fn supports(&self, _encoder: &str) -> bool {
        true
    }
    fn version(&self) -> Result<String, ToolError> {
        Err(ToolError {
            kind: ExitKind::ExternalTool,
            lines: vec!["error: ffmpeg not found".into()],
        })
    }
    fn transcode(&self, _r: &TranscodeRequest<'_>) -> Result<TranscodeOutput, TranscodeError> {
        panic!("no transcode expected");
    }
}

// ---------------------------------------------------------------------------------------------
// Sample corpus (sample-corpus.md sections 2 and 3)
// ---------------------------------------------------------------------------------------------

pub const TRACK_1: &str = "Compras/Ñandú – Ácido, Vol. 1 (2024)/01 – Intro ’A’.flac";
pub const TRACK_2: &str = "Compras/Ñandú – Ácido, Vol. 1 (2024)/02 – Canción.flac";
pub const TRACK_3: &str = "Compras/Ñandú – Ácido, Vol. 1 (2024)/03 – Bonus.flac";
pub const TRACK_4: &str = "Colección/Álbum Ü/CD1/01 – Uno.flac";
pub const TRACK_5: &str = "Colección/Álbum Ü/CD2/01 – Dos.flac";
pub const TRACK_7: &str = "Revisión/Ørsted/01 – Tres.wav";
pub const TRACK_8: &str = "Revisión/Ørsted/02 – Roto.flac";
pub const TRACK_9: &str = "Revisión/Ørsted/03 – Corto.flac";
pub const TRACK_10: &str = "Sueltas/Mixto/05 – Cuatro.flac";
pub const TRACK_11: &str = "Sueltas/Mixto/06 – Falso.flac";

/// Every folder name, file name and file stem of the sample corpus, and every note of the lists:
/// none may appear in `summary.md` (calibration.md section 6).
pub const PRIVATE_NAMES: [&str; 22] = [
    "Compras",
    "Ñandú – Ácido, Vol. 1 (2024)",
    "01 – Intro ’A’",
    "02 – Canción",
    "03 – Bonus",
    "Colección",
    "Álbum Ü",
    "01 – Uno",
    "01 – Dos",
    "Revisión",
    "Ørsted",
    "01 – Tres",
    "02 – Roto",
    "03 – Corto",
    "Sueltas",
    "Mixto",
    "05 – Cuatro",
    "06 – Falso",
    "comprado",
    "otra edición",
    "barras normales y separador final",
    "espectrograma",
];

/// The corpus, the working folder and the test configuration of one test. Everything lives in
/// `<temp>/fakeflac-calibrate-tests/<pid>-<name>/{corpus,calibration,scratch}` and is removed on drop.
pub struct Sample {
    pub root: PathBuf,
    pub corpus: PathBuf,
    pub calibration: PathBuf,
    /// Config, params and grid files of the test: outside both roots.
    pub scratch: PathBuf,
    cleanup: bool,
}

impl Sample {
    /// Builds the 11 files of sample-corpus.md section 2, `trusted.txt`, `confirmed.txt` and the
    /// test configuration (`holdout_fraction` of the spec file, 0.30).
    pub fn new(name: &str) -> Sample {
        let mut s = Sample::build(name);
        s.cleanup = true;
        s
    }

    /// Like `new`, but the folders stay: for runs shared by several tests of one process.
    pub fn new_shared(name: &str) -> Sample {
        Sample::build(name)
    }

    fn build(name: &str) -> Sample {
        let root = scratch_dir(name);
        let s = Sample {
            corpus: root.join("corpus"),
            calibration: root.join("calibration"),
            scratch: root.join("scratch"),
            root,
            cleanup: false,
        };
        for dir in [&s.corpus, &s.calibration, &s.scratch] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let c = &s.corpus;
        place("full_band", Container::Flac, &c.join(TRACK_1));
        place("hf_bursts", Container::Flac, &c.join(TRACK_2));
        place("full_band", Container::Flac, &c.join(TRACK_3));
        place("full_band", Container::Flac, &c.join(TRACK_4));
        place("low_hf", Container::Flac, &c.join(TRACK_5));
        std::fs::write(c.join("Colección/Álbum Ü/folder.jpg"), b"not audio").unwrap();
        place("full_band", Container::Wav, &c.join(TRACK_7));
        place("corrupt", Container::Flac, &c.join(TRACK_8));
        place("short", Container::Flac, &c.join(TRACK_9));
        place("full_band", Container::Flac, &c.join(TRACK_10));
        place("lp16k_dither", Container::Flac, &c.join(TRACK_11));
        s.write_trusted(&s.trusted_lines());
        s.write_confirmed(&s.confirmed_lines());
        s.write_config(None);
        s
    }

    /// The lines of the sample `trusted.txt` (line N is element N-1).
    pub fn trusted_lines(&self) -> Vec<String> {
        vec![
            "# Sample trusted list (CI).".to_string(),
            format!("library_root = {}", self.corpus.display()),
            String::new(),
            "Compras\\Ñandú – Ácido, Vol. 1 (2024) | tienda | comprado".to_string(),
            "!Compras\\Ñandú – Ácido, Vol. 1 (2024)\\03 – Bonus.flac | otra edición".to_string(),
            "Colección\\Álbum Ü | analisis-limpio".to_string(),
            "Revisión/Ørsted/ | revisado | barras normales y separador final".to_string(),
            "Sueltas\\Mixto\\05 – Cuatro.flac | analisis".to_string(),
        ]
    }

    pub fn confirmed_lines(&self) -> Vec<String> {
        vec![
            format!("library_root = {}", self.corpus.display()),
            "Sueltas\\Mixto\\06 – Falso.flac | fake | espectrograma".to_string(),
            "Compras\\Ñandú – Ácido, Vol. 1 (2024)\\02 – Canción.flac | false-positive | detector antiguo"
                .to_string(),
        ]
    }

    pub fn write_trusted(&self, lines: &[String]) {
        write_lines(&self.calibration.join("trusted.txt"), lines);
    }

    pub fn write_confirmed(&self, lines: &[String]) {
        write_lines(&self.calibration.join("confirmed.txt"), lines);
    }

    /// Test configuration: `calibration.v1.toml` with the 4 fixture settings and targets T1 and T2
    /// (sample-corpus.md section 2). `holdout_fraction` overrides the file's 0.30.
    pub fn write_config(&self, holdout_fraction: Option<f64>) -> PathBuf {
        self.write_config_opts(holdout_fraction, false)
    }

    /// Like `write_config`, but sets `lowpass_hz` on fx128 per sample-corpus.md section 1:
    /// fx128=16000, so that Track 5 (low_hf) is classified as undetectable.
    pub fn write_config_with_lowpass(&self, holdout_fraction: Option<f64>) -> PathBuf {
        self.write_config_opts(holdout_fraction, true)
    }

    pub fn write_config_opts(&self, holdout_fraction: Option<f64>, with_lowpass: bool) -> PathBuf {
        let spec = read(&spec_config());
        let head = &spec[..spec.find("[[generate.settings]]").unwrap()];
        let report = &spec[spec.find("[report]").unwrap()..spec.find("[[targets]]").unwrap()];
        let head = match holdout_fraction {
            Some(f) => head.replace(
                "holdout_fraction = 0.30",
                &format!("holdout_fraction = {f:?}"),
            ),
            None => head.to_string(),
        };
        let settings = [
            (
                "fx128",
                Some(128),
                if with_lowpass { Some(16000) } else { None },
                "lp16k_dither",
            ),
            ("fxv0", None, None, "sfb21_gaps"),
            ("fx320", Some(320), None, "lp20k_floor"),
            ("fxweak", None, None, "sfb21_gaps_weak"),
        ];
        let mut text = head;
        for (id, bitrate, lowpass, fixture) in settings {
            text.push_str(&format!(
                "[[generate.settings]]\nid = \"{id}\"\nencoder = \"fixture\"\next = \"mp3\"\n"
            ));
            if let Some(b) = bitrate {
                text.push_str(&format!("bitrate_kbps = {b}\n"));
            }
            if let Some(lp) = lowpass {
                text.push_str(&format!("lowpass_hz = {lp}\n"));
            }
            text.push_str(&format!("args = [\"{fixture}\"]\n\n"));
        }
        text.push_str(report);
        text.push_str(
            "[[targets]]\nid = \"T1\"\ndescription = \"No primary track is Fake\"\nclasses = [\"primary\"]\nverdict = \"fake\"\nmax_count = 0\n\n\
             [[targets]]\nid = \"T2\"\ndescription = \"No fx128 transcode is Real\"\nsettings = [\"fx128\"]\nverdict = \"real\"\nmax_count = 0\n",
        );
        let path = self.config_path();
        std::fs::write(&path, text).unwrap();
        path
    }

    pub fn config_path(&self) -> PathBuf {
        self.scratch.join("calibration.test.toml")
    }

    /// Writes a params, grid or other file into the scratch folder.
    pub fn write_scratch(&self, name: &str, text: &str) -> PathBuf {
        let path = self.scratch.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// `candidate.toml` of sample-corpus.md section 4: v2 with `algorithm_version` set to the next
    /// free version (never 3, which is detection v3) and `verdict.gap_fake_db = 4.0`.
    pub fn candidate_params(&self) -> PathBuf {
        let text = read(&v2_params())
            .replace(
                "algorithm_version = 2",
                &format!("algorithm_version = {}", next_free_version()),
            )
            .replace("gap_fake_db = 6.0", "gap_fake_db = 4.0");
        self.write_scratch("candidate.toml", &text)
    }

    /// v2 with `fft.window = "blackman_harris_4"`: an extraction change.
    pub fn blackman_params(&self) -> PathBuf {
        let text =
            read(&v2_params()).replacen("window = \"hann\"", "window = \"blackman_harris_4\"", 1);
        self.write_scratch("blackman.toml", &text)
    }

    pub fn env(&self) -> Env {
        Env {
            corpus: Some(self.corpus.clone()),
            calibration: Some(self.calibration.clone()),
            ffmpeg: None,
            repo_root: Some(repo_root()),
        }
    }

    pub fn accept_args(&self, params: &[PathBuf]) -> AcceptArgs {
        AcceptArgs {
            params: params.to_vec(),
            config: self.config_path(),
            no_generate: false,
        }
    }

    /// `accept` on this sample with the fixture transcoder.
    pub fn accept(&self, params: &[PathBuf]) -> Outcome {
        run(
            &Command::Accept(self.accept_args(params)),
            &self.env(),
            &FixtureTranscoder,
        )
    }

    /// `accept` with options.
    pub fn accept_opts(&self, params: &[PathBuf], no_generate: bool) -> Outcome {
        run(
            &Command::Accept(AcceptArgs {
                params: params.to_vec(),
                config: self.config_path(),
                no_generate,
            }),
            &self.env(),
            &FixtureTranscoder,
        )
    }

    /// `accept` with the default params file v2.
    pub fn accept_v2(&self) -> Outcome {
        self.accept(&[v2_params()])
    }

    /// `privacy-check` on this sample.
    pub fn privacy_check(&self, base: &str) -> Outcome {
        run(
            &Command::PrivacyCheck(PrivacyArgs {
                base: base.to_string(),
            }),
            &self.env(),
            &FixtureTranscoder,
        )
    }

    pub fn calibrate(&self, grid: &Path) -> Outcome {
        run(
            &Command::Calibrate(CalibrateArgs {
                grid: grid.to_path_buf(),
                config: self.config_path(),
            }),
            &self.env(),
            &FixtureTranscoder,
        )
    }

    /// Another, empty working folder for the same corpus, with the same lists.
    pub fn fresh_calibration(&self, name: &str) -> Sample {
        let root = self.root.join(name);
        let calibration = root.join("calibration");
        std::fs::create_dir_all(&calibration).unwrap();
        for file in ["trusted.txt", "confirmed.txt"] {
            std::fs::copy(self.calibration.join(file), calibration.join(file)).unwrap();
        }
        Sample {
            root,
            corpus: self.corpus.clone(),
            calibration,
            scratch: self.scratch.clone(),
            cleanup: false,
        }
    }

    /// Writes track 1 again with the same bytes and a modification time one hour later.
    pub fn touch_track_1(&self) {
        let path = self.corpus.join(TRACK_1);
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        file.set_modified(later).unwrap();
    }

    /// Absolute folder of `FAKEFLAC_CALIBRATION/features`.
    pub fn features_dir(&self) -> PathBuf {
        self.calibration.join("features")
    }
}

impl Drop for Sample {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

fn write_lines(path: &Path, lines: &[String]) {
    std::fs::write(path, lines.join("\n") + "\n").unwrap();
}

/// Sum of `decoded` over the fingerprints of a run.
pub fn decoded(outcome: &Outcome) -> u32 {
    outcome.cache.iter().map(|c| c.decoded).sum()
}

/// Sum of `transcoded` over the fingerprints of a run.
pub fn transcoded(outcome: &Outcome) -> u32 {
    outcome.cache.iter().map(|c| c.transcoded).sum()
}

/// Fails with the error output when the run did not exit 0.
pub fn assert_ok(outcome: &Outcome) {
    assert_eq!(outcome.exit_code, 0, "stderr: {}", outcome.stderr);
}

/// A cold `accept` on the sample corpus with the given params files, shared by the tests of one
/// process that only read its result.
pub struct SharedRun {
    pub sample: Sample,
    pub outcome: Outcome,
}

/// Cold run with v2 only.
pub fn shared_v2() -> &'static SharedRun {
    static CELL: OnceLock<SharedRun> = OnceLock::new();
    CELL.get_or_init(|| {
        let sample = Sample::new_shared("shared-v2");
        let outcome = sample.accept_v2();
        SharedRun { sample, outcome }
    })
}

/// Cold run with v2 and `candidate.toml` in one invocation.
pub fn shared_v2_and_candidate() -> &'static SharedRun {
    static CELL: OnceLock<SharedRun> = OnceLock::new();
    CELL.get_or_init(|| {
        let sample = Sample::new_shared("shared-two");
        let candidate = sample.candidate_params();
        let outcome = sample.accept(&[v2_params(), candidate]);
        SharedRun { sample, outcome }
    })
}

/// Cold run with v2 and v3 in one invocation (sample-corpus.md section 4, "v3 side by side").
pub fn shared_v2_and_v3() -> &'static SharedRun {
    static CELL: OnceLock<SharedRun> = OnceLock::new();
    CELL.get_or_init(|| {
        let sample = Sample::new_shared("shared-v2-v3");
        let outcome = sample.accept(&[v2_params(), v3_params()]);
        SharedRun { sample, outcome }
    })
}

/// The setting struct of `calibration.v1.toml` with the given id (for the ffmpeg command lines).
pub fn spec_setting(id: &str) -> Setting {
    let config = fakeflac_calibrate::Config::parse(&read(&spec_config())).unwrap();
    config
        .generate
        .settings
        .into_iter()
        .find(|s| s.id == id)
        .unwrap()
}

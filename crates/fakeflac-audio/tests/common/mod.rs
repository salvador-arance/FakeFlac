//! Synthetic test signals of `specs/detection/fixtures.md` (ADR-0022). Generated with a fixed seed,
//! written once per test binary under `CARGO_TARGET_TMPDIR`. No music is ever used.
//!
//! Level note: the spec says "−12 dBFS" (`lp16k_dither`: −40 dBFS). Gaussian-like noise at −12 dBFS
//! RMS has peaks above full scale, and clipping would create content above the low-pass,
//! invalidating the fixture. Every signal is therefore set to its RMS level and then scaled down,
//! if needed, so its peak is at most −1 dBFS. Relative levels inside a fixture (noise vs tone,
//! signal vs floor) are preserved.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use fakeflac_core::ports::FileEntry;
use fakeflac_core::AudioFormat;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use realfft::num_complex::Complex;
use realfft::RealFftPlanner;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

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

/// Path to the fixture `name` (ids of fixtures.md) in the given container, generating it once.
pub fn fixture(name: &str, container: Container) -> PathBuf {
    // One cell per fixture: the map lock is held only to fetch the cell, so building a fixture
    // that depends on another one (corrupt -> full_band) cannot deadlock.
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
            // nextest runs every test in its own process, so the in-process cache does not stop
            // two processes from building the same fixture. Each builds a private copy and links it
            // into place only if no complete file is there yet: an existing fixture is never
            // replaced (its modification time stays stable while other tests read it).
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

/// Fixture folder of this build of the test binary: all its processes share it, and a rebuild
/// (possibly with a changed generator) starts from an empty folder.
fn fixture_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let built = std::fs::metadata(&exe)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis());
    let stem = exe.file_stem().unwrap().to_string_lossy().into_owned();
    Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("fixtures")
        .join(format!("{stem}-{built}"))
}

/// `FileEntry` as the scanner would produce it.
pub fn entry(path: &Path) -> FileEntry {
    let meta = std::fs::metadata(path).unwrap();
    let ext = path.extension().unwrap().to_string_lossy().to_string();
    FileEntry {
        path: path.to_path_buf(),
        folder: path.parent().unwrap().to_path_buf(),
        format: AudioFormat::from_extension(&ext).unwrap(),
        size_bytes: meta.len(),
        modified_unix: meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    }
}

struct Spec {
    sample_rate: u32,
    bits: u16,
    seconds: f64,
    samples: Vec<f64>,
    /// Seed of the TPDF dither added before rounding; `None` is plain rounding.
    dither_seed: Option<u64>,
}

fn build(name: &str, container: Container, path: &Path) {
    if name == "corrupt" {
        // A FLAC file truncated after its header and a few frames.
        let full = fixture("full_band", Container::Flac);
        let bytes = std::fs::read(full).unwrap();
        std::fs::write(path, &bytes[..16 * 1024]).unwrap();
        return;
    }
    let spec = signal(name);
    let ints = quantize(&spec.samples, spec.bits, spec.dither_seed);
    match container {
        Container::Wav => write_wav(path, &ints, spec.sample_rate, spec.bits),
        Container::Flac => write_flac(path, &ints, spec.sample_rate, spec.bits),
    }
    let _ = spec.seconds;
}

fn signal(name: &str) -> Spec {
    let (sr, bits, secs) = match name {
        "lp16k_96k" => (96_000, 24, 30.0),
        "short" => (44_100, 16, 2.0),
        _ => (44_100, 16, 30.0),
    };
    let samples = match name {
        "full_band" | "short" => shaped_noise(sr, secs, |_| 1.0, 1),
        "lp16k" | "lp16k_96k" => shaped_noise(sr, secs, |f| brick(f, 16_000.0), 2),
        "lp19k" => shaped_noise(sr, secs, |f| brick(f, 19_000.0), 3),
        "lp20k" => shaped_noise(sr, secs, |f| brick(f, 20_000.0), 4),
        // 60 dB/octave above 8 kHz: gain = (f / 8000)^-10.
        "gradual" => shaped_noise(
            sr,
            secs,
            |f| {
                if f <= 8_000.0 {
                    1.0
                } else {
                    (f / 8_000.0).powi(-10)
                }
            },
            5,
        ),
        // Pink noise (1/sqrt(f) amplitude) band-limited to 20 Hz – 6 kHz.
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
        "bass_tone_lp16k" => {
            let mut noise = shaped_noise(sr, secs, |f| brick(f, 16_000.0), 2);
            set_rms_dbfs(&mut noise, -12.0);
            let amp = 10f64.powf(-3.0 / 20.0);
            for (i, s) in noise.iter_mut().enumerate() {
                *s += amp * (2.0 * std::f64::consts::PI * 60.0 * i as f64 / f64::from(sr)).sin();
            }
            limit_peak(&mut noise, -1.0);
            return Spec {
                sample_rate: sr,
                bits,
                seconds: secs,
                samples: noise,
                dither_seed: None,
            };
        }
        "silence" => vec![0.0; frames(sr, secs)],
        // Pink noise (amplitude 1/sqrt(f)) through a brick-wall low-pass at 16 kHz.
        "lp16k_dither" => shaped_noise(sr, secs, |f| brick(f, 16_000.0) * pink(f), 7),
        // Pink below 20 kHz, white from 20 kHz up, 15 dB below the pink gain at 20 kHz.
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
        // Pink noise whose part above 16 kHz (8 kHz for hf_bursts) is gated: gain 1 during the last
        // 0.5 s of every 2 s, `gate_db` the rest of the time.
        "sfb21_gaps" => gated_pink(sr, secs, 16_000.0, -20.0, 9),
        "sfb21_gaps_weak" => gated_pink(sr, secs, 16_000.0, -6.0, 10),
        "hf_bursts" => gated_pink(sr, secs, 8_000.0, -20.0, 11),
        other => panic!("unknown fixture {other}"),
    };
    let mut samples = samples;
    if name != "silence" {
        let level = if name == "lp16k_dither" { -40.0 } else { -12.0 };
        set_rms_dbfs(&mut samples, level);
        limit_peak(&mut samples, -1.0);
    }
    Spec {
        sample_rate: sr,
        bits,
        seconds: secs,
        samples,
        dither_seed: (name == "lp16k_dither").then_some(70),
    }
}

/// Number of frames for `secs` seconds, rounded down to a multiple of the FLAC block size (4096).
/// flacenc writes the (shorter) last block's size as the stream's minimum block size; strict
/// decoders such as symphonia then reject every fixed-size frame. Real encoders (libFLAC) write
/// min = max for fixed blocking, so the fixtures avoid a short last block.
fn frames(sr: u32, secs: f64) -> usize {
    const FLAC_BLOCK: usize = 4096;
    (f64::from(sr) * secs) as usize / FLAC_BLOCK * FLAC_BLOCK
}

/// Pink noise gain: amplitude 1/sqrt(f) above 20 Hz (as in `low_hf`).
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

/// Pink noise split exactly in the frequency domain at `edge_hz` (both parts come from the same
/// random phases, so their sum is the full-band pink noise). The part from `edge_hz` up is
/// multiplied by a gate: gain 1 during the last 0.5 s of every 2 s and `gate_db` the rest of the
/// time, with 5 ms raised-cosine ramps placed outside the full-gain window (rising in the 5 ms
/// before it, falling in the 5 ms after it).
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

/// Gate gain at `t` seconds into a 2 s period: 1 in [1.5, 2.0), `low` elsewhere, raised-cosine
/// ramps of 5 ms immediately before 1.5 s (rising) and after 2.0 s (falling, i.e. from 0 to 5 ms
/// of the next period).
fn gate(t: f64, low: f64) -> f64 {
    const RAMP: f64 = 0.005;
    let blend = |x: f64| 0.5 * (1.0 - (std::f64::consts::PI * x).cos()); // 0 -> 1 for x in 0..1
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

fn brick(f: f64, fc: f64) -> f64 {
    if f < fc {
        1.0
    } else {
        0.0
    }
}

/// Noise with magnitude response `gain(f)` and random phases, built in the frequency domain with
/// a single inverse FFT (no block boundaries inside the signal, so the low-pass is exact).
fn shaped_noise(sr: u32, secs: f64, gain: impl Fn(f64) -> f64, seed: u64) -> Vec<f64> {
    let len = frames(sr, secs);
    let n = len.next_power_of_two();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut planner = RealFftPlanner::<f64>::new();
    let ifft = planner.plan_fft_inverse(n);
    let mut spectrum = ifft.make_input_vec();
    let bins = spectrum.len();
    for (k, c) in spectrum.iter_mut().enumerate() {
        if k == 0 || k == bins - 1 {
            *c = Complex::new(0.0, 0.0);
            continue;
        }
        let f = k as f64 * f64::from(sr) / n as f64;
        let phase = rng.random::<f64>() * 2.0 * std::f64::consts::PI;
        *c = Complex::from_polar(gain(f), phase);
    }
    let mut out = ifft.make_output_vec();
    ifft.process(&mut spectrum, &mut out).unwrap();
    out.truncate(len);
    out
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

/// Plain rounding, or with TPDF dither: the sum of two independent uniform values in [-1/2, 1/2)
/// LSB added before rounding.
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

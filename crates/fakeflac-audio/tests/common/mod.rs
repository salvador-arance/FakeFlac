//! Synthetic test signals of `specs/detection/fixtures.md` (ADR-0022). Generated with a fixed seed,
//! written once per test binary under `CARGO_TARGET_TMPDIR`. No music is ever used.
//!
//! Level note: the spec says "−12 dBFS". Gaussian-like noise at −12 dBFS RMS has peaks above full
//! scale, and clipping would create content above the low-pass, invalidating the fixture. Every
//! signal is therefore set to −12 dBFS RMS and then scaled down, if needed, so its peak is −1 dBFS.
//! Relative levels inside a fixture (noise vs tone) are preserved.
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
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fixtures");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.{}", container.ext()));
        build(name, container, &path);
        path
    })
    .clone()
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
    let ints = quantize(&spec.samples, spec.bits);
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
            };
        }
        "silence" => vec![0.0; (f64::from(sr) * secs) as usize],
        other => panic!("unknown fixture {other}"),
    };
    let mut samples = samples;
    if name != "silence" {
        set_rms_dbfs(&mut samples, -12.0);
        limit_peak(&mut samples, -1.0);
    }
    Spec {
        sample_rate: sr,
        bits,
        seconds: secs,
        samples,
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
    let len = (f64::from(sr) * secs) as usize;
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

fn quantize(x: &[f64], bits: u16) -> Vec<i32> {
    let full = f64::from((1i32 << (bits - 1)) - 1);
    x.iter().map(|v| (v * full).round() as i32).collect()
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

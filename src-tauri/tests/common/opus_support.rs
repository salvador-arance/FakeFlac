//! Opus integration fixtures: seeds and signal recipes of lossy-fixtures.md §1.3.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::PathBuf;
#[path = "../../../crates/fakeflac-audio/tests/common/opus.rs"]
pub mod opus;
fn fixture_dir() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("tauri-opus-fixtures");
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn frames(sr: u32, secs: f64) -> usize {
    (f64::from(sr) * secs).round() as usize
}
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

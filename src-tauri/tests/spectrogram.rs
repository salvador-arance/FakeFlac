//! `get_spectrogram` wiring and JSON shape (RF-07, `specs/contracts/commands.md`).
//! Synthetic file only. Protected after the `test(RF-07)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core as core;
use fakeflac_lib::dto::{self, AppErrorCode};
use fakeflac_lib::run::get_spectrogram;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("spectrogram_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn write_noise_wav(path: &Path, seconds: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("wav");
    let mut rng = ChaCha8Rng::seed_from_u64(11);
    for _ in 0..(44100 * seconds) {
        let s: f64 = rng.random::<f64>() * 2.0 - 1.0;
        w.write_sample((s * 8000.0) as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

// @RF-07 Scenario: Spectrogram shape (through the command wiring)
#[test]
fn spectrogram_of_a_wav_has_the_configured_shape() {
    let dir = temp_dir("ok");
    let path = dir.join("noise.wav");
    write_noise_wav(&path, 8);
    let s = get_spectrogram(&path).expect("spectrogram");
    assert_eq!(s.height, 512);
    assert!(s.width > 0 && s.width <= 1200, "width {}", s.width);
    assert_eq!(s.sample_rate, 44100);
    assert_eq!(s.max_frequency_hz, 22050.0);
    assert_eq!(s.db_range, (-120.0, 0.0));
    assert_eq!(s.data.len(), s.width as usize * s.height as usize);
}

// Contract: `INTERNAL` if the file cannot be decoded.
#[test]
fn a_missing_file_fails_with_internal() {
    let dir = temp_dir("missing");
    let err = get_spectrogram(&dir.join("gone.flac")).expect_err("missing file");
    assert_eq!(err.code, AppErrorCode::Internal);
}

#[test]
fn an_undecodable_file_fails_with_internal() {
    let dir = temp_dir("garbage");
    let path = dir.join("garbage.flac");
    std::fs::write(&path, vec![0x5Au8; 4096]).unwrap();
    let err = get_spectrogram(&path).expect_err("garbage file");
    assert_eq!(err.code, AppErrorCode::Internal);
}

// The contract type: camelCase fields, `dbRange` as a pair, `data` as a byte array.
#[test]
fn spectrogram_json_matches_the_contract() {
    let s = core::Spectrogram {
        width: 2,
        height: 3,
        sample_rate: 44100,
        max_frequency_hz: 22050.0,
        db_range: [-120.0, 0.0],
        data: vec![0, 1, 2, 253, 254, 255],
    };
    let v: Value = serde_json::to_value(dto::Spectrogram::from(&s)).expect("serialize");
    assert_eq!(
        v,
        json!({
            "width": 2, "height": 3, "sampleRate": 44100, "maxFrequencyHz": 22050.0,
            "dbRange": [-120.0, 0.0], "data": [0, 1, 2, 253, 254, 255]
        })
    );
}

// @RNF-09 A path with non-ASCII characters reaches the decoder intact.
#[test]
fn a_non_ascii_path_gives_a_spectrogram() {
    let dir = temp_dir("Ñandú – Ü");
    let path = dir.join("Canción ‘x’ – Ü.wav");
    write_noise_wav(&path, 8);
    let s = get_spectrogram(&path).expect("spectrogram of a Unicode path");
    assert_eq!(s.height, 512);
    assert!(s.width > 0);
}

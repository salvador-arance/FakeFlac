//! Tests derived from `specs/features/inspect-track.feature` (@RF-07), scenario "Spectrogram shape",
//! and the `get_spectrogram` error contract (`specs/contracts/commands.md`).
//! Protected after the `test(RF-07)` commit.
//!
//! Level convention (decided here, ADR-0009): the dB value of a bin is normalised by the coherent
//! gain of the window, so a full-scale sine reads 0 dB, which is `db_ceiling`. With that
//! normalisation the white noise of the fixtures (-12 dBFS RMS) reads about -43 dB per bin.
//!
//! "Near the bottom of dbRange" is made measurable as: the mean byte of every row above 16.2 kHz
//! is at most 63 (25 % of 255, i.e. within 30 dB of `db_floor` for the default range of 120 dB).
//! The Hann leakage of the 16 kHz edge at 200 Hz is about -107 dB (byte ~28), so the bound holds
//! with margin; a flat or all-zero image is ruled out by requiring the passband to be bright.
//!
//! The "within 2 seconds" clause of the first scenario is a wall-clock budget of a release
//! build; it is not asserted here (debug test builds are not representative): manual check.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::{DetectionParams, Spectrogram, TrackError};
use std::path::Path;

fn lp16k() -> Spectrogram {
    let path = fixture("lp16k", Container::Flac);
    SymphoniaAnalyzer
        .spectrogram(&path, &DetectionParams::default().spectrogram)
        .expect("spectrogram of lp16k")
}

/// Mean byte of the frequency row `f` over all time columns (`data[t * height + f]`).
fn row_mean(s: &Spectrogram, f: usize) -> f64 {
    let h = s.height as usize;
    let sum: u64 = (0..s.width as usize)
        .map(|t| u64::from(s.data[t * h + f]))
        .sum();
    sum as f64 / f64::from(s.width)
}

// @RF-07 Scenario: Spectrogram shape
#[test]
fn shape_of_a_30_second_track() {
    let p = DetectionParams::default().spectrogram;
    let s = lp16k();
    assert_eq!(s.height, 512);
    assert_eq!(s.height, p.height);
    assert!(s.width > 0 && s.width <= 1200, "width {}", s.width);
    assert!(s.width <= p.max_width);
    assert_eq!(s.sample_rate, 44_100);
    assert_eq!(s.max_frequency_hz, 22_050.0);
    assert_eq!(s.data.len(), s.width as usize * s.height as usize);
}

// @RF-07 Scenario: Spectrogram shape (dbRange comes from the parameters, ADR-0011)
#[test]
fn db_range_is_the_configured_floor_and_ceiling() {
    let p = DetectionParams::default().spectrogram;
    let s = lp16k();
    assert_eq!(s.db_range, [p.db_floor, p.db_ceiling]);
}

// @RF-07 Scenario: Spectrogram shape
#[test]
fn rows_above_16_2_khz_are_near_the_bottom_of_the_range() {
    let s = lp16k();
    let h = s.height as usize;
    let hz_of_row = |f: usize| f as f64 * s.max_frequency_hz / f64::from(s.height);
    let stop: Vec<usize> = (0..h).filter(|f| hz_of_row(*f) > 16_200.0).collect();
    assert!(stop.len() > 100, "rows above 16.2 kHz: {}", stop.len());
    for f in stop {
        let mean = row_mean(&s, f);
        assert!(
            mean <= 63.0,
            "row {f} ({:.0} Hz) has mean byte {mean:.1}, want <= 63",
            hz_of_row(f)
        );
    }
}

// @RF-07 Scenario: Spectrogram shape (the passband is not blank: the image carries the signal)
#[test]
fn rows_below_15_khz_are_bright() {
    let s = lp16k();
    let h = s.height as usize;
    let hz_of_row = |f: usize| f as f64 * s.max_frequency_hz / f64::from(s.height);
    // Skip the lowest rows (DC region of the shaped noise).
    let pass: Vec<usize> = (0..h)
        .filter(|f| hz_of_row(*f) > 500.0 && hz_of_row(*f) < 15_000.0)
        .collect();
    assert!(pass.len() > 100);
    for f in pass {
        let mean = row_mean(&s, f);
        assert!(
            mean >= 128.0,
            "row {f} has mean byte {mean:.1}, want >= 128"
        );
    }
}

// Row-major by time (ADR-0009): the frequency axis of one time column is contiguous, 0 Hz first.
#[test]
fn a_time_column_runs_from_low_to_high_frequency() {
    let s = lp16k();
    let h = s.height as usize;
    let t = s.width as usize / 2;
    let col = &s.data[t * h..(t + 1) * h];
    let low = col[10..40].iter().map(|b| u32::from(*b)).sum::<u32>() / 30;
    let high = col[h - 30..].iter().map(|b| u32::from(*b)).sum::<u32>() / 30;
    assert!(low > high + 100, "low rows {low}, high rows {high}");
}

// Contract: `INTERNAL` if the file cannot be decoded (the adapter reports a TrackError).
#[test]
fn a_missing_file_is_an_unreadable_error() {
    let missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no_such_dir/missing.flac");
    let err = SymphoniaAnalyzer
        .spectrogram(&missing, &DetectionParams::default().spectrogram)
        .expect_err("a missing file has no spectrogram");
    assert!(matches!(err, TrackError::Unreadable(_)), "{err:?}");
}

#[test]
fn a_garbage_file_is_an_error_not_a_panic() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("spectrogram_garbage");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("garbage.flac");
    std::fs::write(&path, vec![0x5Au8; 4096]).unwrap();
    let result = SymphoniaAnalyzer.spectrogram(&path, &DetectionParams::default().spectrogram);
    assert!(result.is_err(), "{result:?}");
}

fn assert_lp16k_like(s: &Spectrogram) {
    let h = s.height as usize;
    assert_eq!(s.height, 512);
    assert!(s.width > 0 && s.width <= 1200, "width {}", s.width);
    assert_eq!(s.data.len(), s.width as usize * h);
    let hz_of_row = |f: usize| f as f64 * s.max_frequency_hz / f64::from(s.height);
    for f in (0..h).filter(|f| hz_of_row(*f) > 16_200.0) {
        let mean = row_mean(s, f);
        assert!(mean <= 63.0, "row {f} mean {mean:.1}");
    }
    for f in (0..h).filter(|f| hz_of_row(*f) > 500.0 && hz_of_row(*f) < 15_000.0) {
        let mean = row_mean(s, f);
        assert!(mean >= 128.0, "row {f} mean {mean:.1}");
    }
}

// @RF-07 Scenario: Spectrogram shape (stream that does not declare its length: two passes)
#[test]
fn unknown_length_stream_gives_the_same_shape_and_stop_band() {
    let path = fixture("lp16k_unknown_length", Container::Flac);
    let s = SymphoniaAnalyzer
        .spectrogram(&path, &DetectionParams::default().spectrogram)
        .expect("spectrogram of lp16k_unknown_length");
    assert_lp16k_like(&s);
}

// A file shorter than one FFT frame: one column, `height` rows, no panic.
#[test]
fn a_file_shorter_than_one_frame_has_one_column() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("spectrogram_tiny");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tiny.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(&path, spec).unwrap();
    for i in 0..1000 {
        w.write_sample(((i as f64 * 0.3).sin() * 8000.0) as i16)
            .unwrap();
    }
    w.finalize().unwrap();
    let s = SymphoniaAnalyzer
        .spectrogram(&path, &DetectionParams::default().spectrogram)
        .expect("spectrogram of a tiny file");
    assert_eq!(s.width, 1);
    assert_eq!(s.height, 512);
    assert_eq!(s.data.len(), 512);
}

// A container that understates its length (STREAMINFO total_samples smaller than the audio):
// the result stays well-formed and bounded.
#[test]
fn an_understated_length_still_gives_a_bounded_spectrogram() {
    let full = fixture("lp16k", Container::Flac);
    let mut bytes = std::fs::read(full).unwrap();
    // STREAMINFO data starts at byte 8; the 36-bit total is bit 108: low nibble of byte 13, then 4 bytes.
    let at = 8 + 13;
    let total: u64 = 100_000;
    bytes[at] = (bytes[at] & 0xF0) | ((total >> 32) & 0x0F) as u8;
    for i in 0..4 {
        bytes[at + 1 + i] = (total >> (24 - 8 * i)) as u8;
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("spectrogram_understated");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("understated.flac");
    std::fs::write(&path, &bytes).unwrap();
    let s = SymphoniaAnalyzer
        .spectrogram(&path, &DetectionParams::default().spectrogram)
        .expect("spectrogram of an understated stream");
    assert_eq!(s.height, 512);
    assert!(s.width > 0 && s.width <= 1200, "width {}", s.width);
    assert_eq!(s.data.len(), s.width as usize * 512);
}

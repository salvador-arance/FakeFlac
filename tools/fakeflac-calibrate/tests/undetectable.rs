//! The unchanged undetectable rule, written down by calibration.md OQ-30 and evaluation.md
//! section 2. All spectra are synthetic 100 Hz bands; no audio, codec or filesystem is needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::cache::{Extracted, ProbeData, StreamData};
use fakeflac_calibrate::extract::undetectable_criterion;
use fakeflac_calibrate::Config;
use fakeflac_core::detect::measure;
use fakeflac_core::{BandSpectra, DetectionParams};

fn spectrum(edge_hz: Option<u32>) -> BandSpectra {
    let mean_db: Vec<_> = (0..221)
        .map(|b| {
            if edge_hz.is_none_or(|edge| b * 100 < edge) {
                -40.0
            } else {
                -130.0
            }
        })
        .collect();
    BandSpectra {
        sample_rate: 44100,
        band_hz: 100.0,
        median_db: mean_db.clone(),
        mean_db,
    }
}

fn extracted(spectrum: BandSpectra) -> Extracted {
    Extracted::Probe(ProbeData {
        sample_rate: 44100,
        bit_depth: Some(16),
        channels: 2,
        duration_ms: 10000,
        spectrum: Some(spectrum),
        stream: StreamData::default(),
    })
}

fn check_rule(edge: Option<u32>, lowpass: Option<u32>, drop: f64, expected: Option<&str>) {
    let params = DetectionParams::default();
    let config = Config::parse(&read(&spec_config())).unwrap();
    assert_eq!(config.generate.min_removed_hf_db, Some(12.0));
    let source = spectrum(edge);
    let fe = measure(&source, &params).extent_hz;
    match edge {
        Some(15000) => assert!(fe <= 16000.0, "source fe {fe}"),
        Some(18000) => assert!(fe > 16000.0 && fe < 22050.0, "source fe {fe}"),
        None => assert_eq!(fe, 22050.0),
        _ => unreachable!(),
    }
    let mut transcode = source.clone();
    for b in 160..transcode.mean_db.len() {
        transcode.mean_db[b] -= drop;
        transcode.median_db[b] -= drop;
    }
    let mut setting = config.generate.settings[0].clone();
    setting.lowpass_hz = lowpass;
    let actual = undetectable_criterion(
        &extracted(source),
        &extracted(transcode),
        &setting,
        &params,
        &config,
    );
    assert_eq!(
        actual, expected,
        "source edge {edge:?}, lowpass {lowpass:?}, drop {drop}"
    );
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 1.
#[test]
fn rp11_no_setting_lowpass_means_detectable() {
    check_rule(Some(15000), None, 0.0, None);
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 2.
#[test]
fn rp11_setting_lowpass_at_nyquist_means_detectable() {
    check_rule(Some(15000), Some(22050), 0.0, None);
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 3.
#[test]
fn rp11_extent_criterion_precedes_hf_drop_when_both_hold() {
    check_rule(Some(15000), Some(16000), 0.0, Some("extent"));
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 4.
#[test]
fn rp11_hf_drop_below_the_minimum_is_undetectable() {
    check_rule(Some(18000), Some(16000), 11.9, Some("hf drop"));
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 5.
#[test]
fn rp11_hf_drop_at_the_minimum_is_detectable() {
    check_rule(Some(18000), Some(16000), 12.0, None);
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 6.
#[test]
fn rp11_infinite_hf_drop_is_detectable() {
    check_rule(Some(18000), Some(16000), f64::INFINITY, None);
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 7.
#[test]
fn rp11_full_band_source_stays_detectable_with_no_drop() {
    check_rule(None, Some(16000), 0.0, None);
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), row 8.
#[test]
fn rp11_full_band_source_stays_detectable_with_a_small_drop() {
    check_rule(None, Some(16000), 11.9, None);
}

// Scenario Outline: The undetectable rule as the tool applies it (@RP-11), evaluation.md section 2:
// NaN drops are ignored, but a set with no finite largest drop does not match criterion 2.
#[test]
fn rp11_hf_drop_ignores_nan_but_needs_a_finite_maximum() {
    let params = DetectionParams::default();
    let config = Config::parse(&read(&spec_config())).unwrap();
    let mut setting = config.generate.settings[0].clone();
    setting.lowpass_hz = Some(16000);
    let mut source = spectrum(Some(18000));
    let mut transcode = source.clone();
    source.mean_db[200] = f64::NEG_INFINITY;
    transcode.mean_db[200] = f64::NEG_INFINITY;
    assert_eq!(
        undetectable_criterion(
            &extracted(source.clone()),
            &extracted(transcode.clone()),
            &setting,
            &params,
            &config
        ),
        Some("hf drop")
    );
    // Keep the source content up to 18 kHz but give the transcode no overlapping HF bands.
    transcode.mean_db.truncate(160);
    transcode.median_db.truncate(160);
    assert_eq!(
        undetectable_criterion(
            &extracted(source.clone()),
            &extracted(transcode),
            &setting,
            &params,
            &config
        ),
        None
    );
    let mut no_threshold = config.clone();
    no_threshold.generate.min_removed_hf_db = None;
    assert_eq!(
        undetectable_criterion(
            &extracted(source.clone()),
            &extracted(source),
            &setting,
            &params,
            &no_threshold
        ),
        None
    );
}

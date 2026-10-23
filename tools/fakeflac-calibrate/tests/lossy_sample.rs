//! Checks of the test support of the lossy variant of the sample corpus (@RP-11 @RF-15,
//! `specs/calibration/lossy.md` section 9, "What the test stage confirms first"): the reported
//! cutoffs of the six sources and the verdicts of the lossy fixtures that the scenarios rely on, with
//! `detection-params.v5.toml`. These tests exercise only `fakeflac-audio` and `fakeflac-core`, so they
//! pass before the tool changes. Protected after the `test(RF-15)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::detect::{detect_track, expected_cutoff_hz, StreamInfo, TrackDetection};
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{AudioCodec, DetectionParams, Verdict};
use std::path::Path;

fn v5() -> DetectionParams {
    toml::from_str(&read(&v5_params())).unwrap()
}

fn detect(path: &Path, codec: Option<AudioCodec>) -> (StreamInfo, TrackDetection) {
    let params = v5();
    let probe = SymphoniaAnalyzer.analyze(path, &params).unwrap();
    let stream = StreamInfo {
        codec: probe.codec.or(codec).unwrap(),
        sample_rate: probe.sample_rate,
        declared_bitrate_kbps: probe.declared_bitrate_kbps,
        encoder_lowpass_hz: probe.encoder_lowpass_hz,
    };
    let detection = detect_track(probe.spectrum.as_ref(), &stream, &params);
    (stream, detection)
}

fn cutoff(detection: &TrackDetection) -> f64 {
    f64::from(
        detection
            .cutoff
            .as_ref()
            .expect("a reported cutoff")
            .frequency_hz,
    )
}

// Scenario: Tracks of the sample corpus feed the lossy entries (@RP-11 @RF-15), lossy.md section 9
#[test]
fn rf15_sample_sources_report_the_cutoffs_the_expected_counts_rest_on() {
    let source_limited =
        |expected_hz: f64| expected_hz - v5().lossy.unwrap().suspicious_shortfall_hz;
    let (l128, _) = detect(&lossy_fixture("mp3_128_lp17k"), None);
    let (l320, _) = detect(&lossy_fixture("mp3_320_lp20k5"), None);
    let t128 = source_limited(expected_cutoff_hz(&l128, &v5()).unwrap());
    let t320 = source_limited(expected_cutoff_hz(&l320, &v5()).unwrap());
    assert_eq!(t128, 16_000.0, "source-limited threshold of l128");
    assert_eq!(t320, 19_500.0, "source-limited threshold of l320");

    let (_, low_hf) = detect(&fixture("low_hf", Container::Flac), Some(AudioCodec::Flac));
    assert!(
        cutoff(&low_hf) < t128,
        "low_hf reports {} Hz",
        cutoff(&low_hf)
    );
    for (name, container) in [
        ("full_band", Container::Flac),
        ("full_band", Container::Wav),
        ("hf_bursts", Container::Flac),
    ] {
        let codec = if container == Container::Wav {
            AudioCodec::Pcm
        } else {
            AudioCodec::Flac
        };
        let (_, d) = detect(&fixture(name, container), Some(codec));
        assert!(
            cutoff(&d) > t320,
            "{name} reports {} Hz, expected above {t320}",
            cutoff(&d)
        );
    }
}

// Scenario: Lossy rows are labelled (@RP-11 @RF-15): the verdicts of the lossy files 12 and 13 and of
// the stage fixtures, lossy-fixtures.md section 2
#[test]
fn rf15_lossy_fixtures_have_the_v5_verdicts() {
    for (name, verdict, estimate, codec) in [
        ("mp3_128_lp17k", Verdict::Real, Some(128), AudioCodec::Mp3),
        ("mp3_320_lp20k5", Verdict::Real, Some(320), AudioCodec::Mp3),
        ("mp3_320_lp17k", Verdict::Fake, Some(128), AudioCodec::Mp3),
        (
            "m4a_aac_128_lp16k",
            Verdict::Real,
            Some(128),
            AudioCodec::Aac,
        ),
    ] {
        let (stream, d) = detect(&lossy_fixture(name), None);
        assert_eq!(stream.codec, codec, "{name}");
        assert_eq!(d.verdict, verdict, "{name}: {d:#?}");
        assert_eq!(d.estimated_source_bitrate_kbps, estimate, "{name}");
    }
    let (stream, d) = detect(&lossy_fixture("m4a_alac_full_band"), None);
    assert_eq!(stream.codec, AudioCodec::Alac);
    assert_eq!(d.verdict, Verdict::Real);
}

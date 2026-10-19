//! Tests derived from `specs/features/decode-lossy.feature` (@RF-14, @RF-07, @RNF-04): MP3 and M4A
//! (AAC-LC, ALAC) decoded with symphonia, stream information, errors and the spectrogram.
//! Spec: `specs/detection/lossy-source.md` section 2, contract version 3
//! (`specs/contracts/commands.md`, "Lossy formats"), fixtures: `specs/detection/lossy-fixtures.md`
//! (bitstream writers in `tests/common/lossy.rs`, checked by `lossy_fixtures.rs`).
//!
//! Scope of this file (the RF-14 run): decoding. The scenarios whose assertions need detection v5
//! (verdicts of lossy fixtures, estimated source bitrate, table cells) belong to RF-15. The
//! assertions about "no estimated source bitrate" hold trivially until RF-15 fills the field.
//! Names chosen by the test author (not pinned by the spec):
//!   `AudioProbe { codec: Option<AudioCodec>, declared_bitrate_kbps: Option<u32>,
//!                 bitrate_mode: Option<BitrateMode>, encoder: Option<String>,
//!                 encoder_lowpass_hz: Option<u32> }` and `TrackInfo { codec, bitrate_mode }`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{entry, fixture, lossy_fixture, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::ports::{FileEntry, SpectrumAnalyzer};
use fakeflac_core::{
    AudioCodec, AudioFormat, BitrateMode, DetectionParams, TrackAnalysis, TrackError, Verdict,
};
use std::path::Path;

/// The `FileEntry` the scanner would produce for a lossy fixture (the format comes from the extension).
fn lossy_entry(path: &Path) -> FileEntry {
    let meta = std::fs::metadata(path).unwrap();
    let format = match path.extension().unwrap().to_str().unwrap() {
        "mp3" => AudioFormat::Mp3,
        "m4a" => AudioFormat::M4a,
        other => panic!("not a lossy fixture: {other}"),
    };
    FileEntry {
        path: path.to_path_buf(),
        folder: path.parent().unwrap().to_path_buf(),
        format,
        size_bytes: meta.len(),
        modified_unix: meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    }
}

/// "When it is analysed" with the default parameters.
fn analyze_lossy(name: &str) -> TrackAnalysis {
    let path = lossy_fixture(name);
    analyze_track(
        &SymphoniaAnalyzer,
        &lossy_entry(&path),
        &DetectionParams::default(),
    )
}

/// "Its duration is 30 s within one frame" of `frame_samples` samples.
fn assert_30_s_within_one_frame(a: &TrackAnalysis, frame_samples: u64) {
    let frame_ms = frame_samples * 1000 / u64::from(a.info.sample_rate) + 1;
    assert!(
        a.info.duration_ms.abs_diff(30_000) <= frame_ms,
        "duration {} ms, want 30 s within {frame_ms} ms",
        a.info.duration_ms
    );
}

// ---- Stream information ---------------------------------------------------------------------------------

/// Scenario Outline: Stream information of lossy fixtures (@RF-14)
fn stream_information(fixture: &str, codec: AudioCodec, rate: u32, kbps: u32, mode: BitrateMode) {
    let a = analyze_lossy(fixture);
    assert_ne!(a.verdict, Verdict::Error, "{fixture}: {a:#?}");
    assert_eq!(a.info.format, AudioFormat::Mp3, "{fixture}");
    assert_eq!(a.info.codec, Some(codec), "{fixture}");
    assert_eq!(a.info.sample_rate, rate, "{fixture}");
    assert_eq!(a.info.channels, 1, "{fixture}");
    assert_eq!(a.info.bit_depth, None, "{fixture}");
    assert_eq!(a.info.declared_bitrate_kbps, Some(kbps), "{fixture}");
    assert_eq!(a.info.bitrate_mode, Some(mode), "{fixture}");
    assert_30_s_within_one_frame(&a, 1152);
}

#[test]
fn rf14_stream_information_mp3_320_lp20k5() {
    stream_information(
        "mp3_320_lp20k5",
        AudioCodec::Mp3,
        44_100,
        320,
        BitrateMode::Cbr,
    );
}
#[test]
fn rf14_stream_information_mp3_128_lp17k() {
    stream_information(
        "mp3_128_lp17k",
        AudioCodec::Mp3,
        44_100,
        128,
        BitrateMode::Cbr,
    );
}
#[test]
fn rf14_stream_information_mp3_192_lp17k_notag() {
    stream_information(
        "mp3_192_lp17k_notag",
        AudioCodec::Mp3,
        44_100,
        192,
        BitrateMode::Cbr,
    );
}
#[test]
fn rf14_stream_information_mp3_vbr_lp19k5() {
    stream_information(
        "mp3_vbr_lp19k5",
        AudioCodec::Mp3,
        44_100,
        240,
        BitrateMode::Vbr,
    );
}
#[test]
fn rf14_stream_information_mp3_abr_lp18k6() {
    stream_information(
        "mp3_abr_lp18k6",
        AudioCodec::Mp3,
        44_100,
        192,
        BitrateMode::Abr,
    );
}
#[test]
fn rf14_stream_information_mp3_320_48k_lp20k() {
    stream_information(
        "mp3_320_48k_lp20k",
        AudioCodec::Mp3,
        48_000,
        320,
        BitrateMode::Cbr,
    );
}

// Scenario: Stream information of an AAC file in M4A (@RF-14)
#[test]
fn rf14_stream_information_of_an_aac_file_in_m4a() {
    let a = analyze_lossy("m4a_aac_128_lp16k");
    assert_ne!(a.verdict, Verdict::Error, "{a:#?}");
    assert_eq!(a.info.format, AudioFormat::M4a);
    assert_eq!(a.info.codec, Some(AudioCodec::Aac));
    assert_eq!(a.info.sample_rate, 44_100);
    assert_eq!(a.info.channels, 1);
    assert_eq!(a.info.bit_depth, None);
    assert_eq!(a.info.declared_bitrate_kbps, Some(128), "exactly 128 kbps");
    assert_eq!(
        a.info.bitrate_mode, None,
        "AAC does not signal its bitrate mode"
    );
}

// Scenario Outline: ALAC in M4A is lossless and judged by the lossless rules (@RF-14)
fn alac_is_judged_by_the_lossless_rules(fixture_name: &str, signal: &str, verdict: Verdict) {
    let a = analyze_lossy(fixture_name);
    assert_eq!(a.info.format, AudioFormat::M4a, "{fixture_name}");
    assert_eq!(a.info.codec, Some(AudioCodec::Alac), "{fixture_name}");
    assert_eq!(a.info.bit_depth, Some(16), "{fixture_name}");
    assert_eq!(a.info.declared_bitrate_kbps, None, "{fixture_name}");
    assert_eq!(a.info.bitrate_mode, None, "{fixture_name}");
    // The verdict, reasons and cutoff that rules.md gives the lossless signal saved as FLAC.
    let flac_path = fixture(signal, Container::Flac);
    let flac = analyze_track(
        &SymphoniaAnalyzer,
        &entry(&flac_path),
        &DetectionParams::default(),
    );
    assert_eq!(flac.verdict, verdict, "the FLAC of {signal}: {flac:#?}");
    assert_eq!(a.verdict, flac.verdict, "{fixture_name}: {a:#?}");
    assert_eq!(a.reasons, flac.reasons, "{fixture_name}");
    assert_eq!(
        a.cutoff.as_ref().map(|c| c.frequency_hz),
        flac.cutoff.as_ref().map(|c| c.frequency_hz),
        "{fixture_name}: cutoff"
    );
    assert_eq!(a.estimated_source_bitrate_kbps, None, "{fixture_name}");
}

#[test]
fn rf14_alac_full_band_is_real() {
    alac_is_judged_by_the_lossless_rules("m4a_alac_full_band", "full_band", Verdict::Real);
}
#[test]
fn rf14_alac_lp16k_is_fake() {
    alac_is_judged_by_the_lossless_rules("m4a_alac_lp16k", "lp16k", Verdict::Fake);
}

// Scenario: The LAME tag is read from the tag frame (@RF-14)
#[test]
fn rf14_the_lame_tag_is_read_from_the_tag_frame() {
    let path = lossy_fixture("mp3_320_lp17k");
    let probe = SymphoniaAnalyzer
        .analyze(&path, &DetectionParams::default())
        .unwrap();
    assert_eq!(probe.encoder.as_deref(), Some("LAME3.100"));
    assert_eq!(probe.encoder_lowpass_hz, Some(20_500));
    // 1148 x 1152 / 44100 s = 29989 ms within 1 ms: the 1148 audio frames, the tag frame excluded
    // and the encoder delay not subtracted.
    assert!(
        probe.duration_ms.abs_diff(29_989) <= 1,
        "duration {} ms, want 29989 ms within 1 ms",
        probe.duration_ms
    );
}

// Scenario: A zero low-pass byte means no low-pass is declared (@RF-14)
#[test]
fn rf14_a_zero_low_pass_byte_means_no_low_pass_is_declared() {
    let path = lossy_fixture("mp3_abr_lp18k6");
    let probe = SymphoniaAnalyzer
        .analyze(&path, &DetectionParams::default())
        .unwrap();
    // The tag is read (its encoder is there), and its low-pass byte of 0 declares nothing.
    assert_eq!(probe.encoder.as_deref(), Some("LAME3.100"));
    assert_eq!(probe.encoder_lowpass_hz, None);
}

// Scenario: WAV and FLAC report their codec and no bitrate (@RF-14)
#[test]
fn rf14_wav_and_flac_report_their_codec_and_no_bitrate() {
    let params = DetectionParams::default();
    for (container, codec) in [
        (Container::Flac, AudioCodec::Flac),
        (Container::Wav, AudioCodec::Pcm),
    ] {
        let path = fixture("full_band", container);
        let a = analyze_track(&SymphoniaAnalyzer, &entry(&path), &params);
        assert_eq!(a.info.codec, Some(codec), "{container:?}");
        assert_eq!(a.info.declared_bitrate_kbps, None, "{container:?}");
        assert_eq!(a.info.bitrate_mode, None, "{container:?}");
        assert_eq!(a.estimated_source_bitrate_kbps, None, "{container:?}");
    }
}

// ---- Spectrogram -----------------------------------------------------------------------------------------

// Scenario: The spectrogram of an MP3 file (@RF-14 @RF-07)
#[test]
fn rf14_the_spectrogram_of_an_mp3_file() {
    let path = lossy_fixture("mp3_320_lp17k");
    let s = SymphoniaAnalyzer
        .spectrogram(&path, &DetectionParams::default().spectrogram)
        .expect("spectrogram of an MP3 file");
    assert_eq!(s.height, 512);
    assert!(s.width > 0 && s.width <= 1200, "width {}", s.width);
    assert_eq!(s.max_frequency_hz, 22_050.0);
    assert_eq!(s.data.len(), s.width as usize * s.height as usize);

    // "The rows above 17.2 kHz are near the bottom of dbRange": the mean byte of every such row is
    // at most 63 (within 30 dB of the floor for the default range), as in `spectrogram.rs`.
    let h = s.height as usize;
    let hz_of_row = |f: usize| f as f64 * s.max_frequency_hz / f64::from(s.height);
    let row_mean = |f: usize| {
        (0..s.width as usize)
            .map(|t| f64::from(s.data[t * h + f]))
            .sum::<f64>()
            / f64::from(s.width)
    };
    let stop: Vec<usize> = (0..h).filter(|f| hz_of_row(*f) > 17_200.0).collect();
    assert!(stop.len() > 100, "rows above 17.2 kHz: {}", stop.len());
    for f in stop {
        let mean = row_mean(f);
        assert!(
            mean <= 63.0,
            "row {f} ({:.0} Hz) has mean byte {mean:.1}, want <= 63",
            hz_of_row(f)
        );
    }
    // The image carries the signal: the passband is bright.
    let pass: Vec<usize> = (0..h)
        .filter(|f| hz_of_row(*f) > 500.0 && hz_of_row(*f) < 16_000.0)
        .collect();
    for f in pass {
        let mean = row_mean(f);
        assert!(
            mean >= 128.0,
            "row {f} has mean byte {mean:.1}, want >= 128"
        );
    }
}

// ---- Errors (RNF-04) ------------------------------------------------------------------------------------

/// Scenario Outline: Broken or unsupported lossy files become error tracks (@RF-14 @RNF-04)
fn broken_file_is_an_error_track(fixture: &str, matches_error: impl Fn(&TrackError) -> bool) {
    let a = analyze_lossy(fixture);
    assert_eq!(a.verdict, Verdict::Error, "{fixture}: {a:#?}");
    let error = a.error.as_ref().expect("an error track has its error");
    assert!(matches_error(error), "{fixture}: {error:?}");
    assert_eq!(a.cutoff, None, "{fixture}: no cutoff");
    assert_eq!(a.estimated_source_bitrate_kbps, None, "{fixture}");
}

#[test]
fn rf14_mp3_corrupt_is_a_corrupt_stream() {
    broken_file_is_an_error_track("mp3_corrupt", |e| matches!(e, TrackError::CorruptStream(_)));
}
#[test]
fn rf14_m4a_corrupt_is_a_corrupt_stream() {
    broken_file_is_an_error_track("m4a_corrupt", |e| matches!(e, TrackError::CorruptStream(_)));
}
#[test]
fn rf14_m4a_he_aac_is_an_unsupported_format() {
    broken_file_is_an_error_track("m4a_he_aac", |e| {
        matches!(e, TrackError::UnsupportedFormat(_))
    });
}
#[test]
fn rf14_mp3_short_is_too_short() {
    broken_file_is_an_error_track("mp3_short", |e| matches!(e, TrackError::TooShort));
}

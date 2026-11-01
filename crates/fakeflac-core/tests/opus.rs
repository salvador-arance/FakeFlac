//! @RF-15 opus.feature: "Opus verdicts from given measurements" and defaults.
#![allow(clippy::unwrap_used)]
use fakeflac_core::detect::{decide_track, expected_cutoff_hz, Measurements, StreamInfo};
use fakeflac_core::{AudioCodec, AudioFormat, DetectionParams, SuspicionReason, Verdict};
use SuspicionReason::{CutoffBelowDeclared, GradualRolloff, LowHighFrequencyContent};
#[test]
fn rf15_opus_measurement_table_and_every_boundary() {
    let p = DetectionParams::default();
    let rows = [
        (
            128,
            -20.0,
            40.0,
            Some(19950.0),
            24000.0,
            Some(20000.0),
            Verdict::Real,
            None,
            Some(128),
        ),
        (
            160,
            -20.0,
            40.0,
            Some(20150.0),
            24000.0,
            Some(20000.0),
            Verdict::Real,
            None,
            Some(160),
        ),
        (
            256,
            -20.0,
            40.0,
            Some(16650.0),
            24000.0,
            Some(20000.0),
            Verdict::Fake,
            None,
            None,
        ),
        (
            160,
            -20.0,
            40.0,
            Some(17500.0),
            24000.0,
            Some(20000.0),
            Verdict::Fake,
            None,
            None,
        ),
        (
            160,
            -20.0,
            40.0,
            Some(17550.0),
            24000.0,
            Some(20000.0),
            Verdict::Suspicious,
            Some(CutoffBelowDeclared),
            None,
        ),
        (
            256,
            -20.0,
            40.0,
            Some(18750.0),
            24000.0,
            Some(20000.0),
            Verdict::Suspicious,
            Some(CutoffBelowDeclared),
            None,
        ),
        (
            128,
            -20.0,
            40.0,
            Some(19000.0),
            24000.0,
            Some(20000.0),
            Verdict::Suspicious,
            Some(CutoffBelowDeclared),
            None,
        ),
        (
            128,
            -20.0,
            40.0,
            Some(19050.0),
            24000.0,
            Some(20000.0),
            Verdict::Real,
            None,
            Some(128),
        ),
        (
            128,
            -20.0,
            8.0,
            Some(16050.0),
            24000.0,
            Some(20000.0),
            Verdict::Suspicious,
            Some(CutoffBelowDeclared),
            None,
        ),
        (
            128,
            -20.0,
            0.0,
            None,
            18500.0,
            Some(20000.0),
            Verdict::Suspicious,
            Some(GradualRolloff),
            None,
        ),
        (
            128,
            -70.0,
            40.0,
            Some(12050.0),
            24000.0,
            Some(20000.0),
            Verdict::Suspicious,
            Some(LowHighFrequencyContent),
            None,
        ),
        (
            48,
            -20.0,
            40.0,
            Some(19950.0),
            24000.0,
            Some(20000.0),
            Verdict::Real,
            None,
            Some(48),
        ),
        (
            40,
            -20.0,
            40.0,
            Some(16650.0),
            24000.0,
            None,
            Verdict::Real,
            None,
            Some(40),
        ),
        (
            16,
            -20.0,
            40.0,
            Some(12050.0),
            24000.0,
            None,
            Verdict::Real,
            None,
            Some(16),
        ),
        (
            140,
            -20.0,
            40.0,
            Some(17050.0),
            24000.0,
            Some(20000.0),
            Verdict::Fake,
            None,
            None,
        ),
        (
            300,
            -20.0,
            40.0,
            Some(19950.0),
            24000.0,
            Some(20000.0),
            Verdict::Real,
            None,
            Some(300),
        ),
        (
            256,
            -20.0,
            40.0,
            Some(20050.0),
            24000.0,
            Some(20000.0),
            Verdict::Real,
            None,
            Some(256),
        ),
    ];
    for (b, h, d, fs, fe, c_exp, verdict, reason, estimate) in rows {
        let stream = StreamInfo {
            codec: AudioCodec::Opus,
            sample_rate: 48000,
            declared_bitrate_kbps: Some(b),
            encoder_lowpass_hz: None,
        };
        let m = Measurements {
            noise_floor_db: -80.0,
            high_frequency_db: h,
            extent_hz: fe,
            step_db: d,
            step_hz: fs,
            gap_db: 0.0,
            upper_level_db: None,
            upper_burst_db: None,
            median_step_db: None,
            top_floor_db: None,
            residual_db: None,
        };
        assert_eq!(
            expected_cutoff_hz(&stream, &p),
            c_exp,
            "bitrate {b}, fs {fs:?}"
        );
        let actual = decide_track(&stream, &m, &p);
        assert_eq!(actual.verdict, verdict, "bitrate {b}, fs {fs:?}");
        assert_eq!(actual.reasons, reason.into_iter().collect::<Vec<_>>());
        assert_eq!(actual.estimated_source_bitrate_kbps, estimate);
        assert_eq!(actual.error, None);
    }
}
/// @RF-15 Scenario: The defaults are the v8 file (pure-data portion).
#[test]
fn rf15_defaults_have_six_opus_rows() {
    let p = DetectionParams::default();
    assert_eq!(p.algorithm_version, 8);
    let rows = p.lossy.unwrap().cutoffs.opus.unwrap();
    assert_eq!(rows.len(), 6);
    assert_eq!(
        rows.iter().map(|r| r.bitrate_kbps).collect::<Vec<_>>(),
        [48, 64, 96, 128, 160, 192]
    );
    assert!(rows.iter().all(|r| r.cutoff_hz == 20000.0));
}
/// @RF-14 Scenario: Broken Ogg or Opus structure is corruptStream (codec from extension).
#[test]
fn rf14_opus_codec_is_available_before_probing() {
    assert_eq!(
        AudioCodec::from_format(AudioFormat::Opus),
        Some(AudioCodec::Opus)
    );
}

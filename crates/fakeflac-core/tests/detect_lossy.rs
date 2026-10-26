//! Rule-level tests of `specs/detection/lossy-source.md` (sections 3 to 5 and 6) derived from
//! `specs/features/detection-lossy.feature` (@RF-15): stream information and given measurements go
//! in, verdict, reasons and estimated source bitrate come out. No audio: hand-built measurements,
//! hand-built band spectra and a fake of the `SpectrumAnalyzer` port. DetectionParams v5 are the
//! defaults of the code without `[lossy.streaming]` (ADR-0041; since detection v7 the defaults add
//! that section, RF-16, and carry `algorithm_version` 7: lossy-source.md Q25 (j)). The core cannot
//! read the v5 file (no `toml` dependency), so `v5()` clears the section.
//!
//! Names chosen by the test author (the spec leaves them open, lossy-source.md section 3):
//!   `fakeflac_core::detect::StreamInfo { codec, sample_rate, declared_bitrate_kbps: Option<u32>,
//!                                        encoder_lowpass_hz: Option<u32> }`
//!   `fakeflac_core::detect::expected_cutoff_hz(&StreamInfo, &DetectionParams) -> Option<f64>`
//!   `fakeflac_core::detect::decide_track(&StreamInfo, &Measurements, &DetectionParams)
//!        -> TrackDecision { verdict, reasons, estimated_source_bitrate_kbps: Option<u32>,
//!                           error: Option<TrackError> }`
//!   `fakeflac_core::params::{LossyParams, LossyCutoffs, CutoffRow}`, `DetectionParams.lossy:
//!   Option<LossyParams>`, `SuspicionReason::CutoffBelowDeclared`.
//! The lossy `Fake` carries no reason; `decide_track` of a lossless codec equals `decide`.

#![allow(clippy::expect_used)]

use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{
    decide, decide_track, expected_cutoff_hz, measure, Measurements, StreamInfo,
};
use fakeflac_core::ports::{AudioProbe, FileEntry, SpectrumAnalyzer};
use fakeflac_core::{
    AudioCodec, AudioFormat, BandSpectra, BitrateMode, DetectionParams, SuspicionReason,
    TrackError, Verdict,
};
use std::path::{Path, PathBuf};
use SuspicionReason::{CutoffBelowDeclared, GradualRolloff, LowHighFrequencyContent};

fn v5() -> DetectionParams {
    let mut p = DetectionParams::default();
    p.lossy
        .as_mut()
        .expect("the defaults have a [lossy] section")
        .streaming = None;
    p
}

/// The measurements named by the feature: H, G, D, fs and fe. `F`, `U` and `B` are not used by the
/// lossy rules (Q7) and are set to arbitrary values.
fn measurements(h: f64, g: f64, d: f64, fs: Option<f64>, fe: f64) -> Measurements {
    Measurements {
        noise_floor_db: -80.0,
        high_frequency_db: h,
        extent_hz: fe,
        step_db: d,
        step_hz: fs,
        gap_db: g,
        upper_level_db: Some(-40.0),
        upper_burst_db: Some(8.0),
        median_step_db: None,
        top_floor_db: None,
        residual_db: None,
    }
}

fn stream(codec: AudioCodec, rate: u32, b: Option<u32>, tag: Option<u32>) -> StreamInfo {
    StreamInfo {
        codec,
        sample_rate: rate,
        declared_bitrate_kbps: b,
        encoder_lowpass_hz: tag,
    }
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Lossy verdict from stream information and measurements (@RF-15)
// ---------------------------------------------------------------------------------------------

struct Row {
    case: &'static str,
    codec: AudioCodec,
    rate: u32,
    b: u32,
    tag: Option<u32>,
    h: f64,
    g: f64,
    d: f64,
    fs: Option<f64>,
    fe: f64,
    c_exp: Option<f64>,
    verdict: Verdict,
    reasons: &'static [SuspicionReason],
    estimate: Option<u32>,
}

#[allow(clippy::too_many_arguments)]
fn row(
    case: &'static str,
    (codec, rate, b, tag): (AudioCodec, u32, u32, Option<u32>),
    (h, g, d, fs, fe): (f64, f64, f64, Option<f64>, f64),
    c_exp: Option<f64>,
    verdict: Verdict,
    reasons: &'static [SuspicionReason],
    estimate: Option<u32>,
) -> Row {
    Row {
        case,
        codec,
        rate,
        b,
        tag,
        h,
        g,
        d,
        fs,
        fe,
        c_exp,
        verdict,
        reasons,
        estimate,
    }
}

fn rows() -> Vec<Row> {
    use AudioCodec::{Aac, Mp3};
    use Verdict::{Fake, Real, Suspicious};
    let m = |rate, b, tag| (Mp3, rate, b, tag);
    vec![
        row(
            "honest 320",
            m(44_100, 320, None),
            (-20.0, 0.0, 40.0, Some(20_450.0), 22_050.0),
            Some(20_500.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "320 from 128",
            m(44_100, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(17_050.0), 22_050.0),
            Some(20_500.0),
            Fake,
            &[],
            Some(128),
        ),
        row(
            "192 from 128",
            m(44_100, 192, None),
            (-20.0, 0.0, 40.0, Some(17_050.0), 22_050.0),
            Some(18_600.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(128),
        ),
        row(
            "shortfall at fake_shortfall_hz",
            m(44_100, 320, None),
            (-20.0, 0.0, 10.0, Some(18_000.0), 22_050.0),
            Some(20_500.0),
            Fake,
            &[],
            Some(160),
        ),
        row(
            "same, step just below fake_min_step_db",
            m(44_100, 320, None),
            (-20.0, 0.0, 9.99, Some(18_000.0), 22_050.0),
            Some(20_500.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(160),
        ),
        row(
            "shortfall just below fake_shortfall_hz (tie)",
            m(44_100, 320, None),
            (-20.0, 0.0, 40.0, Some(18_050.0), 22_050.0),
            Some(20_500.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(160),
        ),
        row(
            "shortfall at suspicious_shortfall_hz",
            m(44_100, 320, None),
            (-20.0, 0.0, 40.0, Some(19_500.0), 22_050.0),
            Some(20_500.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(224),
        ),
        row(
            "shortfall just below suspicious_shortfall_hz",
            m(44_100, 320, None),
            (-20.0, 0.0, 40.0, Some(19_550.0), 22_050.0),
            Some(20_500.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "step at step_suspicious_db: not deep enough",
            m(44_100, 320, None),
            (-20.0, 0.0, 7.0, Some(17_050.0), 22_050.0),
            Some(20_500.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(128),
        ),
        row(
            "weak step ignored, content to Nyquist",
            m(44_100, 320, None),
            (-20.0, 0.0, 6.99, Some(17_050.0), 22_050.0),
            Some(20_500.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "no step, content fades early",
            m(44_100, 320, None),
            (-20.0, 0.0, 3.0, Some(18_050.0), 17_000.0),
            Some(20_500.0),
            Suspicious,
            &[GradualRolloff],
            Some(128),
        ),
        row(
            "no step, content ends below the first row",
            m(44_100, 320, None),
            (-20.0, 0.0, 0.0, None, 4_000.0),
            Some(20_500.0),
            Suspicious,
            &[GradualRolloff],
            Some(32),
        ),
        row(
            "little treble comes first",
            m(44_100, 320, None),
            (-65.0, 0.0, 40.0, Some(17_050.0), 22_050.0),
            Some(20_500.0),
            Suspicious,
            &[LowHighFrequencyContent],
            None,
        ),
        row(
            "gap step is not used",
            m(44_100, 320, None),
            (-20.0, 15.0, 40.0, Some(20_450.0), 22_050.0),
            Some(20_500.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "tag low-pass lowers c_exp",
            m(44_100, 320, Some(16_000)),
            (-20.0, 0.0, 40.0, Some(16_050.0), 22_050.0),
            Some(16_000.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "tag low-pass above the table is ignored",
            m(44_100, 128, Some(20_000)),
            (-20.0, 0.0, 40.0, Some(17_050.0), 22_050.0),
            Some(17_000.0),
            Real,
            &[],
            Some(128),
        ),
        row(
            "tag low-pass below the first row is ignored",
            m(44_100, 320, Some(100)),
            (-20.0, 0.0, 40.0, Some(17_050.0), 22_050.0),
            Some(20_500.0),
            Fake,
            &[],
            Some(128),
        ),
        row(
            "average between rows takes the lower row",
            m(44_100, 300, None),
            (-20.0, 0.0, 40.0, Some(19_750.0), 22_050.0),
            Some(19_700.0),
            Real,
            &[],
            Some(300),
        ),
        row(
            "average between rows, early edge",
            m(44_100, 300, None),
            (-20.0, 0.0, 40.0, Some(18_050.0), 22_050.0),
            Some(19_700.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(160),
        ),
        row(
            "declared below the table: nothing expected",
            m(22_050, 24, None),
            (-20.0, 0.0, 0.0, None, 8_000.0),
            None,
            Real,
            &[],
            Some(24),
        ),
        row(
            "32 kHz: c_exp capped at nyquist - 400",
            m(32_000, 320, None),
            (-20.0, 0.0, 0.0, None, 16_000.0),
            Some(15_600.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "32 kHz: early edge",
            m(32_000, 320, None),
            (-20.0, 0.0, 40.0, Some(13_050.0), 16_000.0),
            Some(15_600.0),
            Fake,
            &[],
            Some(80),
        ),
        row(
            "48 kHz MP3 from Opus (known limitation)",
            m(48_000, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
            Some(20_500.0),
            Real,
            &[],
            Some(320),
        ),
        row(
            "honest AAC 128",
            (Aac, 44_100, 128, None),
            (-20.0, 0.0, 40.0, Some(16_050.0), 22_050.0),
            Some(15_500.0),
            Real,
            &[],
            Some(128),
        ),
        row(
            "AAC 256 from 128",
            (Aac, 44_100, 256, None),
            (-20.0, 0.0, 40.0, Some(15_550.0), 22_050.0),
            Some(19_000.0),
            Fake,
            &[],
            Some(128),
        ),
        row(
            "AAC 192, moderate shortfall",
            (Aac, 44_100, 192, None),
            (-20.0, 0.0, 40.0, Some(15_850.0), 22_050.0),
            Some(17_000.0),
            Suspicious,
            &[CutoffBelowDeclared],
            Some(160),
        ),
    ]
}

// @RF-15 Scenario Outline: Lossy verdict from stream information and measurements: "the expected
// cutoff is <c_exp> Hz", "the verdict is", "the reasons are", "the estimated source bitrate is".
// One loop reports every failing row.
#[test]
fn rf15_lossy_verdict_from_stream_information_and_measurements() {
    let p = v5();
    let mut failures = Vec::new();
    for r in rows() {
        let s = stream(r.codec, r.rate, Some(r.b), r.tag);
        let m = measurements(r.h, r.g, r.d, r.fs, r.fe);
        let c_exp = expected_cutoff_hz(&s, &p);
        let d = decide_track(&s, &m, &p);
        if c_exp != r.c_exp
            || d.verdict != r.verdict
            || d.reasons != r.reasons
            || d.estimated_source_bitrate_kbps != r.estimate
            || d.error.is_some()
        {
            failures.push(format!(
                "{}: c_exp {c_exp:?} (want {:?}), {:?} {:?} estimate {:?} (want {:?} {:?} {:?}), error {:?}",
                r.case, r.c_exp, d.verdict, d.reasons, d.estimated_source_bitrate_kbps, r.verdict, r.reasons, r.estimate, d.error
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// The same rows one by one, so that a boundary row reads as a test of its own (the rows above are
// the scenario; these pin the rows whose boundary is the point). @RF-15
fn check_row(case: &str) {
    let p = v5();
    let r = rows()
        .into_iter()
        .find(|r| r.case == case)
        .expect("row exists");
    let s = stream(r.codec, r.rate, Some(r.b), r.tag);
    let d = decide_track(&s, &measurements(r.h, r.g, r.d, r.fs, r.fe), &p);
    assert_eq!(expected_cutoff_hz(&s, &p), r.c_exp, "{case}: c_exp");
    assert_eq!(d.verdict, r.verdict, "{case}");
    assert_eq!(d.reasons, r.reasons, "{case}");
    assert_eq!(d.estimated_source_bitrate_kbps, r.estimate, "{case}");
    assert_eq!(d.error, None, "{case}");
}

#[test]
fn rf15_l3_the_shortfall_and_step_limits_are_inclusive() {
    check_row("shortfall at fake_shortfall_hz");
}
#[test]
fn rf15_l3_a_step_just_below_fake_min_step_db_is_suspicious() {
    check_row("same, step just below fake_min_step_db");
}
#[test]
fn rf15_l3_a_shortfall_just_below_fake_shortfall_hz_is_suspicious_and_the_estimate_tie_goes_to_the_lower_row(
) {
    check_row("shortfall just below fake_shortfall_hz (tie)");
}
#[test]
fn rf15_l4_the_suspicious_shortfall_limit_is_inclusive() {
    check_row("shortfall at suspicious_shortfall_hz");
    check_row("shortfall just below suspicious_shortfall_hz");
}
#[test]
fn rf15_l4_a_step_at_step_suspicious_db_is_a_clear_step_that_is_not_deep_enough_for_fake() {
    check_row("step at step_suspicious_db: not deep enough");
    check_row("weak step ignored, content to Nyquist");
}
#[test]
fn rf15_l5_without_a_clear_step_a_shortfall_is_a_gradual_rolloff_never_fake() {
    check_row("no step, content fades early");
    check_row("no step, content ends below the first row");
}
#[test]
fn rf15_l2_little_treble_comes_first_and_has_no_estimate() {
    check_row("little treble comes first");
}
#[test]
fn rf15_gap_step_is_not_used_by_the_lossy_rules() {
    check_row("gap step is not used");
}
#[test]
fn rf15_the_tag_low_pass_only_lowers_the_expected_cutoff() {
    check_row("tag low-pass lowers c_exp");
    check_row("tag low-pass above the table is ignored");
    check_row("tag low-pass below the first row is ignored");
}
#[test]
fn rf15_an_average_between_rows_uses_the_lower_row() {
    check_row("average between rows takes the lower row");
    check_row("average between rows, early edge");
}
#[test]
fn rf15_a_declared_bitrate_below_the_table_expects_nothing() {
    check_row("declared below the table: nothing expected");
}
#[test]
fn rf15_the_expected_cutoff_is_capped_below_nyquist() {
    check_row("32 kHz: c_exp capped at nyquist - 400");
    check_row("32 kHz: early edge");
}

// @RF-15 Scenario Outline row "48 kHz MP3 from Opus (known limitation)": Real, a 500 Hz shortfall.
#[test]
fn rf15_known_limitation_a_48k_mp3_from_opus_is_real() {
    check_row("48 kHz MP3 from Opus (known limitation)");
}

// @RF-15 Scenario Outline rows for AAC
#[test]
fn rf15_aac_rows_use_the_aac_table() {
    check_row("honest AAC 128");
    check_row("AAC 256 from 128");
    check_row("AAC 192, moderate shortfall");
}

// ---------------------------------------------------------------------------------------------
// Scenario: The lossless rules never apply to a lossy codec (@RF-15)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_the_lossless_rules_never_apply_to_a_lossy_codec() {
    // G = 15 dB and D = 30 dB in the lossy range would be Fake for rules.md V3/V4; an honest MP3
    // has a low-pass by design.
    let p = v5();
    let s = stream(AudioCodec::Mp3, 44_100, Some(128), None);
    let m = measurements(-20.0, 15.0, 30.0, Some(17_050.0), 22_050.0);
    assert_eq!(
        decide(&m, &p).0,
        Verdict::Fake,
        "the lossless rules call this Fake"
    );
    let d = decide_track(&s, &m, &p);
    assert_eq!(d.verdict, Verdict::Real);
    assert_eq!(d.estimated_source_bitrate_kbps, Some(128));
}

// ---------------------------------------------------------------------------------------------
// Scenario: The lossy rules never apply to a lossless codec (@RF-15)
// ---------------------------------------------------------------------------------------------

/// 44.1 kHz band spectra of the lossless fixture `lp16k`: flat to 16 kHz, then -100 dB.
fn lp16k_spectra() -> BandSpectra {
    let level = |b: usize| if b < 160 { 0.0 } else { -100.0 };
    BandSpectra {
        sample_rate: 44_100,
        band_hz: 100.0,
        mean_db: (0..220).map(level).collect(),
        median_db: vec![0.0; 220],
    }
}

#[test]
fn rf15_the_lossy_rules_never_apply_to_a_lossless_codec() {
    let p = v5();
    let m = measure(&lp16k_spectra(), &p);
    let (verdict, reasons) = decide(&m, &p);
    assert_eq!(verdict, Verdict::Fake, "lp16k saved as FLAC is Fake");
    for codec in [AudioCodec::Alac, AudioCodec::Flac, AudioCodec::Pcm] {
        // A lossless codec declares no bitrate; even a tag low-pass must not matter.
        let s = stream(codec, 44_100, None, None);
        let d = decide_track(&s, &m, &p);
        assert_eq!(d.verdict, Verdict::Fake, "{codec:?}");
        assert_eq!(d.reasons, reasons, "{codec:?}");
        assert_eq!(d.estimated_source_bitrate_kbps, None, "{codec:?}");
        assert_eq!(d.error, None, "{codec:?}");
        assert_eq!(expected_cutoff_hz(&s, &p), None, "{codec:?}: no table");
    }
}

// ---------------------------------------------------------------------------------------------
// Scenario: A lossy codec without table rows cannot be judged (@RF-15), rule L0
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_a_lossy_codec_without_table_rows_cannot_be_judged() {
    let p = v5();
    let s = stream(AudioCodec::Opus, 48_000, Some(128), None);
    // L0 depends only on the codec and the params: not even little treble changes it.
    for h in [-20.0, -65.0] {
        let d = decide_track(
            &s,
            &measurements(h, 0.0, 40.0, Some(17_050.0), 24_000.0),
            &p,
        );
        assert_eq!(d.verdict, Verdict::Error, "H = {h}");
        assert_eq!(
            d.error,
            Some(TrackError::UnsupportedFormat(
                "no [lossy.cutoffs] rows for opus".into()
            )),
            "H = {h}"
        );
        assert!(d.reasons.is_empty());
        assert_eq!(d.estimated_source_bitrate_kbps, None);
    }
    assert_eq!(expected_cutoff_hz(&s, &p), None);
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Params without a [lossy] section do not judge lossy files (@RF-15), rule L0
// ---------------------------------------------------------------------------------------------

#[test]
fn rf15_params_without_a_lossy_section_do_not_judge_lossy_files() {
    let p = DetectionParams {
        algorithm_version: 4,
        lossy: None,
        ..v5()
    };
    for (codec, b) in [(AudioCodec::Mp3, 320), (AudioCodec::Aac, 128)] {
        let s = stream(codec, 44_100, Some(b), None);
        // Even with little treble: L0 comes before L2.
        for h in [-20.0, -65.0] {
            let d = decide_track(
                &s,
                &measurements(h, 0.0, 40.0, Some(17_050.0), 22_050.0),
                &p,
            );
            assert_eq!(d.verdict, Verdict::Error, "{codec:?} H = {h}");
            assert_eq!(
                d.error,
                Some(TrackError::UnsupportedFormat(
                    "these DetectionParams have no [lossy] section".into()
                )),
                "{codec:?} H = {h}"
            );
            assert_eq!(d.estimated_source_bitrate_kbps, None);
        }
    }
    // The lossless codecs are judged as before.
    let s = stream(AudioCodec::Flac, 44_100, None, None);
    let m = measurements(-20.0, 0.0, 0.0, None, 22_050.0);
    assert_eq!(decide_track(&s, &m, &p).verdict, Verdict::Real);
}

// ---------------------------------------------------------------------------------------------
// analyze_track with a fake analyzer: the lossy rules reach the TrackAnalysis (@RF-15)
// ---------------------------------------------------------------------------------------------

struct FakeProbe(Result<AudioProbe, TrackError>);

impl SpectrumAnalyzer for FakeProbe {
    fn analyze(&self, _path: &Path, _p: &DetectionParams) -> Result<AudioProbe, TrackError> {
        self.0.clone()
    }
}

fn mp3_entry() -> FileEntry {
    FileEntry {
        path: PathBuf::from("music").join("a.mp3"),
        folder: PathBuf::from("music"),
        format: AudioFormat::Mp3,
        size_bytes: 1000,
        modified_unix: 1_700_000_000,
    }
}

/// 44.1 kHz spectra with content up to `edge_hz` and nothing above (a clear step).
fn spectra_with_edge(edge_hz: f64) -> BandSpectra {
    let level = move |b: usize| {
        if (b as f64 + 0.5) * 100.0 < edge_hz {
            0.0
        } else {
            -100.0
        }
    };
    BandSpectra {
        sample_rate: 44_100,
        band_hz: 100.0,
        mean_db: (0..220).map(level).collect(),
        median_db: vec![0.0; 220],
    }
}

fn mp3_probe(b: u32, lowpass: Option<u32>, spectrum: Option<BandSpectra>) -> AudioProbe {
    AudioProbe {
        sample_rate: 44_100,
        bit_depth: None,
        channels: 1,
        duration_ms: 30_000,
        codec: Some(AudioCodec::Mp3),
        declared_bitrate_kbps: Some(b),
        bitrate_mode: Some(BitrateMode::Cbr),
        encoder: Some("LAME3.100".into()),
        encoder_lowpass_hz: lowpass,
        spectrum,
    }
}

#[test]
fn rf15_analyze_track_fills_the_estimate_and_the_cutoff_of_a_320_made_from_128() {
    let an = FakeProbe(Ok(mp3_probe(
        320,
        Some(20_500),
        Some(spectra_with_edge(17_000.0)),
    )));
    let a = analyze_track(&an, &mp3_entry(), &v5());
    assert_eq!(a.verdict, Verdict::Fake, "{a:#?}");
    assert!(a.reasons.is_empty());
    assert_eq!(a.estimated_source_bitrate_kbps, Some(128));
    assert_eq!(a.info.declared_bitrate_kbps, Some(320));
    assert_eq!(a.info.bitrate_mode, Some(BitrateMode::Cbr));
    assert_eq!(a.info.codec, Some(AudioCodec::Mp3));
    assert_eq!(a.algorithm_version, 7);
    assert_eq!(a.error, None);
    let c = a.cutoff.expect("a cutoff is reported");
    assert!(
        (c.frequency_hz - 17_000.0).abs() <= 200.0,
        "{}",
        c.frequency_hz
    );
}

#[test]
fn rf15_analyze_track_honours_the_tag_low_pass_of_the_probe() {
    // 320 kbps with --lowpass 16: content to 16 kHz is Real with the declared bitrate (Q6).
    let an = FakeProbe(Ok(mp3_probe(
        320,
        Some(16_000),
        Some(spectra_with_edge(16_000.0)),
    )));
    let a = analyze_track(&an, &mp3_entry(), &v5());
    assert_eq!(a.verdict, Verdict::Real, "{a:#?}");
    assert_eq!(a.estimated_source_bitrate_kbps, Some(320));
    // The same content without the tag low-pass is a 320 made from 128.
    let an = FakeProbe(Ok(mp3_probe(320, None, Some(spectra_with_edge(16_000.0)))));
    let a = analyze_track(&an, &mp3_entry(), &v5());
    assert_eq!(a.verdict, Verdict::Fake, "{a:#?}");
}

#[test]
fn rf15_analyze_track_of_a_silent_lossy_track_has_no_cutoff_and_no_estimate() {
    let an = FakeProbe(Ok(mp3_probe(320, None, None)));
    let a = analyze_track(&an, &mp3_entry(), &v5());
    assert_eq!(a.verdict, Verdict::Suspicious);
    assert_eq!(a.reasons, vec![LowHighFrequencyContent]);
    assert_eq!(a.cutoff, None);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert_eq!(a.algorithm_version, 7);
}

#[test]
fn rf15_analyze_track_with_little_treble_has_no_estimate() {
    // Content only to 6 kHz: H is far below -60 dB (mp3_320_low_hf).
    let an = FakeProbe(Ok(mp3_probe(
        320,
        Some(20_500),
        Some(spectra_with_edge(6_000.0)),
    )));
    let a = analyze_track(&an, &mp3_entry(), &v5());
    assert_eq!(a.verdict, Verdict::Suspicious, "{a:#?}");
    assert_eq!(a.reasons, vec![LowHighFrequencyContent]);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
}

#[test]
fn rf15_analyze_track_without_a_lossy_section_is_an_unsupported_format() {
    let p = DetectionParams {
        algorithm_version: 4,
        lossy: None,
        ..v5()
    };
    let an = FakeProbe(Ok(mp3_probe(
        320,
        Some(20_500),
        Some(spectra_with_edge(17_000.0)),
    )));
    let a = analyze_track(&an, &mp3_entry(), &p);
    assert_eq!(a.verdict, Verdict::Error, "{a:#?}");
    assert_eq!(
        a.error,
        Some(TrackError::UnsupportedFormat(
            "these DetectionParams have no [lossy] section".into()
        ))
    );
    assert_eq!(a.algorithm_version, 4);
    assert_eq!(a.cutoff, None);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert!(a.reasons.is_empty());
    // What was learnt before the error stays in the info.
    assert_eq!(a.info.declared_bitrate_kbps, Some(320));
}

#[test]
fn rf15_analyze_track_of_a_lossless_codec_has_no_estimate() {
    let mut probe = mp3_probe(320, None, Some(spectra_with_edge(16_000.0)));
    probe.codec = Some(AudioCodec::Alac);
    probe.declared_bitrate_kbps = None;
    probe.bitrate_mode = None;
    probe.bit_depth = Some(16);
    let entry = FileEntry {
        format: AudioFormat::M4a,
        path: PathBuf::from("music").join("a.m4a"),
        ..mp3_entry()
    };
    let a = analyze_track(&FakeProbe(Ok(probe)), &entry, &v5());
    assert_eq!(
        a.verdict,
        Verdict::Fake,
        "ALAC is judged by the lossless rules: {a:#?}"
    );
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert_eq!(a.info.codec, Some(AudioCodec::Alac));
}

// ---------------------------------------------------------------------------------------------
// Round 2: order of L0 and boundaries (@RF-15, specs/detection/lossy-source.md sections 3 and 4)
// ---------------------------------------------------------------------------------------------

fn no_lossy_section() -> DetectionParams {
    DetectionParams {
        algorithm_version: 4,
        lossy: None,
        ..v5()
    }
}

fn unsupported_no_lossy() -> Option<TrackError> {
    Some(TrackError::UnsupportedFormat(
        "these DetectionParams have no [lossy] section".into(),
    ))
}

// @RF-15 lossy-source.md section 4: L0 comes before L1 (too short).
#[test]
fn rf15_l0_comes_before_l1_a_too_short_lossy_track_without_lossy_section() {
    let mut probe = mp3_probe(320, Some(20_500), Some(spectra_with_edge(17_000.0)));
    probe.duration_ms = 1_000;
    let a = analyze_track(&FakeProbe(Ok(probe)), &mp3_entry(), &no_lossy_section());
    assert_eq!(a.verdict, Verdict::Error, "{a:#?}");
    assert_eq!(a.error, unsupported_no_lossy(), "{a:#?}");
}

// @RF-15 lossy-source.md section 4: L0 comes before the no-bitrate guard.
#[test]
fn rf15_l0_comes_before_the_no_bitrate_guard_without_lossy_section() {
    let mut probe = mp3_probe(320, Some(20_500), Some(spectra_with_edge(17_000.0)));
    probe.declared_bitrate_kbps = None;
    let a = analyze_track(&FakeProbe(Ok(probe)), &mp3_entry(), &no_lossy_section());
    assert_eq!(a.verdict, Verdict::Error, "{a:#?}");
    assert_eq!(a.error, unsupported_no_lossy(), "{a:#?}");
}

// @RF-15 lossy-source.md section 4: L0 comes before silence (line 128).
#[test]
fn rf15_l0_comes_before_silence_without_lossy_section() {
    let a = analyze_track(
        &FakeProbe(Ok(mp3_probe(320, None, None))),
        &mp3_entry(),
        &no_lossy_section(),
    );
    assert_eq!(a.verdict, Verdict::Error, "{a:#?}");
    assert_eq!(a.error, unsupported_no_lossy(), "{a:#?}");
    assert!(a.reasons.is_empty());
}

// @RF-15 lossy-source.md section 3: the tag low-pass is honoured when it equals the first row's
// cutoff (inclusive lower bound). MP3 first row: 32 kbps at 5500 Hz.
#[test]
fn rf15_the_tag_low_pass_equal_to_the_first_row_cutoff_is_honoured() {
    let p = v5();
    let first = p.lossy.as_ref().expect("v5 has [lossy]").cutoffs.mp3[0].cutoff_hz;
    let tag = Some(first as u32);
    let with = stream(AudioCodec::Mp3, 44_100, Some(320), tag);
    let without = stream(AudioCodec::Mp3, 44_100, Some(320), None);
    assert_eq!(expected_cutoff_hz(&with, &p), Some(first));
    assert_eq!(expected_cutoff_hz(&without, &p), Some(20_500.0));
    // Content to the tag's cutoff: Real with the tag, a 320 made from a low bitrate without it.
    let m = measurements(-20.0, 0.0, 40.0, Some(first + 50.0), 22_050.0);
    assert_eq!(decide_track(&with, &m, &p).verdict, Verdict::Real);
    assert_eq!(decide_track(&without, &m, &p).verdict, Verdict::Fake);
}

// @RF-15 lossy-source.md section 4, L2: strict `<`; H equal to the threshold does not fire.
#[test]
fn rf15_l2_h_equal_to_low_hf_threshold_db_does_not_fire() {
    let p = v5();
    let h = p.verdict.low_hf_threshold_db;
    let s = stream(AudioCodec::Mp3, 44_100, Some(320), None);
    let d = decide_track(
        &s,
        &measurements(h, 0.0, 40.0, Some(20_450.0), 22_050.0),
        &p,
    );
    assert_eq!(d.verdict, Verdict::Real, "{d:?}");
    assert_eq!(d.estimated_source_bitrate_kbps, Some(320));
}

// @RF-15 lossy-source.md section 4: D against step_suspicious_db is the literal `>=`, for fc and
// for L4. D just below: the step is not counted, fc = fe, and the reason is gradualRolloff.
#[test]
fn rf15_d_just_below_step_suspicious_db_is_a_gradual_rolloff_with_fc_equal_to_fe() {
    let p = v5();
    let d = p.verdict.step_suspicious_db - 1e-10;
    let s = stream(AudioCodec::Mp3, 44_100, Some(320), None);
    // c_exp 20500, fe 19000: shortfall 1500 (>= suspicious, < fake); a step is present at 17050.
    let r = decide_track(
        &s,
        &measurements(-20.0, 0.0, d, Some(17_050.0), 19_000.0),
        &p,
    );
    assert_eq!(r.verdict, Verdict::Suspicious, "{r:?}");
    assert_eq!(r.reasons, vec![GradualRolloff], "{r:?}");
}

// @RF-15 lossy-source.md section 4, L3: D just below fake_min_step_db is not Fake.
#[test]
fn rf15_l3_d_just_below_fake_min_step_db_is_not_fake() {
    let p = v5();
    let l = p.lossy.as_ref().expect("v5 has [lossy]");
    let d = l.fake_min_step_db - 1e-10;
    let s = stream(AudioCodec::Mp3, 44_100, Some(320), None);
    // shortfall exactly fake_shortfall_hz (20500 - 18000), a clear step.
    let r = decide_track(
        &s,
        &measurements(-20.0, 0.0, d, Some(18_000.0), 22_050.0),
        &p,
    );
    assert_ne!(r.verdict, Verdict::Fake, "{r:?}");
    assert_eq!(r.verdict, Verdict::Suspicious, "{r:?}");
    assert_eq!(r.reasons, vec![CutoffBelowDeclared], "{r:?}");
}

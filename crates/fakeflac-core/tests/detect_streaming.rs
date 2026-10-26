//! Rule-level tests of rule L6a (`specs/detection/lossy-source.md` section 11, detection v7),
//! derived from `specs/features/detection-streaming-source.feature` (@RF-16): stream information
//! and given measurements go in, verdict, reasons and estimated source bitrate come out. No audio:
//! hand-built measurements, hand-built band spectra and a fake of the `SpectrumAnalyzer` port.
//! "DetectionParams v7" are the defaults of the code (lossy-source.md section 11.4). The scenarios
//! that read a params file (v5, v6, the v7 text with Opus rows) are in
//! `crates/fakeflac-audio/tests/detection_streaming.rs`, which has `toml`.
//!
//! Names chosen by the test author (the spec fixes `lossy.streaming` and its five keys, and the
//! contract fixes the reason `streamingSampleRate`, commands.md "Streaming sample rate (version 6)"):
//!   `LossyParams.streaming: Option<StreamingParams>` (`fakeflac_core::params::StreamingParams`),
//!   `SuspicionReason::StreamingSampleRate`.
//! Every other name is the one of `detect_lossy.rs` (`StreamInfo`, `decide_track`,
//! `expected_cutoff_hz`, `TrackDecision`).

#![allow(clippy::expect_used)]

use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{decide_track, expected_cutoff_hz, Measurements, StreamInfo};
use fakeflac_core::ports::{AudioProbe, FileEntry, SpectrumAnalyzer};
use fakeflac_core::{
    AudioCodec, AudioFormat, BandSpectra, BitrateMode, DetectionParams, SuspicionReason,
    TrackError, Verdict,
};
use std::path::{Path, PathBuf};
use SuspicionReason::{
    CutoffBelowDeclared, GradualRolloff, LowHighFrequencyContent, StreamingSampleRate,
};

/// DetectionParams v7: the defaults (lossy-source.md section 11.4).
fn v7() -> DetectionParams {
    DetectionParams::default()
}

/// The defaults without `[lossy.streaming]` and with `algorithm_version` 6: what
/// `detection-params.v6.toml` holds (`crates/fakeflac-audio/tests/detection_lossless.rs`,
/// `default_params_equal_versioned_file`, checks the equality against the file).
fn v6() -> DetectionParams {
    let mut p = DetectionParams {
        algorithm_version: 6,
        ..DetectionParams::default()
    };
    p.lossy
        .as_mut()
        .expect("the defaults have a [lossy] section")
        .streaming = None;
    p
}

/// The measurements named by the feature: H, G, D, fs and fe. `F`, `U` and `B` are not used by the
/// lossy rules and are set to arbitrary values.
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
// Scenario Outline: Lossy verdict at streaming sample rates (@RF-16)
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
    c_exp: f64,
    verdict: Verdict,
    reasons: &'static [SuspicionReason],
    estimate: Option<u32>,
}

#[allow(clippy::too_many_arguments)]
fn row(
    case: &'static str,
    (codec, rate, b, tag): (AudioCodec, u32, u32, Option<u32>),
    (h, g, d, fs, fe): (f64, f64, f64, Option<f64>, f64),
    c_exp: f64,
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

/// The 26 example rows of the outline, in the order of the feature.
fn rows() -> Vec<Row> {
    use AudioCodec::{Aac, Mp3};
    use Verdict::{Fake, Real, Suspicious};
    const S: &[SuspicionReason] = &[StreamingSampleRate];
    vec![
        row(
            "the motivating case: MP3 320 at 48 kHz, edge 20050",
            (Mp3, 48_000, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
            20_500.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "the other Opus value: MP3 320 at 48 kHz, edge 20150",
            (Mp3, 48_000, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(20_150.0), 24_000.0),
            20_500.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "the same at 44.1 kHz",
            (Mp3, 44_100, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(20_050.0), 22_050.0),
            20_500.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "honest LAME 320 of a 48 kHz master: above the window",
            (Mp3, 48_000, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(20_450.0), 24_000.0),
            20_500.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "honest LAME 256 at 48 kHz: below the window",
            (Mp3, 48_000, 256, Some(19_700)),
            (-20.0, 0.0, 40.0, Some(19_650.0), 24_000.0),
            19_700.0,
            Real,
            &[],
            Some(256),
        ),
        row(
            "48 kHz 320 from 128: L3 comes first",
            (Mp3, 48_000, 320, Some(20_500)),
            (-20.0, 0.0, 40.0, Some(17_050.0), 24_000.0),
            20_500.0,
            Fake,
            &[],
            Some(128),
        ),
        row(
            "48 kHz 320 from 192: L4 comes first",
            (Mp3, 48_000, 320, None),
            (-20.0, 0.0, 40.0, Some(18_550.0), 24_000.0),
            20_500.0,
            Suspicious,
            &[CutoffBelowDeclared],
            Some(192),
        ),
        row(
            "48 kHz large shortfall, step not deep: L4",
            (Mp3, 48_000, 320, None),
            (-20.0, 0.0, 9.0, Some(17_050.0), 24_000.0),
            20_500.0,
            Suspicious,
            &[CutoffBelowDeclared],
            Some(128),
        ),
        row(
            "48 kHz no step, content fades early: L5",
            (Mp3, 48_000, 320, None),
            (-20.0, 0.0, 3.0, Some(18_050.0), 17_000.0),
            20_500.0,
            Suspicious,
            &[GradualRolloff],
            Some(128),
        ),
        row(
            "48 kHz little treble: L2 comes first",
            (Mp3, 48_000, 320, None),
            (-65.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
            20_500.0,
            Suspicious,
            &[LowHighFrequencyContent],
            None,
        ),
        row(
            "fc at min_cutoff_hz",
            (Mp3, 48_000, 256, None),
            (-20.0, 0.0, 40.0, Some(19_800.0), 24_000.0),
            19_700.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "fc just below min_cutoff_hz",
            (Mp3, 48_000, 256, None),
            (-20.0, 0.0, 40.0, Some(19_799.0), 24_000.0),
            19_700.0,
            Real,
            &[],
            Some(256),
        ),
        row(
            "fc just below max_cutoff_hz",
            (Mp3, 48_000, 320, None),
            (-20.0, 0.0, 40.0, Some(20_199.0), 24_000.0),
            20_500.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "fc at max_cutoff_hz",
            (Mp3, 48_000, 320, None),
            (-20.0, 0.0, 40.0, Some(20_200.0), 24_000.0),
            20_500.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "no low-pass: content to Nyquist (fc = fe)",
            (Mp3, 48_000, 320, None),
            (-20.0, 0.0, 6.99, Some(17_050.0), 24_000.0),
            20_500.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "declared at min_declared_bitrate_kbps",
            (Mp3, 48_000, 192, None),
            (-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
            18_600.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "declared below min_declared_bitrate_kbps",
            (Mp3, 48_000, 160, None),
            (-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0),
            17_500.0,
            Real,
            &[],
            Some(160),
        ),
        row(
            "VBR average between rows",
            (Mp3, 48_000, 245, None),
            (-20.0, 0.0, 40.0, Some(19_950.0), 24_000.0),
            19_400.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "tag low-pass lowers c_exp, fc below the window",
            (Mp3, 48_000, 320, Some(19_600)),
            (-20.0, 0.0, 40.0, Some(19_650.0), 24_000.0),
            19_600.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "tag low-pass lowers c_exp, fc in the window",
            (Mp3, 48_000, 320, Some(19_900)),
            (-20.0, 0.0, 40.0, Some(19_950.0), 24_000.0),
            19_900.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "32 kHz is not a listed rate",
            (Mp3, 32_000, 320, None),
            (-20.0, 0.0, 0.0, None, 16_000.0),
            15_600.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "AAC 256 at 48 kHz, edge in the window",
            (Aac, 48_000, 256, None),
            (-20.0, 0.0, 40.0, Some(19_950.0), 24_000.0),
            19_000.0,
            Suspicious,
            S,
            None,
        ),
        row(
            "the same AAC at 44.1 kHz",
            (Aac, 44_100, 256, None),
            (-20.0, 0.0, 40.0, Some(19_950.0), 22_050.0),
            19_000.0,
            Real,
            &[],
            Some(256),
        ),
        row(
            "AAC 128 at 48 kHz: below the bitrate and the window",
            (Aac, 48_000, 128, None),
            (-20.0, 0.0, 40.0, Some(16_050.0), 24_000.0),
            15_500.0,
            Real,
            &[],
            Some(128),
        ),
        row(
            "AAC at 96 kHz is not a listed rate",
            (Aac, 96_000, 320, None),
            (-20.0, 0.0, 40.0, Some(19_950.0), 48_000.0),
            19_500.0,
            Real,
            &[],
            Some(320),
        ),
        row(
            "AAC 256 at 48 kHz from 128: L3 comes first",
            (Aac, 48_000, 256, None),
            (-20.0, 0.0, 40.0, Some(15_550.0), 24_000.0),
            19_000.0,
            Fake,
            &[],
            Some(128),
        ),
    ]
}

fn decide_row(r: &Row, p: &DetectionParams) -> (Option<f64>, fakeflac_core::detect::TrackDecision) {
    let s = stream(r.codec, r.rate, Some(r.b), r.tag);
    let m = measurements(r.h, r.g, r.d, r.fs, r.fe);
    (expected_cutoff_hz(&s, p), decide_track(&s, &m, p))
}

// @RF-16 Scenario Outline: Lossy verdict at streaming sample rates: "the expected cutoff is
// <c_exp> Hz", "the verdict is", "the reasons are", "the estimated source bitrate is". One loop
// reports every failing row.
#[test]
fn rf16_lossy_verdict_at_streaming_sample_rates() {
    let p = v7();
    let all = rows();
    assert_eq!(all.len(), 26, "every example row of the outline");
    let mut failures = Vec::new();
    for r in &all {
        let (c_exp, d) = decide_row(r, &p);
        if c_exp != Some(r.c_exp)
            || d.verdict != r.verdict
            || d.reasons != r.reasons
            || d.estimated_source_bitrate_kbps != r.estimate
            || d.error.is_some()
        {
            failures.push(format!(
                "{}: c_exp {c_exp:?} (want {}), {:?} {:?} estimate {:?} (want {:?} {:?} {:?}), error {:?}",
                r.case, r.c_exp, d.verdict, d.reasons, d.estimated_source_bitrate_kbps, r.verdict, r.reasons, r.estimate, d.error
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// The same rows one by one, so that a boundary row reads as a test of its own. @RF-16
fn check_row(case: &str) {
    let r = rows()
        .into_iter()
        .find(|r| r.case == case)
        .expect("row exists");
    let (c_exp, d) = decide_row(&r, &v7());
    assert_eq!(c_exp, Some(r.c_exp), "{case}: c_exp");
    assert_eq!(d.verdict, r.verdict, "{case}");
    assert_eq!(d.reasons, r.reasons, "{case}");
    assert_eq!(d.estimated_source_bitrate_kbps, r.estimate, "{case}");
    assert_eq!(d.error, None, "{case}");
}

#[test]
fn rf16_the_motivating_case_and_the_other_opus_value_are_suspicious_with_no_estimate() {
    check_row("the motivating case: MP3 320 at 48 kHz, edge 20050");
    check_row("the other Opus value: MP3 320 at 48 kHz, edge 20150");
}
#[test]
fn rf16_the_same_edges_at_44_1_khz_are_real() {
    check_row("the same at 44.1 kHz");
    check_row("the same AAC at 44.1 kHz");
}
#[test]
fn rf16_honest_lame_edges_outside_the_window_are_real() {
    check_row("honest LAME 320 of a 48 kHz master: above the window");
    check_row("honest LAME 256 at 48 kHz: below the window");
}
#[test]
fn rf16_earlier_rules_come_first_at_48_khz() {
    check_row("48 kHz 320 from 128: L3 comes first");
    check_row("48 kHz 320 from 192: L4 comes first");
    check_row("48 kHz large shortfall, step not deep: L4");
    check_row("48 kHz no step, content fades early: L5");
    check_row("48 kHz little treble: L2 comes first");
    check_row("AAC 256 at 48 kHz from 128: L3 comes first");
}
#[test]
fn rf16_the_lower_bound_of_the_window_is_inclusive() {
    check_row("fc at min_cutoff_hz");
    check_row("fc just below min_cutoff_hz");
}
#[test]
fn rf16_the_upper_bound_of_the_window_is_exclusive() {
    check_row("fc just below max_cutoff_hz");
    check_row("fc at max_cutoff_hz");
}
#[test]
fn rf16_without_a_clear_step_fc_is_fe_and_content_to_nyquist_is_real() {
    check_row("no low-pass: content to Nyquist (fc = fe)");
}
#[test]
fn rf16_the_bitrate_floor_is_inclusive_and_a_vbr_average_is_compared() {
    check_row("declared at min_declared_bitrate_kbps");
    check_row("declared below min_declared_bitrate_kbps");
    check_row("VBR average between rows");
}
#[test]
fn rf16_the_tag_low_pass_does_not_change_the_window() {
    check_row("tag low-pass lowers c_exp, fc below the window");
    check_row("tag low-pass lowers c_exp, fc in the window");
}
#[test]
fn rf16_only_listed_sample_rates_count() {
    check_row("32 kHz is not a listed rate");
    check_row("AAC at 96 kHz is not a listed rate");
}
#[test]
fn rf16_aac_at_48_khz_is_judged_by_l6a() {
    check_row("AAC 256 at 48 kHz, edge in the window");
    check_row("AAC 128 at 48 kHz: below the bitrate and the window");
}

// "Only Real -> Suspicious" (lossy-source.md section 11.2, consequences): for every row, the v7
// result equals the v6 result except rows that v6 calls Real and v7 flags. @RF-16
#[test]
fn rf16_v7_only_turns_real_into_suspicious_on_the_outline_rows() {
    let (p6, p7) = (v6(), v7());
    let mut failures = Vec::new();
    for r in rows() {
        let (_, old) = decide_row(&r, &p6);
        let (_, new) = decide_row(&r, &p7);
        let flagged = new.reasons == [StreamingSampleRate];
        let ok = if flagged {
            old.verdict == Verdict::Real
                && new.verdict == Verdict::Suspicious
                && new.estimated_source_bitrate_kbps.is_none()
        } else {
            old == new
        };
        if !ok {
            failures.push(format!("{}: v6 {old:?} / v7 {new:?}", r.case));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Scenario: An Opus track with the defaults is still an error, not streamingSampleRate (@RF-16)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_an_opus_track_with_the_defaults_is_still_an_error() {
    let p = v7();
    assert_eq!(
        p.algorithm_version, 7,
        "DetectionParams v7 are the defaults"
    );
    let s = stream(AudioCodec::Opus, 48_000, Some(128), None);
    // Measurements that would be in the window for an MP3 or AAC track.
    let m = measurements(-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0);
    let d = decide_track(&s, &m, &p);
    assert_eq!(d.verdict, Verdict::Error, "{d:?}");
    assert_eq!(
        d.error,
        Some(TrackError::UnsupportedFormat(
            "no [lossy.cutoffs] rows for opus".into()
        ))
    );
    assert!(d.reasons.is_empty());
    assert_eq!(d.estimated_source_bitrate_kbps, None);
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Lossless codecs at 48 kHz are never judged by L6a (@RF-16 @RF-04), at rule
// level. The fixture-level rows (the measurements of "full_band") are in
// `crates/fakeflac-audio/tests/detection_streaming.rs`; here the measurements are those that flag
// an MP3 (an edge at 20050 Hz with a clear step), so the rule would fire if it read the codec wrong.
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_lossless_codecs_at_48_khz_with_an_edge_in_the_window_are_judged_as_with_v6() {
    let (p6, p7) = (v6(), v7());
    let m = measurements(-20.0, 0.0, 40.0, Some(20_050.0), 24_000.0);
    for codec in [AudioCodec::Pcm, AudioCodec::Flac, AudioCodec::Alac] {
        // A lossless codec declares no bitrate; give it one too, so only the codec keeps L6a away.
        for b in [None, Some(320)] {
            let s = stream(codec, 48_000, b, None);
            let (old, new) = (decide_track(&s, &m, &p6), decide_track(&s, &m, &p7));
            assert_eq!(new, old, "{codec:?} b {b:?}");
            assert!(
                !new.reasons.contains(&StreamingSampleRate),
                "{codec:?} b {b:?}"
            );
            assert_eq!(new.estimated_source_bitrate_kbps, None, "{codec:?} b {b:?}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Scenario Outline: Params without [lossy.streaming] never apply L6a (@RF-16), on the struct (the
// file rows are in `crates/fakeflac-audio/tests/detection_streaming.rs`).
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_params_without_a_streaming_section_never_apply_l6a() {
    let p = v6();
    for (codec, b, fs) in [
        (AudioCodec::Mp3, 320, 20_050.0),
        (AudioCodec::Mp3, 320, 20_450.0),
        (AudioCodec::Aac, 256, 19_950.0),
    ] {
        let s = stream(codec, 48_000, Some(b), None);
        let m = measurements(-20.0, 0.0, 40.0, Some(fs), 24_000.0);
        let d = decide_track(&s, &m, &p);
        assert_eq!(d.verdict, Verdict::Real, "{codec:?} {b} {fs}: {d:?}");
        assert!(d.reasons.is_empty(), "{codec:?} {b} {fs}");
        assert_eq!(
            d.estimated_source_bitrate_kbps,
            Some(b),
            "{codec:?} {b} {fs}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// analyze_track with a fake analyzer: L6a reaches the TrackAnalysis (@RF-16), Scenario Outline
// "Verdict of lossy fixtures with v7" at the level of the use case: reasons, no estimate, the
// reported cutoff unchanged and algorithmVersion 7.
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

/// Band spectra at `rate` with content up to `edge_hz` and nothing above (a clear step).
fn spectra_with_edge(rate: u32, edge_hz: f64) -> BandSpectra {
    let bands = (rate / 2 / 100) as usize;
    let level = move |b: usize| {
        if (b as f64 + 0.5) * 100.0 < edge_hz {
            0.0
        } else {
            -100.0
        }
    };
    BandSpectra {
        sample_rate: rate,
        band_hz: 100.0,
        mean_db: (0..bands).map(level).collect(),
        median_db: vec![0.0; bands],
    }
}

fn mp3_probe(rate: u32, b: u32, edge_hz: f64) -> AudioProbe {
    AudioProbe {
        sample_rate: rate,
        bit_depth: None,
        channels: 2,
        duration_ms: 240_000,
        codec: Some(AudioCodec::Mp3),
        declared_bitrate_kbps: Some(b),
        bitrate_mode: Some(BitrateMode::Cbr),
        encoder: Some("LAME3.100".into()),
        encoder_lowpass_hz: Some(20_500),
        spectrum: Some(spectra_with_edge(rate, edge_hz)),
    }
}

#[test]
fn rf16_analyze_track_flags_a_48k_mp3_320_with_the_opus_edge() {
    let an = FakeProbe(Ok(mp3_probe(48_000, 320, 20_000.0)));
    let a = analyze_track(&an, &mp3_entry(), &v7());
    assert_eq!(a.verdict, Verdict::Suspicious, "{a:#?}");
    assert_eq!(a.reasons, vec![StreamingSampleRate]);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert_eq!(a.error, None);
    assert_eq!(a.algorithm_version, 7);
    assert_eq!(a.info.sample_rate, 48_000);
    assert_eq!(a.info.declared_bitrate_kbps, Some(320));
    let c = a.cutoff.expect("a cutoff is reported");
    assert!(
        (19_800.0..20_200.0).contains(&c.frequency_hz),
        "{}",
        c.frequency_hz
    );
    // "Only Real -> Suspicious": the reported cutoff is the one v6 reports.
    let old = analyze_track(&an, &mp3_entry(), &v6());
    assert_eq!(old.verdict, Verdict::Real, "{old:#?}");
    assert_eq!(old.estimated_source_bitrate_kbps, Some(320));
    assert_eq!(old.cutoff, Some(c));
    assert_eq!(old.algorithm_version, 6);
}

#[test]
fn rf16_analyze_track_leaves_the_same_mp3_at_44_1_khz_real() {
    let an = FakeProbe(Ok(mp3_probe(44_100, 320, 20_000.0)));
    let a = analyze_track(&an, &mp3_entry(), &v7());
    assert_eq!(a.verdict, Verdict::Real, "{a:#?}");
    assert!(a.reasons.is_empty());
    assert_eq!(a.estimated_source_bitrate_kbps, Some(320));
    assert_eq!(a.algorithm_version, 7);
}

#[test]
fn rf16_analyze_track_of_a_short_48k_rip_is_too_short_not_flagged() {
    // Scenario Outline "Broken lossy files stay errors with v7", row mp3_320_48k_short, at the
    // level of the use case: L1b comes before L6a.
    let mut probe = mp3_probe(48_000, 320, 20_000.0);
    probe.duration_ms = 1_992;
    let a = analyze_track(&FakeProbe(Ok(probe)), &mp3_entry(), &v7());
    assert_eq!(a.verdict, Verdict::Error, "{a:#?}");
    assert_eq!(a.error, Some(TrackError::TooShort));
    assert!(a.reasons.is_empty());
    assert_eq!(a.cutoff, None);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
    assert_eq!(a.algorithm_version, 7);
}

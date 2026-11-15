//! @RF-17: Evidence and unchanged detection on synthetic audio.
//! Scenarios: specs/features/fake-evidence.feature, Fixtures and Invariants.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{entry, fixture, lossy_fixture, opus, Container};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{
    cutoff_estimate, decide_track, detect_track, expected_cutoff_hz, fake_evidence, is_edge_step,
    is_full_band, measure, track_guard, Measurements, StreamInfo,
};
use fakeflac_core::domain::{EvidenceKind, FakeEvidence};
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{
    AudioCodec, DetectionParams, SuspicionReason, TrackAnalysis, TrackError, Verdict,
};
use std::path::{Path, PathBuf};
use EvidenceKind::{CutoffStep, GapStep, Shortfall};
use Verdict::{Error, Fake, Real, Suspicious};

const SIGNALS: [&str; 24] = [
    "full_band",
    "lp16k",
    "lp19k",
    "lp20k",
    "lp16k_96k",
    "gradual",
    "low_hf",
    "bass_tone_lp16k",
    "silence",
    "short",
    "corrupt",
    "lp16k_unknown_length",
    "lp16k_dither",
    "lp20k_floor",
    "sfb21_gaps",
    "sfb21_gaps_weak",
    "hf_bursts",
    "sfb21_gaps_lp20k5",
    "sfb21_gaps_aa21k",
    "shelf19k5",
    "shelf19k5_bursts",
    "edge_bump19k3",
    "edge_bump19k3_lp19k5",
    "holes_lp19k5",
];

fn flac(name: &str) -> PathBuf {
    fixture(name, Container::Flac)
}

fn params(version: u32) -> DetectionParams {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../specs/detection/detection-params.v{version}.toml"
    ));
    toml::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

fn analyze(path: &Path, params: &DetectionParams) -> TrackAnalysis {
    analyze_track(&SymphoniaAnalyzer, &entry(path), params)
}

fn kinds(result: &TrackAnalysis) -> Vec<EvidenceKind> {
    result.evidence.iter().map(|item| item.kind).collect()
}

fn assert_margins(result: &TrackAnalysis, label: &str) {
    for item in &result.evidence {
        assert!(item.value.is_finite(), "{label}: {item:?}");
        assert!(item.threshold.is_finite(), "{label}: {item:?}");
        assert!(
            item.margin.is_finite() && item.margin >= 0.0,
            "{label}: {item:?}"
        );
        assert!(
            (item.margin - (item.value - item.threshold)).abs() <= 0.001,
            "{label}: {item:?}"
        );
    }
}

// @RF-17 @RF-04 Evidence kinds of lossless fixtures with the default parameters.
#[test]
fn rf17_lossless_fixture_evidence_kinds() {
    let p = DetectionParams::default();
    let cases: &[(&str, Verdict, &[EvidenceKind])] = &[
        ("sfb21_gaps", Fake, &[GapStep]),
        ("sfb21_gaps_lp20k5", Fake, &[GapStep]),
        ("shelf19k5", Fake, &[CutoffStep]),
        ("shelf19k5_bursts", Fake, &[CutoffStep]),
        ("lp19k", Fake, &[CutoffStep]),
        ("holes_lp19k5", Fake, &[CutoffStep]),
        ("edge_bump19k3_lp19k5", Fake, &[CutoffStep]),
        ("full_band", Real, &[]),
        ("hf_bursts", Real, &[]),
        ("gradual", Suspicious, &[]),
        ("low_hf", Suspicious, &[]),
        ("silence", Suspicious, &[]),
        ("sfb21_gaps_weak", Suspicious, &[]),
        ("sfb21_gaps_aa21k", Suspicious, &[]),
        ("edge_bump19k3", Suspicious, &[]),
    ];
    for &(name, verdict, expected_kinds) in cases {
        let actual = analyze(&flac(name), &p);
        assert_eq!(actual.verdict, verdict, "{name}");
        assert_eq!(kinds(&actual), expected_kinds, "{name}");
        for item in &actual.evidence {
            let threshold = match item.kind {
                GapStep => p.verdict.gap_fake_db,
                CutoffStep => p.verdict.step_fake_db,
                Shortfall => panic!("no shortfall on {name}"),
            };
            assert_eq!(item.threshold, threshold as f32, "{name}");
        }
        assert_margins(&actual, name);
    }
}

// @RF-17 @RF-04 The gap-only fixture with a full-band cutoff.
#[test]
fn rf17_gap_only_has_nyquist_cutoff() {
    let actual = analyze(&flac("sfb21_gaps"), &DetectionParams::default());
    assert_eq!(actual.verdict, Fake);
    assert!(actual.reasons.is_empty());
    assert_eq!(actual.cutoff.as_ref().unwrap().frequency_hz, 22_050.0);
    assert_eq!(kinds(&actual), [GapStep]);
    assert_eq!(actual.evidence[0].threshold, 6.0);
    assert!(actual.evidence[0].value >= 6.0);
}

// @RF-17 @RF-04 @RF-14 Lossless fixtures whose step is the evidence.
#[test]
fn rf17_lossless_cliffs_include_cutoff_step_evidence() {
    let p = DetectionParams::default();
    for path in ["lp16k", "lp16k_dither", "lp16k_96k", "bass_tone_lp16k"]
        .map(flac)
        .into_iter()
        .chain([lossy_fixture("m4a_alac_lp16k")])
    {
        let actual = analyze(&path, &p);
        assert_eq!(actual.verdict, Fake, "{path:?}");
        let kinds = kinds(&actual);
        assert!(
            matches!(kinds.as_slice(), [CutoffStep] | [GapStep, CutoffStep]),
            "{path:?}"
        );
    }
}

// @RF-17 @RF-15 Evidence of lossy fixtures with the default parameters.
#[test]
fn rf17_lossy_fixture_evidence_kinds_and_shortfall() {
    let p = DetectionParams::default();
    for (name, verdict, expected_cutoff) in [
        ("mp3_320_lp17k", Fake, 20500.0),
        ("m4a_aac_256_lp15k3", Fake, 19000.0),
        ("opus_160_src16k", Fake, 20000.0),
        ("mp3_320_lp20k5", Real, 20500.0),
        ("mp3_320_lp18k5", Suspicious, 20500.0),
        ("mp3_320_48k_lp20k", Suspicious, 20500.0),
        ("mp3_320_low_hf", Suspicious, 20500.0),
        ("mp3_silence", Suspicious, 20500.0),
        ("opus_128_fb", Real, 20000.0),
        ("opus_160_swb", Suspicious, 20000.0),
    ] {
        let path = if name.starts_with("opus_") {
            opus::fixture(name)
        } else {
            lossy_fixture(name)
        };
        let actual = analyze(&path, &p);
        assert_eq!(actual.verdict, verdict, "{name}");
        if verdict == Fake {
            assert_eq!(kinds(&actual), [Shortfall, CutoffStep], "{name}");
            let cutoff = actual.cutoff.as_ref().unwrap().frequency_hz;
            assert!(
                (actual.evidence[0].value - (expected_cutoff - cutoff)).abs() <= 0.5,
                "{name}"
            );
            assert_eq!(actual.evidence[0].threshold, 2500.0, "{name}");
            assert_eq!(actual.evidence[1].threshold, 10.0, "{name}");
        } else {
            assert!(actual.evidence.is_empty(), "{name}");
        }
        assert_margins(&actual, name);
    }
}

// @RF-17 @RNF-04 Error tracks carry no evidence.
#[test]
fn rf17_errors_have_empty_evidence() {
    let p = DetectionParams::default();
    for (path, corrupt) in [
        (flac("short"), false),
        (flac("corrupt"), true),
        (lossy_fixture("mp3_corrupt"), true),
        (lossy_fixture("m4a_corrupt"), true),
        (lossy_fixture("mp3_320_48k_short"), false),
        (opus::fixture("opus_short"), false),
    ] {
        let actual = analyze(&path, &p);
        assert_eq!(actual.verdict, Error, "{path:?}");
        if corrupt {
            assert!(
                matches!(actual.error, Some(TrackError::CorruptStream(_))),
                "{path:?}"
            );
        } else {
            assert_eq!(actual.error, Some(TrackError::TooShort), "{path:?}");
        }
        assert!(actual.evidence.is_empty(), "{path:?}");
    }
}

// @RF-17 @RF-04 An older params file gives its own evidence on a fixture.
#[test]
fn rf17_older_params_change_eligible_evidence() {
    let path = flac("edge_bump19k3");
    let old = analyze(&path, &params(5));
    assert_eq!(old.verdict, Fake);
    assert_eq!(kinds(&old), [CutoffStep]);
    let current = analyze(&path, &DetectionParams::default());
    assert_eq!(current.verdict, Suspicious);
    assert_eq!(current.reasons, [SuspicionReason::AmbiguousCutoff]);
    assert!(current.evidence.is_empty());
}

// @RF-17 The evidence follows the parameters the track was analysed with.
#[test]
fn rf17_rule_evidence_uses_versioned_params_and_custom_threshold() {
    let stream = StreamInfo {
        codec: AudioCodec::Flac,
        sample_rate: 44_100,
        declared_bitrate_kbps: None,
        encoder_lowpass_hz: None,
    };
    let w4 = Measurements {
        high_frequency_db: -20.0,
        gap_db: -0.25,
        step_db: 14.22,
        step_hz: Some(19450.0),
        extent_hz: 22050.0,
        upper_level_db: Some(-47.06),
        upper_burst_db: Some(2.86),
        median_step_db: Some(0.32),
        residual_db: Some(17.48),
        top_floor_db: None,
        noise_floor_db: -95.0,
    };
    let fp2 = Measurements {
        gap_db: 13.18,
        step_db: 14.79,
        step_hz: Some(21450.0),
        upper_level_db: Some(-44.74),
        upper_burst_db: Some(30.6),
        median_step_db: Some(0.0),
        residual_db: Some(0.0),
        ..w4
    };
    let custom = Measurements {
        gap_db: 9.6,
        step_db: 5.0,
        step_hz: Some(16050.0),
        upper_level_db: Some(-90.0),
        upper_burst_db: Some(0.2),
        ..fp2
    };
    let mut raised = params(8);
    raised.verdict.gap_fake_db = 8.0;
    for (p, m, verdict, expected) in [
        (
            params(5),
            w4.clone(),
            Fake,
            Some((CutoffStep, 14.22, 10.0, 4.22)),
        ),
        (params(6), w4, Suspicious, None),
        (params(2), fp2, Fake, Some((GapStep, 13.18, 6.0, 7.18))),
        (raised, custom, Fake, Some((GapStep, 9.6, 8.0, 1.6))),
    ] {
        let d = decide_track(&stream, &m, &p);
        assert_eq!(d.verdict, verdict);
        let items = fake_evidence(&m, &p, stream.codec, None, d.verdict);
        assert_eq!(items.len(), usize::from(expected.is_some()));
        if let Some((kind, value, threshold, margin)) = expected {
            assert_eq!(items[0].kind, kind);
            assert!((items[0].value - value).abs() < 0.01);
            assert_eq!(items[0].threshold, threshold);
            assert!((items[0].margin - margin).abs() < 0.01);
        }
    }
}

// Derive an independent expectation from the original rule comparisons and the raw probe. The
// public evidence function is intentionally not used as an oracle here.
fn expected_items(m: &Measurements, stream: &StreamInfo, p: &DetectionParams) -> Vec<FakeEvidence> {
    let item = |kind, value: f64, threshold: f64| FakeEvidence {
        kind,
        value: value as f32,
        threshold: threshold as f32,
        margin: if kind == Shortfall {
            (value - threshold).max(0.0)
        } else {
            value - threshold
        } as f32,
    };
    if stream.codec.is_lossy() {
        let lossy = p.lossy.as_ref().unwrap();
        let cutoff = if m.step_db >= p.verdict.step_suspicious_db {
            m.step_hz.unwrap_or(m.extent_hz)
        } else {
            m.extent_hz
        };
        return vec![
            item(
                Shortfall,
                expected_cutoff_hz(stream, p).unwrap() - cutoff,
                lossy.fake_shortfall_hz,
            ),
            item(CutoffStep, m.step_db, lossy.fake_min_step_db),
        ];
    }
    let full_band = is_full_band(m, p);
    let in_range = m
        .step_hz
        .is_some_and(|hz| hz >= p.verdict.lossy_min_cutoff_hz && hz < p.verdict.fake_max_cutoff_hz);
    let mut items = Vec::new();
    if m.gap_db >= p.verdict.gap_fake_db && !full_band {
        items.push(item(GapStep, m.gap_db, p.verdict.gap_fake_db));
    }
    if m.step_db >= p.verdict.step_fake_db && in_range && !full_band && !is_edge_step(m, p) {
        items.push(item(CutoffStep, m.step_db, p.verdict.step_fake_db));
    }
    items
}

// @RF-17 @RF-04 @RF-15 The evidence agrees with the measurements on every fixture and params file.
// RF-17 changes no verdict, reason, cutoff, estimate or version of any fixture. The existing
// detection, decode and Opus suites continue to check the documented fixture truths.
#[test]
fn rf17_all_fixtures_and_params_match_original_rules() {
    let fixtures = include_str!("../../../specs/detection/lossy-fixtures.md");
    let (_, fixtures) = fixtures.split_once("## 2. Fixtures").unwrap();
    let (fixtures, _) = fixtures
        .split_once("## 3. What the test stage confirms")
        .unwrap();
    let lossy_files = fixtures.lines().filter_map(|line| {
        let fields = line.strip_prefix("| `")?;
        let (id, _) = fields.split_once("` |")?;
        Some(if id.starts_with("opus_") {
            opus::fixture(id)
        } else {
            lossy_fixture(id)
        })
    });
    let files: Vec<_> = SIGNALS.into_iter().map(flac).chain(lossy_files).collect();
    assert_eq!(DetectionParams::default(), params(8));
    std::thread::scope(|scope| {
        for version in 2..=8 {
            let files = &files;
            scope.spawn(move || {
                let p = params(version);
                for path in files {
                    let label = format!("{path:?} (v{version})");
                    let actual = analyze(path, &p);
                    assert_eq!(actual.algorithm_version, version, "{label}");
                    assert_eq!(
                        !actual.evidence.is_empty(),
                        actual.verdict == Fake,
                        "{label}"
                    );
                    assert_margins(&actual, &label);
                    let probe = match SymphoniaAnalyzer.analyze(path, &p) {
                        Ok(probe) => probe,
                        Err(error) => {
                            assert_eq!(
                                (actual.verdict, actual.error),
                                (Error, Some(error)),
                                "{label}"
                            );
                            continue;
                        }
                    };
                    let codec = probe.codec.unwrap_or(AudioCodec::Pcm);
                    if let Some(error) = track_guard(
                        Some(codec),
                        probe.duration_ms,
                        probe.declared_bitrate_kbps,
                        &p,
                    ) {
                        assert_eq!(
                            (actual.verdict, actual.error),
                            (Error, Some(error)),
                            "{label}"
                        );
                        continue;
                    }
                    let stream = StreamInfo {
                        codec,
                        sample_rate: probe.sample_rate,
                        declared_bitrate_kbps: probe.declared_bitrate_kbps,
                        encoder_lowpass_hz: probe.encoder_lowpass_hz,
                    };
                    let original = detect_track(probe.spectrum.as_ref(), &stream, &p);
                    assert_eq!(actual.verdict, original.verdict, "{label}");
                    assert_eq!(actual.error, original.error, "{label}");
                    assert_eq!(actual.reasons, original.reasons, "{label}");
                    assert_eq!(actual.cutoff, original.cutoff, "{label}");
                    assert_eq!(
                        actual.estimated_source_bitrate_kbps,
                        original.estimated_source_bitrate_kbps,
                        "{label}"
                    );
                    if actual.verdict == Fake {
                        let m = measure(probe.spectrum.as_ref().unwrap(), &p);
                        assert_eq!(actual.evidence, expected_items(&m, &stream, &p), "{label}");
                        assert_eq!(
                            actual.cutoff.as_ref(),
                            Some(&cutoff_estimate(&m, &p)),
                            "{label}"
                        );
                    }
                }
            });
        }
    });
}

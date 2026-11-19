//! @RF-17: Given measurements and the existing verdict, report the applicable evidence in order.
//! Scenarios: specs/features/fake-evidence.feature; calculation: specs/detection/fake-evidence.md.
#![allow(clippy::expect_used)]

use fakeflac_core::analysis::analyze_track;
use fakeflac_core::detect::{
    cutoff_estimate, decide, decide_track, expected_cutoff_hz, fake_evidence, lossy_unsupported,
    Measurements, StreamInfo,
};
use fakeflac_core::domain::{EvidenceKind, FakeEvidence};
use fakeflac_core::params::CutoffRow;
use fakeflac_core::ports::{AudioProbe, FileEntry, SpectrumAnalyzer};
use fakeflac_core::{
    AudioCodec, AudioFormat, DetectionParams, SuspicionReason, TrackError, Verdict,
};
use std::path::{Path, PathBuf};
use EvidenceKind::{CutoffStep, GapStep, Shortfall};
use SuspicionReason::{
    AmbiguousCutoff, CutoffBelowDeclared, GradualRolloff, HighBandGaps, LowHighFrequencyContent,
    StreamingSampleRate,
};
use Verdict::{Fake, Real, Suspicious};

type Item = (EvidenceKind, f32, f32, f32);
type Levels = (f64, f64, f64, f64, f64);
type Guards = (f64, f64, f64, f64);

fn measurements((h, g, d, fs, fe): Levels, (u, b, dq, r): Guards) -> Measurements {
    Measurements {
        high_frequency_db: h,
        gap_db: g,
        step_db: d,
        step_hz: Some(fs),
        extent_hz: fe,
        upper_level_db: Some(u),
        upper_burst_db: Some(b),
        median_step_db: Some(dq),
        residual_db: Some(r),
        top_floor_db: None,
        noise_floor_db: -95.0,
    }
}

fn stream(codec: AudioCodec, rate: u32, bitrate: Option<u32>) -> StreamInfo {
    StreamInfo {
        codec,
        sample_rate: rate,
        declared_bitrate_kbps: bitrate,
        encoder_lowpass_hz: None,
    }
}

fn assert_items(actual: &[FakeEvidence], expected: &[Item], case: &str) {
    assert_eq!(actual.len(), expected.len(), "{case}: {actual:?}");
    for (item, &(kind, value, threshold, margin)) in actual.iter().zip(expected) {
        assert_eq!(item.kind, kind, "{case}");
        for (actual_value, expected_value) in [
            (item.value, value),
            (item.threshold, threshold),
            (item.margin, margin),
        ] {
            assert!(
                (actual_value - expected_value).abs() <= 0.01,
                "{case}: {item:?}, expected {expected_value}, got {actual_value}"
            );
        }
    }
}

struct LosslessRow {
    case: &'static str,
    levels: Levels,
    guards: Guards,
    outcome: (Verdict, Option<SuspicionReason>, f32),
    items: &'static [Item],
}

// @RF-17 Scenario Outline: Evidence of a lossless track from given measurements.
#[test]
fn rf17_lossless_evidence_matches_each_eligible_rule() {
    let cases = [
        LosslessRow {
            case: "gap only, cutoff at full band",
            levels: (-20.0, 9.6, 5.0, 16050.0, 22050.0),
            guards: (-90.0, 0.2, 0.0, 0.0),
            outcome: (Fake, None, 22050.0),
            items: &[(GapStep, 9.6, 6.0, 3.6)],
        },
        LosslessRow {
            case: "gap only, lossy step removes full-band guard",
            levels: (-20.0, 8.0, 5.0, 16050.0, 22050.0),
            guards: (-50.0, 10.0, 0.0, 0.0),
            outcome: (Fake, None, 22050.0),
            items: &[(GapStep, 8.0, 6.0, 2.0)],
        },
        LosslessRow {
            case: "gap only, deep step above the lossy range",
            levels: (-20.0, 19.0, 60.0, 20450.0, 20400.0),
            guards: (-100.0, 0.1, 0.0, 0.0),
            outcome: (Fake, None, 20450.0),
            items: &[(GapStep, 19.0, 6.0, 13.0)],
        },
        LosslessRow {
            case: "step only",
            levels: (-20.0, 0.5, 13.2, 19450.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 2.0),
            outcome: (Fake, None, 19450.0),
            items: &[(CutoffStep, 13.2, 10.0, 3.2)],
        },
        LosslessRow {
            case: "both hold",
            levels: (-20.0, 12.0, 24.7, 16050.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 0.0),
            outcome: (Fake, None, 16050.0),
            items: &[(GapStep, 12.0, 6.0, 6.0), (CutoffStep, 24.7, 10.0, 14.7)],
        },
        LosslessRow {
            case: "both at their thresholds",
            levels: (-20.0, 6.0, 10.0, 19450.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 0.0),
            outcome: (Fake, None, 19450.0),
            items: &[(GapStep, 6.0, 6.0, 0.0), (CutoffStep, 10.0, 10.0, 0.0)],
        },
        LosslessRow {
            case: "both just below their thresholds",
            levels: (-20.0, 5.99, 9.99, 19450.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 0.0),
            outcome: (Suspicious, Some(HighBandGaps), 19450.0),
            items: &[],
        },
        LosslessRow {
            case: "gap with an edge step",
            levels: (-20.0, 7.0, 14.22, 19450.0, 22050.0),
            guards: (-47.06, 2.86, 0.32, 17.48),
            outcome: (Fake, None, 19450.0),
            items: &[(GapStep, 7.0, 6.0, 1.0)],
        },
        LosslessRow {
            case: "W4 edge step keeps it from Fake",
            levels: (-20.0, -0.25, 14.22, 19450.0, 22050.0),
            guards: (-47.06, 2.86, 0.32, 17.48),
            outcome: (Suspicious, Some(AmbiguousCutoff), 19450.0),
            items: &[],
        },
        LosslessRow {
            case: "FP2 full-band guard keeps it from Fake",
            levels: (-20.0, 13.18, 14.79, 21450.0, 21400.0),
            guards: (-44.74, 30.6, 0.0, 0.0),
            outcome: (Suspicious, Some(HighBandGaps), 21450.0),
            items: &[],
        },
        LosslessRow {
            case: "little treble",
            levels: (-65.0, 12.0, 24.7, 16050.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 0.0),
            outcome: (Suspicious, Some(LowHighFrequencyContent), 16050.0),
            items: &[],
        },
        LosslessRow {
            case: "deep step below the lossy range",
            levels: (-20.0, 0.0, 30.0, 14450.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 0.0),
            outcome: (Suspicious, Some(AmbiguousCutoff), 14450.0),
            items: &[],
        },
        LosslessRow {
            case: "deep step above the lossy range",
            levels: (-20.0, 0.0, 30.0, 20350.0, 22050.0),
            guards: (-90.0, 0.2, 10.0, 0.0),
            outcome: (Suspicious, Some(AmbiguousCutoff), 20350.0),
            items: &[],
        },
        LosslessRow {
            case: "full band, small step",
            levels: (-20.0, 0.0, 2.0, 16050.0, 22050.0),
            guards: (-50.0, 10.0, 0.0, 0.0),
            outcome: (Real, None, 22050.0),
            items: &[],
        },
    ];
    let params = DetectionParams::default();
    let stream = stream(AudioCodec::Flac, 44100, None);
    for row in cases {
        let m = measurements(row.levels, row.guards);
        let decision = decide_track(&stream, &m, &params);
        let (verdict, reason, cutoff) = row.outcome;
        assert_eq!(decision.verdict, verdict, "{}", row.case);
        assert_eq!(decision.reasons, reason.as_slice(), "{}", row.case);
        assert_eq!(
            cutoff_estimate(&m, &params).frequency_hz,
            cutoff,
            "{}",
            row.case
        );
        assert_items(
            &fake_evidence(&m, &params, stream.codec, None, decision.verdict),
            row.items,
            row.case,
        );
    }
}

struct LossyRow {
    case: &'static str,
    input: (AudioCodec, u32, u32),
    levels: Levels,
    expected_cutoff: f64,
    outcome: (Verdict, Option<SuspicionReason>, Option<u32>),
    items: &'static [Item],
}

// @RF-17 @RF-15 Scenario Outline: Evidence of a lossy track from given measurements.
#[test]
fn rf17_lossy_evidence_describes_shortfall_and_step_only() {
    use AudioCodec::{Aac, Mp3, Opus};
    let cases = [
        LossyRow {
            case: "MP3 320 from 128",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 0.0, 40.0, 17050.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Fake, None, Some(128)),
            items: &[
                (Shortfall, 3450.0, 2500.0, 950.0),
                (CutoffStep, 40.0, 10.0, 30.0),
            ],
        },
        LossyRow {
            case: "large G adds no gap item",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 19.0, 40.0, 17050.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Fake, None, Some(128)),
            items: &[
                (Shortfall, 3450.0, 2500.0, 950.0),
                (CutoffStep, 40.0, 10.0, 30.0),
            ],
        },
        LossyRow {
            case: "both at threshold",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 0.0, 10.0, 18000.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Fake, None, Some(160)),
            items: &[
                (Shortfall, 2500.0, 2500.0, 0.0),
                (CutoffStep, 10.0, 10.0, 0.0),
            ],
        },
        LossyRow {
            case: "shortfall below threshold",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 0.0, 40.0, 18001.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Suspicious, Some(CutoffBelowDeclared), Some(160)),
            items: &[],
        },
        LossyRow {
            case: "step below threshold",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 0.0, 9.99, 17050.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Suspicious, Some(CutoffBelowDeclared), Some(128)),
            items: &[],
        },
        LossyRow {
            case: "large shortfall without a step",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 0.0, 3.0, 18050.0, 17000.0),
            expected_cutoff: 20500.0,
            outcome: (Suspicious, Some(GradualRolloff), Some(128)),
            items: &[],
        },
        LossyRow {
            case: "AAC 256 from 128",
            input: (Aac, 44100, 256),
            levels: (-20.0, 0.0, 40.0, 15350.0, 22050.0),
            expected_cutoff: 19000.0,
            outcome: (Fake, None, Some(128)),
            items: &[
                (Shortfall, 3650.0, 2500.0, 1150.0),
                (CutoffStep, 40.0, 10.0, 30.0),
            ],
        },
        LossyRow {
            case: "Opus 160 from a 16 kHz source",
            input: (Opus, 48000, 160),
            levels: (-20.0, 0.0, 33.0, 16050.0, 24000.0),
            expected_cutoff: 20000.0,
            outcome: (Fake, None, None),
            items: &[
                (Shortfall, 3950.0, 2500.0, 1450.0),
                (CutoffStep, 33.0, 10.0, 23.0),
            ],
        },
        LossyRow {
            case: "honest MP3 320",
            input: (Mp3, 44100, 320),
            levels: (-20.0, 0.0, 40.0, 20450.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Real, None, Some(320)),
            items: &[],
        },
        LossyRow {
            case: "48 kHz streaming rip",
            input: (Mp3, 48000, 320),
            levels: (-20.0, 0.0, 40.0, 20050.0, 24000.0),
            expected_cutoff: 20500.0,
            outcome: (Suspicious, Some(StreamingSampleRate), None),
            items: &[],
        },
        LossyRow {
            case: "little treble",
            input: (Mp3, 44100, 320),
            levels: (-65.0, 0.0, 40.0, 17050.0, 22050.0),
            expected_cutoff: 20500.0,
            outcome: (Suspicious, Some(LowHighFrequencyContent), None),
            items: &[],
        },
    ];
    let params = DetectionParams::default();
    for row in cases {
        let (codec, rate, b) = row.input;
        let stream = stream(codec, rate, Some(b));
        let m = measurements(row.levels, (0.0, 0.0, 0.0, 0.0));
        let decision = decide_track(&stream, &m, &params);
        let (verdict, reason, estimate) = row.outcome;
        assert_eq!(decision.verdict, verdict, "{}", row.case);
        assert_eq!(decision.reasons, reason.as_slice(), "{}", row.case);
        assert_eq!(
            decision.estimated_source_bitrate_kbps, estimate,
            "{}",
            row.case
        );
        let expected_cutoff = expected_cutoff_hz(&stream, &params);
        assert_eq!(expected_cutoff, Some(row.expected_cutoff), "{}", row.case);
        let shortfall = expected_cutoff
            .map(|expected| expected - f64::from(cutoff_estimate(&m, &params).frequency_hz));
        assert_items(
            &fake_evidence(&m, &params, codec, shortfall, decision.verdict),
            row.items,
            row.case,
        );
    }
}

// @RF-17 @RF-15 Scenarios: The shortfall evidence follows the existing L3 tolerance;
// A positive shortfall margin is computed before f32 conversion.
#[test]
fn rf17_shortfall_margin_respects_tolerance_and_preserves_f64_precision() {
    let stream = stream(AudioCodec::Mp3, 44100, Some(320));
    let m = measurements((-20.0, 0.0, 40.0, 18050.0, 22050.0), (0.0, 0.0, 0.0, 0.0));
    let cases = [
        (2450.0_f64, Fake, None, Some(0.0_f32)),
        (2450.0000000005, Fake, None, Some(0.0)),
        (2450.000000002, Suspicious, Some(CutoffBelowDeclared), None),
        (
            2449.9999999995,
            Fake,
            None,
            Some((2450.0_f64 - 2449.9999999995_f64) as f32),
        ),
    ];
    for (threshold, verdict, reason, margin) in cases {
        let mut params = DetectionParams::default();
        params
            .lossy
            .as_mut()
            .expect("lossy defaults")
            .fake_shortfall_hz = threshold;
        params.validate().expect("valid params");
        let expected_cutoff = expected_cutoff_hz(&stream, &params).expect("declared cutoff");
        let cutoff = cutoff_estimate(&m, &params).frequency_hz;
        let shortfall = expected_cutoff - f64::from(cutoff);
        assert_eq!(expected_cutoff, 20500.0);
        assert_eq!(cutoff, 18050.0);
        assert_eq!(shortfall, 2450.0);

        let decision = decide_track(&stream, &m, &params);
        assert_eq!(decision.verdict, verdict, "threshold {threshold}");
        assert_eq!(decision.reasons, reason.as_slice());
        assert_eq!(decision.estimated_source_bitrate_kbps, Some(160));
        let evidence = fake_evidence(&m, &params, stream.codec, Some(shortfall), decision.verdict);
        match margin {
            Some(expected) => {
                assert_eq!(evidence.len(), 2);
                let item = &evidence[0];
                assert_eq!(item.kind, Shortfall);
                assert_eq!(item.value.to_bits(), (shortfall as f32).to_bits());
                assert_eq!(item.threshold.to_bits(), (threshold as f32).to_bits());
                assert_eq!(
                    item.margin.to_bits(),
                    expected.to_bits(),
                    "threshold {threshold}"
                );
                assert_items(&evidence[1..], &[(CutoffStep, 40.0, 10.0, 30.0)], "L3 step");
            }
            None => assert!(evidence.is_empty(), "threshold {threshold}"),
        }
    }
}

// @RF-17 Scenario Outline: Evidence fields saturate independently after the margin calculation.
#[test]
fn rf17_saturation_bounds_the_fields_after_calculating_margin() {
    let m = f64::from(f32::MAX);
    let stream = stream(AudioCodec::Flac, 44100, None);
    for (gap, threshold, value, sent_threshold, margin) in [
        (9.6, -1e40, 9.6_f32, -f32::MAX, f32::MAX),
        (2.0 * m, m, f32::MAX, f32::MAX, f32::MAX),
        (-2.0 * m, -3.0 * m, -f32::MAX, -f32::MAX, f32::MAX),
        (m, m, f32::MAX, f32::MAX, 0.0),
        (-m, -m, -f32::MAX, -f32::MAX, 0.0),
        (f64::MAX, -f64::MAX, f32::MAX, -f32::MAX, f32::MAX),
        (9.6, f64::NEG_INFINITY, 9.6, -f32::MAX, f32::MAX),
        (f64::INFINITY, f64::INFINITY, f32::MAX, f32::MAX, 0.0),
        (
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            -f32::MAX,
            -f32::MAX,
            0.0,
        ),
    ] {
        let mut params = DetectionParams::default();
        params.verdict.gap_suspicious_db = threshold;
        params.verdict.gap_fake_db = threshold;
        params.validate().expect("valid params");
        let measurements =
            measurements((-20.0, gap, 5.0, 16050.0, 22050.0), (-90.0, 0.2, 0.0, 0.0));
        let decision = decide_track(&stream, &measurements, &params);
        assert_eq!(decision.verdict, Fake, "G={gap}, threshold={threshold}");
        assert!(decision.reasons.is_empty());
        assert_eq!(
            cutoff_estimate(&measurements, &params).frequency_hz,
            22050.0
        );
        let evidence = fake_evidence(&measurements, &params, stream.codec, None, decision.verdict);
        assert_eq!(evidence.len(), 1);
        let item = &evidence[0];
        assert_eq!(item.kind, GapStep);
        assert_eq!(item.value.to_bits(), value.to_bits());
        assert_eq!(item.threshold.to_bits(), sent_threshold.to_bits());
        assert_eq!(item.margin.to_bits(), margin.to_bits());
    }
}

// @RF-17 Scenario: Evidence saturation follows the original verdict comparison.
#[test]
fn rf17_equal_saturated_fields_do_not_make_the_original_comparison_tie() {
    let m = f64::from(f32::MAX);
    let stream = stream(AudioCodec::Flac, 44100, None);
    let mut params = DetectionParams::default();
    params.verdict.gap_suspicious_db = 2.0 * m;
    params.verdict.gap_fake_db = 2.0 * m;
    params.validate().expect("valid params");
    let measurements = measurements((-20.0, m, 5.0, 16050.0, 22050.0), (-90.0, 0.2, 0.0, 0.0));
    let decision = decide_track(&stream, &measurements, &params);
    assert_eq!(decision.verdict, Real);
    assert!(decision.reasons.is_empty());
    assert_eq!(
        cutoff_estimate(&measurements, &params).frequency_hz,
        22050.0
    );
    assert!(fake_evidence(&measurements, &params, stream.codec, None, decision.verdict).is_empty());
}

// ---- X8 (issue #146): a Fake never has an empty evidence ------------------------------------

/// The parameters of `specs/detection/detection-params.v{version}.toml`, read from the spec.
fn params_file(version: u32) -> DetectionParams {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../specs/detection/detection-params.v{version}.toml"
    ));
    let text = std::fs::read_to_string(&path).expect("the params file of the spec");
    let params: DetectionParams = toml::from_str(&text).expect("a params file the core loads");
    params.validate().expect("valid params");
    assert_eq!(params.algorithm_version, version, "{}", path.display());
    params
}

/// The default parameters and those of every params file the core loads (v2 to v8).
fn every_params() -> Vec<(String, DetectionParams)> {
    let mut sets = vec![("the defaults".to_string(), DetectionParams::default())];
    for version in 2..=8 {
        sets.push((
            format!("detection-params.v{version}.toml"),
            params_file(version),
        ));
    }
    sets
}

/// Values that cross every threshold: one below the lowest, each threshold, the midpoint between
/// two consecutive thresholds and one above the highest. So every value is below, at or above each
/// threshold, and the product of several axes holds every combination of those positions.
fn crossing(thresholds: &[f64], delta: f64) -> Vec<f64> {
    let mut sorted = thresholds.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup();
    let (Some(&lowest), Some(&highest)) = (sorted.first(), sorted.last()) else {
        return Vec::new();
    };
    let mut values = vec![lowest - delta];
    for pair in sorted.windows(2) {
        values.push(pair[0]);
        values.push((pair[0] + pair[1]) / 2.0);
    }
    values.push(highest);
    values.push(highest + delta);
    values
}

/// `D`, `fs`, `D_Q` and `R`: a cutoff step and the edge measurements at its `fs`.
type Step = (f64, Option<f64>, Option<f64>, Option<f64>);

fn grid_measurements(
    (h, g, fe): (f64, f64, f64),
    (u, b): (Option<f64>, Option<f64>),
    (d, fs, dq, r): Step,
) -> Measurements {
    Measurements {
        noise_floor_db: -95.0,
        high_frequency_db: h,
        extent_hz: fe,
        step_db: d,
        step_hz: fs,
        gap_db: g,
        upper_level_db: u,
        upper_burst_db: b,
        median_step_db: dq,
        top_floor_db: None,
        residual_db: r,
    }
}

/// Every lossless measurement of the grid: `H`, `G`, `D`, `fs`, `fe`, `U`, `B`, `D_Q` and `R` across
/// every threshold of `[verdict]`, `[full_band]` and `[edge]` that these params have. As `measure`
/// gives them: no step means `D = 0` and no `fs`, `D_Q` or `R`; `U` and `B` are defined together.
fn for_each_lossless(params: &DetectionParams, mut visit: impl FnMut(&Measurements)) {
    let v = &params.verdict;
    let hs = crossing(&[v.low_hf_threshold_db], 0.5);
    let gs = crossing(&[v.gap_fake_db, v.gap_suspicious_db], 0.5);
    let mut step_thresholds = vec![
        v.step_fake_db,
        v.step_suspicious_db,
        v.step_low_suspicious_db,
        v.step_high_suspicious_db,
    ];
    step_thresholds.extend(params.full_band.as_ref().and_then(|f| f.max_lossy_step_db));
    step_thresholds.extend(params.edge.as_ref().map(|e| e.max_step_db));
    let ds = crossing(&step_thresholds, 0.5);
    let fss = crossing(
        &[
            v.lossy_min_cutoff_hz,
            v.fake_max_cutoff_hz,
            v.ambiguous_max_cutoff_hz,
        ],
        1.0,
    );
    let fes = crossing(&[v.gradual_min_cutoff_hz], 1.0);
    let mut upper = vec![(None, None)];
    if let Some(fb) = &params.full_band {
        for u in crossing(&[fb.min_level_db], 0.5) {
            for b in crossing(&[fb.min_burst_db], 0.5) {
                upper.push((Some(u), Some(b)));
            }
        }
    }
    let mut edge = Vec::new();
    match &params.edge {
        Some(e) => {
            for dq in crossing(&[e.max_median_step_db], 0.5) {
                edge.push((Some(dq), None));
                for r in crossing(&[e.min_residual_db], 0.5) {
                    edge.push((Some(dq), Some(r)));
                }
            }
        }
        None => edge.push((None, None)),
    }
    let mut steps: Vec<Step> = vec![(0.0, None, None, None)];
    for &fs in &fss {
        for &d in &ds {
            for &(dq, r) in &edge {
                steps.push((d, Some(fs), dq, r));
            }
        }
    }
    for &h in &hs {
        for &g in &gs {
            for &fe in &fes {
                for &guards in &upper {
                    for &step in &steps {
                        visit(&grid_measurements((h, g, fe), guards, step));
                    }
                }
            }
        }
    }
}

fn cutoff_rows(codec: AudioCodec, params: &DetectionParams) -> Option<&[CutoffRow]> {
    let cutoffs = &params.lossy.as_ref()?.cutoffs;
    match codec {
        AudioCodec::Mp3 => Some(&cutoffs.mp3),
        AudioCodec::Aac => Some(&cutoffs.aac),
        AudioCodec::Opus => cutoffs.opus.as_deref(),
        AudioCodec::Pcm | AudioCodec::Flac | AudioCodec::Alac => None,
    }
}

/// The streams of a lossy codec: declared bitrates around the table and the streaming minimum (and
/// none), the sample rates of the streaming rule and 44.1 kHz, and LAME low-passes below and at the
/// first row and inside the table (and none).
fn lossy_streams(codec: AudioCodec, params: &DetectionParams) -> Vec<StreamInfo> {
    let streaming = params.lossy.as_ref().and_then(|l| l.streaming.as_ref());
    let mut bitrates = vec![None];
    let mut lowpasses = vec![None];
    match cutoff_rows(codec, params) {
        Some(rows) if !rows.is_empty() => {
            let (first, middle, last) = (&rows[0], &rows[rows.len() / 2], &rows[rows.len() - 1]);
            for b in [
                first.bitrate_kbps - 1,
                first.bitrate_kbps,
                middle.bitrate_kbps,
                last.bitrate_kbps,
            ] {
                bitrates.push(Some(b));
            }
            lowpasses.push(Some(first.cutoff_hz as u32 - 1));
            lowpasses.push(Some(first.cutoff_hz as u32));
            lowpasses.push(Some(middle.cutoff_hz as u32));
        }
        _ => bitrates.extend([Some(128), Some(320)]),
    }
    let mut rates = vec![44100];
    if let Some(st) = streaming {
        let min = st.min_declared_bitrate_kbps;
        bitrates.extend([Some(min - 1), Some(min), Some(min + 1)]);
        rates.extend(st.sample_rates_hz.iter().copied());
    } else {
        rates.push(48000);
    }
    rates.sort_unstable();
    rates.dedup();
    let mut streams = Vec::new();
    for &sample_rate in &rates {
        for &declared_bitrate_kbps in &bitrates {
            for &encoder_lowpass_hz in &lowpasses {
                streams.push(StreamInfo {
                    codec,
                    sample_rate,
                    declared_bitrate_kbps,
                    encoder_lowpass_hz,
                });
            }
        }
    }
    streams
}

/// Every lossy measurement of the grid for one stream: the reported cutoff `fc` (as `fs` with a
/// step that counts, or as `fe`) across the shortfall thresholds `c_exp - fake_shortfall_hz` and
/// `c_exp - suspicious_shortfall_hz` and the streaming window; `D` across `step_suspicious_db` and
/// `fake_min_step_db`; `H` across `low_hf_threshold_db`; `G` across `gap_fake_db`. Every value is a
/// multiple of 0.5 Hz below 2^23, so the reported cutoff is exact in `f32`.
fn for_each_lossy(
    params: &DetectionParams,
    stream: &StreamInfo,
    mut visit: impl FnMut(&Measurements),
) {
    let lossy = params.lossy.as_ref();
    let mut cutoffs = Vec::new();
    if let (Some(l), Some(c)) = (lossy, expected_cutoff_hz(stream, params)) {
        cutoffs.extend([c - l.fake_shortfall_hz, c - l.suspicious_shortfall_hz]);
    }
    if let Some(st) = lossy.and_then(|l| l.streaming.as_ref()) {
        cutoffs.extend([st.min_cutoff_hz, st.max_cutoff_hz]);
    }
    if cutoffs.is_empty() {
        cutoffs.push(16000.0);
    }
    let fcs = crossing(&cutoffs, 1.0);
    let mut step_thresholds = vec![params.verdict.step_suspicious_db];
    step_thresholds.extend(lossy.map(|l| l.fake_min_step_db));
    let ds = crossing(&step_thresholds, 0.5);
    let hs = crossing(&[params.verdict.low_hf_threshold_db], 0.5);
    let gs = crossing(&[params.verdict.gap_fake_db], 0.5);
    let mut steps: Vec<Step> = vec![(0.0, None, None, None)];
    for &fs in &fcs {
        for &d in &ds {
            steps.push((d, Some(fs), None, None));
        }
    }
    for &h in &hs {
        for &g in &gs {
            for &fe in &fcs {
                for &step in &steps {
                    visit(&grid_measurements((h, g, fe), (None, None), step));
                }
            }
        }
    }
}

#[derive(Default, Debug)]
struct Tally {
    total: usize,
    fake: usize,
    suspicious: usize,
    real: usize,
    error: usize,
    gap_only: usize,
    step_only: usize,
    both: usize,
}

impl Tally {
    fn count(&mut self, verdict: Verdict, evidence: &[FakeEvidence]) {
        self.total += 1;
        match verdict {
            Verdict::Fake => self.fake += 1,
            Verdict::Suspicious => self.suspicious += 1,
            Verdict::Real => self.real += 1,
            Verdict::Error => self.error += 1,
        }
        match evidence {
            [a] if a.kind == GapStep => self.gap_only += 1,
            [a] if a.kind == CutoffStep => self.step_only += 1,
            [a, b] if a.kind == GapStep && b.kind == CutoffStep => self.both += 1,
            _ => {}
        }
    }
}

/// Section 2.4: not empty exactly for `Fake`; a lossless `Fake` has one or two items, a lossy one
/// exactly two.
fn assert_fake_has_evidence(
    verdict: Verdict,
    evidence: &[FakeEvidence],
    lossy: bool,
    case: impl Fn() -> String,
) {
    let fake = verdict == Fake;
    assert!(
        evidence.is_empty() != fake,
        "{}: verdict {verdict:?} with evidence {evidence:?}. A Fake must have evidence and only a \
         Fake has it: a rule that makes a track Fake defines its evidence in the same change \
         (fake-evidence.md section 2.5, issue #146)",
        case()
    );
    if fake {
        let expected = if lossy { 2..=2 } else { 1..=2 };
        assert!(
            expected.contains(&evidence.len()),
            "{}: {verdict:?} with evidence {evidence:?}",
            case()
        );
    }
}

// @RF-17 @RF-04 @RF-15 Scenario: A Fake never has an empty evidence, on every verdict path
// (issue #146). Criterion X8 of specs/detection/fake-evidence.md: the lossless verdict function
// (`decide`, and `decide_track` for pcm, flac and alac) and the lossy one (`decide_track` for mp3,
// aac and opus), then `fake_evidence`, over measurements that cross every threshold of the default
// params and of each params file v2 to v8, in every combination.
#[test]
fn rf17_x8_a_fake_never_has_an_empty_evidence_on_any_verdict_path() {
    use AudioCodec::{Aac, Alac, Flac, Mp3, Opus, Pcm};
    for (name, params) in every_params() {
        let mut lossless = [Tally::default(), Tally::default(), Tally::default()];
        for_each_lossless(&params, |m| {
            let (verdict, _) = decide(m, &params);
            for (tally, codec) in lossless.iter_mut().zip([Pcm, Flac, Alac]) {
                let decision = decide_track(&stream(codec, 44100, None), m, &params);
                assert_eq!(decision.verdict, verdict, "{name}, {codec:?}: {m:?}");
                let evidence = fake_evidence(m, &params, codec, None, decision.verdict);
                assert_fake_has_evidence(decision.verdict, &evidence, false, || {
                    format!("{name}, {codec:?}, {m:?}")
                });
                tally.count(decision.verdict, &evidence);
            }
        });
        for (tally, codec) in lossless.iter().zip([Pcm, Flac, Alac]) {
            // The grid reaches every lossless verdict and every evidence shape.
            assert!(
                tally.fake > 0
                    && tally.suspicious > 0
                    && tally.real > 0
                    && tally.gap_only > 0
                    && tally.step_only > 0
                    && tally.both > 0
                    && tally.error == 0,
                "{name}, {codec:?}: {tally:?}"
            );
        }

        for codec in [Mp3, Aac, Opus] {
            let mut tally = Tally::default();
            for stream in lossy_streams(codec, &params) {
                for_each_lossy(&params, &stream, |m| {
                    let decision = decide_track(&stream, m, &params);
                    // As analyze_track: the shortfall from the expected and the reported cutoff.
                    let shortfall = expected_cutoff_hz(&stream, &params).map(|expected| {
                        expected - f64::from(cutoff_estimate(m, &params).frequency_hz)
                    });
                    let evidence = fake_evidence(m, &params, codec, shortfall, decision.verdict);
                    assert_fake_has_evidence(decision.verdict, &evidence, true, || {
                        format!("{name}, {stream:?}, {m:?}")
                    });
                    tally.count(decision.verdict, &evidence);
                });
            }
            if lossy_unsupported(codec, &params).is_some() {
                // L0: these params cannot judge the codec.
                assert_eq!(tally.error, tally.total, "{name}, {codec:?}: {tally:?}");
            } else {
                assert!(
                    tally.fake > 0 && tally.suspicious > 0 && tally.real > 0,
                    "{name}, {codec:?}: {tally:?}"
                );
            }
        }
    }
}

/// A fake of the analyzer port that returns one probe for every file.
struct FixedProbe(Result<AudioProbe, TrackError>);

impl SpectrumAnalyzer for FixedProbe {
    fn analyze(&self, _: &Path, _: &DetectionParams) -> Result<AudioProbe, TrackError> {
        self.0.clone()
    }
}

// @RF-17 @RF-04 @RF-15 @RNF-04 Scenario: A Fake never has an empty evidence, on every verdict path
// (issue #146), the paths without measurements: a silent track (every frame below the silence
// level), a track too short, a lossy track without a declared bitrate, L0 and an unreadable file,
// through `analyze_track` with a fake of the analyzer port, for every codec and params set.
#[test]
fn rf17_x8_silent_and_error_paths_have_no_evidence() {
    use AudioCodec::{Aac, Alac, Flac, Mp3, Opus, Pcm};
    let formats = [
        (Pcm, AudioFormat::Wav),
        (Flac, AudioFormat::Flac),
        (Alac, AudioFormat::M4a),
        (Mp3, AudioFormat::Mp3),
        (Aac, AudioFormat::M4a),
        (Opus, AudioFormat::Opus),
    ];
    for (name, params) in every_params() {
        let min_ms = (params.input.min_duration_s * 1000.0) as u64;
        for (codec, format) in formats {
            let entry = FileEntry {
                path: PathBuf::from(format!("Álbum/01 – {codec:?}")),
                folder: PathBuf::from("Álbum"),
                format,
                size_bytes: 1,
                modified_unix: 0,
            };
            let rate = if codec == Opus { 48000 } else { 44100 };
            let mut probes = vec![Err(TrackError::Unreadable("fake".into()))];
            for duration_ms in [min_ms - 1, min_ms, min_ms + 1, 60_000] {
                for declared_bitrate_kbps in [None, Some(160), Some(320)] {
                    probes.push(Ok(AudioProbe {
                        sample_rate: rate,
                        bit_depth: None,
                        channels: 2,
                        duration_ms,
                        codec: Some(codec),
                        declared_bitrate_kbps,
                        bitrate_mode: None,
                        encoder: None,
                        encoder_lowpass_hz: None,
                        spectrum: None,
                    }));
                }
            }
            let mut silent = 0;
            for probe in probes {
                let result = analyze_track(&FixedProbe(probe.clone()), &entry, &params);
                assert_fake_has_evidence(
                    result.verdict,
                    &result.evidence,
                    codec.is_lossy(),
                    || format!("{name}, {codec:?}, {probe:?}"),
                );
                if result.verdict == Suspicious {
                    silent += 1;
                }
            }
            // The silent path is reached whenever these params can judge the codec.
            assert_eq!(
                silent > 0,
                lossy_unsupported(codec, &params).is_none(),
                "{name}, {codec:?}"
            );
        }
    }
}

// ---- X9 (issue #147): the "16 kHz" of the gap texts -----------------------------------------

/// The English and Spanish cells of `key` in section 6 of `specs/ui/fake-evidence.md`, read as
/// `src/lib/i18n/catalogue.test.ts` reads them (that test checks that the app's catalogue equals
/// them in both languages).
fn evidence_text(key: &str) -> (String, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/ui/fake-evidence.md");
    let spec = std::fs::read_to_string(path).expect("specs/ui/fake-evidence.md");
    let section = spec
        .split("## 6. Exact new translations")
        .nth(1)
        .and_then(|rest| rest.split("## 7. Acceptance").next())
        .expect("section 6 of specs/ui/fake-evidence.md");
    let prefix = format!("| `{key}` |");
    let row = section
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .unwrap_or_else(|| panic!("the row {key} of specs/ui/fake-evidence.md section 6"));
    let cells: Vec<&str> = row
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    assert_eq!(cells.len(), 3, "{row}");
    (cells[1].to_string(), cells[2].to_string())
}

// @RF-17 @RF-04 Scenario: The "16 kHz" of the gap texts is the start of the gap band of the default
// parameters (issue #147). Criterion X9 of specs/detection/fake-evidence.md.
#[test]
fn rf17_x9_the_16_khz_of_the_gap_texts_is_the_default_gap_edge() {
    let edge_hz = DetectionParams::default().gaps.edge_hz;
    assert_eq!(
        edge_hz, 16000.0,
        "the default gaps.edge_hz is {edge_hz} Hz, but the texts evidence.gapStep and \
         evidence.gapOnly of specs/ui/fake-evidence.md section 6 say \"16 kHz\" in English and \
         Spanish: a params version that moves gaps.edge_hz must change those two texts at G1 \
         (fake-evidence.md Q12 and section 7, issue #147)"
    );
    for key in ["evidence.gapStep", "evidence.gapOnly"] {
        let (english, spanish) = evidence_text(key);
        for (language, text) in [("English", english), ("Spanish", spanish)] {
            assert!(
                text.contains("16 kHz"),
                "{key} ({language}) of specs/ui/fake-evidence.md section 6 must name the gap \
                 edge of the default params, 16 kHz: {text}"
            );
        }
    }
}

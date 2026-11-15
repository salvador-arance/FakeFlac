//! @RF-17: Given measurements and the existing verdict, report the applicable evidence in order.
//! Scenarios: specs/features/fake-evidence.feature; calculation: specs/detection/fake-evidence.md.
#![allow(clippy::expect_used)]

use fakeflac_core::detect::{
    cutoff_estimate, decide_track, expected_cutoff_hz, fake_evidence, Measurements, StreamInfo,
};
use fakeflac_core::domain::{EvidenceKind, FakeEvidence};
use fakeflac_core::{AudioCodec, DetectionParams, SuspicionReason, Verdict};
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

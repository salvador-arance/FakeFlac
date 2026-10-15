//! Contract conversions (core -> DTO) and JSON shape of `specs/contracts/commands.md`
//! (RF-01, RF-05, RF-06, RNF-09). Protected after the `test(RF-01,RF-05)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core as core;
use fakeflac_lib::dto;
use serde_json::{json, Value};
use std::path::PathBuf;

const UNICODE_PATH: &str = "Álbum ″Ñandú″/01 – Canción ‘x’.flac";

fn info(path: &str, format: core::AudioFormat) -> core::TrackInfo {
    let p = PathBuf::from(path);
    core::TrackInfo {
        folder: p.parent().map(PathBuf::from).unwrap_or_default(),
        path: p,
        format,
        sample_rate: 44100,
        bit_depth: Some(16),
        channels: 2,
        declared_bitrate_kbps: None,
        duration_ms: 6000,
        size_bytes: 1234,
        modified_unix: 1_700_000_000,
    }
}

fn track(verdict: core::Verdict, error: Option<core::TrackError>) -> core::TrackAnalysis {
    core::TrackAnalysis {
        info: info(UNICODE_PATH, core::AudioFormat::Flac),
        verdict,
        cutoff: None,
        estimated_source_bitrate_kbps: None,
        reasons: vec![],
        error,
        algorithm_version: 1,
    }
}

fn track_json(t: &core::TrackAnalysis) -> Value {
    serde_json::to_value(dto::TrackAnalysis::from(t)).expect("serialize")
}

fn summary() -> core::AnalysisSummary {
    core::AnalysisSummary {
        root: PathBuf::from("C:/Music"),
        total: 4,
        done: 3,
        real: 1,
        suspicious: 1,
        fake: 0,
        errors: 1,
        cancelled: false,
        elapsed_ms: 250,
    }
}

fn summary_json() -> Value {
    json!({
        "root": "C:/Music", "total": 4, "done": 3, "real": 1, "suspicious": 1,
        "fake": 0, "errors": 1, "cancelled": false, "elapsedMs": 250
    })
}

fn event_json(e: core::ProgressEvent) -> Value {
    serde_json::to_value(dto::ProgressEvent::from(&e)).expect("serialize")
}

// @RF-06 @RNF-09
#[test]
fn track_analysis_json_matches_the_contract() {
    let mut t = track(core::Verdict::Fake, None);
    t.cutoff = Some(core::CutoffEstimate {
        frequency_hz: 16000.0,
        steepness_db_per_khz: 40.5,
        noise_floor_db: -90.0,
    });
    t.reasons = vec![core::SuspicionReason::GradualRolloff];
    let expected = json!({
        "info": {
            "path": UNICODE_PATH,
            "folder": "Álbum ″Ñandú″",
            "format": "flac",
            "sampleRate": 44100,
            "bitDepth": 16,
            "channels": 2,
            "declaredBitrateKbps": null,
            "durationMs": 6000,
            "sizeBytes": 1234,
            "modifiedUnix": 1_700_000_000
        },
        "verdict": "fake",
        "cutoff": { "frequencyHz": 16000.0, "steepnessDbPerKhz": 40.5, "noiseFloorDb": -90.0 },
        "estimatedSourceBitrateKbps": null,
        "reasons": ["gradualRolloff"],
        "error": null,
        "algorithmVersion": 1
    });
    assert_eq!(track_json(&t), expected);
}

// @RNF-04
#[test]
fn track_errors_map_to_tagged_objects() {
    let cases = [
        (core::TrackError::TooShort, json!({"kind": "tooShort"})),
        (
            core::TrackError::CorruptStream("x".into()),
            json!({"kind": "corruptStream", "detail": "x"}),
        ),
        (
            core::TrackError::Unreadable("y".into()),
            json!({"kind": "unreadable", "detail": "y"}),
        ),
        (
            core::TrackError::UnsupportedFormat("z".into()),
            json!({"kind": "unsupportedFormat", "detail": "z"}),
        ),
    ];
    for (err, expected) in cases {
        let t = track(core::Verdict::Error, Some(err));
        let v = track_json(&t);
        assert_eq!(v["verdict"], "error");
        assert_eq!(v["error"], expected);
    }
}

#[test]
fn verdicts_map_for_every_variant() {
    use core::Verdict as V;
    for (v, s) in [
        (V::Real, "real"),
        (V::Suspicious, "suspicious"),
        (V::Fake, "fake"),
        (V::Error, "error"),
    ] {
        assert_eq!(track_json(&track(v, None))["verdict"], s);
    }
}

#[test]
fn formats_map_for_every_variant() {
    use core::AudioFormat as F;
    for (f, s) in [
        (F::Wav, "wav"),
        (F::Flac, "flac"),
        (F::Mp3, "mp3"),
        (F::M4a, "m4a"),
        (F::Opus, "opus"),
    ] {
        let mut t = track(core::Verdict::Real, None);
        t.info.format = f;
        assert_eq!(track_json(&t)["info"]["format"], s);
    }
}

#[test]
fn reasons_map_for_every_variant() {
    use core::SuspicionReason as R;
    let mut t = track(core::Verdict::Suspicious, None);
    t.reasons = vec![
        R::LowHighFrequencyContent,
        R::GradualRolloff,
        R::AmbiguousCutoff,
    ];
    assert_eq!(
        track_json(&t)["reasons"],
        json!([
            "lowHighFrequencyContent",
            "gradualRolloff",
            "ambiguousCutoff"
        ])
    );
}

// @RF-05
#[test]
fn progress_events_match_the_contract() {
    assert_eq!(
        event_json(core::ProgressEvent::Scanned { total: 4 }),
        json!({"kind": "scanned", "total": 4})
    );
    assert_eq!(
        event_json(core::ProgressEvent::TrackStarted {
            index: 0,
            path: PathBuf::from("C:/Music/a.flac")
        }),
        json!({"kind": "trackStarted", "index": 0, "path": "C:/Music/a.flac"})
    );
    let t = track(core::Verdict::Real, None);
    let done = event_json(core::ProgressEvent::TrackDone {
        index: 2,
        analysis: t.clone(),
    });
    assert_eq!(
        done,
        json!({"kind": "trackDone", "index": 2, "analysis": track_json(&t)})
    );
    assert_eq!(
        event_json(core::ProgressEvent::Finished { summary: summary() }),
        json!({"kind": "finished", "summary": summary_json()})
    );
    let mut s = summary();
    s.cancelled = true;
    let mut sj = summary_json();
    sj["cancelled"] = json!(true);
    assert_eq!(
        event_json(core::ProgressEvent::Cancelled { summary: s }),
        json!({"kind": "cancelled", "summary": sj})
    );
}

#[test]
fn summary_json_uses_camel_case_and_string_root() {
    let v = serde_json::to_value(dto::AnalysisSummary::from(&summary())).expect("serialize");
    assert_eq!(v, summary_json());
}

#[test]
fn app_errors_map_every_code() {
    use core::AppErrorCode as C;
    for (c, s) in [
        (C::NoAudioFound, "NO_AUDIO_FOUND"),
        (C::FolderUnreadable, "FOLDER_UNREADABLE"),
        (C::AnalysisRunning, "ANALYSIS_RUNNING"),
        (C::NotAnalyzed, "NOT_ANALYZED"),
        (C::ExportFailed, "EXPORT_FAILED"),
        (C::Internal, "INTERNAL"),
    ] {
        let e = dto::AppError::from(core::AppError::new(c, "msg"));
        assert_eq!(
            serde_json::to_value(e).expect("serialize"),
            json!({"code": s, "message": "msg"})
        );
    }
}

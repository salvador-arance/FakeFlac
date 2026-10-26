//! Contract version 6 (`specs/contracts/commands.md`, "Streaming sample rate (version 6)", RF-16):
//! the core's `SuspicionReason::StreamingSampleRate` maps to the DTO and serialises as
//! `streamingSampleRate`; a flagged track carries `estimatedSourceBitrateKbps` null and
//! `algorithmVersion` 7. Supports the scenarios of `specs/features/detection-streaming-source.feature`
//! that read the contract identifier (the real run, the CSV record, the table and the detail line).
//!
//! Names chosen by the test author: `fakeflac_core::SuspicionReason::StreamingSampleRate` and
//! `dto::SuspicionReason::StreamingSampleRate`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core as core;
use fakeflac_lib::dto;
use serde_json::{json, Value};
use std::path::PathBuf;

fn flagged() -> core::TrackAnalysis {
    let p = PathBuf::from("Álbum ″Ñandú″/01 – Canción ‘x’.mp3");
    core::TrackAnalysis {
        info: core::TrackInfo {
            folder: p.parent().map(PathBuf::from).unwrap_or_default(),
            path: p,
            format: core::AudioFormat::Mp3,
            codec: Some(core::AudioCodec::Mp3),
            sample_rate: 48_000,
            bit_depth: None,
            channels: 2,
            declared_bitrate_kbps: Some(320),
            bitrate_mode: Some(core::BitrateMode::Cbr),
            duration_ms: 240_000,
            size_bytes: 9_600_000,
            modified_unix: 1_700_000_000,
        },
        verdict: core::Verdict::Suspicious,
        cutoff: Some(core::CutoffEstimate {
            frequency_hz: 20_050.0,
            steepness_db_per_khz: 85.0,
            noise_floor_db: -95.0,
        }),
        estimated_source_bitrate_kbps: None,
        reasons: vec![core::SuspicionReason::StreamingSampleRate],
        error: None,
        algorithm_version: 7,
    }
}

// @RF-16 Contract version 6: the new reason, a null estimate and algorithmVersion 7.
#[test]
fn rf16_a_flagged_track_json_matches_the_contract() {
    let dto_track = dto::TrackAnalysis::from(&flagged());
    assert_eq!(
        dto_track.reasons,
        vec![dto::SuspicionReason::StreamingSampleRate]
    );
    let v: Value = serde_json::to_value(dto_track).expect("serialize");
    assert_eq!(v["verdict"], "suspicious");
    assert_eq!(v["reasons"], json!(["streamingSampleRate"]));
    assert_eq!(v["estimatedSourceBitrateKbps"], Value::Null);
    assert_eq!(v["info"]["sampleRate"], 48_000);
    assert_eq!(v["info"]["declaredBitrateKbps"], 320);
    assert_eq!(v["algorithmVersion"], 7);
}

// @RF-16 The generated bindings list the new member (`just gen`, commands.md version 6).
#[test]
fn rf16_the_committed_bindings_list_streaming_sample_rate() {
    let bindings = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/api/bindings.ts"),
    )
    .unwrap();
    let line = bindings
        .lines()
        .find(|l| l.starts_with("export type SuspicionReason ="))
        .expect("the SuspicionReason type");
    assert!(line.contains("\"streamingSampleRate\""), "{line}");
}

//! Contract version 7 (RF-11, `specs/contracts/commands.md` "Result cache (version 7)"):
//! scenario "The contract of version 7" of `specs/features/result-cache.feature`. Core -> DTO
//! conversions, their JSON, and the generated bindings. The `CacheFailed` -> `"CACHE_FAILED"`
//! pair is new coverage here, not an edit of `contract.rs` (result-cache.md 12.2).
//! Protected after the `test(RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core as core;
use fakeflac_lib::dto;
use serde_json::{json, Value};
use std::path::PathBuf;

fn summary(from_cache: u32, cache: core::CacheStatus) -> core::AnalysisSummary {
    core::AnalysisSummary {
        root: PathBuf::from("C:/Music"),
        total: 4,
        done: 4,
        real: 2,
        suspicious: 1,
        fake: 1,
        errors: 0,
        cancelled: false,
        elapsed_ms: 250,
        from_cache,
        cache,
    }
}

fn summary_json(s: &core::AnalysisSummary) -> Value {
    serde_json::to_value(dto::AnalysisSummary::from(s)).expect("serialize")
}

// @RF-11 Scenario: The contract of version 7
#[test]
fn rf11_a_summary_with_cache_figures_serialises_through_the_dto() {
    let v = summary_json(&summary(2, core::CacheStatus::Degraded));
    assert_eq!(
        v,
        json!({
            "root": "C:/Music", "total": 4, "done": 4, "real": 2, "suspicious": 1,
            "fake": 1, "errors": 0, "cancelled": false, "elapsedMs": 250,
            "fromCache": 2, "cache": "degraded"
        })
    );
}

#[test]
fn rf11_the_four_cache_statuses_serialise_camel_case() {
    for (status, text) in [
        (core::CacheStatus::Off, "off"),
        (core::CacheStatus::Ok, "ok"),
        (core::CacheStatus::Degraded, "degraded"),
        (core::CacheStatus::Unavailable, "unavailable"),
    ] {
        assert_eq!(summary_json(&summary(0, status))["cache"], text);
    }
}

#[test]
fn rf11_the_finished_event_carries_the_cache_figures() {
    let e = core::ProgressEvent::Finished {
        summary: summary(3, core::CacheStatus::Ok),
    };
    let v = serde_json::to_value(dto::ProgressEvent::from(&e)).expect("serialize");
    assert_eq!(v["kind"], "finished");
    assert_eq!(v["summary"]["fromCache"], 3);
    assert_eq!(v["summary"]["cache"], "ok");
}

#[test]
fn rf11_cache_failed_serialises_as_in_the_contract() {
    let e = dto::AppError::from(core::AppError::new(core::AppErrorCode::CacheFailed, "msg"));
    assert_eq!(
        serde_json::to_value(e).expect("serialize"),
        json!({"code": "CACHE_FAILED", "message": "msg"})
    );
    assert_eq!(core::AppErrorCode::CacheFailed.as_str(), "CACHE_FAILED");
}

#[test]
fn rf11_the_generated_bindings_have_the_version_7_contract() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/lib/api/bindings.ts");
    let ts = std::fs::read_to_string(path).expect("bindings");
    assert!(ts.contains("clearCache:"), "the command clearCache");
    assert!(ts.contains("__TAURI_INVOKE(\"clear_cache\")"));
    assert!(
        ts.contains(r#"export type CacheStatus = "off" | "ok" | "degraded" | "unavailable""#),
        "the type CacheStatus"
    );
    let summary = ts
        .split("export type AnalysisSummary = {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("AnalysisSummary");
    assert!(summary.contains("fromCache: number"), "{summary}");
    assert!(summary.contains("cache: CacheStatus"), "{summary}");
    assert!(ts.contains("\"CACHE_FAILED\""), "the error code");
}

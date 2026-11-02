//! Contract version 8 (@RF-13, `specs/contracts/settings.md` section 2 and
//! `specs/contracts/commands.md` "Version 8"): the settings DTOs, their JSON, the two new
//! `AppError` codes and the generated bindings. Scenario "Invalid IPC input cannot write a
//! language preference" of `specs/features/language-settings.feature` (typed deserialization of
//! the `settings` argument) and acceptance "Bindings/contract tests must cover contract version 8"
//! of `specs/ui/language-settings.md` section 6.
//!
//! The DTOs these tests fix, in `fakeflac_lib::dto`: `Language { En, Es }`, `Settings { language }`,
//! `SettingsLoadStatus { Default, Loaded, Invalid, Unavailable, NewerVersion }`,
//! `SettingsSnapshot { settings, load_status }` and `AppErrorCode::{SettingsWriteFailed,
//! SettingsReadOnly}`. `Settings` and `Language` deserialize (they are a command argument).
//! Protected after the `test(RF-13)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_lib::dto::{
    AppError, AppErrorCode, Language, Settings, SettingsLoadStatus, SettingsSnapshot,
};
use serde_json::{json, Value};
use std::path::PathBuf;

fn to_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("serialize")
}

// @RF-13 `type Language = 'en' | 'es'`
#[test]
fn rf13_languages_serialise_as_their_codes() {
    assert_eq!(to_json(&Language::En), json!("en"));
    assert_eq!(to_json(&Language::Es), json!("es"));
}

// @RF-13 `type Settings = { language: Language }`
#[test]
fn rf13_settings_round_trip_through_json() {
    let s = Settings {
        language: Language::Es,
    };
    assert_eq!(to_json(&s), json!({"language": "es"}));
    let back: Settings = serde_json::from_value(json!({"language": "en"})).expect("valid");
    assert_eq!(back.language, Language::En);
}

// @RF-13 Scenario Outline: Invalid IPC input cannot write a language preference
// Examples: | language fr | language null | missing language | (plus other unsupported forms)
#[test]
fn rf13_invalid_settings_arguments_are_rejected_by_deserialization() {
    for input in [
        json!({"language": "fr"}),
        json!({"language": null}),
        json!({}),
        json!({"language": "ES"}),
        json!({"language": "es-ES"}),
        json!({"language": 1}),
        json!("es"),
    ] {
        assert!(
            serde_json::from_value::<Settings>(input.clone()).is_err(),
            "rejected: {input}"
        );
    }
}

// @RF-13 `type SettingsLoadStatus = 'default' | 'loaded' | 'invalid' | 'unavailable' | 'newerVersion'`
#[test]
fn rf13_the_five_load_statuses_serialise_camel_case() {
    for (status, text) in [
        (SettingsLoadStatus::Default, "default"),
        (SettingsLoadStatus::Loaded, "loaded"),
        (SettingsLoadStatus::Invalid, "invalid"),
        (SettingsLoadStatus::Unavailable, "unavailable"),
        (SettingsLoadStatus::NewerVersion, "newerVersion"),
    ] {
        assert_eq!(to_json(&status), json!(text));
    }
}

// @RF-13 `type SettingsSnapshot = { settings: Settings; loadStatus: SettingsLoadStatus }`
#[test]
fn rf13_a_snapshot_serialises_with_camel_case_fields() {
    let s = SettingsSnapshot {
        settings: Settings {
            language: Language::Es,
        },
        load_status: SettingsLoadStatus::NewerVersion,
    };
    assert_eq!(
        to_json(&s),
        json!({"settings": {"language": "es"}, "loadStatus": "newerVersion"})
    );
}

// @RF-13 the additional AppError codes, only from set_settings
#[test]
fn rf13_the_settings_error_codes_serialise_as_in_the_contract() {
    for (code, text) in [
        (AppErrorCode::SettingsWriteFailed, "SETTINGS_WRITE_FAILED"),
        (AppErrorCode::SettingsReadOnly, "SETTINGS_READ_ONLY"),
    ] {
        let e = AppError {
            code,
            message: "technical detail".into(),
        };
        assert_eq!(
            to_json(&e),
            json!({"code": text, "message": "technical detail"})
        );
    }
}

fn block<'a>(ts: &'a str, start: &str) -> &'a str {
    ts.split(start)
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .unwrap_or_else(|| panic!("no `{start}` in the bindings"))
}

// @RF-13 the generated bindings have the version 8 contract (S3: `just gen`)
#[test]
fn rf13_the_generated_bindings_have_the_version_8_contract() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/lib/api/bindings.ts");
    let ts = std::fs::read_to_string(path)
        .expect("bindings")
        .replace("\r\n", "\n");

    assert!(ts.contains("getSettings: ()"), "the command getSettings");
    assert!(ts.contains("\"get_settings\""));
    assert!(
        ts.contains("setSettings: (settings: Settings)"),
        "the command setSettings"
    );
    assert!(ts.contains("__TAURI_INVOKE(\"set_settings\", { settings })"));

    assert!(
        ts.contains(r#"export type Language = "en" | "es""#),
        "the type Language"
    );
    assert!(
        ts.contains(
            r#"export type SettingsLoadStatus = "default" | "loaded" | "invalid" | "unavailable" | "newerVersion""#
        ),
        "the type SettingsLoadStatus"
    );
    let settings = block(&ts, "export type Settings = {");
    assert!(settings.contains("language: Language"), "{settings}");
    let snapshot = block(&ts, "export type SettingsSnapshot = {");
    assert!(snapshot.contains("settings: Settings"), "{snapshot}");
    assert!(
        snapshot.contains("loadStatus: SettingsLoadStatus"),
        "{snapshot}"
    );

    let codes = ts
        .lines()
        .find(|l| l.starts_with("export type AppErrorCode ="))
        .expect("AppErrorCode");
    assert!(codes.contains("\"SETTINGS_WRITE_FAILED\""), "{codes}");
    assert!(codes.contains("\"SETTINGS_READ_ONLY\""), "{codes}");
    for unchanged in ["\"CACHE_FAILED\"", "\"EXPORT_FAILED\"", "\"INTERNAL\""] {
        assert!(codes.contains(unchanged), "{codes}");
    }
}

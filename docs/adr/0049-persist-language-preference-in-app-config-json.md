# ADR-0049: Persist the language preference in backend-managed app configuration JSON

- Status: proposed
- Date: 2026-11-01

## Context and rationale

RF-13 requires Spanish and English. The owner confirmed the scope on 2026-11-01: language selection with a persistent preference. Historical planning also placed sampling and detection parameters in Settings; those advanced controls are deferred. The current English default and English fallback are already specified, but there is no preference storage or settings command.

The preference must survive an app restart without changing audio, detection results, CSV bytes or the result-cache engine identity. It must remain usable when configuration is missing, unreadable or from a newer app.

## Decision

Use a backend-managed UTF-8 JSON file at Tauri's `app_config_dir()/settings.json`, `%APPDATA%\io.github.fakeflac.app\settings.json` on Windows. Schema 1 contains `schemaVersion: 1` and `language: "en" | "es"`. The detailed normative behavior and generated command surface are in [the RF-13 contract](../../specs/contracts/settings.md).

Load once per app process. A missing file defaults to English without creating a file. An invalid or unreadable file defaults to English with a localized warning and is preserved. A newer schema also defaults to English, but saves are refused to protect configuration from a newer release. An explicit successful Save may replace malformed schema-1 configuration.

Serialize settings operations in the backend. Save through a temporary sibling file, flush it and atomically replace the destination; publish the new in-memory preference only after replacement succeeds. Failure preserves the previous preference and destination. Recheck the destination for a newer schema before saving. No file watcher or synchronization between independent app processes is promised.

Keep this small application preference adapter in `src-tauri`, using existing `std`, `serde` and `serde_json` facilities. No new dependency or core I/O is introduced. Components use the typed client in `src/lib/api/`; the UI applies the locale only after a successful Save. Contract version 8 adds settings commands, snapshots and errors; existing analysis commands remain unchanged.

## Alternatives considered

- Browser local storage: rejected because the preference would live in WebView storage rather than the application's explicit, versioned settings contract.
- SQLite alongside the result cache: rejected because clearing or rebuilding disposable results must not remove a preference.
- A new settings plugin or crate: unnecessary for one versioned language field with existing serialization facilities.
- Windows-language detection on each launch: rejected because it does not persist the user's explicit choice and changes the established English default.
- Advanced analysis settings in this requirement: deferred by the owner's scope decision; no detection parameter, sampling or cache-enable control is added.

## Consequences

The app gains one small configuration file, independent from music and result-cache storage. Failed reads and writes have explicit, testable recovery behavior. Unsupported future schemas are preserved. English remains the initial default; both catalogues must be complete for the specified application UI. Installer and Windows-owned dialog language remain governed by existing release and dialog specifications.

This is a G1 proposal, not an implemented feature or an approved persistence design.

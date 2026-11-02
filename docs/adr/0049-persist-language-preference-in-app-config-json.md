# ADR-0049: Persist the language preference in backend-managed app configuration JSON

- Status: accepted (specification PR #107 merged at G1, 2026-11-01); amended 2026-11-02 (Windows language source, below)
- Date: 2026-11-01

## Context and rationale

RF-13 requires Spanish and English. The owner confirmed the scope on 2026-11-01: language selection with a persistent preference. Historical planning also placed sampling and detection parameters in Settings; those advanced controls are deferred. The current English default and English fallback are already specified, but there is no preference storage or settings command.

The preference must survive an app restart without changing audio, detection results, CSV bytes or the result-cache engine identity. It must remain usable when configuration is missing, unreadable or from a newer app.

## Decision

Use a backend-managed UTF-8 JSON file at Tauri's `app_config_dir()/settings.json`, `%APPDATA%\io.github.fakeflac.app\settings.json` on Windows. Schema 1 contains `schemaVersion: 1` and `language: "en" | "es"`. The detailed normative behavior and generated command surface are in [the RF-13 contract](../../specs/contracts/settings.md).

Load once per app process. A valid saved language takes priority. Only when the file is missing, choose the first supported language in the Windows user's ordered preferred UI languages, matching the primary language subtag case-insensitively (`es-MX` selects Spanish, `en-GB` English). Unsupported entries are skipped; an empty, unavailable or unsupported list defaults to English. This detection creates no file, adds no Auto option and does not track system changes during the process. An invalid or unreadable file defaults to English with a localized warning and is preserved. A newer schema also defaults to English, but saves are refused to protect the observed configuration from a newer release. An explicit successful Save may replace malformed schema-1 configuration.

Serialize settings operations in the backend. Save through a temporary sibling file, flush it and atomically replace the destination; publish the new in-memory preference only after replacement succeeds. Failure preserves the previous preference and destination, assuming no external writer. Recheck the destination for a newer schema before saving. No file watcher or synchronization between independent app processes is promised. The version check protects the file observed at that check; another process can replace it before the atomic write, so preservation of an intervening replacement is not guaranteed.

Keep this small application preference adapter in `src-tauri`. JSON storage uses existing `std`, `serde` and `serde_json` facilities. Obtaining the Windows language list is a separate safe backend adapter; std/serde do not themselves provide that native facility. This decision specifies its behavior and injectable test input, not a library choice. Any dependency required for that adapter needs explicit owner approval and an ADR amendment before implementation. Browser language or a runtime PowerShell helper must not be silently substituted. No core I/O or new dependency is introduced by this spec amendment. Components use the typed client in `src/lib/api/`; the UI applies the locale only after a successful Save. Contract version 8 adds settings commands, snapshots and errors; existing analysis commands remain unchanged.

## Alternatives considered

- Browser local storage: rejected because the preference would live in WebView storage rather than the application's explicit, versioned settings contract.
- SQLite alongside the result cache: rejected because clearing or rebuilding disposable results must not remove a preference.
- A new settings plugin or crate: unnecessary for one versioned language field with existing serialization facilities.
- Unconditional English for a missing settings file: replaced by the owner-approved first-run detection so Spanish Windows users can start in Spanish.
- Using Windows language instead of a valid saved preference, or providing a live Auto mode: rejected because the explicit saved choice must take priority; detection applies only to missing settings.
- Advanced analysis settings in this requirement: deferred by the owner's scope decision; no detection parameter, sampling or cache-enable control is added.

## Consequences

The app gains one small configuration file, independent from music and result-cache storage. Failed reads and writes have explicit, testable recovery behavior. Observed unsupported future schemas are protected, subject to the stated cross-process limitation. The first supported Windows preference selects the initial language when no file exists; English remains the fallback. Both catalogues must be complete for the specified application UI. Installer and Windows-owned dialog language remain governed by existing release and dialog specifications.

The owner approved Q1-Q4, including this persistence behavior and its concurrency limitation, in chat on 2026-11-01 ("de acuerdo"). The owner merged the specification PR (#107) on 2026-11-01, which accepts this ADR. The feature was implemented in #109 (merged 2026-11-02).

## Amendment (2026-11-02): Windows language source

The native language-list adapter reads `Windows.System.UserProfile.GlobalizationPreferences.Languages()`, the ordered list the user maintains in Windows Settings › Language, through the `windows` crate. It is a Windows-only (`cfg(windows)`) dependency of `src-tauri`, pinned to the 0.62 line that `tauri` already brings into `Cargo.lock`, with only the `System_UserProfile` and `Foundation_Collections` features. No new crate enters the supply chain; only features of an existing crate are enabled. The WinRT projection is a safe API, so the workspace's `unsafe_code = "forbid"` stays in force. A failed call is reported as an unavailable list, which falls back to English as specified. The JSON storage adapter is unchanged and still uses only `std`, `serde` and `serde_json`.

Rejected: hand-written `extern "system"` bindings and `windows-sys` (both need `unsafe`, which the workspace forbids); `sys-locale` (a new third-party crate that reads UI display languages through `GetUserPreferredUILanguages` rather than the Settings preference list).

The owner approved this choice in chat on 2026-11-02, during `/feature RF-13`, before implementation.

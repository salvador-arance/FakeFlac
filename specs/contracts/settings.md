---
id: SPEC-CONTRACT-SETTINGS
requirements: [RF-13, RNF-04, RNF-05, RNF-09]
status: approved
---

# Language settings (RF-13, contract version 8)

Approved at G1 (#107, merged 2026-11-01); implemented in #109 (merged 2026-11-02). The owner selected Spanish/English with a persistent preference in
chat on 2026-11-01. The Settings object is limited to language. Historical proposals for
sampling controls, editable DetectionParams and cache controls require a separate spec.
Architecture decision: ADR-0049. UI: `../ui/language-settings.md`.

## 1. Owner-approved Q1-Q4 decisions

| ID | Decision | Approved answer (2026-11-01) |
|---|---|---|
| Q1 | Initial language and operating-system detection | Saved preference first. Only when the file is missing, choose the first supported language in the Windows user's ordered language preferences; fall back to English. No Auto option. |
| Q2 | Apply a selection immediately or on Save | A modal draft with Save and Cancel. Apply the language only after persistence succeeds; no restart. Available during an analysis. |
| Q3 | Storage and recovery | The versioned JSON and failure rules in sections 3–4; invalid files are preserved on read and may be replaced by explicit Save; observed newer versions are read-only. No synchronization between independent processes; the pre-save version check is not a cross-process guarantee. |
| Q4 | Translation and numeric presentation | The full catalogue in `../ui/translations.md`, including singular forms; keep existing plain digits, decimal point, units and duration notation in both languages. Native Windows dialogs retain their OS language. |

The owner approved these four recommendations in chat ("de acuerdo") on 2026-11-01,
including the Q1 change from unconditional English and the Q3 concurrency clarification.
This recorded decision approval; the owner then merged PR #107 (G1) and the feature PR #109
(G2).

## 2. Commands and types

Rust defines these DTOs and tauri-specta generates the TypeScript bindings (S3). Field names
shown here are the JSON/TypeScript names. No component invokes Tauri directly; the page uses
the typed API client.

```ts
type Language = 'en' | 'es'
type Settings = { language: Language }
type SettingsLoadStatus = 'default' | 'loaded' | 'invalid' | 'unavailable' | 'newerVersion'
type SettingsSnapshot = { settings: Settings; loadStatus: SettingsLoadStatus }
```

| Command | Input | Output | AppError codes |
|---|---|---|---|
| `get_settings` | none | `SettingsSnapshot` | `INTERNAL` for unexpected command/transport failures only |
| `set_settings` | `settings: Settings` | `SettingsSnapshot` | `SETTINGS_WRITE_FAILED`, `SETTINGS_READ_ONLY`, `INTERNAL` |

The API exports `getSettings(): Promise<SettingsSnapshot>` and
`setSettings(settings: Settings): Promise<SettingsSnapshot>`. They throw `ApiError` for
contract errors and normalize unexpected rejections to `INTERNAL`, following the existing
client convention. Invalid IPC payloads (unsupported language, missing/wrongly typed language)
are rejected without changing memory or disk; typed deserialization may reject them before
the handler. No new validation error code is required for that unreachable-in-the-UI case.

`get_settings` initializes the process state on first use, then returns the current snapshot.
`set_settings` initializes it first if necessary. Calls serialize through one backend-owned
settings state. Ordinary storage problems are represented in `loadStatus` on read, never as
an analysis failure. A successful save returns the supplied language with `loadStatus: 'loaded'`.
After a failed save, `get_settings` still returns the snapshot from before the attempt.

No progress event is added. Settings commands do not acquire the analysis slot, reject because
an analysis is running, create a run, cancel a run, clear results or clear the cache. Language
does not enter DetectionParams, the cache engine identity, analysis DTOs or the CSV serializer.
The existing `get_spectrogram` request/result remains valid through a language change.

## 3. Storage and loading

The file is `settings.json` inside Tauri's `app_config_dir()` for `io.github.fakeflac.app`
(on Windows, normally `%APPDATA%\io.github.fakeflac.app\settings.json`). The backend resolves
the directory; the frontend cannot choose a path. It is independent of the selected audio
folder and of the results-cache database. Tests inject a repository-local temporary root.
The feature never writes beside audio files or migrates the layout's existing localStorage.

Schema 1 is a UTF-8 JSON object with exactly these fields (key order and whitespace are not
semantic; a UTF-8 BOM is not written):

```json
{"schemaVersion":1,"language":"es"}
```

`schemaVersion` is a positive integer. Language is exactly `en` or `es`; case variants and
region tags are invalid. Version 1 rejects missing, duplicate and unknown fields, wrong
types and trailing non-whitespace. A readable JSON object with a positive integer
`schemaVersion > 1` is classified as newer before interpreting its other fields. Everything
else that fails schema 1 is invalid (including zero/negative/noninteger versions).

| Condition at initial load | Settings | loadStatus | Filesystem change |
|---|---|---|---|
| No file or no config directory yet | first supported Windows preference, otherwise English | `default` | none |
| Valid schema 1 | saved language | `loaded` | none |
| Invalid JSON/schema/language | English | `invalid` | none |
| Directory resolution or file read fails, including file path being a directory | English | `unavailable` | none |
| Newer version | English | `newerVersion` | none |

Loading never creates, truncates, renames or deletes anything. There is no migration from a
previous shipped schema: RF-13 is the first persisted settings version. Unicode app-data
paths are supported. A missing setting does not create a file until an explicit Save.

For a missing file, inspect the current Windows user's ordered preferred UI languages once
during settings initialization. Match each language tag's primary subtag case-insensitively:
`es`, `es-ES` and `es-MX` select Spanish; `en`, `en-US` and `en-GB` select English. Skip
unsupported tags and select the first supported entry in preference order. An empty,
unavailable or wholly unsupported list falls back silently to English. Tests inject this
list rather than depend on the test host's language. The initial detected choice is not
written until Save. Valid saved settings always take priority, even after Windows preferences
change. Invalid, unreadable and newer settings retain their English warning fallback from
the table; Windows detection does not apply to those recovery cases. There is no browser
language source, Auto selection or live tracking of Windows language changes.
The native language-list source belongs to a safe backend adapter, separate from JSON
storage. This behavior approval does not approve a new dependency: any required library
must receive explicit owner approval and an ADR amendment before implementation. Do not
silently replace the Windows source with browser language or a runtime PowerShell helper.

## 4. Saving and recovery

1. Validate the input language before any write. Serialize settings operations within this
   process. Immediately before writing, inspect an existing destination for a newer version.
   If it is newer, return `SETTINGS_READ_ONLY`; preserve it. The initial `newerVersion` state
   also refuses saving for this process even if the file changes externally.
2. Create the app configuration directory if it is missing. Write the complete schema 1 JSON
   to a new, uniquely named temporary sibling using exclusive creation, then flush the file
   before replacing the destination. Replace atomically; never truncate the destination in
   place or delete it first. This uses existing std/serde facilities, with no new dependency.
3. Only after replacement succeeds update the process snapshot to the requested language,
   `loaded`, and resolve. Reading the saved file from a fresh process must return that language.
4. Directory/read-before-write/temporary-write/flush/replace failures return
   `SETTINGS_WRITE_FAILED`; keep the old in-memory snapshot and any prior destination bytes.
   Clean up a temporary file created by this attempt on a best-effort basis; an orphan must
   not be loaded as the settings file. Errors may include English technical details in
   `AppError.message`; the UI presents catalogue text, not raw paths or OS diagnostics.

Explicit Save may replace a malformed schema-1-or-older file with a valid schema 1 file. If
the previous load was unavailable, Save retries the directory/file operations. A newer file
always requires a compatible app version; the UI offers Close but disables Save.

Saving the currently active language is allowed, including creating the first file or
recovering an invalid one. Backend calls persist it by the same procedure. No file watcher or
cross-process live synchronization is promised: each process loads its own snapshot; among
successful writes of supported files the last replacement wins on the next launch. Atomic
replacement protects complete-file publication, not an OS/power-loss durability guarantee.
The newer-version guard applies to the file observed at load or the pre-save check. Another
process can replace that file between the check and atomic replacement; without cross-process
coordination this release does not guarantee preservation of that intervening replacement.
Failure preservation likewise assumes another process has not modified the destination.

## 5. Contract changes and implementation boundaries

Contract version 7 becomes 8 only for the new commands/types and the two AppError codes.
Analysis, cache, CSV and spectrogram data remain locale-independent. Backend storage belongs
to the Tauri adapter (no core I/O; no new dependency approved by this spec). The core's shared error enum
may gain the two codes solely to keep existing error conversion/generated bindings coherent.

`../ui/language-settings.md` section 6 lists the approved changes to existing UI expectations
for the RF-13 test stage. The spec PR (#107) changed no production code or executable test.
Gherkin: `../features/language-settings.feature`. No delay or performance threshold is
introduced: acceptance observes completed saves and UI render cycles, never sleeps.

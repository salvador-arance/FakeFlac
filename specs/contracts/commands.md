---
id: SPEC-CONTRACT-H1
requirements: [RF-01, RF-02, RF-05, RF-06, RF-07, RF-09, RF-10, RF-11, RF-13, RF-14, RF-15, RF-16]
status: approved
version: 8
---

# Backend ↔ frontend contract (H1, with the H2 lossy formats, CSV export and album summaries)

> The commands, events and types the frontend can use. They are defined once in Rust and the TypeScript bindings are **generated** with `tauri-specta` (S3, ADR-0003); this document is the readable reference. Field names are camelCase on the TS side. Settings commands arrive later in H2 (the cache command is version 7). Version 3 adds MP3 and M4A files (RF-14 H2) and the source-bitrate estimate (RF-15); changes are listed in "Lossy formats (version 3)". Version 4 adds `export_csv` (RF-09); changes are listed in "CSV export (version 4)". Version 5 adds the album summary to every `trackDone` event (RF-10); changes are listed in "Album summary (version 5)". Version 6 adds the reason `streamingSampleRate` (RF-16, detection v7, ADR-0043); changes are listed in "Streaming sample rate (version 6)". Version 7 adds the result cache (RF-11, ADR-0044): the command `clear_cache`, the error `CACHE_FAILED` and the summary fields `fromCache` and `cache`; changes are listed in "Result cache (version 7)". Detection v8 adds Opus files (RF-14, RF-15, ADR-0047) **without a new contract version**: no type changes; changes are listed in "Opus files (detection v8)".

## Commands

**Version 8 (RF-13, approved at G1 in #107; not implemented yet):** `settings.md` defines the language-only
`get_settings` / `set_settings` commands, `Language`, `Settings`, `SettingsSnapshot`,
`SettingsLoadStatus` and the additional `AppError.code` values `SETTINGS_WRITE_FAILED` and
`SETTINGS_READ_ONLY`. It supersedes the introductory deferral of Settings above. All version
7 analysis/cache/export/spectrogram fields remain unchanged. Language never changes their
values, the cache engine key or CSV bytes. See `../ui/language-settings.md` for startup gating
and live translation of already displayed results and messages. ADR-0049 records persistence.

| Command | Input | Output | Errors (`AppError.code`) | Req. |
|---|---|---|---|---|
| `pick_folder` | — | `string \| null` (absolute path; `null` if the dialog is cancelled) | — | RF-01 |
| `start_analysis` | `root: string` (absolute path), `onProgress: Channel<ProgressEvent>` | `AnalysisSummary` when the run ends (finished or cancelled) | `NOT_A_FOLDER`, `FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`, `ANALYSIS_RUNNING`, `INTERNAL` | RF-01..06, RF-10 (version 5) |
| `cancel_analysis` | — | `void` (no-op if nothing is running) | — | RF-05 |
| `get_spectrogram` | `path: string` | `Spectrogram` | `NOT_ANALYZED` is not used in H1; `INTERNAL` if the file cannot be decoded | RF-07 |
| `export_csv` (version 4) | — (the destination comes from the native save dialog, never from the frontend) | `ExportResult \| null` (`null` if the dialog is cancelled) | `NOT_ANALYZED`, `ANALYSIS_RUNNING`, `EXPORT_FAILED`, `INTERNAL` | RF-09 |
| `clear_cache` (version 7) | — | `void` | `ANALYSIS_RUNNING`, `CACHE_FAILED`, `INTERNAL` | RF-11 |
| `get_settings` (version 8, not implemented yet) | — | `SettingsSnapshot` (`settings.md`) | `INTERNAL` | RF-13 |
| `set_settings` (version 8, not implemented yet) | `settings: Settings` | `SettingsSnapshot` (`settings.md`) | `SETTINGS_WRITE_FAILED`, `SETTINGS_READ_ONLY`, `INTERNAL` | RF-13 |

Drag and drop uses Tauri's native drag-drop event. The frontend cannot tell a folder from a file (it has no file-system access), so it calls `start_analysis` with each dropped path in order until one does not fail with `NOT_A_FOLDER`. Dropped files are ignored in H1: if every dropped path is a file, nothing starts and no error is shown. Any other error is shown as usual.

`start_analysis` errors before the run starts (no event is sent):
- `NOT_A_FOLDER`: `root` exists but is a file.
- `FOLDER_UNREADABLE`: `root` does not exist, cannot be listed, or is not an absolute path.
- `NO_AUDIO_FOUND`: the folder has no supported file.
- `ANALYSIS_RUNNING`: another run holds the analysis.

`INTERNAL` means the run failed unexpectedly, possibly after some events were sent. No `finished`/`cancelled` event follows: a rejected `start_analysis` is always the end of the run. When the call resolves, every event of the run has already been delivered.

Only one analysis runs at a time: a second `start_analysis` while one is running fails with `ANALYSIS_RUNNING`.

## Events (`ProgressEvent`, through the `Channel`)

Ordered per run. Exactly one of `finished` / `cancelled` is the last event.

```ts
// Paths are strings. A Windows name that is not valid Unicode arrives with U+FFFD replacement
// characters, so a path may not be unique: the frontend keys rows by `index` within a run.
type ProgressEvent =
  | { kind: 'scanned'; total: number }                          // files found (supported formats only)
  | { kind: 'trackStarted'; index: number; path: string }       // RF-05 per-track progress
  | { kind: 'trackDone'; index: number; analysis: TrackAnalysis; album: AlbumUpdate } // `album`: version 5 (RF-10)
  | { kind: 'finished'; summary: AnalysisSummary }
  | { kind: 'cancelled'; summary: AnalysisSummary }
```

`index` is the 0-based position of the file in the scan order (sorted by path, case-insensitive). `trackDone` events may arrive out of index order (parallel analysis). `album` is the summary of the track's album after counting that track ("Album summary (version 5)").

## Types

```ts
type Verdict = 'real' | 'suspicious' | 'fake' | 'error'
type AudioFormat = 'wav' | 'flac' | 'mp3' | 'm4a' | 'opus'      // the container, from the file extension
type AudioCodec = 'pcm' | 'flac' | 'alac' | 'mp3' | 'aac' | 'opus' // version 3: the codec inside it
type BitrateMode = 'cbr' | 'vbr' | 'abr'                        // version 3
type SuspicionReason =
  | 'lowHighFrequencyContent' | 'gradualRolloff' | 'ambiguousCutoff'
  | 'highBandGaps'           // algorithm_version 2 (lossless files)
  | 'cutoffBelowDeclared'    // algorithm_version 5 (lossy files: cutoff lower than expected for the declared bitrate)
  | 'streamingSampleRate'    // version 6, algorithm_version 7 (lossy files: full band at a streaming sample rate, 48 kHz; RF-16)

type TrackInfo = {
  path: string; folder: string; format: AudioFormat
  codec: AudioCodec | null                           // version 3; null only when an M4A could not be probed
  sampleRate: number; bitDepth: number | null; channels: number
  declaredBitrateKbps: number | null                 // lossy codecs (mp3, aac, opus); null for lossless codecs and when not known (errors before the stream was read)
  bitrateMode: BitrateMode | null                    // version 3; MP3 only (AAC and Opus do not signal it)
  durationMs: number; sizeBytes: number; modifiedUnix: number
}

type CutoffEstimate = { frequencyHz: number; steepnessDbPerKhz: number; noiseFloorDb: number }

type TrackError =
  | { kind: 'unreadable'; detail: string }
  | { kind: 'corruptStream'; detail: string }
  | { kind: 'unsupportedFormat'; detail: string }
  | { kind: 'tooShort' }

type TrackAnalysis = {
  info: TrackInfo
  verdict: Verdict
  cutoff: CutoffEstimate | null
  estimatedSourceBitrateKbps: number | null          // RF-15: lossy codecs only (specs/detection/lossy-source.md section 5); null with streamingSampleRate (version 6); null for an Opus track that is not real (detection v8)
  reasons: SuspicionReason[]
  error: TrackError | null
  algorithmVersion: number                           // 8 with the default DetectionParams (detection v8: lossless rules of rules.md, lossy rules of lossy-source.md)
}

type AnalysisSummary = {
  root: string; total: number; done: number
  real: number; suspicious: number; fake: number; errors: number
  cancelled: boolean; elapsedMs: number
  fromCache: number                                  // version 7 (RF-11): tracks whose trackDone carried a cached result; ≤ done
  cache: CacheStatus                                 // version 7 (RF-11)
}

type CacheStatus = 'off' | 'ok' | 'degraded' | 'unavailable'  // version 7 (RF-11); 'off' is never sent by the app

type Spectrogram = {
  width: number            // time bins, ≤ spectrogram.max_width
  height: number           // frequency bins (0 Hz at row 0)
  sampleRate: number
  maxFrequencyHz: number   // = sampleRate / 2
  dbRange: [number, number] // u8 0 → dbRange[0], 255 → dbRange[1]
  data: number[]           // width*height bytes (0–255), row-major by time (ADR-0009); sent as a JSON array of numbers
}

type AlbumSummary = {     // version 5 (RF-10); rule: specs/detection/album-summary.md
  folder: string           // the album's folder: the direct parent folder of its tracks, equal to their `TrackInfo.folder`
  total: number            // tracks of this album with a `trackDone` so far in the run; = real + suspicious + fake + errors
  real: number; suspicious: number; fake: number; errors: number
  verdict: Verdict         // most severe of real < suspicious < fake; 'error' only when every counted track is an error
}

type AlbumUpdate = {      // version 5 (RF-10)
  index: number            // the album's key within the run (0-based, path order of the album folders); never key by path
  name: string             // display name: the folder relative to the parent of the analysed root (rule in "Album summary (version 5)")
  files: number            // supported files the scan found directly in this folder; summary.total ≤ files
  summary: AlbumSummary    // the album after counting the track of this event
}

type ExportResult = {     // version 4 (RF-09)
  path: string             // absolute path of the CSV written (the one chosen in the dialog)
  rows: number             // tracks written (header not counted), ≥ 1
}

type AppError = {
  code: 'NO_AUDIO_FOUND' | 'NOT_A_FOLDER' | 'FOLDER_UNREADABLE' | 'ANALYSIS_RUNNING' | 'NOT_ANALYZED' | 'EXPORT_FAILED'
      | 'CACHE_FAILED'                               // version 7 (RF-11): only from clear_cache
      | 'SETTINGS_WRITE_FAILED' | 'SETTINGS_READ_ONLY' // version 8 (RF-13, not implemented yet): only from set_settings
      | 'INTERNAL'
  message: string
}
```

Per-track problems never become an `AppError`: they travel inside `TrackAnalysis` (`verdict: 'error'` + `error`), ADR-0013.

Detection v3 (ADR-0038) does not change these types: a track that the full-band guard keeps from `fake` is `suspicious` with the existing reasons `highBandGaps` or `ambiguousCutoff`. Its new measurements (`U`, `B`, `FullBand`) and its new `DetectionParams` section `[full_band]` are internal to the backend (schema and semantics: `specs/detection/rules.md` sections 2 and 4, values: `specs/detection/detection-params.v3.toml`), so `just gen` produces the same bindings.

Detection v4 (ADR-0039) does not change them either. It adds the optional field `full_band.max_lossy_step_db` to `DetectionParams` (internal to the backend; values: `specs/detection/detection-params.v4.toml`) and narrows `FullBand`, so with the defaults a guarded track is always `suspicious` with `highBandGaps`. The only visible change is `algorithmVersion` = 4, which invalidates cached results (ADR-0008).

Detection v6 (ADR-0042) does not change them either. It adds the optional section `edge` to `DetectionParams` (internal to the backend; values: `specs/detection/detection-params.v6.toml`), which can only turn a lossless `fake` into `suspicious` with `ambiguousCutoff` (or `highBandGaps`). The only visible change is `algorithmVersion` = 6 with the defaults, which invalidates cached results (ADR-0008); lossy results equal version 5's.

Detection v7 (RF-16, ADR-0043) **does** change them: it adds the reason `streamingSampleRate` (contract version 6, "Streaming sample rate (version 6)").

Detection v8 (RF-14, RF-15, ADR-0047) does not change them: `AudioFormat` and `AudioCodec` already hold `opus`. It adds Opus files to the scan and the `opus` rows of `[lossy.cutoffs]` ("Opus files (detection v8)"); `algorithmVersion` is 8 with the defaults.

## Lossy formats (version 3)

Version 3 (RF-14 H2, RF-15; ADR-0040, ADR-0041; rules: `specs/detection/lossy-source.md`):

- **New fields** of `TrackInfo`: `codec` and `bitrateMode` (lossy-source.md section 2). `declaredBitrateKbps` is now filled for MP3 and AAC files: the frame bitrate of a CBR MP3, the average of a VBR or ABR MP3 or of an AAC track. For lossless codecs (`pcm`, `flac`, `alac`) it stays `null`, as `bitrateMode`. For an `Error` track the fields hold what was learnt before the error (`codec` from the extension for `.wav`, `.flac` and `.mp3`; `null` for an `.m4a` that could not be probed).
- **`estimatedSourceBitrateKbps`** (unchanged type) is filled for lossy codecs: the declared bitrate when the content is consistent with it, otherwise the bitrate of the codec's table row whose low-pass is nearest to the measured cutoff, at most the declared one. It is `null` for lossless codecs, `Error` tracks, silent tracks and tracks with little treble (`lowHighFrequencyContent`). It is expressed in the declared codec's terms (an MP3 made from an AAC 128 stream shows an MP3-equivalent bitrate).
- **New reason** `cutoffBelowDeclared` (lossy files only): the content stops clearly below the expected cutoff of the declared bitrate, but not far enough or not sharply enough for `fake`. A lossy `fake` carries no reason, as a lossless one. A lossy track can also be `suspicious` with `lowHighFrequencyContent` or `gradualRolloff`.
- **`cutoff`** of a lossy track is the measured content edge the rules judge (lossy-source.md section 4, "Reported cutoff").
- **ALAC** in M4A is lossless: `format` `m4a`, `codec` `alac`, `bitDepth` from the stream, judged like FLAC.
- **Errors.** HE-AAC and MP3 layers I/II are `unsupportedFormat`; an M4A or MP3 that cannot be parsed is `corruptStream` (`TrackError` unchanged).
- **`algorithmVersion`** is 5 with the defaults (cached results of version 4 are recomputed, ADR-0008). For WAV and FLAC the verdicts, reasons and cutoffs equal version 4's.
- **`get_spectrogram`** decodes MP3 and M4A files like the others; its contract does not change.
- `AppError` codes do not change; `NO_AUDIO_FOUND` now means "no WAV, FLAC, MP3 or M4A file".
- `encoder` and the LAME tag's low-pass are internal to the backend (not in the contract).

`just gen` produces new bindings (`AudioCodec`, `BitrateMode`, the two `TrackInfo` fields and the reason); `just gen-check` must pass. UI impact: `specs/ui/lossy-formats.md`.

## CSV export (version 4)

Version 4 (RF-09; file format and layers: `specs/contracts/csv-export.md`, ADR-0012; UI: `specs/ui/export-csv.md`; scenarios: `specs/features/export-csv.feature`):

- **New command `export_csv`**, no input. It opens the native **save** dialog (modal to the window, like `pick_folder`) with the default file name `FakeFlac - <name of the analysed folder>.csv` (`FakeFlac.csv` for a drive root or a UNC share root; rules in csv-export.md section 5.1, `default_file_name`) and the filter "CSV (*.csv)", whose extension the Windows dialog appends to a name typed without one (csv-export.md Q9); the app does not set the dialog's initial folder (csv-export.md Q8). The dialog asks before replacing an existing file. When the user confirms, the backend writes the CSV of the results it keeps (below) to the chosen path and resolves with `ExportResult`; when the user cancels, it resolves with `null` and writes nothing.
- **The destination never comes from the frontend** (csv-export.md Q7): the page cannot ask the app to write anywhere. Capabilities do not change.
- **Results kept for export.** The backend keeps in memory the `TrackAnalysis` of every `trackDone` event of the **latest run**, with that run's `root`; nothing is persisted. They are replaced when a new run sends its `scanned` event. A `start_analysis` that fails before the run starts (`NOT_A_FOLDER`, `FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`, `ANALYSIS_RUNNING`) keeps them, as the table keeps its rows (`ui/main-screen.md`, state Error). A cancelled run keeps the tracks done before it stopped; a run that ends with `INTERNAL` keeps the tracks it reported. The CSV lists them **in `index` order** (scan order), whatever the order `trackDone` arrived in and whatever the table's sort.
- **Snapshot.** The results are taken when `export_csv` is called, before the dialog opens: a run started while the dialog is open does not change what is written.
- **One export at a time** (csv-export.md section 5.2). A second `export_csv` call while one is in progress (its dialog open or its file being written) fails at once with `INTERNAL`, message `an export is already in progress`: no second dialog opens, nothing is written, and the first call is not affected. The UI never makes it (the button is disabled while exporting).
- **Errors**, all before the dialog opens unless stated, checked in this order:
  - `INTERNAL`: another export is in progress (above).
  - `ANALYSIS_RUNNING`: a run holds the analysis (scanning or analysing). Nothing is exported.
  - `NOT_ANALYZED`: there are no results (no run yet, or the latest run reported no track). The dialog does not open.
  - `EXPORT_FAILED` (after the dialog): the chosen path does not end in `.csv` (case-insensitive; nothing is written), or the file cannot be written (csv-export.md section 6). A failed export leaves the destination as it was.
  - `INTERNAL` (at any point): an unexpected failure (for example the export task panicked).
- **Read-only (RNF-05).** The export reads no audio file and writes no file other than the chosen CSV (and its temporary file while writing).
- `NOT_ANALYZED` gets its first use here (csv-export.md Q10); `AppErrorCode` does not change.

`just gen` produces new bindings (the command `exportCsv` and the type `ExportResult`); `just gen-check` must pass. The API client adds `exportCsv(): Promise<ExportResult | null>`, which rejects with `ApiError` on a contract error, like `getSpectrogram`.

## Album summary (version 5)

Version 5 (RF-10; rule: `specs/detection/album-summary.md`, unchanged; UI: `specs/ui/album-summary.md`; scenarios: `specs/features/album-summary-app.feature`):

- **Computed in the backend.** The `analyze_folder` use case of `fakeflac-core` groups the tracks of the run by album and summarises each album with the existing rule `summarize_album`. The frontend never recomputes a summary verdict: it shows the latest `AlbumUpdate` it received for each album. The rule already lives and is tested in `fakeflac-core`; a TypeScript copy would put a verdict rule outside the core (ADR-0001, `src-tauri/CLAUDE.md`) and would have to be kept equal by hand. No new port, crate or dependency; the core stays free of I/O (grouping and the name rule are pure path operations on the scanned entries).
- **Types and layers.**
  - `AlbumSummary` of the contract **is the existing core type** `fakeflac_core::domain::AlbumSummary` (`folder: PathBuf`, the value `summarize_album` returns; unchanged), mapped by a new src-tauri DTO `AlbumSummary` in `src-tauri/src/dto.rs` that turns `folder` into a string with the same lossy conversion as `TrackInfo.folder` (`path_string`).
  - `AlbumUpdate` is a **new core type defined in `fakeflac-core`**, next to `ProgressEvent` in `analyze_folder.rs`, because the use case emits it: `index: u32`, `name: PathBuf`, `files: u32`, `summary: AlbumSummary` (core). `ProgressEvent::TrackDone` gains `album: AlbumUpdate`. src-tauri adds the DTO `AlbumUpdate` (`name` through `path_string`) and the `album` field of its `TrackDone` DTO.
  - **The display-name rule lives in `fakeflac-core`**: a pure public function of `root` and the album folder that returns the path to show (`PathBuf`; rule below). The core never converts it to a lossy string; the DTO does, like every other path.
  - **src-tauri only maps** core types to DTOs (`From` impls in `dto.rs`, as for `TrackInfo`): it does no grouping, counting, ordering, naming or verdict logic (`src-tauri/CLAUDE.md`).
- **Album.** An album is the set of scanned files with the same **direct parent folder** (`FileEntry.folder`, which becomes `TrackInfo.folder`; album-summary.md "Inputs"). Each subfolder is a separate album (`Album 1` and `Album 1\CD2` are two albums). A folder that holds no supported file directly (only subfolders, or only unsupported files) is no album. **The analysed root is an album** when it holds supported files directly, like any other folder.
- **`trackDone.album`.** Every `trackDone` event carries `album: AlbumUpdate`, the album of that track **after** counting it:
  - `index`: the album's key within the run. The run's albums are numbered 0, 1, … by sorting their folders path component by component, case-insensitively, then by exact path (the same key the scan uses for files, applied to the folder path). So a folder comes before its subfolders (`Artist`, `Artist\Album 1`, `Artist\Album 1\CD2`, `Artist\Album 2`). The numbering is fixed once the scan ends: it does not depend on which track finishes first. Two folders whose names differ only in characters that are not valid Unicode get different `index` values even when their `folder` strings are equal (U+FFFD, see "Events"): the frontend keys albums by `index`, never by `folder` or `name`.
  - `name`: the display name (rule below).
  - `files`: the supported files the scan found directly in this folder (`≥ 1`). `summary.total ≤ files`; they are equal once every file of the album has had its `trackDone`.
  - `summary`: equal to `summarize_album(folder, T)`, where `T` is the tracks of this album whose `trackDone` was sent in this run up to and including this one. `summary.folder` equals `analysis.info.folder`. The implementation may keep running counts instead of calling `summarize_album` on every event, provided every summary sent equals what `summarize_album` returns (the tests compare against it).
- **Order.** For each album, the summaries of its `trackDone` events grow one track at a time in channel order: the n-th `trackDone` of an album carries `summary.total == n`. So the latest update received for an album is always its newest state, even with parallel workers.
  - **Album lock.** The use case holds **one album lock**, a mutex of its own (not the lock that guards `next` and sends `trackStarted`), around exactly two steps: updating the album state (count the track in its album, build the `AlbumUpdate`) and the `on_progress(TrackDone)` call for that track. It is **never held during a track's analysis**, and never together with the `next` lock (no lock-order deadlock). So workers still analyse in parallel; only the counting and the sending of `trackDone` are serialised.
  - **`on_progress` must not block.** Every worker's `trackDone` passes through the album lock, so the sink must return without waiting for the page: src-tauri's sink records the event for the export (a short mutex) and hands it to the Tauri `Channel`, whose `send` does not wait for the frontend. A blocking sink would stall every worker at its next `trackDone` (it would not break the order). The sink must not call back into the use case.
- **When summaries change.** Only with `trackDone`. `scanned`, `trackStarted`, `finished` and `cancelled` carry no album data, and `AnalysisSummary` does not change.
  - Finished run: the last update of every album has `summary.total == files`.
  - Cancelled run: each album with at least one track done has its last update, which may have `summary.total < files`; an album with no track done never appears in any event.
  - Run that ends with `INTERNAL`: as cancelled, for the tracks reported. `INTERNAL` after events means the run's task failed (for example a panic in the use case outside the per-track guard, or in the sink; `src-tauri/src/commands.rs` turns the failed task into `INTERNAL`): every album update already sent obeys the rules above, and no further event follows.
  - A new run numbers its albums from 0 again; its albums replace the previous run's (the frontend drops them when it starts the run, `ui/album-summary.md`).
- **Error tracks** (RNF-04) are counted in `errors` and decide the summary verdict only when every counted track of the album is an error (album-summary.md rule 3). An album whose files are all unreadable, corrupt or too short has `verdict: 'error'`. While the run goes on, an album can change from `error` to another verdict when one of its readable tracks arrives later.
- **Name rule** (pure, computed in `fakeflac-core` from the run's `root` and the album folder): when the root has a parent folder and the album folder starts with it (component-wise), `name` is the album folder with that parent removed, so it starts with the root folder's own name; otherwise (the root is a drive root such as `D:\` or `\\?\C:\`, or a UNC share root such as `\\nas\music`, which have no parent) `name` is the album folder in full. The separator is the one in the folder path (`\` on Windows). Names that are not valid Unicode get U+FFFD like any other path string (in the DTO).
  - **Over `Path` components, not strings.** The rule uses `std::path::Path` (`parent`, component-wise prefix), like the scan order already does, so `\\?\`, UNC prefixes and a trailing separator are handled by the standard library. A Windows path is only parsed as one on Windows, so the tests of the rule, and every use-case test of this feature whose paths are Windows paths (`C:\…`, `\\?\…`, `\\nas\…`), are **`#[cfg(windows)]`** (for example one test file with `#![cfg(windows)]`). CI runs the Rust tests on `windows-latest` (`.github/workflows/ci.yml`, job `test-rust`), so they always run there; the Linux `lint` job only compiles test targets (clippy), where they are left out.
  - **The table is the test**: the Scenario Outline "Album name" of `features/album-summary-app.feature` has exactly these rows, one test case per row.

  | Root | Album folder | `name` |
  |---|---|---|
  | `C:\Music\Artist` | `C:\Music\Artist` (the root's own tracks) | `Artist` |
  | `C:\Music\Artist` | `C:\Music\Artist\Album 1` | `Artist\Album 1` |
  | `C:\Music\Artist\` (trailing separator) | `C:\Music\Artist\Album 1\CD2` | `Artist\Album 1\CD2` |
  | `C:\Music\Álbum ″Ñandú″` | `C:\Music\Álbum ″Ñandú″` | `Álbum ″Ñandú″` |
  | `\\?\C:\Music\Album A` | `\\?\C:\Music\Album A` | `Album A` |
  | `\\?\C:\` (verbatim drive root) | `\\?\C:\` | `\\?\C:\` |
  | `\\?\C:\` | `\\?\C:\Album A` | `\\?\C:\Album A` |
  | `\\nas\music\Rock` | `\\nas\music\Rock\Album B` | `Rock\Album B` |
  | `D:\` (drive root) | `D:\` | `D:\` |
  | `D:\` | `D:\Album A` | `D:\Album A` |
  | `\\nas\music` (UNC share root) | `\\nas\music` | `\\nas\music` |
  | `\\nas\music` | `\\nas\music\Album B` | `\\nas\music\Album B` |

- **Unchanged:** the commands and their inputs and outputs, `AppError` codes, `AnalysisSummary`, `TrackAnalysis`, the results kept for export and the CSV file. **The CSV export does not include album summaries**: it stays one record per track, `csv_format_version` 1 (csv-export.md unchanged); the `folder` column already lets a spreadsheet group the tracks (G1 decision Q8 of `ui/album-summary.md`).

- **Consequence on existing tests (declared).** `ProgressEvent::TrackDone` (`crates/fakeflac-core/src/analyze_folder.rs`) and its DTO gain `album`, so tests derived from approved specs that build a `trackDone` event or match it without `..` stop compiling or type-checking. **Updating them is a declared consequence of this spec**: the test author does it in the `test(RF-10)` commit (stage `test`); it only adds the `album` field or `..`, and their assertions on the other fields do not change. The implementer does not touch them.
  - `crates/fakeflac-core/tests/analyze_folder.rs`: `done_by_index` matches `TrackDone { index, analysis }` exhaustively.
  - `src-tauri/tests/contract.rs`: `progress_events_match_the_contract` builds a core `TrackDone` and expects its JSON without `album`.
  - `src-tauri/tests/export_csv.rs`: `track_done` matches the DTO `TrackDone { index, analysis }` exhaustively; `done` builds a core `TrackDone`.
  - `src/lib/pages/AnalysisPage.test.ts`, `src/lib/pages/AnalysisPage.export.test.ts`, `src/lib/pages/AnalysisPage.spectrogram.test.ts`: their stubbed `trackDone` events have no `album` (the type check fails and the store reads `event.album`).
  - No change needed: tests that match `TrackDone { .. }` or `TrackDone { index, .. }` (the other patterns in `crates/fakeflac-core/tests/analyze_folder.rs`, `src-tauri/tests/{run_analysis,run_analysis_lossy,run_analysis_lossy_v5}.rs`). Production code that maps the event (`src-tauri/src/dto.rs`, `ExportState::record` in `src-tauri/src/run.rs`, the store) changes in stage `implement`; `src/lib/api/bindings.ts` is regenerated.

`just gen` produces new bindings (the types `AlbumSummary` and `AlbumUpdate`, the field `album` of the `trackDone` event); `just gen-check` must pass. The API client's `startAnalysis` does not change its signature. The frontend's `ResultRow` gains `albumIndex` (`ui/album-summary.md`).

## Sort and filter (RF-08): no change

RF-08 (UI: `specs/ui/sort-filter.md`; scenarios: `specs/features/sort-filter.feature`) sorts and filters the results table in the frontend, over the rows the store already holds. **The contract does not change**: version 5 stays, no command, event, type or `AppError` code is added or changed, and `just gen` produces the same bindings (`just gen-check` passes). `export_csv` keeps exporting every track of the latest run in scan order, whatever the table's sort and filters ("CSV export (version 4)"); exporting only the shown rows would be a new contract version (sort-filter.md Q12).

## Streaming sample rate (version 6)

Version 6 (RF-16; ADR-0043; rule L6a of `specs/detection/lossy-source.md` section 11, detection v7, `detection-params.v7.toml`; UI: `specs/ui/lossy-formats.md`, "Streaming sample rate (RF-16)"; scenarios: `specs/features/detection-streaming-source.feature`):

- **New reason** `streamingSampleRate` (lossy files only: `mp3` and `aac` codecs). The track's decoded sample rate is a streaming rate (48 kHz with the defaults) and its content ends at Opus's 20 kHz band edge (a reported cutoff in `[19800, 20200)` Hz with the defaults): the signature of a rip of a full-band lossy streaming source such as Opus from YouTube, but also of a genuine 48 kHz release. Such a track is always `suspicious` with exactly `reasons: ['streamingSampleRate']`; the reason never appears on a `real`, `fake` or `error` track, nor together with another reason, and never on a lossless (`pcm`, `flac`, `alac`) or Opus track.
- **`estimatedSourceBitrateKbps`** is `null` for a track with `streamingSampleRate` (lossy-source.md section 11.2, Q19), in addition to the cases of version 3.
- **`algorithmVersion`** is 7 with the defaults (cached results of version 6 are recomputed, ADR-0008). For WAV, FLAC and ALAC the verdicts, reasons and cutoffs equal version 6's; for MP3 and AAC they equal version 6's except tracks that move from `real` to `suspicious` with the new reason.
- **CSV export** (csv-export.md): the `reasons` column may now hold `streamingSampleRate`. The column's encoding does not change, so `csv_format_version` stays 1 (lossy-source.md Q23).
- **Unchanged:** the commands and their inputs and outputs, events, `AppError` codes, `TrackInfo` (the sample rate the rule reads is the existing `sampleRate`), `AlbumSummary`, `AlbumUpdate` (a flagged track counts as `suspicious` in its album, like any other), `AnalysisSummary`, `Spectrogram`, `ExportResult`. The new `DetectionParams` section `[lossy.streaming]` is internal to the backend.
- **Layers.** The core's `SuspicionReason` gains the variant `StreamingSampleRate`; `src-tauri/src/dto.rs` maps it like the others (serialised `streamingSampleRate`); the CSV serialiser (`fakeflac-export`) and the calibration tool write the same identifier. No new type, port or dependency.

`just gen` produces new bindings (one more `SuspicionReason` member); `just gen-check` must pass. The API client does not change. The frontend needs the text `reason.streamingSampleRate` (`ui/lossy-formats.md`).

## Result cache (version 7)

Version 7 (RF-11; ADR-0007, ADR-0008 as extended by ADR-0044; full spec: `specs/contracts/result-cache.md`; UI: `specs/ui/result-cache.md`; scenarios: `specs/features/result-cache.feature`):

- **`start_analysis`** reuses the stored result of a file when it is unchanged (same canonical path, size and `modifiedUnix`) and was analysed by the same engine (every `DetectionParams` value including `algorithm_version`, the decoder's extraction version and the app version; result-cache.md sections 3 and 4). Otherwise it analyses the file and stores the result. Its inputs, outputs, errors and events do not change:
  - **The events are the same.** A reused track still gets `trackStarted` and then `trackDone`, and it counts in its album as any other track.
  - **The analysis is the same.** Its `TrackAnalysis` equals what a fresh analysis would give. `info.path` and `info.folder` are those of the current run.
  - **What is not stored.** `unreadable` and `corruptStream` results are analysed again on every run (result-cache.md 2.2, Q2).
- **The cache never fails a run.** If it is missing it is created. If it is corrupt or not a FakeFlac cache, it is replaced (result-cache.md Q10). If it is locked, unwritable or written by a newer FakeFlac, it is not used, and every file is analysed. None of these is an `AppError`. They show only in `AnalysisSummary.cache`.
- **New `AnalysisSummary` fields:**
  - `fromCache`: tracks whose `trackDone` carried a cached result (`≤ done`).
  - `cache`:
    - `'ok'`: the cache was used with no failure;
    - `'degraded'`: a lookup or a write failed during the run, and the run stopped using the cache from then on (results are still correct; some may not be saved);
    - `'unavailable'`: the cache could not be opened, so nothing was reused or saved;
    - `'off'`: the run was made without a cache. The app never sends it; tests and tools use it.

  They are set in the summary of `finished`, of `cancelled` and in the value `start_analysis` resolves with.
- **New command `clear_cache`**, no input, resolves with nothing. It deletes the cache files (`results-cache.sqlite3` and its `-wal` and `-shm` files in the app's local cache folder, `%LOCALAPPDATA%\io.github.fakeflac.app`). Clearing an absent cache succeeds. It works even when the cache is corrupt or from a newer version. It changes nothing else: not the results kept for the export, and not any analysed file. The next run analyses every file again.
  - It fails with `ANALYSIS_RUNNING` when a run holds the analysis. While it works it holds the analysis itself, so a `start_analysis` made meanwhile fails with `ANALYSIS_RUNNING`; the UI prevents that (`ui/result-cache.md`).
  - It fails with `CACHE_FAILED` when a file cannot be deleted (for example another FakeFlac window has it open) or the cache folder cannot be resolved. `message` holds the OS error.
  - It fails with `INTERNAL` on an unexpected failure.
- **New `AppError` code `CACHE_FAILED`**, used only by `clear_cache`.
- **Read-only (RNF-05).** The cache is never in an analysed folder; its location never depends on `root`. Looking files up opens no audio file for writing.
- **Unchanged:** `pick_folder`, `cancel_analysis`, `get_spectrogram` (the spectrogram is never cached on disk), `export_csv` and the CSV (`csv_format_version` 1; a reused track is exported like any other), `ProgressEvent`, `TrackAnalysis`, `TrackInfo`, `AlbumUpdate`, `AlbumSummary`, `Spectrogram`, `ExportResult`.
- **Layers.**
  - **`fakeflac-core`:** the `ResultStore` port (`get`, `put`), `ResultCache`, `CacheStatus`, `analyze_folder_cached`, the summary fields and `AppErrorCode::CacheFailed`. `analyze_folder` keeps its signature, with no cache.
  - **`fakeflac-store`:** `SqliteResultStore`, `clear_cache_files`.
  - **`src-tauri`:** `run::run_analysis_cached`, `run::clear_cache`, the command and the DTOs.

  Details: result-cache.md section 10.
- **Consequence on existing tests (declared):** tests that build an `AnalysisSummary` field by field gain the two fields in `test(RF-11)` (result-cache.md section 12).

`just gen` produces new bindings (the command `clearCache`, the type `CacheStatus`, the two summary fields and the error code); `just gen-check` must pass. The API client adds `clearCache(): Promise<void>`, which rejects with `ApiError` on a contract error, like `exportCsv`. The frontend needs the texts of `ui/result-cache.md`.

## Opus files (detection v8)

Detection v8 (RF-14, RF-15; ADR-0047; rules: `specs/detection/lossy-source.md` sections 2.4 and 12, `detection-params.v8.toml`; UI: `specs/ui/lossy-formats.md`, "Opus files (detection v8)"; scenarios: `specs/features/opus.feature`). **The contract version stays 7** (lossy-source.md Q34): `just gen` produces the same bindings.

- **Scan.** `.opus` files are listed (any letter case), so `scanned.total`, `AlbumUpdate.files` and `NO_AUDIO_FOUND` count them. `NO_AUDIO_FOUND` now means "no WAV, FLAC, MP3, M4A or Opus file".
- **`TrackInfo` of an Opus track:** `format` `opus`, `codec` `opus` (also for an `Error` track: from the extension), `sampleRate` 48000 (Opus always decodes at 48 kHz), `bitDepth` null, `channels` 1 or 2 (channel mapping family 0), `declaredBitrateKbps` the average of the audio packets over the duration (lossy-source.md section 2.4.3), `bitrateMode` null, `durationMs` the audible length (pre-skip and end trim excluded).
- **Verdicts.** The lossy rules with the `opus` rows; the reasons are the existing ones (`lowHighFrequencyContent`, `cutoffBelowDeclared`, `gradualRolloff`); never `streamingSampleRate` on an Opus track.
- **`estimatedSourceBitrateKbps`** of an Opus track is the declared bitrate when it is `real` and `null` otherwise (lossy-source.md section 12.4, Q29), in addition to the null cases of versions 3 and 6.
- **Errors.** A broken Ogg or Opus structure or packet is `corruptStream`; an Opus stream with a channel mapping family other than 0 (multichannel) or a chained or multiplexed Ogg file is `unsupportedFormat` (`TrackError` unchanged; details in lossy-source.md section 2.4.4).
- **`algorithmVersion`** is 8 with the defaults (cached results of version 7 are recomputed, ADR-0008). For every file that is not Opus the verdicts, reasons, cutoffs and estimates equal version 7's.
- **`get_spectrogram`** decodes Opus files like the others (`sampleRate` 48000, `maxFrequencyHz` 24000); its contract does not change.
- **CSV export:** `format` and `codec` already allow `opus` (csv-export.md); `csv_format_version` stays 1.
- **Unchanged:** the commands, events, `AppError` codes and every type.
- **Layers.** `fakeflac-fs` lists `.opus`; `fakeflac-audio` reads the Ogg layer and decodes with `rusty-opus` (ADR-0047); `fakeflac-core` gains no type (it already has `AudioCodec::Opus`) and computes the null estimate in its pure lossy functions; `src-tauri` maps nothing new.

## Supported files (RF-14)

Extensions, case-insensitive: `.wav`, `.flac` (H1); `.mp3`, `.m4a` (H2, version 3); `.opus` (H2, detection v8, ADR-0047; Ogg Opus with channel mapping family 0). `.mp4`, `.m4b`, `.aac`, `.ogg` and `.oga` are not supported (`.ogg` and `.oga` may hold Vorbis or FLAC, ADR-0047 point 4). Other files are skipped silently by the scan. Hidden files and folders (starting with `.`) are skipped. Symbolic links are not followed.

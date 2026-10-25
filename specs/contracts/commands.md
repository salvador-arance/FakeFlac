---
id: SPEC-CONTRACT-H1
requirements: [RF-01, RF-02, RF-05, RF-06, RF-07, RF-09, RF-14, RF-15]
status: proposed
version: 4
---

# Backend ↔ frontend contract (H1, with the H2 lossy formats and CSV export)

> The commands, events and types the frontend can use. They are defined once in Rust and the TypeScript bindings are **generated** with `tauri-specta` (S3, ADR-0003); this document is the readable reference. Field names are camelCase on the TS side. Settings and cache commands arrive later in H2. Version 3 adds MP3 and M4A files (RF-14 H2) and the source-bitrate estimate (RF-15); changes are listed in "Lossy formats (version 3)". Version 4 adds `export_csv` (RF-09); changes are listed in "CSV export (version 4)".

## Commands

| Command | Input | Output | Errors (`AppError.code`) | Req. |
|---|---|---|---|---|
| `pick_folder` | — | `string \| null` (absolute path; `null` if the dialog is cancelled) | — | RF-01 |
| `start_analysis` | `root: string` (absolute path), `onProgress: Channel<ProgressEvent>` | `AnalysisSummary` when the run ends (finished or cancelled) | `NOT_A_FOLDER`, `FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`, `ANALYSIS_RUNNING`, `INTERNAL` | RF-01..06 |
| `cancel_analysis` | — | `void` (no-op if nothing is running) | — | RF-05 |
| `get_spectrogram` | `path: string` | `Spectrogram` | `NOT_ANALYZED` is not used in H1; `INTERNAL` if the file cannot be decoded | RF-07 |
| `export_csv` (version 4) | — (the destination comes from the native save dialog, never from the frontend) | `ExportResult \| null` (`null` if the dialog is cancelled) | `NOT_ANALYZED`, `ANALYSIS_RUNNING`, `EXPORT_FAILED`, `INTERNAL` | RF-09 |

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
  | { kind: 'trackDone'; index: number; analysis: TrackAnalysis }
  | { kind: 'finished'; summary: AnalysisSummary }
  | { kind: 'cancelled'; summary: AnalysisSummary }
```

`index` is the 0-based position of the file in the scan order (sorted by path, case-insensitive). `trackDone` events may arrive out of index order (parallel analysis).

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

type TrackInfo = {
  path: string; folder: string; format: AudioFormat
  codec: AudioCodec | null                           // version 3; null only when an M4A could not be probed
  sampleRate: number; bitDepth: number | null; channels: number
  declaredBitrateKbps: number | null                 // lossy codecs (mp3, aac, opus); null for lossless codecs and when not known (errors before the stream was read)
  bitrateMode: BitrateMode | null                    // version 3; MP3 only (AAC does not signal it)
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
  estimatedSourceBitrateKbps: number | null          // RF-15: lossy codecs only (specs/detection/lossy-source.md section 5)
  reasons: SuspicionReason[]
  error: TrackError | null
  algorithmVersion: number                           // 6 with the default DetectionParams (detection v6: lossless rules of rules.md, lossy rules of lossy-source.md)
}

type AnalysisSummary = {
  root: string; total: number; done: number
  real: number; suspicious: number; fake: number; errors: number
  cancelled: boolean; elapsedMs: number
}

type Spectrogram = {
  width: number            // time bins, ≤ spectrogram.max_width
  height: number           // frequency bins (0 Hz at row 0)
  sampleRate: number
  maxFrequencyHz: number   // = sampleRate / 2
  dbRange: [number, number] // u8 0 → dbRange[0], 255 → dbRange[1]
  data: number[]           // width*height bytes (0–255), row-major by time (ADR-0009); sent as a JSON array of numbers
}

type ExportResult = {     // version 4 (RF-09)
  path: string             // absolute path of the CSV written (the one chosen in the dialog)
  rows: number             // tracks written (header not counted), ≥ 1
}

type AppError = {
  code: 'NO_AUDIO_FOUND' | 'NOT_A_FOLDER' | 'FOLDER_UNREADABLE' | 'ANALYSIS_RUNNING' | 'NOT_ANALYZED' | 'EXPORT_FAILED' | 'INTERNAL'
  message: string
}
```

Per-track problems never become an `AppError`: they travel inside `TrackAnalysis` (`verdict: 'error'` + `error`), ADR-0013.

Detection v3 (ADR-0038) does not change these types: a track that the full-band guard keeps from `fake` is `suspicious` with the existing reasons `highBandGaps` or `ambiguousCutoff`. Its new measurements (`U`, `B`, `FullBand`) and its new `DetectionParams` section `[full_band]` are internal to the backend (schema and semantics: `specs/detection/rules.md` sections 2 and 4, values: `specs/detection/detection-params.v3.toml`), so `just gen` produces the same bindings.

Detection v4 (ADR-0039) does not change them either. It adds the optional field `full_band.max_lossy_step_db` to `DetectionParams` (internal to the backend; values: `specs/detection/detection-params.v4.toml`) and narrows `FullBand`, so with the defaults a guarded track is always `suspicious` with `highBandGaps`. The only visible change is `algorithmVersion` = 4, which invalidates cached results (ADR-0008).

Detection v6 (ADR-0042) does not change them either. It adds the optional section `edge` to `DetectionParams` (internal to the backend; values: `specs/detection/detection-params.v6.toml`), which can only turn a lossless `fake` into `suspicious` with `ambiguousCutoff` (or `highBandGaps`). The only visible change is `algorithmVersion` = 6 with the defaults, which invalidates cached results (ADR-0008); lossy results equal version 5's.

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

## Supported files (RF-14)

Extensions, case-insensitive: `.wav`, `.flac` (H1); `.mp3`, `.m4a` (H2, version 3). **`.opus` is still skipped** until the Opus decoder is chosen (ADR-0030 spike; pending scenarios in `specs/pending/opus.feature`). `.mp4`, `.m4b`, `.aac` and `.ogg` are not supported. Other files are skipped silently by the scan. Hidden files and folders (starting with `.`) are skipped. Symbolic links are not followed.

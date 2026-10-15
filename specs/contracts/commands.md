---
id: SPEC-CONTRACT-H1
requirements: [RF-01, RF-02, RF-05, RF-06, RF-07]
status: proposed
version: 2
---

# Backend ↔ frontend contract (H1)

> The commands, events and types the frontend can use. They are defined once in Rust and the TypeScript bindings are **generated** with `tauri-specta` (S3, ADR-0003); this document is the readable reference. Field names are camelCase on the TS side. `export_csv`, settings and cache commands arrive in H2.

## Commands

| Command | Input | Output | Errors (`AppError.code`) | Req. |
|---|---|---|---|---|
| `pick_folder` | — | `string \| null` (absolute path; `null` if the dialog is cancelled) | — | RF-01 |
| `start_analysis` | `root: string` (absolute path), `onProgress: Channel<ProgressEvent>` | `AnalysisSummary` when the run ends (finished or cancelled) | `NOT_A_FOLDER`, `FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`, `ANALYSIS_RUNNING`, `INTERNAL` | RF-01..06 |
| `cancel_analysis` | — | `void` (no-op if nothing is running) | — | RF-05 |
| `get_spectrogram` | `path: string` | `Spectrogram` | `NOT_ANALYZED` is not used in H1; `INTERNAL` if the file cannot be decoded | RF-07 |

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
type AudioFormat = 'wav' | 'flac' | 'mp3' | 'm4a' | 'opus'
type SuspicionReason = 'lowHighFrequencyContent' | 'gradualRolloff' | 'ambiguousCutoff'

type TrackInfo = {
  path: string; folder: string; format: AudioFormat
  sampleRate: number; bitDepth: number | null; channels: number
  declaredBitrateKbps: number | null                 // lossy formats (H2)
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
  estimatedSourceBitrateKbps: number | null          // RF-15 (H2)
  reasons: SuspicionReason[]
  error: TrackError | null
  algorithmVersion: number
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
  data: number[]           // width*height bytes, row-major by time (ADR-0009); sent as raw bytes
}

type AppError = {
  code: 'NO_AUDIO_FOUND' | 'NOT_A_FOLDER' | 'FOLDER_UNREADABLE' | 'ANALYSIS_RUNNING' | 'NOT_ANALYZED' | 'EXPORT_FAILED' | 'INTERNAL'
  message: string
}
```

Per-track problems never become an `AppError`: they travel inside `TrackAnalysis` (`verdict: 'error'` + `error`), ADR-0013.

## Supported files in H1 (RF-14)

Extensions, case-insensitive: `.wav`, `.flac`. Other files are skipped silently by the scan. Hidden files and folders (starting with `.`) are skipped. Symbolic links are not followed.

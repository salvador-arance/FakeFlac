---
id: SPEC-CONTRACT-H1
requirements: [RF-01, RF-02, RF-05, RF-06, RF-07, RF-14, RF-15]
status: proposed
version: 3
---

# Backend ↔ frontend contract (H1, with the H2 lossy formats)

> The commands, events and types the frontend can use. They are defined once in Rust and the TypeScript bindings are **generated** with `tauri-specta` (S3, ADR-0003); this document is the readable reference. Field names are camelCase on the TS side. `export_csv`, settings and cache commands arrive in H2. Version 3 adds MP3 and M4A files (RF-14 H2) and the source-bitrate estimate (RF-15); changes are listed in "Lossy formats (version 3)".

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

## Supported files (RF-14)

Extensions, case-insensitive: `.wav`, `.flac` (H1); `.mp3`, `.m4a` (H2, version 3). **`.opus` is still skipped** until the Opus decoder is chosen (ADR-0030 spike; pending scenarios in `specs/pending/opus.feature`). `.mp4`, `.m4b`, `.aac` and `.ogg` are not supported. Other files are skipped silently by the scan. Hidden files and folders (starting with `.`) are skipped. Symbolic links are not followed.

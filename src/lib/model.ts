// UI model types shared by the presentational components, the store and the page.
// Components may not import `src/lib/api`, so the domain types they need are re-exported here.
import type { AnalysisSummary, TrackAnalysis } from './api/bindings'

export type {
  AnalysisSummary,
  AudioCodec,
  AudioFormat,
  BitrateMode,
  CutoffEstimate,
  Spectrogram,
  SuspicionReason,
  TrackAnalysis,
  TrackError,
  TrackInfo,
  Verdict,
} from './api/bindings'

/** A table row. `index` comes from the progress events and is the row key (never the path). */
export type ResultRow = { index: number; analysis: TrackAnalysis }

/** Columns of the results table, in display order (specs/ui/main-screen.md). */
export type SortColumn =
  | 'state'
  | 'file'
  | 'folder'
  | 'format'
  | 'sampleRate'
  | 'bitDepth'
  | 'duration'
  | 'cutoff'
  | 'bitrate'
  | 'realBitrate'

export type SortState = { column: SortColumn; direction: 'asc' | 'desc' }

export type StatusPhase = 'ready' | 'scanning' | 'analysing' | 'finished' | 'cancelled' | 'error'
export type AnalysisPhase = StatusPhase

/** Counters shown by the status bar (a live subset of `AnalysisSummary`). */
export type StatusSummary = Pick<
  AnalysisSummary,
  'total' | 'done' | 'real' | 'suspicious' | 'fake' | 'errors'
>

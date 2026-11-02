// UI model types shared by the presentational components, the store and the page.
// Components may not import `src/lib/api`, so the domain types they need are re-exported here.
import type { AnalysisSummary, CacheStatus, TrackAnalysis, Verdict } from './api/bindings'

export type {
  AlbumSummary,
  AlbumUpdate,
  AnalysisSummary,
  AudioCodec,
  AudioFormat,
  BitrateMode,
  CacheStatus,
  CutoffEstimate,
  Language,
  SettingsLoadStatus,
  Spectrogram,
  SuspicionReason,
  TrackAnalysis,
  TrackError,
  TrackInfo,
  Verdict,
} from './api/bindings'

/**
 * A table row. `index` comes from the progress events and is the row key (never the path);
 * `albumIndex` is the `album.index` of its `trackDone` (specs/ui/album-summary.md).
 */
export type ResultRow = { index: number; analysis: TrackAnalysis; albumIndex: number }

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

/**
 * The table's verdict filter and name search (specs/ui/sort-filter.md section 10, "Model"). UI type,
 * not a contract type. `verdicts` empty = no verdict filter; otherwise each pressed verdict once, in
 * the canonical order fake, suspicious, real, error. `query` is the search field's raw text.
 */
export type RowFilter = { verdicts: Verdict[]; query: string }

export type StatusPhase = 'ready' | 'scanning' | 'analysing' | 'finished' | 'cancelled' | 'error'
export type AnalysisPhase = StatusPhase

/** Counters shown by the status bar (a live subset of `AnalysisSummary`). */
export type StatusSummary = Pick<
  AnalysisSummary,
  'total' | 'done' | 'real' | 'suspicious' | 'fake' | 'errors'
>

/**
 * RF-13 (specs/ui/language-settings.md section 4, `SettingsPanel` row): the save failure shown in
 * the Settings panel. `internal` is an unexpected failure (`settings.saveFailed`).
 */
export type SettingsError = 'writeFailed' | 'readOnly' | 'internal'

/** RF-11 (specs/ui/result-cache.md section 4): the cache figures of the latest ended run. */
export type CacheFigures = { fromCache: number; status: CacheStatus }

// Synthetic data for component tests and stories. No real audio, no corpus.
import type {
  AnalysisSummary,
  AudioFormat,
  ResultRow,
  SuspicionReason,
  TrackAnalysis,
  TrackError,
  Verdict,
} from '../model'

export type TrackOptions = {
  path?: string
  format?: AudioFormat
  sampleRate?: number
  bitDepth?: number | null
  durationMs?: number
  verdict?: Verdict
  cutoffHz?: number | null
  reasons?: SuspicionReason[]
  error?: TrackError | null
}

/** A `TrackAnalysis` with sensible defaults (a real 44.1 kHz / 16-bit FLAC). */
export function makeTrack(o: TrackOptions = {}): TrackAnalysis {
  const path = o.path ?? 'C:/Music/Album A/track.flac'
  const cut = path.lastIndexOf('/')
  return {
    info: {
      path,
      folder: cut >= 0 ? path.slice(0, cut) : '',
      format: o.format ?? 'flac',
      sampleRate: o.sampleRate ?? 44100,
      bitDepth: o.bitDepth === undefined ? 16 : o.bitDepth,
      channels: 2,
      declaredBitrateKbps: null,
      durationMs: o.durationMs ?? 181000,
      sizeBytes: 1_000_000,
      modifiedUnix: 1_700_000_000,
    },
    verdict: o.verdict ?? 'real',
    cutoff:
      o.cutoffHz === undefined || o.cutoffHz === null
        ? null
        : { frequencyHz: o.cutoffHz, steepnessDbPerKhz: 40, noiseFloorDb: -90 },
    estimatedSourceBitrateKbps: null,
    reasons: o.reasons ?? [],
    error: o.error ?? null,
    algorithmVersion: 2,
  }
}

export function makeRow(index: number, o: TrackOptions = {}): ResultRow {
  return { index, analysis: makeTrack(o) }
}

export function makeSummary(o: Partial<AnalysisSummary> = {}): AnalysisSummary {
  return {
    root: 'C:/Music/Album A',
    total: 4,
    done: 4,
    real: 4,
    suspicious: 0,
    fake: 0,
    errors: 0,
    cancelled: false,
    elapsedMs: 1000,
    ...o,
  }
}

/** `count` rows named `file-0000.flac`, `file-0001.flac`, ... in index order. */
export function makeManyRows(count: number): ResultRow[] {
  return Array.from({ length: count }, (_, i) =>
    makeRow(i, { path: `C:/Music/Big/file-${String(i).padStart(4, '0')}.flac` }),
  )
}

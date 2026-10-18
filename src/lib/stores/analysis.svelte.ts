// The `analysis` store: rows, phase, selection, sort and counters (specs/ui/components.md).
import type { ProgressEvent } from '../api'
import type { ResultRow, SortColumn, SortState, StatusPhase, StatusSummary } from '../model'

export interface AnalysisStore {
  readonly rows: ResultRow[]
  readonly phase: StatusPhase
  readonly summary: StatusSummary
  readonly message: string | undefined
  readonly selectedIndex: number | null
  readonly sort: SortState
  /** Marks the run as requested (scanning). */
  begin(): void
  /** Resets to ready state. */
  reset(): void
  /** Applies one progress event (rows are batched, at most one update per 100 ms). */
  apply(event: ProgressEvent): void
  /** The run ended with a contract error. */
  fail(message: string): void
  select(index: number): void
  toggleSort(column: SortColumn): void
}

export function createAnalysisStore(): AnalysisStore {
  let rows = $state<ResultRow[]>([])
  let pendingRows: ResultRow[] = []

  let phase = $state<StatusPhase>('ready')
  let summary = $state<StatusSummary>({
    total: 0,
    done: 0,
    real: 0,
    suspicious: 0,
    fake: 0,
    errors: 0,
  })
  let pendingSummary: StatusSummary = {
    total: 0,
    done: 0,
    real: 0,
    suspicious: 0,
    fake: 0,
    errors: 0,
  }

  let message = $state<string | undefined>(undefined)
  let selectedIndex = $state<number | null>(null)
  let sort = $state<SortState>({ column: 'file', direction: 'asc' })

  let batchTimer: ReturnType<typeof setTimeout> | null = null

  function flushBatch() {
    if (batchTimer !== null) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    if (pendingRows.length > 0) {
      rows = [...rows, ...pendingRows]
      pendingRows = []
    }
    summary = { ...pendingSummary }
  }

  function begin() {
    if (batchTimer !== null) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    rows = []
    pendingRows = []
    phase = 'scanning'
    summary = { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 }
    pendingSummary = { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 }
    message = undefined
    selectedIndex = null
  }

  function reset() {
    if (batchTimer !== null) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    pendingRows = []
    phase = 'ready'
    message = undefined
  }

  function apply(event: ProgressEvent) {
    if (event.kind === 'scanned') {
      summary.total = event.total
      pendingSummary.total = event.total
      phase = 'analysing'
    } else if (event.kind === 'trackDone') {
      phase = 'analysing'
      pendingRows.push({ index: event.index, analysis: event.analysis })
      pendingSummary.done += 1
      if (event.analysis.verdict === 'real') {
        pendingSummary.real += 1
      } else if (event.analysis.verdict === 'suspicious') {
        pendingSummary.suspicious += 1
      } else if (event.analysis.verdict === 'fake') {
        pendingSummary.fake += 1
      } else if (event.analysis.verdict === 'error') {
        pendingSummary.errors += 1
      }

      if (batchTimer === null) {
        batchTimer = setTimeout(() => {
          flushBatch()
        }, 100)
      }
    } else if (event.kind === 'finished') {
      flushBatch()
      phase = 'finished'
      summary = {
        total: event.summary.total,
        done: event.summary.done,
        real: event.summary.real,
        suspicious: event.summary.suspicious,
        fake: event.summary.fake,
        errors: event.summary.errors,
      }
      pendingSummary = { ...summary }
    } else if (event.kind === 'cancelled') {
      flushBatch()
      phase = 'cancelled'
      summary = {
        total: event.summary.total,
        done: event.summary.done,
        real: event.summary.real,
        suspicious: event.summary.suspicious,
        fake: event.summary.fake,
        errors: event.summary.errors,
      }
      pendingSummary = { ...summary }
    }
  }

  function fail(errorMessage: string) {
    flushBatch()
    phase = 'error'
    message = errorMessage
  }

  function select(index: number) {
    selectedIndex = index
  }

  function toggleSort(column: SortColumn) {
    if (sort.column === column) {
      sort = { column, direction: sort.direction === 'asc' ? 'desc' : 'asc' }
    } else {
      sort = { column, direction: 'asc' }
    }
  }

  return {
    get rows() {
      return rows
    },
    get phase() {
      return phase
    },
    get summary() {
      return summary
    },
    get message() {
      return message
    },
    get selectedIndex() {
      return selectedIndex
    },
    get sort() {
      return sort
    },
    begin,
    reset,
    apply,
    fail,
    select,
    toggleSort,
  }
}

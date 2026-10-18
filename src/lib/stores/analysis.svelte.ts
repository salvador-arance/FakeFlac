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
  /** Applies one progress event (rows are batched, at most one update per 100 ms). */
  apply(event: ProgressEvent): void
  /** The run ended with a contract error. */
  fail(message: string): void
  select(index: number): void
  toggleSort(column: SortColumn): void
}

export function createAnalysisStore(): AnalysisStore {
  throw new Error('not implemented')
}

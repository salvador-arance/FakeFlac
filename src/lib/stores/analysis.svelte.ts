// The `analysis` store: rows, phase, selection, sort and counters (specs/ui/components.md).
import type { ProgressEvent } from '../api'
import type { AlbumUpdate, ResultRow, SortColumn, SortState, StatusPhase, StatusSummary } from '../model'

export interface AnalysisStore {
  readonly rows: ResultRow[]
  /** Latest update of each album of the run, sorted by `index` (specs/ui/album-summary.md). */
  readonly albums: AlbumUpdate[]
  readonly phase: StatusPhase
  readonly summary: StatusSummary
  readonly message: string | undefined
  readonly selectedIndex: number | null
  readonly sort: SortState
  /** Marks the run as requested (scanning). */
  begin(): void
  /** Puts back the screen state saved by the last `begin()` (a start that did not begin a run). */
  restore(): void
  /** Resets to ready state. */
  reset(): void
  /** Applies one progress event (rows are batched, at most one update per 100 ms). */
  apply(event: ProgressEvent): void
  /** The run ended with a contract error. */
  fail(message: string): void
  select(index: number): void
  /** Selects the row with the lowest `index` among the rows of the album. */
  selectAlbum(albumIndex: number): void
  toggleSort(column: SortColumn): void
}

export function createAnalysisStore(): AnalysisStore {
  let rows = $state<ResultRow[]>([])
  let pendingRows: ResultRow[] = []
  // Albums: `latest` holds the newest update of each album (flushed and pending); `albums` is the
  // flushed view, sorted by index. They are published with the rows, in the same batch.
  let albums = $state.raw<AlbumUpdate[]>([])
  let latest = new Map<number, AlbumUpdate>()

  function publishAlbums() {
    albums = [...latest.values()].sort((a, b) => a.index - b.index)
  }

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
    publishAlbums()
    summary = { ...pendingSummary }
  }

  let snapshot: {
    rows: ResultRow[]
    albums: Map<number, AlbumUpdate>
    phase: StatusPhase
    summary: StatusSummary
    message: string | undefined
    selectedIndex: number | null
  } | null = null

  function begin() {
    snapshot = {
      rows: [...rows, ...pendingRows],
      albums: new Map(latest),
      phase,
      summary: { ...pendingSummary },
      message,
      selectedIndex,
    }
    if (batchTimer !== null) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    rows = []
    pendingRows = []
    latest = new Map()
    albums = []
    phase = 'scanning'
    summary = { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 }
    pendingSummary = { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 }
    message = undefined
    selectedIndex = null
  }

  function restore() {
    if (snapshot === null) return
    if (batchTimer !== null) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    rows = snapshot.rows
    pendingRows = []
    latest = snapshot.albums
    publishAlbums()
    phase = snapshot.phase
    summary = { ...snapshot.summary }
    pendingSummary = { ...snapshot.summary }
    message = snapshot.message
    selectedIndex = snapshot.selectedIndex
    snapshot = null
  }

  function reset() {
    if (batchTimer !== null) {
      clearTimeout(batchTimer)
      batchTimer = null
    }
    pendingRows = []
    latest = new Map(albums.map((a) => [a.index, a]))
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
      pendingRows.push({ index: event.index, analysis: event.analysis, albumIndex: event.album.index })
      const stored = latest.get(event.album.index)
      if (!stored || event.album.summary.total > stored.summary.total) {
        latest.set(event.album.index, event.album)
      }
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

  function selectAlbum(albumIndex: number) {
    let lowest: number | null = null
    for (const row of [...rows, ...pendingRows]) {
      if (row.albumIndex === albumIndex && (lowest === null || row.index < lowest)) {
        lowest = row.index
      }
    }
    if (lowest !== null) selectedIndex = lowest
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
    get albums() {
      return albums
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
    restore,
    reset,
    apply,
    fail,
    select,
    selectAlbum,
    toggleSort,
  }
}

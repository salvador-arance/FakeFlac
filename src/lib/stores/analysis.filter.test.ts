// Store checks F1-F6 of specs/ui/sort-filter.md section 11 and the store rules of section 10
// ("Model, store and the view rules") (@RF-08): the filter is a view over the flushed rows, kept
// across runs like the sort, and `selectAlbum` picks the album's lowest-index shown row.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "Flush" is the store's 100 ms batch timer, driven with fake timers as in analysis.albums.test.ts.
// - `visibleRows` is compared by row `index`, in store order (the order the rows were flushed).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProgressEvent } from '../api'
import type { Verdict } from '../model'
import { makeAlbum, makeSummary, makeTrack } from '../testing/fixtures'
import { createAnalysisStore } from './analysis.svelte'

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

const flush = () => vi.advanceTimersByTime(100)

/** A trackDone of track `index` in the album `albumIndex` named `albumName`. */
function done(
  index: number,
  verdict: Verdict,
  albumIndex = 0,
  albumName = 'Lib\\A',
  file = `${String(index).padStart(2, '0')}.flac`,
): ProgressEvent {
  return {
    kind: 'trackDone',
    index,
    analysis: makeTrack({ path: `C:/Music/${albumName.replaceAll('\\', '/')}/${file}`, verdict }),
    album: makeAlbum({ index: albumIndex, name: albumName, real: 1 }),
  }
}

const TEN: Verdict[] = ['real', 'fake', 'real', 'suspicious', 'fake', 'error', 'real', 'real', 'fake', 'real']

/** A store with the ten rows of `TEN` (index = position), flushed. */
function tenRows() {
  const store = createAnalysisStore()
  store.begin()
  store.apply({ kind: 'scanned', total: 10 })
  TEN.forEach((verdict, i) => store.apply(done(i, verdict)))
  flush()
  return store
}

const indexes = (rows: { index: number }[]) => rows.map((r) => r.index)

describe('analysis store: filter (RF-08)', () => {
  it('starts with no filter and every flushed row visible', () => {
    const store = createAnalysisStore()
    expect(store.filter).toEqual({ verdicts: [], query: '' })
    expect(store.visibleRows).toEqual([])
    const ten = tenRows()
    expect(indexes(ten.visibleRows)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
  })

  // @RF-08 F1 (filter is a view)
  it('F1: hides rows without removing them from the store', () => {
    const store = tenRows()
    store.toggleVerdictFilter('fake')
    expect(store.rows).toHaveLength(10)
    expect(indexes(store.visibleRows)).toEqual([1, 4, 8])
    store.toggleVerdictFilter('fake')
    expect(store.rows).toHaveLength(10)
    expect(indexes(store.visibleRows)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
  })

  // @RF-08 several verdicts: the union
  it('shows the union of the pressed verdicts, in store order', () => {
    const store = tenRows()
    store.toggleVerdictFilter('error')
    store.toggleVerdictFilter('suspicious')
    expect(indexes(store.visibleRows)).toEqual([3, 5])
  })

  // @RF-08 the name search narrows the visible rows
  it('applies the query to the visible rows and combines it with the verdicts (AND)', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply(done(0, 'fake', 0, 'Lib\\A', '01.flac'))
    store.apply(done(1, 'real', 0, 'Lib\\A', '02.flac'))
    store.apply(done(2, 'fake', 1, 'Lib\\B', '01.flac'))
    flush()
    store.setQuery('lib\\a')
    expect(indexes(store.visibleRows)).toEqual([0, 1])
    store.toggleVerdictFilter('fake')
    expect(indexes(store.visibleRows)).toEqual([0])
  })

  it('stores the query as given (not trimmed or folded)', () => {
    const store = createAnalysisStore()
    store.setQuery('  \u00c1lbum \u2033\u00d1and\u00fa\u2033 ')
    expect(store.filter.query).toBe('  \u00c1lbum \u2033\u00d1and\u00fa\u2033 ')
  })

  // @RF-08 F2 (kept across runs)
  it('F2: begin, restore, begin, fail and reset change neither the filter nor the sort', () => {
    const store = tenRows()
    store.toggleVerdictFilter('fake')
    store.setQuery('x')
    store.toggleSort('state')
    store.toggleSort('state')
    const sort = { column: 'state', direction: 'desc' }
    expect(store.sort).toEqual(sort)
    const check = () => {
      expect(store.filter).toEqual({ verdicts: ['fake'], query: 'x' })
      expect(store.sort).toEqual(sort)
    }
    store.begin()
    check()
    store.restore()
    check()
    store.begin()
    check()
    store.fail('The analysis stopped because of an unexpected error')
    check()
    store.reset()
    check()
  })

  it('F2: also after a run that left no row (begin, restore, reset)', () => {
    const store = createAnalysisStore()
    store.toggleVerdictFilter('fake')
    store.setQuery('x')
    store.toggleSort('state')
    store.toggleSort('state')
    store.begin()
    store.apply({ kind: 'scanned', total: 10 })
    store.apply({
      kind: 'cancelled',
      summary: makeSummary({ total: 10, done: 0, real: 0, cancelled: true }),
    })
    store.begin()
    store.restore()
    store.reset()
    expect(store.phase).toBe('ready')
    expect(store.rows).toEqual([])
    expect(store.visibleRows).toEqual([])
    expect(store.filter).toEqual({ verdicts: ['fake'], query: 'x' })
    expect(store.sort).toEqual({ column: 'state', direction: 'desc' })
  })

  it('the visible rows follow the rows through begin and restore', () => {
    const store = tenRows()
    store.toggleVerdictFilter('fake')
    store.begin()
    expect(store.visibleRows).toEqual([])
    store.restore()
    expect(indexes(store.visibleRows)).toEqual([1, 4, 8])
  })

  // @RF-08 F3 (batches)
  it('F3: pending rows are not visible until the flush; then exactly the flushed rows that pass', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 6 })
    store.toggleVerdictFilter('fake')
    store.apply(done(0, 'fake'))
    store.apply(done(1, 'real'))
    store.apply(done(2, 'fake'))
    expect(store.visibleRows).toEqual([])
    flush()
    expect(indexes(store.visibleRows)).toEqual([0, 2])
    store.apply(done(3, 'fake'))
    expect(indexes(store.visibleRows)).toEqual([0, 2])
    flush()
    expect(indexes(store.visibleRows)).toEqual([0, 2, 3])
  })

  it('F3: a name filter finds a just-flushed row by its album name', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 2 })
    store.setQuery('album 1')
    store.apply(done(0, 'real', 0, 'Artist\\Album 1', '01.flac'))
    store.apply(done(1, 'real', 1, 'Artist\\Other', '02.flac'))
    expect(store.visibleRows).toEqual([])
    flush()
    expect(indexes(store.visibleRows)).toEqual([0])
  })

  it('F3: the flush at "finished" publishes the visible rows', () => {
    const store = createAnalysisStore()
    store.begin()
    store.toggleVerdictFilter('suspicious')
    store.apply({ kind: 'scanned', total: 2 })
    store.apply(done(0, 'suspicious'))
    store.apply(done(1, 'real'))
    store.apply({ kind: 'finished', summary: makeSummary({ total: 2, done: 2, real: 1, suspicious: 1 }) })
    expect(indexes(store.visibleRows)).toEqual([0])
  })

  // @RF-08 F4 (selectAlbum with a filter)
  it('F4: selects the lowest-index shown row of the album, else its lowest-index row', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 4 })
    store.apply(done(0, 'real', 0, 'Lib\\A'))
    store.apply(done(7, 'fake', 1, 'Lib\\B'))
    store.apply(done(9, 'fake', 1, 'Lib\\B'))
    flush()
    store.apply(done(4, 'real', 1, 'Lib\\B')) // pending
    store.toggleVerdictFilter('fake')
    store.selectAlbum(1)
    expect(store.selectedIndex).toBe(7)
    store.toggleVerdictFilter('fake')
    store.toggleVerdictFilter('suspicious')
    store.selectAlbum(1)
    expect(store.selectedIndex).toBe(4)
  })

  it('F4: judges pending rows with the album names of the latest updates', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 3 })
    store.apply(done(2, 'fake', 0, 'Lib\\B'))
    store.apply(done(9, 'fake', 0, 'Lib\\B'))
    flush()
    store.apply(done(7, 'real', 0, 'Lib\\B')) // pending, the only one that passes
    store.toggleVerdictFilter('real')
    store.setQuery('lib\\b')
    store.selectAlbum(0)
    expect(store.selectedIndex).toBe(7)
  })

  it('a filter change never changes the selection', () => {
    const store = tenRows()
    store.select(0)
    store.toggleVerdictFilter('fake')
    expect(store.selectedIndex).toBe(0)
    store.setQuery('zzz')
    expect(store.selectedIndex).toBe(0)
    store.clearFilter()
    expect(store.selectedIndex).toBe(0)
  })

  // @RF-08 F5 (clearFilter)
  it('F5: clearFilter clears verdicts and query, not the sort or the selection', () => {
    const store = tenRows()
    store.toggleVerdictFilter('fake')
    store.toggleVerdictFilter('error')
    store.setQuery('01')
    store.toggleSort('duration')
    store.select(4)
    store.clearFilter()
    expect(store.filter).toEqual({ verdicts: [], query: '' })
    expect(store.sort).toEqual({ column: 'duration', direction: 'asc' })
    expect(store.selectedIndex).toBe(4)
    expect(store.visibleRows).toHaveLength(10)
  })

  // @RF-08 F6 (canonical order)
  it('F6: keeps the verdicts in the canonical order fake, suspicious, real, error', () => {
    const store = createAnalysisStore()
    store.toggleVerdictFilter('error')
    store.toggleVerdictFilter('fake')
    store.toggleVerdictFilter('real')
    expect(store.filter.verdicts).toEqual(['fake', 'real', 'error'])
    store.toggleVerdictFilter('suspicious')
    expect(store.filter.verdicts).toEqual(['fake', 'suspicious', 'real', 'error'])
    store.toggleVerdictFilter('real')
    expect(store.filter.verdicts).toEqual(['fake', 'suspicious', 'error'])
  })
})

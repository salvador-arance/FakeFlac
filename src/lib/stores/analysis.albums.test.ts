// Store checks S1-S5 of specs/ui/album-summary.md ("Store checks") and the store rules around them
// (@RF-10): albums are kept per album index, flushed with the rows, saved by begin(), brought back
// by restore() and kept by fail(). The store never computes a count: it keeps the latest update.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProgressEvent } from '../api'
import type { Verdict } from '../model'
import { makeAlbum, makeSummary, makeTrack } from '../testing/fixtures'
import type { AlbumOptions } from '../testing/fixtures'
import { createAnalysisStore } from './analysis.svelte'

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

/** A stubbed trackDone of track `index` whose album update is `makeAlbum(album)`. */
function done(index: number, album: AlbumOptions, verdict: Verdict = 'real'): ProgressEvent {
  return {
    kind: 'trackDone',
    index,
    analysis: makeTrack({ path: `C:/Music/${album.name ?? 'Album A'}/${index}.flac`, verdict }),
    album: makeAlbum(album),
  }
}

const flush = () => vi.advanceTimersByTime(100)

/** Two albums: `A` (index 0, tracks 0, 1) and `B` (index 1, track 2), flushed. */
function twoAlbumsFlushed() {
  const store = createAnalysisStore()
  store.begin()
  store.apply({ kind: 'scanned', total: 3 })
  store.apply(done(0, { index: 0, name: 'Lib\\A', real: 1, files: 2 }))
  store.apply(done(1, { index: 0, name: 'Lib\\A', real: 2, files: 2 }))
  store.apply(done(2, { index: 1, name: 'Lib\\B', real: 1, files: 1 }))
  flush()
  return store
}

describe('analysis store: albums (RF-10)', () => {
  // @RF-10 the row carries the album index of its trackDone
  it('gives every row the album index of its trackDone', () => {
    const store = twoAlbumsFlushed()
    expect(store.rows.map((r) => [r.index, r.albumIndex])).toEqual([
      [0, 0],
      [1, 0],
      [2, 1],
    ])
  })

  // @RF-10 "Albums appear in path order, whatever order they arrive in"
  it('keeps the albums sorted by index, whatever order they arrive in', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply(done(0, { index: 2, name: 'Lib\\B' }))
    flush()
    store.apply(done(1, { index: 0, name: 'Lib\\A' }))
    flush()
    expect(store.albums.map((a) => [a.index, a.name])).toEqual([
      [0, 'Lib\\A'],
      [2, 'Lib\\B'],
    ])
  })

  // @RF-10 the store keeps the latest update of each album (replaces the previous one)
  it('keeps the latest update of each album', () => {
    const store = twoAlbumsFlushed()
    expect(store.albums).toEqual([
      makeAlbum({ index: 0, name: 'Lib\\A', real: 2, files: 2 }),
      makeAlbum({ index: 1, name: 'Lib\\B', real: 1, files: 1 }),
    ])
  })

  // @RF-10 "Summaries update during the analysis": rows and albums are flushed together (A4)
  it('flushes the albums with the rows, not before', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 1 })
    store.apply(done(0, { index: 0 }))
    expect(store.rows).toHaveLength(0)
    expect(store.albums).toHaveLength(0)
    flush()
    expect(store.rows).toHaveLength(1)
    expect(store.albums).toHaveLength(1)
  })

  // @RF-10 the batches end at "finished" and "cancelled" with the albums in them
  it.each(['finished', 'cancelled'] as const)('flushes the pending albums on %s', (kind) => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 1 })
    store.apply(done(0, { index: 0 }))
    store.apply({ kind, summary: makeSummary({ total: 1, done: 1, real: 1 }) })
    expect(store.albums).toHaveLength(1)
    expect(store.rows).toHaveLength(1)
  })

  // @RF-10 S1 (begin)
  describe('S1: begin()', () => {
    it('clears the albums with the rows', () => {
      const store = twoAlbumsFlushed()
      store.begin()
      expect(store.albums).toEqual([])
      expect(store.rows).toEqual([])
    })

    it('clears the albums also when some updates were still pending', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply(done(0, { index: 0, name: 'Lib\\A' }))
      store.apply(done(1, { index: 1, name: 'Lib\\B' }))
      store.begin()
      expect(store.albums).toEqual([])
      flush()
      expect(store.albums).toEqual([])
      expect(store.rows).toEqual([])
    })
  })

  // @RF-10 S2 (restore)
  describe('S2: restore()', () => {
    it('brings back the albums and the current album', () => {
      const store = twoAlbumsFlushed()
      store.select(2) // a row of the second album
      const before = store.albums.map((a) => structuredClone(a))
      store.begin()
      store.restore()
      expect(store.albums).toEqual(before)
      expect(store.albums.map((a) => a.index)).toEqual([0, 1])
      const current = store.rows.find((r) => r.index === store.selectedIndex)?.albumIndex
      expect(current).toBe(1)
    })

    it('brings back the updates that were still pending at begin()', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply(done(0, { index: 0, name: 'Lib\\A', real: 1, files: 2 }))
      flush()
      store.apply(done(1, { index: 0, name: 'Lib\\A', real: 2, files: 2 }))
      store.apply(done(2, { index: 1, name: 'Lib\\B', real: 1, files: 1 }))
      store.begin()
      store.restore()
      expect(store.albums).toEqual([
        makeAlbum({ index: 0, name: 'Lib\\A', real: 2, files: 2 }),
        makeAlbum({ index: 1, name: 'Lib\\B', real: 1, files: 1 }),
      ])
      expect(store.rows).toHaveLength(3)
    })
  })

  // @RF-10 @RNF-04 S3 (fail)
  describe('S3: fail()', () => {
    it('flushes the pending batch and keeps the albums and the rows', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply({ kind: 'scanned', total: 5 })
      store.apply(done(0, { index: 0, name: 'Album A', real: 1, files: 5 }))
      flush()
      store.apply(done(1, { index: 0, name: 'Album A', real: 2, files: 5 }))
      store.fail('The analysis stopped')
      expect(store.albums).toHaveLength(1)
      expect(store.albums[0].summary.total).toBe(2)
      expect(store.rows.map((r) => r.index)).toEqual([0, 1])
      expect(store.phase).toBe('error')
    })
  })

  // @RF-10 S4 (stale update)
  describe('S4: stale update', () => {
    it('keeps the stored update when a smaller total arrives in the same batch', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply(done(0, { index: 0, real: 3, files: 5 }))
      store.apply(done(1, { index: 0, real: 2, files: 5 }))
      flush()
      expect(store.albums).toHaveLength(1)
      expect(store.albums[0].summary.total).toBe(3)
    })

    it('keeps the stored update when a smaller total arrives after the flush', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply(done(0, { index: 0, real: 3, files: 5 }))
      flush()
      store.apply(done(1, { index: 0, real: 2, files: 5 }))
      flush()
      expect(store.albums[0].summary.total).toBe(3)
    })

    it('also keeps it when the total is equal', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply(done(0, { index: 0, name: 'First', real: 2, files: 5 }))
      store.apply(done(1, { index: 0, name: 'Second', real: 2, files: 5 }))
      flush()
      expect(store.albums[0].name).toBe('First')
    })
  })

  // @RF-10 S5 (selectAlbum)
  describe('S5: selectAlbum()', () => {
    it('selects the row with the lowest index of the album, pending rows included', () => {
      const store = createAnalysisStore()
      store.begin()
      store.apply(done(7, { index: 1, name: 'Lib\\B', real: 1, files: 3 }))
      store.apply(done(2, { index: 0, name: 'Lib\\A', real: 1, files: 1 }))
      store.apply(done(9, { index: 1, name: 'Lib\\B', real: 2, files: 3 }))
      flush()
      store.apply(done(4, { index: 1, name: 'Lib\\B', real: 3, files: 3 })) // still pending
      store.selectAlbum(1)
      expect(store.selectedIndex).toBe(4)
    })

    it('does nothing for an album without rows', () => {
      const store = twoAlbumsFlushed()
      store.select(1)
      store.selectAlbum(5)
      expect(store.selectedIndex).toBe(1)
    })
  })
})

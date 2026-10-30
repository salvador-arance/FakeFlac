// Store rules of the cache figures (specs/ui/result-cache.md section 4, "Store"; @RF-11): `cache`
// starts null; the finished and cancelled events set it from their summary; begin() clears it
// and restore() brings it back; reset() and fail() leave it as it is.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { makeSummary, makeTrack, makeAlbum } from '../testing/fixtures'
import { createAnalysisStore } from './analysis.svelte'

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

function finishedStore(fromCache = 31, cache: 'ok' | 'degraded' | 'unavailable' | 'off' = 'ok') {
  const store = createAnalysisStore()
  store.begin()
  store.apply({ kind: 'scanned', total: 1 })
  store.apply({ kind: 'trackDone', index: 0, analysis: makeTrack(), album: makeAlbum() })
  store.apply({
    kind: 'finished',
    summary: makeSummary({ total: 1, done: 1, real: 1, fromCache, cache }),
  })
  return store
}

describe('analysis store: cache figures (@RF-11)', () => {
  it('starts with no figures', () => {
    expect(createAnalysisStore().cache).toBeNull()
  })

  it('a finished event sets them from its summary', () => {
    expect(finishedStore(31, 'ok').cache).toEqual({ fromCache: 31, status: 'ok' })
    expect(finishedStore(10, 'degraded').cache).toEqual({ fromCache: 10, status: 'degraded' })
  })

  it('a cancelled event sets them from its summary', () => {
    const store = createAnalysisStore()
    store.begin()
    store.apply({ kind: 'scanned', total: 31 })
    store.apply({
      kind: 'cancelled',
      summary: makeSummary({ total: 31, done: 18, cancelled: true, fromCache: 12, cache: 'ok' }),
    })
    expect(store.cache).toEqual({ fromCache: 12, status: 'ok' })
  })

  it('no figures while a run is in progress', () => {
    const store = finishedStore()
    store.begin()
    expect(store.cache).toBeNull()
    store.apply({ kind: 'scanned', total: 2 })
    store.apply({ kind: 'trackDone', index: 0, analysis: makeTrack(), album: makeAlbum() })
    vi.advanceTimersByTime(100)
    expect(store.cache).toBeNull()
  })

  it('begin() clears them and restore() brings them back', () => {
    const store = finishedStore(31, 'unavailable')
    store.begin()
    expect(store.cache).toBeNull()
    store.restore()
    expect(store.cache).toEqual({ fromCache: 31, status: 'unavailable' })
  })

  it('reset() and fail() leave them as they are', () => {
    const store = finishedStore(5, 'ok')
    store.reset()
    expect(store.cache).toEqual({ fromCache: 5, status: 'ok' })
    store.fail('The analysis stopped because of an unexpected error')
    expect(store.cache).toEqual({ fromCache: 5, status: 'ok' })
  })

  it('the next run replaces them', () => {
    const store = finishedStore(31, 'ok')
    store.begin()
    store.apply({ kind: 'scanned', total: 1 })
    store.apply({ kind: 'finished', summary: makeSummary({ fromCache: 0, cache: 'ok' }) })
    expect(store.cache).toEqual({ fromCache: 0, status: 'ok' })
  })
})

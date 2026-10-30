// StatusBar cache figures (specs/ui/result-cache.md sections 3 and 4 `StatusBar` row; @RF-11
// @RNF-08): `<state text>[ · {n} from cache][ · <cache warning>][ · {shown} shown][ · <notice>]`,
// in the phases finished and cancelled only. `cache` is optional; without it nothing is appended.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import type { AnalysisPhase, CacheFigures, CacheStatus, StatusSummary } from '../../model'
import StatusBar from './StatusBar.svelte'

const text = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()
const FINISHED: StatusSummary = { total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 }
const CANCELLED: StatusSummary = { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 }
const NONE: StatusSummary = { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 }
const STATE = '31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors'
const figures = (fromCache: number, status: CacheStatus): CacheFigures => ({ fromCache, status })

// @RF-11 Scenario Outline: Cache figures in the status bar
describe('StatusBar: cache figures (@RF-11)', () => {
  it.each([
    ['finished', 31, 'ok', `${STATE} · 31 from cache`],
    ['finished', 0, 'ok', STATE],
    ['finished', 0, 'unavailable', `${STATE} · Results cache unavailable`],
    [
      'finished',
      10,
      'degraded',
      `${STATE} · 10 from cache · Results cache error: some results were not saved`,
    ],
    ['finished', 0, 'off', STATE],
    ['cancelled', 12, 'ok', 'Cancelled · 18 of 31 · 12 from cache'],
  ] as [AnalysisPhase, number, CacheStatus, string][])(
    'ended %s with fromCache %i and cache %s',
    (phase, n, status, expected) => {
      render(StatusBar, {
        props: { phase, summary: phase === 'finished' ? FINISHED : CANCELLED, cache: figures(n, status) },
      })
      expect(text()).toBe(expected)
    },
  )

  it('cancelled with degraded: the count, then the warning', () => {
    render(StatusBar, { props: { phase: 'cancelled', summary: CANCELLED, cache: figures(4, 'degraded') } })
    expect(text()).toBe('Cancelled · 18 of 31 · 4 from cache · Results cache error: some results were not saved')
  })

  // @RF-11 Scenario: Cache figures come before the shown count and the notice
  it('the cache figures come before the shown count and the notice', () => {
    render(StatusBar, {
      props: {
        phase: 'finished',
        summary: FINISHED,
        cache: figures(25, 'ok'),
        shown: 3,
        notice: 'Saved 31 rows to a.csv',
      },
    })
    expect(text()).toBe(`${STATE} · 25 from cache · 3 shown · Saved 31 rows to a.csv`)
  })

  it('the warning also comes before the shown count and the notice', () => {
    render(StatusBar, {
      props: {
        phase: 'finished',
        summary: FINISHED,
        cache: figures(10, 'degraded'),
        shown: 3,
        notice: 'Saved 31 rows to FakeFlac - Album A.csv',
      },
    })
    expect(text()).toBe(
      `${STATE} · 10 from cache · Results cache error: some results were not saved · 3 shown · Saved 31 rows to FakeFlac - Album A.csv`,
    )
  })

  it('after clearing, the count still describes the last run', () => {
    render(StatusBar, {
      props: { phase: 'finished', summary: FINISHED, cache: figures(31, 'ok'), notice: 'Results cache cleared' },
    })
    expect(text()).toBe(`${STATE} · 31 from cache · Results cache cleared`)
  })

  // Section 3: neither part in ready, scanning, analysing or error
  it.each([
    ['ready', NONE, undefined, 'Ready'],
    ['scanning', NONE, undefined, 'Scanning…'],
    ['analysing', CANCELLED, undefined, '18/31 · 2 suspicious · 1 fake'],
    [
      'error',
      NONE,
      'The analysis stopped because of an unexpected error',
      'The analysis stopped because of an unexpected error',
    ],
  ] as [AnalysisPhase, StatusSummary, string | undefined, string][])(
    '%s with cache set: not appended',
    (phase, summary, message, expected) => {
      render(StatusBar, { props: { phase, summary, message, cache: figures(31, 'degraded') } })
      expect(text()).toBe(expected)
    },
  )

  it('without cache: unchanged', () => {
    render(StatusBar, { props: { phase: 'finished', summary: FINISHED } })
    expect(text()).toBe(STATE)
  })

  // @RF-11 @RNF-08 Scenario: The button and the warning are named in text: status bar text in
  // the role "status" region, never colour alone
  it('the warning is text inside the role="status" region', () => {
    render(StatusBar, { props: { phase: 'finished', summary: FINISHED, cache: figures(0, 'unavailable') } })
    const region = screen.getByRole('status')
    expect(region.textContent).toContain('Results cache unavailable')
  })
})

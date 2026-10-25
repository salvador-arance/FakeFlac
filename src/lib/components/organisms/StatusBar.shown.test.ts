// StatusBar `shown` (specs/ui/sort-filter.md sections 9 and 10 `StatusBar` row; @RF-08): " · {n}
// shown" after the state text and before the notice, only in the phases analysing, finished and
// cancelled; never in ready, scanning or error.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import type { AnalysisPhase, StatusSummary } from '../../model'
import StatusBar from './StatusBar.svelte'

const text = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()
const FINISHED: StatusSummary = { total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 }
const RUNNING: StatusSummary = { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 }
const NONE: StatusSummary = { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 }

describe('StatusBar: shown count (@RF-08)', () => {
  it('finished with shown', () => {
    render(StatusBar, { props: { phase: 'finished', summary: FINISHED, shown: 3 } })
    expect(text()).toBe('31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 3 shown')
  })

  it('analysing with shown', () => {
    render(StatusBar, { props: { phase: 'analysing', summary: RUNNING, shown: 1 } })
    expect(text()).toBe('18/31 · 2 suspicious · 1 fake · 1 shown')
  })

  it('cancelled with shown', () => {
    render(StatusBar, {
      props: { phase: 'cancelled', summary: { ...NONE, total: 10, done: 3, real: 2, fake: 1 }, shown: 1 },
    })
    expect(text()).toBe('Cancelled · 3 of 10 · 1 shown')
  })

  it('finished with 0 shown', () => {
    render(StatusBar, { props: { phase: 'finished', summary: FINISHED, shown: 0 } })
    expect(text()).toBe('31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 0 shown')
  })

  it('error with shown: not appended', () => {
    render(StatusBar, {
      props: {
        phase: 'error',
        summary: NONE,
        message: 'The analysis stopped because of an unexpected error',
        shown: 1,
      },
    })
    expect(text()).toBe('The analysis stopped because of an unexpected error')
  })

  it.each([
    ['ready', 'Ready'],
    ['scanning', 'Scanning…'],
  ] as [AnalysisPhase, string][])('%s with shown: not appended', (phase, expected) => {
    render(StatusBar, { props: { phase, summary: NONE, shown: 2 } })
    expect(text()).toBe(expected)
  })

  it('with shown and notice: the count goes before the notice', () => {
    render(StatusBar, {
      props: { phase: 'finished', summary: FINISHED, shown: 1, notice: 'Saved 31 rows to results.csv' },
    })
    expect(text()).toBe(
      '31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 1 shown · Saved 31 rows to results.csv',
    )
  })

  it('without shown: unchanged', () => {
    render(StatusBar, { props: { phase: 'finished', summary: FINISHED } })
    expect(text()).toBe('31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors')
  })
})

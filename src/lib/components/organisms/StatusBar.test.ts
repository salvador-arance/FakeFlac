// StatusBar (specs/ui/components.md, Organisms; main-screen.md "Screen states"): one state per phase.
// Readings chosen: the root has role="status"; aria-valuenow is a percentage (see ProgressBar).
// The analysing counters read "<done>/<total> · <n> suspicious · <n> fake" (layout in main-screen.md).
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import type { StatusSummary } from '../../model'
import StatusBar from './StatusBar.svelte'

const summary = (o: Partial<StatusSummary> = {}): StatusSummary => ({
  total: 0,
  done: 0,
  real: 0,
  suspicious: 0,
  fake: 0,
  errors: 0,
  ...o,
})
const statusText = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()

describe('StatusBar', () => {
  // main-screen.md: Empty (start) -> "Ready"
  it('ready: shows "Ready" and no progress bar', () => {
    render(StatusBar, { props: { phase: 'ready', summary: summary() } })
    expect(statusText()).toBe('Ready')
    expect(screen.queryByRole('progressbar')).toBeNull()
  })

  // main-screen.md: Scanning -> indeterminate progress, "Scanning…"
  it('scanning: shows "Scanning…" with an indeterminate progress bar', () => {
    render(StatusBar, { props: { phase: 'scanning', summary: summary() } })
    expect(statusText()).toBe('Scanning…')
    const bar = screen.getByRole('progressbar')
    expect(bar.hasAttribute('aria-valuenow')).toBe(false)
  })

  // @RF-05 main-screen.md: Analysing -> determinate done/total + counters
  it('analysing: shows a determinate bar and the counters', () => {
    render(StatusBar, {
      props: {
        phase: 'analysing',
        summary: summary({ total: 31, done: 18, real: 15, suspicious: 2, fake: 1 }),
      },
    })
    expect(statusText()).toContain('18/31 · 2 suspicious · 1 fake')
    const now = Number(screen.getByRole('progressbar').getAttribute('aria-valuenow'))
    expect(now).toBeCloseTo((18 / 31) * 100, 0)
  })

  // @RF-05 Progress is reported per track and globally: "the progress bar shows 4 of 4"
  it('analysing: shows 4 of 4 as a full bar', () => {
    render(StatusBar, { props: { phase: 'analysing', summary: summary({ total: 4, done: 4, real: 4 }) } })
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('100')
    expect(statusText()).toContain('4/4')
  })

  it('analysing: half way is 50', () => {
    render(StatusBar, { props: { phase: 'analysing', summary: summary({ total: 8, done: 4 }) } })
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('50')
  })

  // main-screen.md: Finished -> "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors"
  it('finished: shows the final summary', () => {
    render(StatusBar, {
      props: {
        phase: 'finished',
        summary: summary({ total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 }),
      },
    })
    expect(statusText()).toBe('31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors')
    expect(screen.queryByRole('progressbar')).toBeNull()
  })

  // @RF-05 main-screen.md: Cancelled -> "Cancelled · 18 of 31"
  it('cancelled: shows how far the run got', () => {
    render(StatusBar, { props: { phase: 'cancelled', summary: summary({ total: 31, done: 18 }) } })
    expect(statusText()).toBe('Cancelled · 18 of 31')
    expect(screen.queryByRole('progressbar')).toBeNull()
  })

  // main-screen.md: Error -> message in the status bar
  it('error: shows the message', () => {
    render(StatusBar, {
      props: { phase: 'error', summary: summary(), message: 'No supported audio files in this folder' },
    })
    expect(statusText()).toBe('No supported audio files in this folder')
    expect(screen.queryByRole('progressbar')).toBeNull()
  })
})

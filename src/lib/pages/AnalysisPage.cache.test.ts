// AnalysisPage: the result cache (RF-11; specs/ui/result-cache.md; result-cache.feature,
// "Interface"). The API client is mocked: the page is the only component that talks to it.
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - The status bar text is compared after collapsing whitespace; "ends with" is checked on the
//   text after the last " · ".
// - "I drop two files and start_analysis rejects with NOT_A_FOLDER for both" is, at the page,
//   `startAnalysisOfDropped` resolving with `null` (the client skips every NOT_A_FOLDER answer).
// - "the table, the selection, the filters and the Albums panel are unchanged" compares the text
//   of the data rows, the selected row, the pressed verdict buttons and the search text, and the
//   text of the Albums panel before and after clearing.
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AnalysisSummary, ProgressEvent } from '../api'
import * as api from '../api'
import type { Verdict } from '../model'
import { makeAlbum, makeSummary, makeTrack } from '../testing/fixtures'
import AnalysisPage from './AnalysisPage.svelte'

vi.mock('../api', () => {
  class ApiError extends Error {
    code: string
    constructor(error: { code: string; message: string }) {
      super(error.message)
      this.name = 'ApiError'
      this.code = error.code
    }
  }
  return {
    ApiError,
    pickFolder: vi.fn(),
    startAnalysis: vi.fn(),
    startAnalysisOfDropped: vi.fn(),
    cancelAnalysis: vi.fn(),
    exportCsv: vi.fn(),
    clearCache: vi.fn(),
    getSpectrogram: vi.fn(),
    onDragDrop: vi.fn(),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
  }
})

let dragHandler: (event: api.DragDropEvent) => void = () => {}

function fakeRun() {
  let onEvent: (event: ProgressEvent) => void = () => {}
  let resolve: (summary: AnalysisSummary | null) => void = () => {}
  let reject: (error: unknown) => void = () => {}
  const promise = new Promise<AnalysisSummary | null>((res, rej) => {
    resolve = res
    reject = rej
  })
  promise.catch(() => {})
  vi.mocked(api.startAnalysis).mockImplementation((_root, cb) => {
    onEvent = cb
    return promise as Promise<AnalysisSummary>
  })
  return {
    scanned: (total: number) => onEvent({ kind: 'scanned', total }),
    done: (index: number, verdict: Verdict = 'real') =>
      onEvent({
        kind: 'trackDone',
        index,
        analysis: makeTrack({
          path: `C:/Music/Album A/t${String(index).padStart(2, '0')}.flac`,
          verdict,
          cutoffHz: verdict === 'fake' ? 16000 : 20000,
        }),
        album: makeAlbum({ index: 0, name: 'Album A', files: 31, real: index + 1 }),
      }),
    finish: (summary: AnalysisSummary) => {
      onEvent({ kind: 'finished', summary })
      resolve(summary)
    },
    cancelled: (summary: AnalysisSummary) => {
      onEvent({ kind: 'cancelled', summary })
      resolve(summary)
    },
    fail: (code: string) => reject(new api.ApiError({ code, message: 'backend message' } as never)),
  }
}

const button = (name: string) => screen.getByRole('button', { name }) as HTMLButtonElement
const clearButton = () => button('Clear results cache')
const openButton = () => button('Open folder')
const exportButton = () => button('Export CSV')
const statusText = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()
const lastPart = () => statusText().split(' · ').pop()
const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
const STATE31 = '31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors'

async function mountPage() {
  render(AnalysisPage)
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled())
}

async function openFolder(path = 'C:/Music/Album A') {
  vi.mocked(api.pickFolder).mockResolvedValue(path)
  vi.mocked(api.startAnalysis).mockClear()
  await userEvent.click(openButton())
  await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled())
}

/** Verdicts of the 31 tracks: 28 real, 2 suspicious, 1 fake. */
const VERDICTS: Verdict[] = [...Array(28).fill('real'), 'suspicious', 'suspicious', 'fake']

/** A finished run of 31 tracks with the given cache figures (the page already mounted). */
async function finishRun(fromCache: number, cache: AnalysisSummary['cache']) {
  const run = fakeRun()
  await openFolder()
  run.scanned(31)
  VERDICTS.forEach((v, i) => run.done(i, v))
  run.finish(makeSummary({ total: 31, done: 31, real: 28, suspicious: 2, fake: 1, fromCache, cache }))
  await vi.waitFor(() => expect(statusText()).toMatch(/^31 tracks/))
  return run
}

async function finishedWith(fromCache: number, cache: AnalysisSummary['cache'] = 'ok') {
  await mountPage()
  return finishRun(fromCache, cache)
}

/** Controllable `clearCache`. */
function pendingClear() {
  let resolve: () => void = () => {}
  let reject: (e: unknown) => void = () => {}
  vi.mocked(api.clearCache).mockImplementation(
    () =>
      new Promise<void>((res, rej) => {
        resolve = res
        reject = rej
      }),
  )
  return {
    resolve: () => resolve(),
    reject: (code: string) => reject(new api.ApiError({ code, message: 'backend message' } as never)),
  }
}

beforeEach(() => {
  vi.mocked(api.pickFolder).mockReset()
  vi.mocked(api.startAnalysis).mockReset()
  vi.mocked(api.startAnalysisOfDropped).mockReset()
  vi.mocked(api.cancelAnalysis).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.exportCsv).mockReset()
  vi.mocked(api.clearCache).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.getSpectrogram).mockReset().mockReturnValue(new Promise(() => {}))
  vi.mocked(api.minimizeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.toggleMaximizeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.closeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.onDragDrop)
    .mockReset()
    .mockImplementation(async (handler) => {
      dragHandler = handler
      return () => {}
    })
})

afterEach(() => {
  vi.useRealTimers()
})

// @RF-11 Scenario Outline: Cache figures in the status bar (through the page and the store)
describe('Cache figures in the status bar (@RF-11)', () => {
  it('finished with 31 from cache', async () => {
    await finishedWith(31, 'ok')
    await vi.waitFor(() => expect(statusText()).toBe(`${STATE31} · 31 from cache`))
  })

  it('finished with cache unavailable', async () => {
    await finishedWith(0, 'unavailable')
    await vi.waitFor(() => expect(statusText()).toBe(`${STATE31} · Results cache unavailable`))
  })

  it('finished with cache degraded', async () => {
    await finishedWith(10, 'degraded')
    await vi.waitFor(() =>
      expect(statusText()).toBe(
        `${STATE31} · 10 from cache · Results cache error: some results were not saved`,
      ),
    )
  })

  it('cancelled with 12 from cache', async () => {
    await mountPage()
    const run = fakeRun()
    await openFolder()
    run.scanned(31)
    for (let i = 0; i < 18; i++) run.done(i)
    run.cancelled(makeSummary({ total: 31, done: 18, real: 18, cancelled: true, fromCache: 12, cache: 'ok' }))
    await vi.waitFor(() => expect(statusText()).toBe('Cancelled · 18 of 31 · 12 from cache'))
  })

  // @RF-11 Scenario: Cache figures come before the shown count and the notice
  it('come before the shown count and the notice', async () => {
    await finishedWith(25, 'ok')
    await userEvent.click(button('Show fake tracks'))
    await userEvent.click(button('Show suspicious tracks'))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(3))
    vi.mocked(api.exportCsv).mockResolvedValue({ path: 'C:\\Out\\a.csv', rows: 31 })
    await userEvent.click(exportButton())
    await vi.waitFor(() =>
      expect(statusText()).toBe(`${STATE31} · 25 from cache · 3 shown · Saved 31 rows to a.csv`),
    )
  })

  // @RF-11 Scenario: No cache figures while analysing or after an unexpected failure
  it('shows none while analysing, nor after INTERNAL', async () => {
    await finishedWith(31, 'ok')
    const run = fakeRun()
    await openFolder('C:/Music/Album B')
    run.scanned(31)
    run.done(0)
    await vi.waitFor(() => expect(statusText()).toMatch(/^\d+\/31/))
    expect(statusText()).not.toContain('from cache')
    run.fail('INTERNAL')
    await vi.waitFor(() =>
      expect(statusText()).toBe('The analysis stopped because of an unexpected error'),
    )
  })

  // @RF-11 Scenario: A drop that starts nothing keeps the previous figures
  it('a drop that starts nothing keeps the previous figures', async () => {
    await finishedWith(31, 'ok')
    await vi.waitFor(() => expect(statusText()).toMatch(/ · 31 from cache$/))
    vi.mocked(api.startAnalysisOfDropped).mockResolvedValue(null)
    dragHandler({ type: 'drop', paths: ['C:/a.flac', 'C:/b.flac'] })
    await vi.waitFor(() => expect(api.startAnalysisOfDropped).toHaveBeenCalled())
    await new Promise((r) => setTimeout(r, 30))
    expect(statusText()).toBe(`${STATE31} · 31 from cache`)
  })

  // @RF-11 Scenario: A new run replaces the figures
  it('a new run replaces the figures', async () => {
    await finishedWith(31, 'ok')
    await vi.waitFor(() => expect(statusText()).toMatch(/ · 31 from cache$/))
    const run = fakeRun()
    await openFolder('C:/Music/Album B')
    await vi.waitFor(() => expect(statusText()).toBe('Scanning…'))
    run.scanned(31)
    VERDICTS.forEach((v, i) => run.done(i, v))
    run.finish(makeSummary({ total: 31, done: 31, real: 28, suspicious: 2, fake: 1, fromCache: 0, cache: 'ok' }))
    await vi.waitFor(() => expect(statusText()).toBe(STATE31))
    expect(statusText()).not.toContain('from cache')
  })
})

// @RF-11 Scenario Outline: When "Clear results cache" is enabled
describe('When "Clear results cache" is enabled (@RF-11)', () => {
  it('Empty: enabled', async () => {
    await mountPage()
    expect(clearButton().disabled).toBe(false)
  })

  it('Scanning: disabled', async () => {
    await mountPage()
    fakeRun()
    await openFolder()
    await vi.waitFor(() => expect(statusText()).toBe('Scanning…'))
    expect(clearButton().disabled).toBe(true)
  })

  it('Analysing: disabled', async () => {
    await mountPage()
    const run = fakeRun()
    await openFolder()
    run.scanned(31)
    run.done(0)
    await vi.waitFor(() => expect(statusText()).toMatch(/^\d+\/31/))
    expect(clearButton().disabled).toBe(true)
  })

  it('Finished: enabled', async () => {
    await finishedWith(31)
    expect(clearButton().disabled).toBe(false)
  })

  it('Cancelled: enabled', async () => {
    await mountPage()
    const run = fakeRun()
    await openFolder()
    run.scanned(31)
    run.done(0)
    run.cancelled(makeSummary({ total: 31, done: 1, real: 1, cancelled: true, fromCache: 0, cache: 'ok' }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^Cancelled/))
    expect(clearButton().disabled).toBe(false)
  })

  it('Error: enabled', async () => {
    await mountPage()
    const run = fakeRun()
    await openFolder()
    run.scanned(31)
    run.done(0)
    run.fail('INTERNAL')
    await vi.waitFor(() => expect(statusText()).toMatch(/unexpected error/))
    expect(clearButton().disabled).toBe(false)
  })

  it('Finished, while exporting: enabled', async () => {
    await finishedWith(31)
    vi.mocked(api.exportCsv).mockReturnValue(new Promise(() => {}))
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(exportButton().disabled).toBe(true))
    expect(clearButton().disabled).toBe(false)
  })
})

// @RF-11 Scenario: While clearing, nothing else can start
describe('While clearing, nothing else can start (@RF-11)', () => {
  it('disables Clear and Open folder, ignores a drop, and enables both when clearCache resolves', async () => {
    await finishedWith(31)
    const pending = pendingClear()
    await userEvent.click(clearButton())
    await vi.waitFor(() => expect(api.clearCache).toHaveBeenCalledTimes(1))
    await vi.waitFor(() => expect(clearButton().disabled).toBe(true))
    expect(openButton().disabled).toBe(true)

    vi.mocked(api.startAnalysisOfDropped).mockResolvedValue(null)
    dragHandler({ type: 'drop', paths: ['C:/Music/Album B'] })
    await new Promise((r) => setTimeout(r, 30))
    expect(api.startAnalysisOfDropped).not.toHaveBeenCalled()

    pending.resolve()
    await vi.waitFor(() => expect(clearButton().disabled).toBe(false))
    expect(openButton().disabled).toBe(false)
  })
})

/** What clearing must leave unchanged (section 2.2). */
function screenState() {
  const panel = screen.queryByRole('listbox', { name: 'Album summaries' })
  return {
    rows: dataRows().map((r) => r.textContent),
    selected: dataRows()
      .filter((r) => r.getAttribute('aria-selected') === 'true')
      .map((r) => r.textContent),
    pressed: ['Show fake tracks', 'Show suspicious tracks', 'Show genuine tracks', 'Show tracks with errors'].map(
      (name) => button(name).getAttribute('aria-pressed'),
    ),
    query: (screen.getByRole('searchbox', { name: 'Search by name' }) as HTMLInputElement).value,
    albums: panel?.textContent ?? null,
  }
}

// @RF-11 Scenario Outline: The outcome of clearing is a notice
describe('The outcome of clearing is a notice (@RF-11)', () => {
  it.each([
    ['resolves', null, 'Results cache cleared'],
    ['rejects with CACHE_FAILED', 'CACHE_FAILED', 'Cannot clear the results cache'],
    ['rejects with INTERNAL', 'INTERNAL', 'Cannot clear the results cache'],
    ['rejects with ANALYSIS_RUNNING', 'ANALYSIS_RUNNING', 'An analysis is already running'],
  ] as [string, string | null, string][])('clearCache %s: "%s"', async (_outcome, code, text) => {
    await finishedWith(31)
    await userEvent.click(dataRows()[1])
    await vi.waitFor(() =>
      expect(dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')).toHaveLength(1),
    )
    const before = screenState()
    const pending = pendingClear()
    await userEvent.click(clearButton())
    if (code === null) pending.resolve()
    else pending.reject(code)
    await vi.waitFor(() => expect(statusText()).toBe(`${STATE31} · 31 from cache · ${text}`))
    expect(lastPart()).toBe(text)
    expect(statusText()).not.toContain('backend message')
    expect(screenState()).toEqual(before)
  })

  it('a non-ApiError rejection shows "Cannot clear the results cache"', async () => {
    await finishedWith(31)
    vi.mocked(api.clearCache).mockRejectedValue(new Error('boom'))
    await userEvent.click(clearButton())
    await vi.waitFor(() => expect(lastPart()).toBe('Cannot clear the results cache'))
  })

  it('the notice replaces an export notice', async () => {
    await finishedWith(31)
    vi.mocked(api.exportCsv).mockResolvedValue({ path: 'C:\\Out\\a.csv', rows: 31 })
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(lastPart()).toBe('Saved 31 rows to a.csv'))
    await userEvent.click(clearButton())
    await vi.waitFor(() => expect(statusText()).toBe(`${STATE31} · 31 from cache · Results cache cleared`))
  })

  it('an attempt to start an analysis clears the notice', async () => {
    await finishedWith(31)
    await userEvent.click(clearButton())
    await vi.waitFor(() => expect(lastPart()).toBe('Results cache cleared'))
    fakeRun()
    await openFolder('C:/Music/Album B')
    await vi.waitFor(() => expect(statusText()).toBe('Scanning…'))
  })
})

// @RF-11 Scenario: Clearing from the empty screen
describe('Clearing from the empty screen (@RF-11)', () => {
  it('shows "Ready · Results cache cleared"', async () => {
    await mountPage()
    await userEvent.click(clearButton())
    await vi.waitFor(() => expect(statusText()).toBe('Ready · Results cache cleared'))
    expect(api.clearCache).toHaveBeenCalledTimes(1)
  })
})

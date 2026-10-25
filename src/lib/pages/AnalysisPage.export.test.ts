// AnalysisPage: Export CSV (RF-09; specs/ui/export-csv.md; export-csv.feature, "Interface").
// The API client is mocked: the page is the only component that talks to it.
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - The button is found by its accessible name "Export CSV" (`toolbar.export`).
// - The status bar text is compared after collapsing whitespace; the notice is the text after
//   the last " · " ("· Saved 31 rows to FakeFlac - Album A.csv").
// - A drop of files only resolves `startAnalysisOfDropped` with `null` (the client skips every
//   NOT_A_FOLDER answer): nothing may change on the screen, notice included.
// - NOT_A_FOLDER from Open folder: the spec says the state becomes Error "with that error's
//   text", but main-screen.md gives NOT_A_FOLDER no text. Only what is unambiguous is asserted
//   (the old notice is gone, the rows are kept, Export CSV stays enabled).
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AnalysisSummary, ProgressEvent } from '../api'
import * as api from '../api'
import { makeSummary, makeTrack } from '../testing/fixtures'
import type { TrackOptions } from '../testing/fixtures'
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
    done: (index: number, o: TrackOptions) =>
      onEvent({ kind: 'trackDone', index, analysis: makeTrack(o) }),
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

const exportButton = () => screen.getByRole('button', { name: 'Export CSV' }) as HTMLButtonElement
const openButton = () => screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement
const statusText = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()
const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)

async function mountPage() {
  render(AnalysisPage)
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled())
}

async function openFolder(path = 'C:/Music/Album A') {
  vi.mocked(api.pickFolder).mockResolvedValue(path)
  await userEvent.click(openButton())
  await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled())
}

/** A finished analysis with `count` tracks on screen. */
async function finishedWith(count: number) {
  const run = fakeRun()
  await mountPage()
  await openFolder()
  run.scanned(count)
  for (let i = 0; i < count; i++) run.done(i, { path: `C:/Music/Album A/t${i}.flac` })
  run.finish(makeSummary({ total: count, done: count, real: count }))
  await vi.waitFor(() => expect(dataRows()).toHaveLength(count))
  await vi.waitFor(() => expect(statusText()).toMatch(/^\d+ tracks/))
  return run
}

/** Controllable `exportCsv`. */
function pendingExport() {
  let resolve: (v: api.ExportResult | null) => void = () => {}
  let reject: (e: unknown) => void = () => {}
  vi.mocked(api.exportCsv).mockImplementation(
    () =>
      new Promise((res, rej) => {
        resolve = res
        reject = rej
      }),
  )
  return {
    resolve: (v: api.ExportResult | null) => resolve(v),
    reject: (code: string) => reject(new api.ApiError({ code, message: 'backend message' } as never)),
  }
}

beforeEach(() => {
  vi.mocked(api.pickFolder).mockReset()
  vi.mocked(api.startAnalysis).mockReset()
  vi.mocked(api.startAnalysisOfDropped).mockReset()
  vi.mocked(api.cancelAnalysis).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.exportCsv).mockReset()
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

// @RF-09 Scenario Outline: When Export CSV is enabled
describe('When Export CSV is enabled', () => {
  it('Empty, 0 rows: disabled', async () => {
    await mountPage()
    expect(exportButton().disabled).toBe(true)
  })

  it('Scanning, 0 rows: disabled', async () => {
    fakeRun()
    await mountPage()
    await openFolder()
    expect(exportButton().disabled).toBe(true)
  })

  it('Analysing, 5 rows: disabled', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(10)
    for (let i = 0; i < 5; i++) run.done(i, { path: `C:/Music/Album A/t${i}.flac` })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(5))
    expect(exportButton().disabled).toBe(true)
  })

  it('Finished, rows: enabled', async () => {
    await finishedWith(4)
    expect(exportButton().disabled).toBe(false)
  })

  it('Cancelled with rows: enabled', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(31)
    for (let i = 0; i < 3; i++) run.done(i, { path: `C:/Music/Big/t${i}.flac` })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(3))
    run.cancelled(makeSummary({ total: 31, done: 3, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^Cancelled/))
    expect(exportButton().disabled).toBe(false)
  })

  it('Cancelled with no row: disabled', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(31)
    run.cancelled(makeSummary({ total: 31, done: 0, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^Cancelled/))
    expect(exportButton().disabled).toBe(true)
  })

  it('Error with 2 rows (INTERNAL after reporting tracks): enabled', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(5)
    run.done(0, { path: 'C:/Music/Album A/a.flac' })
    run.done(1, { path: 'C:/Music/Album A/b.flac' })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(2))
    run.fail('INTERNAL')
    await vi.waitFor(() => expect(statusText()).toMatch(/unexpected error/))
    expect(exportButton().disabled).toBe(false)
  })

  it('Error with no row: disabled', async () => {
    fakeRun().fail('NO_AUDIO_FOUND')
    await mountPage()
    await openFolder()
    await vi.waitFor(() => expect(statusText()).toBe('No supported audio files in this folder'))
    expect(exportButton().disabled).toBe(true)
  })
})

describe('After the export', () => {
  // @RF-09 Scenario: A successful export is confirmed in the status bar
  it('confirms a successful export in the status bar and changes nothing else', async () => {
    await finishedWith(3)
    const before = statusText()
    const rowsBefore = dataRows().map((r) => r.textContent)
    vi.mocked(api.exportCsv).mockResolvedValue({
      path: 'C:\\Out\\FakeFlac - Album A.csv',
      rows: 31,
    })
    await userEvent.click(exportButton())
    await vi.waitFor(() =>
      expect(statusText()).toBe(`${before} · Saved 31 rows to FakeFlac - Album A.csv`),
    )
    expect(api.exportCsv).toHaveBeenCalledTimes(1)
    expect(dataRows().map((r) => r.textContent)).toEqual(rowsBefore)
  })

  // @RF-09 Scenario: A single row uses the singular
  it('uses the singular for one row', async () => {
    await finishedWith(1)
    vi.mocked(api.exportCsv).mockResolvedValue({ path: 'C:\\Out\\one.csv', rows: 1 })
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(statusText()).toMatch(/ · Saved 1 row to one\.csv$/))
  })

  // @RF-09 Scenario: Export CSV is disabled while the export is in progress
  it('disables Export CSV while the export is pending and enables it again after', async () => {
    await finishedWith(3)
    const pending = pendingExport()
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(exportButton().disabled).toBe(true))
    expect(openButton().disabled).toBe(false)
    pending.resolve({ path: 'C:\\Out\\a.csv', rows: 3 })
    await vi.waitFor(() => expect(exportButton().disabled).toBe(false))
  })

  // @RF-09 Scenario: Cancelling the dialog changes nothing
  it('keeps the previous notice when the dialog is cancelled', async () => {
    await finishedWith(3)
    vi.mocked(api.exportCsv).mockResolvedValueOnce({ path: 'C:\\Out\\a.csv', rows: 3 })
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(statusText()).toMatch(/ · Saved 3 rows to a\.csv$/))
    const before = statusText()
    vi.mocked(api.exportCsv).mockResolvedValueOnce(null)
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(api.exportCsv).toHaveBeenCalledTimes(2))
    await vi.waitFor(() => expect(exportButton().disabled).toBe(false))
    expect(statusText()).toBe(before)
  })

  // @RF-09 Scenario Outline: A failed export is reported in the status bar
  it.each([
    ['EXPORT_FAILED', 'Cannot save the CSV file'],
    ['INTERNAL', 'Cannot save the CSV file'],
    ['NOT_ANALYZED', 'There are no results to export'],
    ['ANALYSIS_RUNNING', 'An analysis is already running'],
  ])('%s shows "%s" and leaves the table', async (code, text) => {
    await finishedWith(3)
    const before = statusText()
    const rowsBefore = dataRows().map((r) => r.textContent)
    const pending = pendingExport()
    await userEvent.click(exportButton())
    pending.reject(code)
    await vi.waitFor(() => expect(statusText()).toBe(`${before} · ${text}`))
    expect(statusText()).not.toContain('backend message')
    expect(dataRows().map((r) => r.textContent)).toEqual(rowsBefore)
  })
})

describe('The notice is cleared by an attempt to start an analysis', () => {
  async function withNotice() {
    await finishedWith(3)
    vi.mocked(api.exportCsv).mockResolvedValue({ path: 'C:\\Out\\a.csv', rows: 3 })
    await userEvent.click(exportButton())
    await vi.waitFor(() => expect(statusText()).toMatch(/ · Saved 3 rows to a\.csv$/))
  }

  // @RF-09 Scenario: The notice is cleared when a new analysis starts
  it('a start that succeeds shows "Scanning…" without the notice', async () => {
    await withNotice()
    fakeRun()
    await openFolder('C:/Music/Album B')
    await vi.waitFor(() => expect(statusText()).toBe('Scanning…'))
  })

  // @RF-09 Scenario Outline: A start that fails before Scanning replaces the notice with its error
  it.each([
    ['NO_AUDIO_FOUND', 'No supported audio files in this folder'],
    ['FOLDER_UNREADABLE', 'Cannot open this folder'],
  ])('%s replaces the notice with its error text and keeps the rows', async (code, text) => {
    await withNotice()
    vi.mocked(api.startAnalysis).mockRejectedValue(
      new api.ApiError({ code, message: 'backend message' } as never),
    )
    vi.mocked(api.pickFolder).mockResolvedValue('C:/Docs')
    await userEvent.click(openButton())
    await vi.waitFor(() => expect(statusText()).toBe(text))
    expect(statusText()).not.toContain('Saved 3 rows')
    expect(dataRows()).toHaveLength(3)
    expect(exportButton().disabled).toBe(false)
  })

  it('NOT_A_FOLDER from Open folder drops the notice and keeps the rows', async () => {
    await withNotice()
    vi.mocked(api.startAnalysis).mockRejectedValue(
      new api.ApiError({ code: 'NOT_A_FOLDER', message: 'backend message' } as never),
    )
    vi.mocked(api.pickFolder).mockResolvedValue('C:/Docs/file.flac')
    await userEvent.click(openButton())
    await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalledTimes(2))
    await vi.waitFor(() => expect(openButton().disabled).toBe(false))
    expect(statusText()).not.toContain('Saved 3 rows')
    expect(dataRows()).toHaveLength(3)
    expect(exportButton().disabled).toBe(false)
  })

  // @RF-09 Scenario: A drop of files only keeps the notice
  it('a drop of files only keeps the notice', async () => {
    await withNotice()
    const before = statusText()
    vi.mocked(api.startAnalysisOfDropped).mockResolvedValue(null)
    dragHandler({ type: 'drop', paths: ['C:/a.flac', 'C:/b.flac'] })
    await vi.waitFor(() => expect(api.startAnalysisOfDropped).toHaveBeenCalled())
    await new Promise((r) => setTimeout(r, 30))
    expect(statusText()).toBe(before)
    expect(dataRows()).toHaveLength(3)
    expect(exportButton().disabled).toBe(false)
  })
})

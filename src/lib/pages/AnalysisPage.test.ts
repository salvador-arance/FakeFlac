// AnalysisPage (specs/ui/main-screen.md "Screen states"; analyze-folder.feature, UI side).
// The API client is mocked: the page is the only component that talks to it.
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - Rows are batched: events inside one 100 ms window become visible together, never more than
//   one visible update per 100 ms. Whether the first row shows at once or after 100 ms is open.
// - A drop while a run is in progress is ignored (docs/agents/progress.md).
// - The error text shown comes from i18n by `AppError.code`, never from the backend message.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AnalysisSummary, ProgressEvent, TrackAnalysis } from '../api'
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
    onDragDrop: vi.fn(),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
  }
})

const HINT = 'Drop a folder here or click Open folder'

let dragHandler: (event: api.DragDropEvent) => void = () => {}
const unlisten = vi.fn()

/** A controllable analysis run behind `startAnalysis` / `startAnalysisOfDropped`. */
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
  vi.mocked(api.startAnalysisOfDropped).mockImplementation((_paths, cb) => {
    onEvent = cb
    return promise
  })
  return {
    emit: (event: ProgressEvent) => onEvent(event),
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
    fail: (code: string, message = 'backend message') =>
      reject(new api.ApiError({ code, message } as never)),
  }
}

async function mountPage() {
  const view = render(AnalysisPage)
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled())
  return view
}

const openButton = () => screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement
const cancelButton = () => screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement
const statusText = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()
const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')
const fileColumn = () => dataRows().map((r) => cells(r)[1].textContent?.trim())

/** Click "Open folder" with the dialog answering `path`, then wait for the run to be requested. */
async function openFolder(path: string | null = 'C:/Music/Album A') {
  vi.mocked(api.pickFolder).mockResolvedValue(path)
  await userEvent.click(openButton())
  if (path !== null) await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled())
}

const flush = () => vi.advanceTimersByTimeAsync(100)

beforeEach(() => {
  vi.mocked(api.pickFolder).mockReset()
  vi.mocked(api.startAnalysis).mockReset()
  vi.mocked(api.startAnalysisOfDropped).mockReset()
  vi.mocked(api.cancelAnalysis).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.minimizeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.toggleMaximizeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.closeWindow).mockReset().mockResolvedValue(undefined)
  unlisten.mockReset()
  vi.mocked(api.onDragDrop)
    .mockReset()
    .mockImplementation(async (handler) => {
      dragHandler = handler
      return unlisten
    })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('screen state: Empty (start)', () => {
  // main-screen.md: table hidden, hint shown, Open enabled, Cancel disabled, "Ready"
  it('shows the hint, "Ready", Open enabled and Cancel disabled', async () => {
    await mountPage()
    expect(screen.getByText(HINT)).not.toBeNull()
    expect(screen.queryByRole('grid')).toBeNull()
    expect(statusText()).toBe('Ready')
    expect(openButton().disabled).toBe(false)
    expect(cancelButton().disabled).toBe(true)
  })

  it('shows the application title', async () => {
    await mountPage()
    expect(screen.getByText('FakeFlac')).not.toBeNull()
  })
})

describe('screen state: Dragging over', () => {
  // @RF-01 main-screen.md: overlay highlights the whole window
  it('highlights on drag enter and clears on leave', async () => {
    await mountPage()
    dragHandler({ type: 'enter' })
    expect((await screen.findByTestId('folder-drop-zone')).getAttribute('data-active')).toBe('true')
    dragHandler({ type: 'leave' })
    await vi.waitFor(() =>
      expect(screen.getByTestId('folder-drop-zone').getAttribute('data-active')).toBe('false'),
    )
  })
})

describe('RF-01: choose a folder', () => {
  // @RF-01 Choose a folder with the dialog
  it('starts an analysis of the picked folder', async () => {
    fakeRun()
    await mountPage()
    await openFolder('C:/Music/Album A')
    expect(api.startAnalysis).toHaveBeenCalledWith('C:/Music/Album A', expect.any(Function))
  })

  // @RF-01 Cancel the folder dialog
  it('does nothing when the dialog is cancelled', async () => {
    fakeRun()
    await mountPage()
    await openFolder(null)
    await vi.waitFor(() => expect(api.pickFolder).toHaveBeenCalled())
    expect(api.startAnalysis).not.toHaveBeenCalled()
    expect(statusText()).toBe('Ready')
    expect(openButton().disabled).toBe(false)
    expect(screen.getByText(HINT)).not.toBeNull()
  })

  // @RF-01 Drop a folder on the window
  it('starts an analysis of a dropped folder', async () => {
    fakeRun()
    await mountPage()
    dragHandler({ type: 'drop', paths: ['C:/Music/Album A'] })
    await vi.waitFor(() => expect(api.startAnalysisOfDropped).toHaveBeenCalled())
    expect(api.startAnalysisOfDropped).toHaveBeenCalledWith(
      ['C:/Music/Album A'],
      expect.any(Function),
    )
  })

  // @RF-01 Drop files and a folder together: the client picks the folder, the page passes all paths
  it('passes every dropped path to the client', async () => {
    fakeRun()
    await mountPage()
    dragHandler({ type: 'drop', paths: ['C:/a.flac', 'C:/Music/Album A'] })
    await vi.waitFor(() => expect(api.startAnalysisOfDropped).toHaveBeenCalled())
    expect(vi.mocked(api.startAnalysisOfDropped).mock.calls[0][0]).toEqual([
      'C:/a.flac',
      'C:/Music/Album A',
    ])
  })

  it('removes the highlight when something is dropped', async () => {
    fakeRun()
    await mountPage()
    dragHandler({ type: 'enter' })
    dragHandler({ type: 'drop', paths: ['C:/Music/Album A'] })
    await vi.waitFor(() => expect(api.startAnalysisOfDropped).toHaveBeenCalled())
    const zone = screen.queryByTestId('folder-drop-zone')
    expect(zone === null || zone.getAttribute('data-active') === 'false').toBe(true)
  })

  // @RF-01 Dropped files are ignored: no analysis and no error
  it('ignores dropped files without an error', async () => {
    vi.mocked(api.startAnalysisOfDropped).mockResolvedValue(null)
    await mountPage()
    dragHandler({ type: 'drop', paths: ['C:/cover.jpg'] })
    await vi.waitFor(() => expect(api.startAnalysisOfDropped).toHaveBeenCalled())
    await vi.waitFor(() => expect(statusText()).toBe('Ready'))
    expect(openButton().disabled).toBe(false)
    expect(cancelButton().disabled).toBe(true)
    expect(screen.getByText(HINT)).not.toBeNull()
    expect(screen.queryByText(/No WAV or FLAC|Cannot open|already running|unexpected/)).toBeNull()
  })

  // docs/agents/progress.md: a drop during a run would otherwise show ANALYSIS_RUNNING
  it('ignores a drop while an analysis is running', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(3)
    await vi.waitFor(() => expect(cancelButton().disabled).toBe(false))
    dragHandler({ type: 'drop', paths: ['C:/Music/Album B'] })
    await new Promise((r) => setTimeout(r, 20))
    expect(api.startAnalysisOfDropped).not.toHaveBeenCalled()
    expect(screen.queryByText(/already running/)).toBeNull()
  })
})

describe('screen state: Scanning', () => {
  // main-screen.md: empty table, Open disabled, Cancel enabled, indeterminate progress, "Scanning…"
  it('shows scanning until the first event arrives', async () => {
    fakeRun()
    await mountPage()
    await openFolder()
    await vi.waitFor(() => expect(statusText()).toBe('Scanning…'))
    expect(screen.getByRole('progressbar').hasAttribute('aria-valuenow')).toBe(false)
    expect(openButton().disabled).toBe(true)
    expect(cancelButton().disabled).toBe(false)
    expect(screen.queryByText(HINT)).toBeNull()
    expect(dataRows()).toHaveLength(0)
  })
})

describe('screen state: Analysing', () => {
  // @RF-05 Progress is reported per track and globally
  it('shows a determinate progress bar and the counters', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(4)
    run.done(0, { path: 'C:/A/a.flac', verdict: 'real' })
    run.done(1, { path: 'C:/A/b.flac', verdict: 'fake', cutoffHz: 16000 })
    await vi.waitFor(() => expect(statusText()).toMatch(/2\/4/))
    expect(statusText()).toMatch(/1 fake/)
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('50')
    expect(openButton().disabled).toBe(true)
    expect(cancelButton().disabled).toBe(false)
  })

  // @RF-05 "the progress bar shows 4 of 4"
  it('shows 4 of 4 when the last track is done', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(4)
    for (let i = 0; i < 4; i++) run.done(i, { path: `C:/A/${i}.flac` })
    await vi.waitFor(() => expect(statusText()).toMatch(/4\/4/))
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('100')
  })

  // @RF-06 Results appear in the table as they finish
  it('shows the rows with state, format, rate, bits and cutoff', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(2)
    run.done(0, { path: 'C:/Album A/full_band.flac', verdict: 'real', cutoffHz: null })
    run.done(1, { path: 'C:/Album A/lp16k.flac', verdict: 'fake', cutoffHz: 16000 })
    await screen.findByText('lp16k.flac')
    expect(fileColumn()).toEqual(['full_band.flac', 'lp16k.flac'])
    const [real, fake] = dataRows()
    expect(cells(real)[0].textContent).toContain('Genuine')
    expect(cells(real)[3].textContent?.trim()).toBe('FLAC')
    expect(cells(real)[4].textContent).toContain('44100')
    expect(cells(real)[5].textContent).toContain('16')
    expect(cells(fake)[0].textContent).toContain('Fake')
    expect(cells(fake)[7].textContent?.trim()).toBe('16.0 kHz')
    // @RNF-08 each state is shown with an icon, a text and a colour
    expect(real.querySelector('svg')).not.toBeNull()
    expect(fake.querySelector('svg')).not.toBeNull()
    // the table replaces the hint and nothing is selected
    expect(screen.queryByText(HINT)).toBeNull()
    expect(dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')).toHaveLength(0)
  })

  // trackDone events may arrive out of index order; rows are keyed by index, not path
  it('keeps rows with the same path apart, whatever the arrival order', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(3)
    run.done(2, { path: 'C:/A/c.flac' })
    run.done(0, { path: 'C:/A/dup.flac' })
    run.done(1, { path: 'C:/A/dup.flac' })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(3))
    expect(fileColumn()).toEqual(['c.flac', 'dup.flac', 'dup.flac'])
  })

  // @RNF-04 A corrupt file does not stop the analysis
  it('shows a corrupt file as an error row and keeps the others', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(3)
    run.done(0, { path: 'C:/Mixed/corrupt.flac', verdict: 'error', error: { kind: 'corruptStream', detail: 'x' } })
    run.done(1, { path: 'C:/Mixed/full_band.flac', verdict: 'real' })
    run.done(2, { path: 'C:/Mixed/lp16k.flac', verdict: 'fake', cutoffHz: 16000 })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(3))
    const corrupt = dataRows()[0]
    expect(cells(corrupt)[1].textContent?.trim()).toBe('corrupt.flac')
    expect(cells(corrupt)[0].textContent).toContain('Error')
    const hasReason = corrupt.matches('[title*="Corrupt stream"]') || corrupt.querySelector('[title*="Corrupt stream"]')
    expect(hasReason).toBeTruthy()
    expect(cells(dataRows()[1])[0].textContent).toContain('Genuine')
    expect(cells(dataRows()[2])[0].textContent).toContain('Fake')
  })

  // @RNF-09 Unicode names
  it('shows Unicode file names exactly', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(1)
    run.done(0, { path: 'C:/Música/Álbum ″Ñandú″/01 – Canción ‘x’.flac' })
    await screen.findByText('01 – Canción ‘x’.flac')
  })

  // Interaction: single selection, the first row is not selected automatically
  it('selects a row when it is clicked', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(2)
    run.done(0, { path: 'C:/A/a.flac' })
    run.done(1, { path: 'C:/A/b.flac' })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(2))
    await userEvent.click(cells(dataRows()[1])[1])
    await vi.waitFor(() =>
      expect(dataRows().map((r) => r.getAttribute('aria-selected'))).toEqual(['false', 'true']),
    )
  })

  // Interaction: clicking a header sorts by it (asc -> desc)
  it('sorts by a column: ascending, then descending', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(3)
    run.done(0, { path: 'C:/A/a.flac', durationMs: 120000 })
    run.done(1, { path: 'C:/A/b.flac', durationMs: 60000 })
    run.done(2, { path: 'C:/A/c.flac', durationMs: 180000 })
    await vi.waitFor(() => expect(dataRows()).toHaveLength(3))
    expect(fileColumn()).toEqual(['a.flac', 'b.flac', 'c.flac']) // default: by path
    await userEvent.click(screen.getByRole('button', { name: /Duration/ }))
    await vi.waitFor(() => expect(fileColumn()).toEqual(['b.flac', 'a.flac', 'c.flac']))
    expect(screen.getByRole('columnheader', { name: /Duration/ }).getAttribute('aria-sort')).toBe('ascending')
    await userEvent.click(screen.getByRole('button', { name: /Duration/ }))
    await vi.waitFor(() => expect(fileColumn()).toEqual(['c.flac', 'a.flac', 'b.flac']))
    expect(screen.getByRole('columnheader', { name: /Duration/ }).getAttribute('aria-sort')).toBe('descending')
  })

  // @RF-05 Only one analysis at a time: a second run cannot be started from the toolbar
  it('does not open the dialog while a run is in progress', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(2)
    await vi.waitFor(() => expect(openButton().disabled).toBe(true))
    await userEvent.click(openButton())
    expect(api.pickFolder).toHaveBeenCalledTimes(1)
  })

  // A new run replaces the rows of the previous one (rows are keyed by index within a run)
  it('replaces the rows of the previous run', async () => {
    const run1 = fakeRun()
    await mountPage()
    await openFolder('C:/Music/Album A')
    run1.scanned(1)
    run1.done(0, { path: 'C:/Music/Album A/old.flac' })
    run1.finish(makeSummary({ total: 1, done: 1, real: 1 }))
    await screen.findByText('old.flac')
    await vi.waitFor(() => expect(openButton().disabled).toBe(false))

    const run2 = fakeRun()
    vi.mocked(api.startAnalysis).mockClear()
    await openFolder('C:/Music/Album B')
    run2.scanned(1)
    run2.done(0, { path: 'C:/Music/Album B/new.flac' })
    await screen.findByText('new.flac')
    expect(screen.queryByText('old.flac')).toBeNull()
  })
})

describe('results are batched (at most one update per 100 ms)', () => {
  async function startWithFakeTimers() {
    const run = fakeRun()
    await mountPage()
    vi.useFakeTimers()
    vi.mocked(api.pickFolder).mockResolvedValue('C:/Music/Album A')
    await fireEvent.click(openButton())
    await vi.advanceTimersByTimeAsync(0)
    run.scanned(50)
    await vi.advanceTimersByTimeAsync(0)
    return run
  }

  // main-screen.md, Analysing: "batched <= 1 update / 100 ms"
  it('shows events that arrived together once the batch interval has passed', async () => {
    const run = await startWithFakeTimers()
    run.done(0, { path: 'C:/A/a.flac' })
    run.done(1, { path: 'C:/A/b.flac' })
    run.done(2, { path: 'C:/A/c.flac' })
    await flush()
    expect(fileColumn()).toEqual(['a.flac', 'b.flac', 'c.flac'])
  })

  it('makes at most one visible update in a 100 ms window', async () => {
    const run = await startWithFakeTimers()
    const seen = new Set<number>()
    seen.add(dataRows().length)
    for (let i = 0; i < 30; i++) {
      run.done(i, { path: `C:/A/${String(i).padStart(2, '0')}.flac` })
      await vi.advanceTimersByTimeAsync(1)
      seen.add(dataRows().length)
    }
    // 30 ms elapsed: the row count may change once (0 -> something), not once per event
    expect(seen.size).toBeLessThanOrEqual(2)
    await flush()
    expect(dataRows()).toHaveLength(30)
  })
})

describe('screen state: Finished', () => {
  // main-screen.md: "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors"
  it('shows the summary and re-enables Open', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(31)
    run.done(0, { path: 'C:/A/a.flac' })
    run.finish(makeSummary({ total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 }))
    await vi.waitFor(() =>
      expect(statusText()).toBe('31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors'),
    )
    expect(openButton().disabled).toBe(false)
    expect(cancelButton().disabled).toBe(true)
    await screen.findByText('a.flac')
  })

  // @RF-05 the last event is "finished" with done 4 (a short album)
  it('lists all the rows of a finished run', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(4)
    for (let i = 0; i < 4; i++) run.done(i, { path: `C:/A/t${i}.flac` })
    run.finish(makeSummary({ total: 4, done: 4, real: 4 }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^4 tracks/))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(4))
  })
})

describe('screen state: Cancelled', () => {
  // @RF-05 Cancel a running analysis: "Cancelled · 18 of 31", tracks already analysed stay
  it('asks the backend to cancel and shows how far the run got', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(31)
    run.done(0, { path: 'C:/Big/a.flac' })
    await vi.waitFor(() => expect(cancelButton().disabled).toBe(false))
    await userEvent.click(cancelButton())
    expect(api.cancelAnalysis).toHaveBeenCalledTimes(1)
    run.cancelled(makeSummary({ total: 31, done: 18, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toBe('Cancelled · 18 of 31'))
    expect(openButton().disabled).toBe(false)
    expect(cancelButton().disabled).toBe(true)
    await screen.findByText('a.flac')
  })
})

describe('screen state: Error', () => {
  // main-screen.md: texts come from i18n by code
  it.each([
    ['NO_AUDIO_FOUND', 'No WAV or FLAC files in this folder'],
    ['FOLDER_UNREADABLE', 'Cannot open this folder'],
    ['ANALYSIS_RUNNING', 'An analysis is already running'],
  ])('%s before the run starts shows its text', async (code, text) => {
    fakeRun().fail(code)
    await mountPage()
    await openFolder()
    await vi.waitFor(() => expect(statusText()).toBe(text))
    expect(openButton().disabled).toBe(false)
    expect(cancelButton().disabled).toBe(true)
    expect(dataRows()).toHaveLength(0)
  })

  // @RF-02 Folder without supported audio: "the table stays empty"
  it('NO_AUDIO_FOUND from a dropped folder shows the text in the status bar', async () => {
    fakeRun().fail('NO_AUDIO_FOUND')
    await mountPage()
    dragHandler({ type: 'drop', paths: ['C:/Docs'] })
    await vi.waitFor(() => expect(statusText()).toBe('No WAV or FLAC files in this folder'))
    expect(dataRows()).toHaveLength(0)
    expect(openButton().disabled).toBe(false)
  })

  // @RNF-04 An unexpected failure ends the run: the reported tracks stay, Open is enabled again
  it('INTERNAL keeps the rows already reported and shows the error text', async () => {
    vi.useFakeTimers()
    const run = fakeRun()
    await mountPage_fake()
    run.scanned(5)
    run.done(0, { path: 'C:/Album A/a.flac' })
    run.done(1, { path: 'C:/Album A/b.flac' })
    await flush()
    expect(dataRows()).toHaveLength(2)
    run.fail('INTERNAL', 'panic: index out of bounds')
    await vi.advanceTimersByTimeAsync(150)
    expect(statusText()).toMatch(/^The analysis stopped because of an unexpected error/)
    expect(statusText()).not.toContain('panic')
    expect(fileColumn()).toEqual(['a.flac', 'b.flac'])
    expect(openButton().disabled).toBe(false)
    expect(cancelButton().disabled).toBe(true)
  })

  // NOT_A_FOLDER has no text: ignored silently
  it('NOT_A_FOLDER shows no error', async () => {
    fakeRun().fail('NOT_A_FOLDER')
    await mountPage()
    await openFolder()
    await vi.waitFor(() => expect(openButton().disabled).toBe(false))
    await new Promise((r) => setTimeout(r, 20))
    expect(statusText()).toBe('Ready')
  })
})

/** Mount with fake timers already installed, then start a run through the dialog. */
async function mountPage_fake() {
  render(AnalysisPage)
  await vi.advanceTimersByTimeAsync(0)
  vi.mocked(api.pickFolder).mockResolvedValue('C:/Music/Album A')
  await fireEvent.click(openButton())
  await vi.advanceTimersByTimeAsync(0)
}

describe('window controls', () => {
  // main-screen.md Interaction: the title bar buttons go through the API client
  it('minimises, toggles maximise and closes through the client', async () => {
    await mountPage()
    await userEvent.click(screen.getByRole('button', { name: 'Minimize' }))
    expect(api.minimizeWindow).toHaveBeenCalledTimes(1)
    await userEvent.click(screen.getByRole('button', { name: 'Maximize' }))
    expect(api.toggleMaximizeWindow).toHaveBeenCalledTimes(1)
    await userEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(api.closeWindow).toHaveBeenCalledTimes(1)
  })

  it('maximises on double click of the title', async () => {
    await mountPage()
    await userEvent.dblClick(screen.getByText('FakeFlac'))
    expect(api.toggleMaximizeWindow).toHaveBeenCalledTimes(1)
  })
})

describe('unused type import guard', () => {
  it('keeps TrackAnalysis imported for typing helpers', () => {
    const t: TrackAnalysis = makeTrack()
    expect(t.verdict).toBe('real')
  })
})

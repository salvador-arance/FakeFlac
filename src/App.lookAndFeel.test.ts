// The main screen as a whole, on the real page (@RNF-10; specs/ui/look-and-feel.md section 1, the
// paragraph "The page as a whole", elements L1, L3, L4 and L5). Scenarios S1 "A finished run shows
// the regions the requirement names" (@RF-06 @RF-07 @RF-10 @RNF-04 @RNF-09) and S2 "The empty state
// shows the window frame without the content regions" of specs/features/look-and-feel.feature.
// MainLayout.test.ts and MainLayout.sidebar.test.ts check the same order with stub content; here
// the regions are the real organisms, after a synthetic run (no audio). The API client is mocked as
// in App.test.ts and AnalysisPage.spectrogram.test.ts.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - jsdom has no layout (ADR-0037), so "top to bottom" and "left/right" are read from the DOM:
//   "top to bottom" is document order; "on the left ... and on its right the table, the splitter
//   and the detail panel" means the Albums panel comes first inside the content area (`main`), and
//   the table, the horizontal splitter and the detail panel share a container inside `main` that
//   does not hold the Albums panel (they form the column to its right).
// - The title bar is the `banner` landmark (TitleBar's `header`); the toolbar is the `toolbar`
//   named "Main toolbar"; the content area is the `main` landmark; the status bar is the `status`.
// - The detail panel is the region right after the table's horizontal splitter
//   (main-screen.md "Layout": below the table, splitter). "Shows the spectrogram of the selected
//   row": `getSpectrogram` was asked for the Fake row's path and its image is drawn in that region,
//   next to the Fake row's summary text.
// - The Error row's name is compared after trimming the cell's surrounding whitespace only.
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App.svelte'
import type { AnalysisSummary, ProgressEvent } from './lib/api'
import * as api from './lib/api'
import type { Spectrogram } from './lib/model'
import { mockSettings, waitForMainScreen } from './lib/pages/tests/settingsHarness'
import { makeAlbum, makeSummary, makeTrack } from './lib/testing/fixtures'
import type { TrackOptions } from './lib/testing/fixtures'
import { installCanvasStub, makeSpectrogram } from './lib/testing/spectrogram'
import type { CanvasStub } from './lib/testing/spectrogram'

vi.mock('./lib/api', () => {
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
    getSpectrogram: vi.fn(),
    onDragDrop: vi.fn(() => Promise.resolve(() => {})),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    getSettings: vi.fn(),
    setSettings: vi.fn(),
  }
})

const FOLDER = 'C:/Music/Album A'
const GENUINE = `${FOLDER}/genuine.flac`
const FAKE = `${FOLDER}/fake16k.flac`
const UNICODE_NAME = 'Ñandú – 夜.flac'
const CORRUPT = `${FOLDER}/${UNICODE_NAME}`
const HINT = 'Drop a folder here or click Open folder'

let stub: CanvasStub

beforeEach(() => {
  // RF-13: English default settings, locale reset (lib/pages/tests/settingsHarness.ts)
  mockSettings()
  vi.mocked(api.getSpectrogram).mockReset()
  vi.mocked(api.cancelAnalysis).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.onDragDrop)
    .mockReset()
    .mockImplementation(async () => () => {})
  stub = installCanvasStub()
})

afterEach(() => {
  stub.restore()
})

const follows = (a: Node, b: Node) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING)
const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
const fileName = (row: HTMLElement) => within(row).getAllByRole('gridcell')[1].textContent?.trim()
const rowOf = (name: string) => dataRows().find((r) => fileName(r) === name)!
const horizontalSplitter = () =>
  screen.queryAllByRole('separator').find((el) => el.getAttribute('aria-orientation') === 'horizontal')
const spectrogramImage = () => screen.queryByRole('img', { name: /spectrogram/i })

/** The smallest element that contains every node given. */
function commonAncestor(...nodes: Node[]): Node {
  let candidate: Node | null = nodes[0]
  while (candidate && !nodes.every((n) => candidate!.contains(n))) candidate = candidate.parentNode
  return candidate!
}

/** Starts the app, opens a folder and finishes a synthetic run with `tracks` in index order. */
async function finishedRun(tracks: TrackOptions[]) {
  let onEvent: (event: ProgressEvent) => void = () => {}
  let resolveRun: (s: AnalysisSummary) => void = () => {}
  vi.mocked(api.pickFolder).mockResolvedValue(FOLDER)
  vi.mocked(api.startAnalysis).mockImplementation((_root, cb) => {
    onEvent = cb
    return new Promise<AnalysisSummary>((res) => {
      resolveRun = res
    })
  })
  render(App)
  await waitForMainScreen()
  await userEvent.click(screen.getByRole('button', { name: 'Open folder' }))
  await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled())
  onEvent({ kind: 'scanned', total: tracks.length })
  const album = makeAlbum({ real: 1, fake: 1, errors: 1 })
  tracks.forEach((o, index) => onEvent({ kind: 'trackDone', index, analysis: makeTrack(o), album }))
  const summary = makeSummary({ total: 3, done: 3, real: 1, fake: 1, errors: 1 })
  onEvent({ kind: 'finished', summary })
  resolveRun(summary)
  await vi.waitFor(() => expect(dataRows()).toHaveLength(tracks.length))
}

/** S1 Given: a Genuine, a Fake and a corrupt Error row, then the Fake row selected and drawn. */
async function finishedRunWithFakeSelected() {
  let answer: (s: Spectrogram) => void = () => {}
  vi.mocked(api.getSpectrogram).mockImplementation(
    () => new Promise<Spectrogram>((resolve) => (answer = resolve)),
  )
  await finishedRun([
    { path: GENUINE, verdict: 'real', cutoffHz: null },
    { path: FAKE, verdict: 'fake', cutoffHz: 16000 },
    { path: CORRUPT, verdict: 'error', cutoffHz: null, error: { kind: 'corruptStream', detail: 'synthetic' } },
  ])
  await userEvent.click(within(rowOf('fake16k.flac')).getAllByRole('gridcell')[1])
  await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(FAKE))
  answer(makeSpectrogram({ width: 8, height: 4 }))
  await vi.waitFor(() => expect(spectrogramImage()).not.toBeNull())
}

/** The four bands of the window, by role. */
function windowFrame() {
  return {
    titleBar: screen.getByRole('banner'),
    toolbar: screen.getByRole('toolbar', { name: 'Main toolbar' }),
    content: screen.getByRole('main'),
    statusBar: screen.getByRole('status'),
  }
}

describe('@RNF-10 @RF-06 @RF-07 @RF-10 @RNF-04 @RNF-09 Scenario: A finished run shows the regions the requirement names', () => {
  it('shows from top to bottom the title bar with its three buttons, the toolbar, the content area and the status bar', async () => {
    await finishedRunWithFakeSelected()
    const { titleBar, toolbar, content, statusBar } = windowFrame()
    expect(within(titleBar).getByText('FakeFlac')).not.toBeNull()
    for (const name of ['Minimize', 'Maximize', 'Close']) {
      expect(within(titleBar).getByRole('button', { name })).not.toBeNull()
    }
    expect(follows(titleBar, toolbar)).toBe(true)
    expect(follows(toolbar, content)).toBe(true)
    expect(follows(content, statusBar)).toBe(true)
    // the bands are separate: none holds another
    for (const [a, b] of [
      [titleBar, toolbar],
      [toolbar, content],
      [content, statusBar],
      [titleBar, content],
    ]) {
      expect(a.contains(b)).toBe(false)
      expect(b.contains(a)).toBe(false)
    }
  })

  it('shows the Albums panel on the left, and on its right the table, the splitter and the detail panel, top to bottom', async () => {
    await finishedRunWithFakeSelected()
    const content = screen.getByRole('main')
    const albums = screen.getByRole('listbox', { name: 'Album summaries' })
    const table = screen.getByRole('grid')
    const splitter = horizontalSplitter()
    expect(splitter).toBeDefined()
    const detail = splitter!.nextElementSibling as HTMLElement
    expect(detail).not.toBeNull()

    for (const region of [albums, table, splitter!, detail]) expect(content.contains(region)).toBe(true)
    // the Albums panel comes first, then the column at its right
    expect(follows(albums, table)).toBe(true)
    // the column: table, splitter and detail panel together, top to bottom, without the Albums panel
    expect(follows(table, splitter!)).toBe(true)
    expect(follows(splitter!, detail)).toBe(true)
    const column = commonAncestor(table, splitter!, detail)
    expect(column).not.toBe(content)
    expect(content.contains(column)).toBe(true)
    expect(column.contains(albums)).toBe(false)
  })

  it('shows the three rows in the table, the Error row with the name "Ñandú – 夜.flac" exactly', async () => {
    await finishedRunWithFakeSelected()
    const names = dataRows().map(fileName)
    expect(names).toHaveLength(3)
    expect(names).toEqual(expect.arrayContaining(['genuine.flac', 'fake16k.flac', UNICODE_NAME]))
    const errorRow = rowOf(UNICODE_NAME)
    expect(errorRow).toBeDefined()
    expect(fileName(errorRow)).toBe(UNICODE_NAME)
    expect(errorRow.textContent).toContain('Error')
  })

  it('shows the spectrogram of the selected row in the detail panel', async () => {
    await finishedRunWithFakeSelected()
    expect(rowOf('fake16k.flac').getAttribute('aria-selected')).toBe('true')
    expect(vi.mocked(api.getSpectrogram).mock.calls.at(-1)).toEqual([FAKE])
    const detail = horizontalSplitter()!.nextElementSibling as HTMLElement
    const image = within(detail).getByRole('img', { name: /spectrogram/i }) as HTMLCanvasElement
    expect(image.width).toBe(8)
    expect((detail.textContent ?? '').replace(/\s+/g, ' ')).toContain('Cutoff 16.0 kHz · Fake')
    expect(within(detail).getByTestId('cutoff-line').getAttribute('data-cutoff-hz')).toBe('16000')
  })
})

describe('@RNF-10 Scenario: The empty state shows the window frame without the content regions', () => {
  it('shows the title bar "FakeFlac", the toolbar "Main toolbar", the drop hint and the status bar "Ready"', async () => {
    render(App)
    await waitForMainScreen()
    const { titleBar, toolbar, content, statusBar } = windowFrame()
    expect(within(titleBar).getByText('FakeFlac')).not.toBeNull()
    expect(toolbar).not.toBeNull()
    expect(within(content).getByText(HINT)).not.toBeNull()
    expect((statusBar.textContent ?? '').replace(/\s+/g, ' ').trim()).toBe('Ready')
    expect(follows(titleBar, toolbar)).toBe(true)
    expect(follows(toolbar, content)).toBe(true)
    expect(follows(content, statusBar)).toBe(true)
  })

  it('shows no Albums panel, no results table and no detail panel', async () => {
    render(App)
    await waitForMainScreen()
    expect(screen.queryByRole('listbox', { name: 'Album summaries' })).toBeNull()
    expect(screen.queryByRole('grid')).toBeNull()
    // no detail panel: no splitter below a table, no spectrogram, no detail text
    expect(horizontalSplitter()).toBeUndefined()
    expect(spectrogramImage()).toBeNull()
    expect(screen.queryByTestId('spectrogram-loading')).toBeNull()
    expect(api.getSpectrogram).not.toHaveBeenCalled()
  })
})

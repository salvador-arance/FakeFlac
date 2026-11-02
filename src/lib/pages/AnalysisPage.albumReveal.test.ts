// AnalysisPage: selecting the current album again brings its first row back into view
// (@RF-10; features/album-summary-app.feature, scenario "Selecting the current album again brings
// its first track back into view"; specs/ui/album-summary.md acceptance A5, "Interaction" and
// "Components" AnalysisPage row, decided at G1 Q16).
//
// Same fakes and locators as AnalysisPage.albums.test.ts: the API client is mocked, the progress
// events are stubbed, the panel is the listbox "Album summaries" with role "option" items.
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - jsdom has no layout, so the page's table would not virtualise (its height is unknown). The
//   test gives the table a measurable viewport: the role="grid" element reports a `clientHeight`
//   of 220 px (10 rows of the default 22 px row height), every other element 0 as in jsdom.
// - "The selected row is within the table's visible area": with display position p, the grid's
//   `scrollTop <= p * 22` and `scrollTop + 220 >= (p + 1) * 22` (the sticky header measures 0 in
//   jsdom), and the selected row is rendered (aria-selected="true").
// - "I scrolled the table until the selected row is outside its visible area": `scrollTop` set
//   on the grid plus a `scroll` event (the user's scroll).
// - The table is sorted by File ascending (the default), so "Lib\A" (indexes 0..9) is on top and
//   its first row is at display position 0; "Lib\B" holds the other 490 tracks.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import type { MockInstance } from 'vitest'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AnalysisSummary, ProgressEvent } from '../api'
import * as api from '../api'
import type { Verdict } from '../model'
import { makeAlbum, makeSummary, makeTrack } from '../testing/fixtures'
import { mockSettings, waitForMainScreen } from './tests/settingsHarness'
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
    getSpectrogram: vi.fn(),
    exportCsv: vi.fn(),
    onDragDrop: vi.fn(),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    getSettings: vi.fn(),
    setSettings: vi.fn(),
  }
})

const ROW = 22
const VIEWPORT = 220
const unlisten = vi.fn()

function fakeRun() {
  let onEvent: (event: ProgressEvent) => void = () => {}
  let resolve: (summary: AnalysisSummary | null) => void = () => {}
  const promise = new Promise<AnalysisSummary | null>((res) => {
    resolve = res
  })
  vi.mocked(api.startAnalysis).mockImplementation((_root, cb) => {
    onEvent = cb
    return promise as Promise<AnalysisSummary>
  })
  return {
    emit: (event: ProgressEvent) => onEvent(event),
    scanned: (total: number) => onEvent({ kind: 'scanned', total }),
    finish: (summary: AnalysisSummary) => {
      onEvent({ kind: 'finished', summary })
      resolve(summary)
    },
  }
}
type Run = ReturnType<typeof fakeRun>

type AlbumSpec = { index: number; name: string; files: number }
const folderOf = (a: AlbumSpec) => `C:/Music/${a.name.replaceAll('\\', '/')}`
const pathOf = (a: AlbumSpec, index: number) => `${folderOf(a)}/${String(index).padStart(3, '0')}.flac`

/** One trackDone per verdict, indexes `first`, `first + 1`, ..., with the cumulative counts. */
function feed(run: Run, spec: AlbumSpec, first: number, verdicts: Verdict[]) {
  const counted: Verdict[] = []
  verdicts.forEach((verdict, i) => {
    counted.push(verdict)
    const count = (v: Verdict) => counted.filter((x) => x === v).length
    run.emit({
      kind: 'trackDone',
      index: first + i,
      analysis: makeTrack({ path: pathOf(spec, first + i), verdict }),
      album: makeAlbum({
        index: spec.index,
        name: spec.name,
        folder: folderOf(spec),
        files: spec.files,
        real: count('real'),
        suspicious: count('suspicious'),
        fake: count('fake'),
        errors: count('error'),
      }),
    })
  })
  return first + verdicts.length
}

const norm = (s: string | null) => (s ?? '').replace(/\s+/g, ' ').trim()
const statusText = () => norm(screen.getByRole('status').textContent)
const openButton = () => screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement
const grid = () => screen.getByRole('grid')
const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
const fileOf = (row: HTMLElement) => within(row).getAllByRole('gridcell')[1].textContent?.trim()
const selectedRows = () => dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')
const items = () => screen.queryAllByRole('option')
const panel = () => screen.getByRole('listbox', { name: 'Album summaries' })
const item = (name: string) => items().filter((o) => within(o).queryByText(name))[0]
const current = (o: HTMLElement) => o.getAttribute('aria-selected') === 'true'

const A: AlbumSpec = { index: 0, name: 'Lib\\A', files: 10 }
const B: AlbumSpec = { index: 1, name: 'Lib\\B', files: 490 }
const FIRST_OF_A = '000.flac'

/** "Given an analysis finished with 500 tracks, 10 of them in the album "Lib\A" ..." */
async function finishedRun() {
  const run = fakeRun()
  render(AnalysisPage)
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled())
  await waitForMainScreen()
  vi.mocked(api.pickFolder).mockResolvedValue('C:/Music/Lib')
  await userEvent.click(openButton())
  await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled())
  run.scanned(500)
  const next = feed(run, A, 0, Array<Verdict>(10).fill('real'))
  feed(run, B, next, Array<Verdict>(490).fill('real'))
  run.finish(makeSummary({ root: 'C:/Music/Lib', total: 500, done: 500, real: 500 }))
  await vi.waitFor(() => expect(statusText()).toMatch(/^500 tracks/))
  await vi.waitFor(() => expect(items()).toHaveLength(2))
  expect(within(items()[0]).queryByText('Lib\\A')).not.toBeNull()
}

/** The selected row is the first row of "Lib\A" (display position 0) and is in view. */
function expectFirstOfAVisible() {
  const top = grid().scrollTop
  expect(top, 'the selected row is above the visible area').toBeLessThanOrEqual(0 * ROW)
  expect(top + VIEWPORT, 'the selected row is below the visible area').toBeGreaterThanOrEqual(ROW)
  expect(selectedRows().map(fileOf)).toEqual([FIRST_OF_A])
}

/** "I scrolled the table until the selected row is outside its visible area" */
async function scrollAway() {
  grid().scrollTop = 300 * ROW
  await fireEvent.scroll(grid())
  expect(grid().scrollTop).toBe(300 * ROW)
  // the selected row (position 0) is out of the visible area and not rendered
  await vi.waitFor(() => expect(selectedRows()).toHaveLength(0))
}

let clientHeight: MockInstance<(this: Element) => number>

beforeEach(() => {
  // RF-13: English default settings, locale reset (tests/settingsHarness.ts)
  mockSettings()
  vi.mocked(api.pickFolder).mockReset()
  vi.mocked(api.startAnalysis).mockReset()
  vi.mocked(api.startAnalysisOfDropped).mockReset()
  vi.mocked(api.cancelAnalysis).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.getSpectrogram).mockReset().mockReturnValue(new Promise(() => {}))
  vi.mocked(api.exportCsv).mockReset()
  vi.mocked(api.minimizeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.toggleMaximizeWindow).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.closeWindow).mockReset().mockResolvedValue(undefined)
  unlisten.mockReset()
  vi.mocked(api.onDragDrop)
    .mockReset()
    .mockImplementation(async () => unlisten)
  // a measurable table viewport: only the grid has a height (see the readings above)
  clientHeight = vi
    .spyOn(Element.prototype, 'clientHeight', 'get')
    .mockImplementation(function (this: Element) {
      return this.getAttribute('role') === 'grid' ? VIEWPORT : 0
    })
})

afterEach(() => {
  clientHeight.mockRestore()
})

describe('Albums panel: selecting the current album again', () => {
  // @RF-10 Scenario: Selecting the current album again brings its first track back into view
  it('brings the first row back into view on a click and on Home, without a new spectrogram request', async () => {
    await finishedRun()
    // the table virtualises: not every one of the 500 rows is rendered
    expect(dataRows().length).toBeLessThan(100)

    // And I clicked the item "Lib\A", so its first row is selected
    await userEvent.click(item('Lib\\A'))
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual([FIRST_OF_A]))
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1)
    expect(api.getSpectrogram).toHaveBeenCalledWith(pathOf(A, 0))
    // And I scrolled the table until the selected row is outside its visible area
    await scrollAway()

    // When I click the item "Lib\A"
    await userEvent.click(item('Lib\\A'))
    // Then the same row is still selected and is within the table's visible area
    await vi.waitFor(() => expectFirstOfAVisible())
    // And the spectrogram has been requested only once, by the first click
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1)
    expect(api.getSpectrogram).toHaveBeenCalledWith(pathOf(A, 0))
    // And "Lib\A" is still the current album
    expect(items().map(current)).toEqual([true, false])

    // When I scroll the table until the selected row is outside its visible area again
    await scrollAway()
    // And I focus the Albums panel and press Home
    panel().focus()
    expect(document.activeElement).toBe(panel())
    await fireEvent.keyDown(panel(), { key: 'Home' })
    // Then the same row is still selected and is within the table's visible area
    await vi.waitFor(() => expectFirstOfAVisible())
    // And the spectrogram has been requested only once, by the first click
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1)
    expect(items().map(current)).toEqual([true, false])
  })
})

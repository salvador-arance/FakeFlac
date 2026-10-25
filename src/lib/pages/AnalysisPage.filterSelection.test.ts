// AnalysisPage: selection, keyboard, Albums panel and export under a filter
// (features/sort-filter.feature, "Selection and keyboard", "Albums panel (RF-10)" and "Export
// (RF-09)"; @RF-08; specs/ui/sort-filter.md sections 5, 6 and 7, acceptance A2, A3 and A4). The API
// client is mocked and the progress events are stubbed; steps, row specs and locators are those of
// ./tests/sortFilterHarness.ts (see its header for the interface of the feature).
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - "The detail panel still shows a.flac": no new spectrogram request was made since the click on
//   a.flac (its request is the last one), the detail panel still shows that track's spectrogram
//   area (loading, the stub never answers) and its album stays current.
// - "Lib\X (n tracks, ...)" for two albums of one run: the indexes continue in path order across
//   the albums (Lib\A 0..11, Lib\B 12..20), since a row index is unique in a run.
// - "Lib\A (fake tracks only)": two fake rows, indexes 0 and 1. The two rows of "Lib\B" are named
//   "01.flac" (index 4) and "02.flac" (index 9).
// - "Within the table's visible area": with a measurable viewport (the grid reports 220 px, as in
//   AnalysisPage.albumReveal.test.ts) and display position p, `scrollTop <= p * 22` and
//   `scrollTop + 220 >= (p + 1) * 22`, and the row is rendered with aria-selected="true".
// - A2: 200 trackDone events in a shuffled order (fixed seed), in five batches, with filter and
//   sort changes between batches. After every flush and every change, the whole table (read by
//   scrolling) equals the stored rows that pass `rowMatches`, in `compareRows` order (the functions
//   of src/lib/table-view.ts, checked by themselves in table-view.test.ts); the shown count and the
//   Albums panel follow.
import { fireEvent, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import type { MockInstance } from 'vitest'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as api from '../api'
import type { ResultRow, RowFilter, SortState, Verdict } from '../model'
import { compareRows, isFilterActive, rowMatches } from '../table-view'
import {
  Feeder,
  MUSIC,
  ROW,
  VIEWPORT,
  albumOf,
  cells,
  fileOf,
  finishedAnalysis,
  grid,
  installViewport,
  isCurrent,
  item,
  items,
  listed,
  mountPage,
  nTracks,
  norm,
  pressVerdict,
  readAllRows,
  replaceSearch,
  resetApiMocks,
  rowByFile,
  selectedRows,
  settle,
  shownFiles,
  sortBy,
  spectrogramLoading,
  startRun,
  statusText,
  summaryOf,
  trackOf,
  typeSearch,
  verdictButton,
} from './tests/sortFilterHarness'
import type { Listed } from './tests/sortFilterHarness'

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
  }
})

let viewport: MockInstance<(this: Element) => number>

beforeEach(() => {
  resetApiMocks()
  viewport = installViewport()
})

afterEach(() => {
  viewport.mockRestore()
  vi.useRealTimers()
})

const selectedFiles = () => selectedRows().map(fileOf)
const lastSpectrogramPath = () => vi.mocked(api.getSpectrogram).mock.calls.at(-1)?.[0]

/** The selected row is rendered and inside the visible area of the grid. */
function expectSelectedVisible(file: string) {
  const position = shownFiles().indexOf(file)
  expect(position).toBeGreaterThanOrEqual(0)
  const top = grid().scrollTop
  expect(top).toBeLessThanOrEqual(position * ROW)
  expect(top + VIEWPORT).toBeGreaterThanOrEqual((position + 1) * ROW)
  expect(selectedFiles()).toEqual([file])
}

describe('Selection and keyboard (@RF-08)', () => {
  // @RF-08 Scenario: A selected track hidden by a filter stays selected
  it('keeps a selected track hidden by a filter selected', async () => {
    await finishedAnalysis([
      { name: 'A\\a.flac', verdict: 'real' },
      { name: 'A\\b.flac', verdict: 'fake' },
    ])
    await userEvent.click(cells(rowByFile('a.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['a.flac']))
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledTimes(1))
    expect(lastSpectrogramPath()).toBe(`${MUSIC}A\\a.flac`)

    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['b.flac']))
    expect(selectedRows()).toHaveLength(0)
    expect(spectrogramLoading()).not.toBeNull()
    expect(isCurrent(item('A'))).toBe(true)
    await settle()
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1)

    await pressVerdict('fake')
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['a.flac']))
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1)
  })

  // @RF-08 Scenario: The keyboard moves over the visible rows only
  it('moves the keyboard over the visible rows only', async () => {
    await finishedAnalysis([
      { name: 'A\\a.flac', verdict: 'fake' },
      { name: 'A\\b.flac', verdict: 'real' },
      { name: 'A\\c.flac', verdict: 'fake' },
    ])
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['a.flac', 'c.flac']))
    grid().focus()
    await fireEvent.keyDown(grid(), { key: 'ArrowDown' })
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['a.flac']))
    await fireEvent.keyDown(grid(), { key: 'ArrowDown' })
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['c.flac']))
    await fireEvent.keyDown(grid(), { key: 'Home' })
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['a.flac']))
  })

  // @RF-08 Scenario: ↓ with a hidden selection selects the first visible row
  it('↓ with a hidden selection selects the first visible row', async () => {
    await finishedAnalysis([
      { name: 'A\\a.flac', verdict: 'real' },
      { name: 'A\\b.flac', verdict: 'fake' },
      { name: 'A\\c.flac', verdict: 'fake' },
    ])
    await userEvent.click(cells(rowByFile('a.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['a.flac']))
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['b.flac', 'c.flac']))
    grid().focus()
    await fireEvent.keyDown(grid(), { key: 'ArrowDown' })
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['b.flac']))
    await vi.waitFor(() => expect(lastSpectrogramPath()).toBe(`${MUSIC}A\\b.flac`))
  })
})

describe('Albums panel under a filter (@RF-08 @RF-10)', () => {
  // @RF-08 Scenario: The Albums panel ignores the filters
  it('ignores the filters', async () => {
    await finishedAnalysis([...nTracks({ real: 11, fake: 1 }, 'Lib\\A', 0), ...nTracks({ real: 9 }, 'Lib\\B', 12)])
    await vi.waitFor(() => expect(items()).toHaveLength(2))
    const before = items().map((o) => norm(o.textContent))
    await pressVerdict('fake')
    await typeSearch('zzz')
    await vi.waitFor(() => expect(shownFiles()).toEqual([]))
    expect(norm(item('Lib\\A').textContent)).toContain('12 tracks · 11 real · 1 fake')
    expect(norm(item('Lib\\B').textContent)).toContain('9 tracks · 9 real')
    expect(items().map((o) => norm(o.textContent))).toEqual(before)
  })

  // @RF-08 Scenario: Selecting an album selects its lowest-index visible track
  it('selecting an album selects its lowest-index visible track', async () => {
    await finishedAnalysis([
      { name: 'Lib\\B\\01.flac', verdict: 'real', index: 4 },
      { name: 'Lib\\B\\02.flac', verdict: 'fake', index: 7 },
      { name: 'Lib\\B\\03.flac', verdict: 'fake', index: 9 },
    ])
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['02.flac', '03.flac']))
    await userEvent.click(item('Lib\\B'))
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['02.flac']))
    expectSelectedVisible('02.flac')
    expect(isCurrent(item('Lib\\B'))).toBe(true)
  })

  // @RF-08 Scenario: Under the State sort the lowest-index visible track is not the first shown
  it('under the State sort selects the lowest-index shown track, not the first shown', async () => {
    await finishedAnalysis([
      { name: 'Lib\\B\\01.flac', verdict: 'real', index: 4 },
      { name: 'Lib\\B\\02.flac', verdict: 'suspicious', index: 7 },
      { name: 'Lib\\B\\03.flac', verdict: 'fake', index: 9 },
    ])
    await sortBy('State', 'ascending')
    await pressVerdict('fake')
    await pressVerdict('suspicious')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['03.flac', '02.flac']))
    await userEvent.click(item('Lib\\B'))
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['02.flac']))
    expect(shownFiles().indexOf('02.flac')).toBe(1)

    await pressVerdict('fake')
    await pressVerdict('suspicious')
    await userEvent.click(item('Lib\\B'))
    await vi.waitFor(() => expect(shownFiles()).toEqual(['03.flac', '02.flac', '01.flac']))
    await vi.waitFor(() => expect(selectedFiles()).toEqual(['01.flac']))
  })

  // @RF-08 Scenario: Selecting an album whose tracks are all hidden
  it('selecting an album whose tracks are all hidden selects its lowest-index row, hidden', async () => {
    await finishedAnalysis([
      { name: 'Lib\\A\\01.flac', verdict: 'fake', index: 0 },
      { name: 'Lib\\A\\02.flac', verdict: 'fake', index: 1 },
      { name: 'Lib\\B\\01.flac', verdict: 'real', index: 4 },
      { name: 'Lib\\B\\02.flac', verdict: 'real', index: 9 },
    ])
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['01.flac', '02.flac']))
    await userEvent.click(item('Lib\\B'))
    await vi.waitFor(() => expect(lastSpectrogramPath()).toBe(`${MUSIC}Lib\\B\\01.flac`))
    expect(selectedRows()).toHaveLength(0)
    expect(shownFiles()).toEqual(['01.flac', '02.flac'])
    expect(isCurrent(item('Lib\\B'))).toBe(true)
    expect(isCurrent(item('Lib\\A'))).toBe(false)
    expect(verdictButton('fake').getAttribute('aria-pressed')).toBe('true')
  })
})

describe('Export under sort and filter (@RF-08 @RF-09)', () => {
  // @RF-08 Scenario: Export ignores the sort and the filters (A4)
  it('exports with no argument whatever the sort and the filters', async () => {
    await finishedAnalysis(nTracks({ real: 30, fake: 1 }))
    await sortBy('State', 'descending')
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['31.flac']))
    const exportButton = screen.getByRole('button', { name: 'Export CSV' }) as HTMLButtonElement
    expect(exportButton.disabled).toBe(false)
    vi.mocked(api.exportCsv).mockResolvedValue({ rows: 31, path: 'C:\\Out\\results.csv' })
    await userEvent.click(exportButton)
    await vi.waitFor(() =>
      expect(statusText().endsWith('· 1 shown · Saved 31 rows to results.csv')).toBe(true),
    )
    expect(api.exportCsv).toHaveBeenCalledTimes(1)
    expect(api.exportCsv).toHaveBeenCalledWith()
  })

  it('keeps Export CSV enabled when a filter hides every row', async () => {
    await finishedAnalysis(nTracks({ real: 3 }))
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual([]))
    expect((screen.getByRole('button', { name: 'Export CSV' }) as HTMLButtonElement).disabled).toBe(false)
  })
})

// ---------------------------------------------------------------------------------------------
// A2: 200 events in shuffled order, filter and sort changes between batches.

/** Deterministic shuffle (mulberry32). */
function shuffle<T>(list: T[], seed: number): T[] {
  let a = seed
  const random = () => {
    a |= 0
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
  const out = [...list]
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1))
    ;[out[i], out[j]] = [out[j], out[i]]
  }
  return out
}

describe('A2: filters over shuffled batches (@RF-08)', () => {
  it('shows exactly the stored rows that pass the filters, in compareRows order', async () => {
    const VERDICTS: Verdict[] = ['real', 'real', 'fake', 'suspicious', 'real', 'error', 'fake']
    const specs: Listed[] = listed(
      Array.from({ length: 200 }, (_, i) => ({
        name: `Lib\\Album ${i % 5}\\${String(i).padStart(3, '0')} \u2013 Track ${i % 13}.flac`,
        verdict: VERDICTS[i % VERDICTS.length],
        cutoffHz: VERDICTS[i % VERDICTS.length] === 'error' ? null : 15000 + ((i * 131) % 7000),
        durationMs: 1000 + ((i * 7919) % 300000),
      })),
    )
    const order = shuffle(specs, 2026)
    const albumIndex = new Map(
      [...new Set(specs.map((s) => albumOf(s.name)))].sort().map((name, index) => [name, index]),
    )
    const toRow = (s: Listed): ResultRow => ({
      index: s.index,
      analysis: trackOf(s),
      albumIndex: albumIndex.get(albumOf(s.name)) as number,
    })

    await mountPage()
    const run = await startRun()
    run.scanned(200)
    const feeder = new Feeder(run, specs)
    const stored: Listed[] = []
    let filter: RowFilter = { verdicts: [], query: '' }
    let sort: SortState = { column: 'file', direction: 'asc' }

    const check = async (label: string) => {
      const expected = stored
        .map(toRow)
        .filter((r) => rowMatches(r, filter, albumOf(specs.find((s) => s.index === r.index)!.name)))
        .sort((a, b) => compareRows(a, b, sort, 'en'))
        .map((r) => r.analysis.info.path.slice(MUSIC.length))
      const { order: shown } = await readAllRows()
      expect(shown, label).toEqual(expected)
      if (isFilterActive(filter)) expect(statusText(), label).toMatch(new RegExp(` · ${expected.length} shown$`))
      else expect(statusText(), label).not.toContain('shown')
    }
    const batch = async (from: number, to: number) => {
      feeder.emit(order.slice(from, to))
      stored.push(...order.slice(from, to))
      await settle()
    }
    const panel = () => items().map((o) => norm(o.textContent))

    await batch(0, 40)
    await check('batch 1')

    let albums = panel()
    await pressVerdict('fake')
    filter = { ...filter, verdicts: ['fake'] }
    await check('fake pressed')
    expect(panel()).toEqual(albums)

    await batch(40, 80)
    await check('batch 2')

    albums = panel()
    await typeSearch('1')
    filter = { ...filter, query: '1' }
    await check('query 1')
    expect(panel()).toEqual(albums)

    await sortBy('State', 'ascending')
    sort = { column: 'state', direction: 'asc' }
    await check('state ascending')

    await batch(80, 120)
    await check('batch 3')

    albums = panel()
    await pressVerdict('suspicious')
    filter = { ...filter, verdicts: ['fake', 'suspicious'] }
    await check('fake and suspicious')
    await replaceSearch('album 3')
    filter = { ...filter, query: 'album 3' }
    await check('query album 3')
    expect(panel()).toEqual(albums)

    await sortBy('Duration', 'descending')
    sort = { column: 'duration', direction: 'desc' }
    await batch(120, 160)
    await check('batch 4')

    await pressVerdict('fake')
    filter = { ...filter, verdicts: ['suspicious'] }
    await check('suspicious only')
    await replaceSearch('')
    filter = { ...filter, query: '' }
    await check('no query')

    feeder.emit(order.slice(160))
    stored.push(...order.slice(160))
    run.finish(summaryOf(specs))
    await vi.waitFor(() => expect(statusText()).toMatch(/^200 tracks/))
    await check('finished')
  }, 120000)
})

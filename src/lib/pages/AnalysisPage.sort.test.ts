// AnalysisPage: sorting (features/sort-filter.feature, "Sorting" scenarios; @RF-08; acceptance A1 of
// specs/ui/sort-filter.md). The API client is mocked and the progress events are stubbed; steps,
// row specs and locators are those of ./tests/sortFilterHarness.ts (see its header for the
// interface of the feature).
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - "The first row is the one with index 3" (two rows with the same path): the two rows differ only
//   by their verdict (index 3 fake, index 5 real), so the row is identified by its State cell.
// - "Scroll the table to half its maximum scroll position": jsdom has no layout, so the maximum is
//   computed as rows x 22 px - the 220 px viewport (the sticky header measures 0 in jsdom).
// - "The table is sorted by state descending" after the next run: the State column of the new rows
//   reads in descending order and the State header has aria-sort "descending".
// - "The Folder cell shows its path's folder" (empty TrackInfo.folder): the cell keeps today's
//   fallback text, which uses `/` separators, so it is compared with `/` read as `\`.
import userEvent from '@testing-library/user-event'
import type { MockInstance } from 'vitest'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  Feeder,
  ROW,
  VIEWPORT,
  cells,
  clickHeader,
  dataRows,
  fileOf,
  finishRun,
  finishedAnalysis,
  folderCellOf,
  grid,
  header,
  installViewport,
  listed,
  mountPage,
  readAllRows,
  rowByName,
  scrollGridTo,
  selectedRows,
  settle,
  shownFiles,
  shownNames,
  sortBy,
  startRun,
  stateColumn,
  stateOf,
} from './tests/sortFilterHarness'
import type { RowSpec } from './tests/sortFilterHarness'
import { resetApiMocks } from './tests/sortFilterHarness'

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

beforeEach(() => {
  resetApiMocks()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('Sorting (@RF-08)', () => {
  // @RF-08 Scenario: The default order is by name, natural and case-insensitive
  it('the default order is by name, natural and case-insensitive', async () => {
    await finishedAnalysis([
      { name: 'Album\\Track 10.flac' },
      { name: 'Album\\track 2.flac' },
      { name: 'Album\\Track 1.flac' },
    ])
    expect(shownFiles()).toEqual(['Track 1.flac', 'track 2.flac', 'Track 10.flac'])
    expect(header('File').getAttribute('aria-sort')).toBe('ascending')
  })

  // @RF-08 Scenario: A folder's tracks come before a sibling folder whose name extends it
  it("puts a folder's tracks before a sibling folder whose name extends it", async () => {
    await finishedAnalysis([
      { name: 'Lib\\Album (deluxe)\\01.flac' },
      { name: 'Lib\\Album\\CD2\\01.flac' },
      { name: 'Lib\\Album\\02.flac' },
    ])
    expect(shownNames()).toEqual(['Lib\\Album\\02.flac', 'Lib\\Album\\CD2\\01.flac', 'Lib\\Album (deluxe)\\01.flac'])
  })

  // @RF-08 Scenario: A subfolder sorts among its parent folder's files by its name
  it("sorts a subfolder among its parent folder's files by its name", async () => {
    await finishedAnalysis([
      { name: 'Lib\\Album\\Z.flac' },
      { name: 'Lib\\Album\\Bonus\\x.flac' },
      { name: 'Lib\\Album\\01.flac' },
    ])
    expect(shownNames()).toEqual(['Lib\\Album\\01.flac', 'Lib\\Album\\Bonus\\x.flac', 'Lib\\Album\\Z.flac'])
    await clickHeader('File')
    await vi.waitFor(() =>
      expect(shownNames()).toEqual(['Lib\\Album\\Z.flac', 'Lib\\Album\\Bonus\\x.flac', 'Lib\\Album\\01.flac']),
    )
  })

  // @RF-08 Scenario: Sorting by state puts the tracks that need attention first
  it('sorting by state puts the tracks that need attention first', async () => {
    await finishedAnalysis([
      { name: 'A\\a.flac', verdict: 'real' },
      { name: 'A\\b.flac', verdict: 'error' },
      { name: 'A\\c.flac', verdict: 'fake' },
      { name: 'A\\d.flac', verdict: 'suspicious' },
    ])
    await clickHeader('State')
    await vi.waitFor(() => expect(header('State').getAttribute('aria-sort')).toBe('ascending'))
    expect(stateColumn()).toEqual(['Fake', 'Suspicious', 'Error', 'Genuine'])
    await clickHeader('State')
    await vi.waitFor(() => expect(header('State').getAttribute('aria-sort')).toBe('descending'))
    expect(stateColumn()).toEqual(['Genuine', 'Error', 'Suspicious', 'Fake'])
  })

  // @RF-08 Scenario: Rows with the same verdict stay in name order in both directions
  it('keeps rows with the same verdict in name order in both directions, whatever the arrival order', async () => {
    const order = ['c.flac', 'b.flac', 'a.flac']
    await finishedAnalysis(
      [
        { name: 'A\\b.flac', verdict: 'fake', index: 0 },
        { name: 'A\\a.flac', verdict: 'fake', index: 1 },
        { name: 'A\\c.flac', verdict: 'real', index: 2 },
      ],
      undefined,
      (all) => order.map((f) => all.find((r) => r.name === `A\\${f}`)!),
    )
    await sortBy('State', 'ascending')
    expect(shownFiles()).toEqual(['a.flac', 'b.flac', 'c.flac'])
    await sortBy('State', 'descending')
    expect(shownFiles()).toEqual(['c.flac', 'a.flac', 'b.flac'])
  })

  // @RF-08 @RNF-09 Scenario: Two rows with the same path keep their scan order
  it('keeps the scan order of two rows with the same path', async () => {
    const same = 'Lib\\Disc \uFFFD\\01.flac'
    await finishedAnalysis([
      { name: same, index: 5, verdict: 'real' },
      { name: same, index: 3, verdict: 'fake' },
    ])
    expect(dataRows()).toHaveLength(2)
    // index 3 is the fake one
    expect(stateColumn()).toEqual(['Fake', 'Genuine'])
    await clickHeader('File')
    await vi.waitFor(() => expect(header('File').getAttribute('aria-sort')).toBe('descending'))
    expect(stateOf(dataRows()[0])).toBe('Fake')
  })

  // @RF-08 @RNF-04 Scenario: Empty cells go last in both directions
  it('puts empty cells last in both directions', async () => {
    await finishedAnalysis([
      { name: 'A\\low.flac', verdict: 'fake', cutoffHz: 16000 },
      { name: 'A\\corrupt.flac', verdict: 'error', cutoffHz: null },
      { name: 'A\\full.flac', verdict: 'real', cutoffHz: 20000 },
    ])
    await sortBy('Cutoff', 'ascending')
    expect(shownFiles()).toEqual(['low.flac', 'full.flac', 'corrupt.flac'])
    await sortBy('Cutoff', 'descending')
    expect(shownFiles()).toEqual(['full.flac', 'low.flac', 'corrupt.flac'])
  })

  // @RF-08 Scenario: A row with an empty folder sorts by the folder its cell shows
  it('sorts a row with an empty folder by the folder its cell shows', async () => {
    await finishedAnalysis([
      { name: 'L\\B\\01.flac', folder: '' },
      { name: 'L\\A\\01.flac', folder: 'C:\\Music\\L\\A' },
      { name: 'L\\C\\01.flac', folder: 'C:\\Music\\L\\C' },
    ])
    await clickHeader('Folder')
    await vi.waitFor(() => expect(header('Folder').getAttribute('aria-sort')).toBe('ascending'))
    expect(shownNames()).toEqual(['L\\A\\01.flac', 'L\\B\\01.flac', 'L\\C\\01.flac'])
    const b = rowByName('L\\B\\01.flac') as HTMLElement
    expect(folderCellOf(b).replaceAll('/', '\\')).toBe('C:\\Music\\L\\B')
  })

  // @RF-08 @RNF-09 Scenario: Unicode names sort by letter, accents only breaking ties
  it('sorts Unicode names by letter, accents only breaking ties, and shows them exactly', async () => {
    const files = [
      'zebra.flac',
      'Canci\u00f3n.flac',
      'apple.flac',
      '\u00c7a va.flac',
      'Cancion.flac',
      '\u00c1lbum \u2033\u00d1and\u00fa\u2033.flac',
    ]
    await finishedAnalysis(files.map((f) => ({ name: `A\\${f}` })))
    const expected = [
      '\u00c1lbum \u2033\u00d1and\u00fa\u2033.flac',
      'apple.flac',
      '\u00c7a va.flac',
      'Cancion.flac',
      'Canci\u00f3n.flac',
      'zebra.flac',
    ]
    expect(shownFiles()).toEqual(expected)
    // code point for code point
    expect(dataRows().map((r) => [...fileOf(r)].map((c) => c.codePointAt(0)))).toEqual(
      expected.map((f) => [...f].map((c) => c.codePointAt(0))),
    )
  })

  // @RF-08 Scenario: The sort is kept for the next run
  it('keeps the sort for the next run', async () => {
    await finishedAnalysis(
      [
        { name: 'Lib A\\1.flac', verdict: 'fake' },
        { name: 'Lib A\\2.flac', verdict: 'real' },
      ],
      'C:\\Music\\Lib A',
    )
    await sortBy('State', 'descending')
    const run = await startRun('C:\\Music\\Lib B')
    await finishRun(
      run,
      [
        { name: 'Lib B\\1.flac', verdict: 'fake' },
        { name: 'Lib B\\2.flac', verdict: 'real' },
        { name: 'Lib B\\3.flac', verdict: 'suspicious' },
        { name: 'Lib B\\4.flac', verdict: 'error' },
      ],
      'C:\\Music\\Lib B',
    )
    await vi.waitFor(() => expect(dataRows()).toHaveLength(4))
    expect(header('State').getAttribute('aria-sort')).toBe('descending')
    expect(stateColumn()).toEqual(['Genuine', 'Error', 'Suspicious', 'Fake'])
  })
})

describe('Sorting during a run (@RF-08)', () => {
  let viewport: MockInstance<(this: Element) => number>

  beforeEach(() => {
    viewport = installViewport()
  })

  afterEach(() => {
    viewport.mockRestore()
  })

  const four = (n: number) => String(n).padStart(4, '0')
  const spec = (n: number): RowSpec => ({ name: `A\\${four(n)}.flac`, index: n, verdict: n % 2 === 1 ? 'fake' : 'real' })

  // @RF-08 Scenario: New rows take their sorted place during a run without moving the selection or the scroll
  it('puts new rows at their sorted place without moving the selection or the scroll', async () => {
    const all = listed(Array.from({ length: 520 }, (_, n) => spec(n)))
    await mountPage()
    const run = await startRun()
    run.scanned(520)
    const feeder = new Feeder(run, all)
    feeder.emit(all.slice(0, 500))
    await settle()
    await vi.waitFor(() => expect(dataRows().length).toBeGreaterThan(0))
    // the table virtualises
    expect(dataRows().length).toBeLessThan(100)

    await sortBy('State', 'ascending')
    // "A\0100.flac" is genuine: after the 250 fake rows, the 51st genuine row (position 300)
    await scrollGridTo(300 * ROW)
    await vi.waitFor(() => expect(rowByName('A\\0100.flac')).toBeDefined())
    await userEvent.click(cells(rowByName('A\\0100.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['0100.flac']))
    const half = Math.round((500 * ROW - VIEWPORT) / 2)
    await scrollGridTo(half)
    expect(grid().scrollTop).toBe(half)

    feeder.emit(all.slice(500))
    await settle()
    await vi.waitFor(() => expect(grid().scrollTop).toBe(half))
    await settle()
    expect(grid().scrollTop).toBe(half)

    const { order, selected } = await readAllRows()
    const fakes = Array.from({ length: 260 }, (_, k) => `A\\${four(2 * k + 1)}.flac`)
    const genuine = Array.from({ length: 260 }, (_, k) => `A\\${four(2 * k)}.flac`)
    expect(order).toEqual([...fakes, ...genuine])
    expect(selected).toEqual(['A\\0100.flac'])
  }, 60000)
})

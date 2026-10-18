// ResultsTable (specs/ui/components.md, Organisms; main-screen.md "Interaction"; analyze-folder.feature).
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - The grid is `div`-based: role="grid" (scroll container) > role="row" > "columnheader" / "gridcell".
//   Cells are, in order: State, File, Folder, Format, Samplerate, Bits, Duration, Cutoff.
// - The table sorts `rows` itself by the `sort` prop, so keyboard order = visual order.
//   The File column sorts by the full path ("sorted by file path").
// - Virtualisation is testable through `rowHeight` and `viewportHeight` (px). The role="grid"
//   element scrolls; when the viewport height is 0 / unknown (jsdom), at least 50 rows render.
// - An error row exposes its error text (e.g. "Corrupt stream") as a `title` on the row or a cell.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { ResultRow, SortState } from '../../model'
import { makeManyRows, makeRow } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

const byPath: SortState = { column: 'file', direction: 'asc' }

function dataRows(): HTMLElement[] {
  return screen
    .queryAllByRole('row')
    .filter((row) => within(row).queryAllByRole('gridcell').length > 0)
}
const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')
const fileColumn = () => dataRows().map((row) => cells(row)[1].textContent?.trim())

function setup(rows: ResultRow[], extra: Record<string, unknown> = {}) {
  const onselect = vi.fn()
  const onsort = vi.fn()
  const view = render(ResultsTable, {
    props: { rows, selectedIndex: null, sort: byPath, onselect, onsort, ...extra },
  })
  return { onselect, onsort, ...view }
}

describe('ResultsTable: structure', () => {
  // @RF-06 state: empty
  it('is a grid with the eight columns and no data rows when empty', () => {
    setup([])
    expect(screen.getByRole('grid')).not.toBeNull()
    expect(screen.getAllByRole('columnheader').map((h) => h.textContent?.trim())).toEqual([
      'State',
      'File',
      'Folder',
      'Format',
      'Samplerate',
      'Bits',
      'Duration',
      'Cutoff',
    ])
    expect(dataRows()).toHaveLength(0)
  })

  // @RF-06 Results appear in the table as they finish
  it('shows the cells of a real FLAC row', () => {
    setup([makeRow(0, { path: 'C:/Music/Album A/full_band.flac', durationMs: 181000 })])
    const [state, file, folder, format, rate, bits, duration] = cells(dataRows()[0])
    expect(state.textContent).toContain('Genuine')
    expect(file.textContent?.trim()).toBe('full_band.flac')
    expect(folder.textContent).toContain('C:/Music/Album A')
    expect(format.textContent?.trim()).toBe('FLAC')
    expect(rate.textContent).toContain('44100')
    expect(bits.textContent).toContain('16')
    expect(duration.textContent).toMatch(/3:01/)
  })

  // @RF-06 "lp16k.flac with state Fake and cutoff 16.0 kHz"
  it('shows the fake verdict with its cutoff', () => {
    setup([makeRow(0, { path: 'C:/A/lp16k.flac', verdict: 'fake', cutoffHz: 16000 })])
    const row = dataRows()[0]
    expect(cells(row)[0].textContent).toContain('Fake')
    expect(cells(row)[7].textContent?.trim()).toBe('16.0 kHz')
  })

  // CutoffValue: nyquist = sampleRate / 2
  it('shows "Full band" when the cutoff is the nyquist frequency of the row', () => {
    setup([makeRow(0, { sampleRate: 48000, cutoffHz: 24000 })])
    expect(cells(dataRows()[0])[7].textContent?.trim()).toBe('Full band')
  })

  it('shows an empty cutoff cell without a cutoff', () => {
    setup([makeRow(0, { cutoffHz: null })])
    expect(cells(dataRows()[0])[7].textContent?.trim()).toBe('')
  })

  // @RNF-04 "corrupt.flac has state Error with the reason Corrupt stream"
  it.each([
    [{ kind: 'corruptStream', detail: 'bad frame' }, 'Corrupt stream'],
    [{ kind: 'tooShort' }, 'Too short to analyse'],
    [{ kind: 'unreadable', detail: 'denied' }, 'Cannot read the file'],
  ] as const)('shows an error row with the reason %j', (error, text) => {
    setup([makeRow(0, { path: 'C:/A/corrupt.flac', verdict: 'error', error })])
    const row = dataRows()[0]
    expect(cells(row)[0].textContent).toContain('Error')
    const selector = `[title*="${text}"]`
    expect(row.matches(selector) || row.querySelector(selector) !== null).toBe(true)
  })

  // @RNF-09 Unicode names
  it('shows a Unicode file name exactly', () => {
    setup([makeRow(0, { path: 'C:/Música/Álbum ″Ñandú″/01 – Canción ‘x’.flac' })])
    expect(fileColumn()).toEqual(['01 – Canción ‘x’.flac'])
    expect(cells(dataRows()[0])[2].textContent).toContain('Álbum ″Ñandú″')
  })

  // contracts/commands.md: paths may repeat; rows are keyed by `index`
  it('renders two rows with the same path and different index', () => {
    setup([makeRow(0, { path: 'C:/A/same.flac' }), makeRow(1, { path: 'C:/A/same.flac' })])
    expect(dataRows()).toHaveLength(2)
  })

  // @RF-06 rows appear as they finish
  it('shows rows added later', async () => {
    const { rerender } = setup([makeRow(0, { path: 'C:/A/a.flac' })])
    await rerender({ rows: [makeRow(0, { path: 'C:/A/a.flac' }), makeRow(1, { path: 'C:/A/b.flac' })] })
    expect(fileColumn()).toEqual(['a.flac', 'b.flac'])
  })
})

describe('ResultsTable: sorting', () => {
  const rows = [
    makeRow(0, { path: 'C:/B/a.flac', durationMs: 60000, sampleRate: 96000 }),
    makeRow(1, { path: 'C:/A/z.flac', durationMs: 300000, sampleRate: 44100 }),
    makeRow(2, { path: 'C:/A/b.flac', durationMs: 120000, sampleRate: 48000 }),
  ]

  // main-screen.md: "Rows are sorted by file path by default"
  it('sorts by full path ascending', () => {
    setup(rows)
    expect(fileColumn()).toEqual(['b.flac', 'z.flac', 'a.flac'])
  })

  it('sorts by full path descending', () => {
    setup(rows, { sort: { column: 'file', direction: 'desc' } })
    expect(fileColumn()).toEqual(['a.flac', 'z.flac', 'b.flac'])
  })

  it.each([
    ['duration', 'asc', ['a.flac', 'b.flac', 'z.flac']],
    ['duration', 'desc', ['z.flac', 'b.flac', 'a.flac']],
    ['sampleRate', 'asc', ['z.flac', 'b.flac', 'a.flac']],
    ['sampleRate', 'desc', ['a.flac', 'b.flac', 'z.flac']],
  ] as const)('sorts by %s %s', (column, direction, expected) => {
    setup(rows, { sort: { column, direction } })
    expect(fileColumn()).toEqual(expected)
  })

  // aria-sort on the active column only
  it('marks the sorted column with aria-sort', () => {
    setup(rows, { sort: { column: 'duration', direction: 'desc' } })
    const headers = screen.getAllByRole('columnheader')
    const sorts = headers.map((h) => h.getAttribute('aria-sort'))
    expect(sorts[6]).toBe('descending')
    expect(sorts.filter((s) => s !== 'none' && s !== 'descending')).toEqual([])
    expect(sorts.filter((s) => s === 'descending')).toHaveLength(1)
  })

  // "clicking a column header sorts by it": the table only reports the column
  it('emits sort(column) when a header is clicked', async () => {
    const { onsort } = setup(rows)
    await userEvent.click(screen.getByRole('button', { name: /Duration/ }))
    expect(onsort).toHaveBeenCalledTimes(1)
    expect(onsort).toHaveBeenCalledWith('duration')
  })
})

describe('ResultsTable: selection and keyboard', () => {
  // display order by path asc: a.flac (index 1), b.flac (index 2), c.flac (index 0)
  const rows = [
    makeRow(0, { path: 'C:/A/c.flac' }),
    makeRow(1, { path: 'C:/A/a.flac' }),
    makeRow(2, { path: 'C:/A/b.flac' }),
  ]

  // main-screen.md: "The first row is not auto-selected"
  it('selects nothing by default', () => {
    const { onselect } = setup(rows)
    expect(dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')).toHaveLength(0)
    expect(onselect).not.toHaveBeenCalled()
  })

  // accessibility: "row selection announced"
  it('marks the selected row with aria-selected', () => {
    setup(rows, { selectedIndex: 2 })
    const selected = dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')
    expect(selected).toHaveLength(1)
    expect(cells(selected[0])[1].textContent?.trim()).toBe('b.flac')
  })

  // single selection; the event carries the row `index`, not its position
  it('emits select(index) when a row is clicked', async () => {
    const { onselect } = setup(rows)
    await userEvent.click(cells(dataRows()[0])[1]) // a.flac -> index 1
    expect(onselect).toHaveBeenCalledTimes(1)
    expect(onselect).toHaveBeenCalledWith(1)
  })

  // ↓ from no selection goes to the first row (first row is never selected automatically)
  it('ArrowDown with no selection selects the first row', async () => {
    const { onselect } = setup(rows)
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenCalledWith(1)
  })

  it('ArrowDown moves to the next row in display order', async () => {
    const { onselect } = setup(rows, { selectedIndex: 1 })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenCalledWith(2)
  })

  it('ArrowUp moves to the previous row in display order', async () => {
    const { onselect } = setup(rows, { selectedIndex: 2 })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowUp' })
    expect(onselect).toHaveBeenCalledWith(1)
  })

  it('follows the sorted order, not the index order', async () => {
    const { onselect } = setup(rows, {
      selectedIndex: 0,
      sort: { column: 'file', direction: 'desc' }, // c(0), b(2), a(1)
    })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenCalledWith(2)
  })

  it('Home selects the first row and End the last', async () => {
    const { onselect } = setup(rows, { selectedIndex: 2 })
    const grid = screen.getByRole('grid')
    await fireEvent.keyDown(grid, { key: 'Home' })
    expect(onselect).toHaveBeenLastCalledWith(1)
    await fireEvent.keyDown(grid, { key: 'End' })
    expect(onselect).toHaveBeenLastCalledWith(0)
  })

  it('does not select anything with an empty table', async () => {
    const { onselect } = setup([])
    const grid = screen.getByRole('grid')
    for (const key of ['ArrowDown', 'ArrowUp', 'Home', 'End']) await fireEvent.keyDown(grid, { key })
    expect(onselect).not.toHaveBeenCalled()
  })
})

describe('ResultsTable: virtualisation', () => {
  // RF-06 / frontend.md "Table performance": only the visible rows (plus a margin) are in the DOM
  it('renders only a window of a long list', () => {
    setup(makeManyRows(1000), { rowHeight: 22, viewportHeight: 220 })
    const rendered = dataRows().length
    expect(rendered).toBeGreaterThanOrEqual(10)
    expect(rendered).toBeLessThan(100)
    expect(fileColumn()[0]).toBe('file-0000.flac')
  })

  it('renders the rows that scroll into view', async () => {
    setup(makeManyRows(1000), { rowHeight: 22, viewportHeight: 220 })
    const grid = screen.getByRole('grid')
    grid.scrollTop = 22 * 500
    await fireEvent.scroll(grid)
    expect(fileColumn()).toContain('file-0505.flac')
    expect(fileColumn()).not.toContain('file-0000.flac')
    expect(dataRows().length).toBeLessThan(100)
  })

  it('End selects the last row even when it is not rendered', async () => {
    const { onselect } = setup(makeManyRows(1000), {
      selectedIndex: 0,
      rowHeight: 22,
      viewportHeight: 220,
    })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'End' })
    expect(onselect).toHaveBeenCalledWith(999)
  })

  it('renders a short list completely when the viewport height is unknown (jsdom)', () => {
    setup(makeManyRows(30))
    expect(dataRows()).toHaveLength(30)
  })
})

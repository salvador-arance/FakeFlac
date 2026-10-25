// ResultsTable with the RF-08 rules (specs/ui/sort-filter.md sections 3, 5, 8 and 10 `ResultsTable`
// row; @RF-08 @RNF-04): `rows` are the rows to show and the table sorts them with `compareRows`; the
// no-match state (`noMatches`, `clearFilters`); the keyboard with a `selectedIndex` that is not in
// `rows`.
//
// Same locators as ResultsTable.test.ts: role="grid" > role="row" > "gridcell"; cells in the order
// State, File, Folder, ...
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { ResultRow, SortState, Verdict } from '../../model'
import { makeRow } from '../../testing/fixtures'
import type { TrackOptions } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

const byPath: SortState = { column: 'file', direction: 'asc' }
const dataRows = () => screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')
const fileColumn = () => dataRows().map((r) => cells(r)[1].textContent?.trim())
const stateColumn = () => dataRows().map((r) => cells(r)[0].textContent?.trim())
const selectedRows = () => dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')
const NO_MATCHES = 'No tracks match the filters'

function row(index: number, path: string, o: TrackOptions = {}, folder?: string): ResultRow {
  const r = makeRow(index, { ...o, path })
  const cut = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  r.analysis.info.folder = folder ?? path.slice(0, cut)
  return r
}

function setup(rows: ResultRow[], extra: Record<string, unknown> = {}) {
  const onselect = vi.fn()
  const onsort = vi.fn()
  const onclearfilters = vi.fn()
  const view = render(ResultsTable, {
    props: { rows, selectedIndex: null, sort: byPath, onselect, onsort, onclearfilters, ...extra },
  })
  return { onselect, onsort, onclearfilters, ...view }
}

describe('ResultsTable: sort rules (@RF-08)', () => {
  const every: [Verdict, string][] = [
    ['real', 'a.flac'],
    ['error', 'b.flac'],
    ['fake', 'c.flac'],
    ['suspicious', 'd.flac'],
  ]
  const rows = every.map(([verdict, name], i) => row(i, `C:\\L\\A\\${name}`, { verdict }))

  it('State ascending with every verdict: Fake, Suspicious, Error, Genuine', () => {
    setup(rows, { sort: { column: 'state', direction: 'asc' } })
    expect(stateColumn()).toEqual(['Fake', 'Suspicious', 'Error', 'Genuine'])
  })

  it('State descending with every verdict: Genuine, Error, Suspicious, Fake', () => {
    setup(rows, { sort: { column: 'state', direction: 'desc' } })
    expect(stateColumn()).toEqual(['Genuine', 'Error', 'Suspicious', 'Fake'])
  })

  it('File: natural and case-insensitive', () => {
    setup([
      row(0, 'C:\\L\\Album\\Track 10.flac'),
      row(1, 'C:\\L\\Album\\track 2.flac'),
      row(2, 'C:\\L\\Album\\Track 1.flac'),
    ])
    expect(fileColumn()).toEqual(['Track 1.flac', 'track 2.flac', 'Track 10.flac'])
  })

  // @RNF-04 empty cells last
  it('empty cells last in both directions (Cutoff, Bits)', async () => {
    const r = [
      row(0, 'C:\\L\\A\\low.flac', { cutoffHz: 16000, bitDepth: 16, verdict: 'fake' }),
      row(1, 'C:\\L\\A\\corrupt.flac', { cutoffHz: null, bitDepth: null, verdict: 'error' }),
      row(2, 'C:\\L\\A\\full.flac', { cutoffHz: 20000, bitDepth: 24 }),
    ]
    const view = setup(r, { sort: { column: 'cutoff', direction: 'asc' } })
    expect(fileColumn()).toEqual(['low.flac', 'full.flac', 'corrupt.flac'])
    await view.rerender({ sort: { column: 'cutoff', direction: 'desc' } })
    expect(fileColumn()).toEqual(['full.flac', 'low.flac', 'corrupt.flac'])
    await view.rerender({ sort: { column: 'bitDepth', direction: 'asc' } })
    expect(fileColumn()).toEqual(['low.flac', 'full.flac', 'corrupt.flac'])
    await view.rerender({ sort: { column: 'bitDepth', direction: 'desc' } })
    expect(fileColumn()).toEqual(['full.flac', 'low.flac', 'corrupt.flac'])
  })

  it('Folder sort with an empty folder uses the fallback the cell shows', async () => {
    const r = [
      row(0, 'C:\\Music\\L\\B\\01.flac', {}, ''),
      row(1, 'C:\\Music\\L\\A\\01.flac', {}, 'C:\\Music\\L\\A'),
      row(2, 'C:\\Music\\L\\C\\01.flac', {}, 'C:\\Music\\L\\C'),
    ]
    const view = setup(r, { sort: { column: 'folder', direction: 'asc' } })
    // the fallback text of an empty folder keeps today's `/` separators: read `/` as `\`
    const folders = () => dataRows().map((x) => cells(x)[2].textContent?.trim().replaceAll('/', '\\'))
    expect(folders()).toEqual(['C:\\Music\\L\\A', 'C:\\Music\\L\\B', 'C:\\Music\\L\\C'])
    await view.rerender({ sort: { column: 'folder', direction: 'desc' } })
    expect(folders()).toEqual(['C:\\Music\\L\\C', 'C:\\Music\\L\\B', 'C:\\Music\\L\\A'])
  })

  it('ties: same verdict in name order, then index, in both directions', async () => {
    const r = [
      row(0, 'C:\\L\\A\\b.flac', { verdict: 'fake' }),
      row(1, 'C:\\L\\A\\a.flac', { verdict: 'fake' }),
      row(2, 'C:\\L\\A\\c.flac', { verdict: 'real' }),
    ]
    const view = setup(r, { sort: { column: 'state', direction: 'asc' } })
    expect(fileColumn()).toEqual(['a.flac', 'b.flac', 'c.flac'])
    await view.rerender({ sort: { column: 'state', direction: 'desc' } })
    expect(fileColumn()).toEqual(['c.flac', 'a.flac', 'b.flac'])
  })
})

describe('ResultsTable: no matches (@RF-08)', () => {
  it('no matches: keeps the header and shows the message and the Clear filters button', () => {
    setup([], { noMatches: true })
    expect(screen.getAllByRole('columnheader')).toHaveLength(10)
    expect(dataRows()).toHaveLength(0)
    expect(screen.getByText(NO_MATCHES)).not.toBeNull()
    expect(screen.getByRole('button', { name: 'Clear filters' })).not.toBeNull()
    expect(screen.getByRole('grid').contains(screen.getByText(NO_MATCHES))).toBe(true)
  })

  it('emits clearFilters when Clear filters is clicked', async () => {
    const { onclearfilters, onselect } = setup([], { noMatches: true })
    await userEvent.click(screen.getByRole('button', { name: 'Clear filters' }))
    expect(onclearfilters).toHaveBeenCalledTimes(1)
    expect(onselect).not.toHaveBeenCalled()
  })

  it('Clear filters is reachable with Tab', async () => {
    setup([], { noMatches: true })
    const user = userEvent.setup()
    const target = screen.getByRole('button', { name: 'Clear filters' })
    for (let i = 0; i < 20 && document.activeElement !== target; i++) await user.tab()
    expect(document.activeElement).toBe(target)
  })

  it('no message without `noMatches` (the table is just empty)', () => {
    setup([])
    expect(screen.queryByText(NO_MATCHES)).toBeNull()
    expect(screen.queryByRole('button', { name: 'Clear filters' })).toBeNull()
  })

  it('no message while there are rows to show', () => {
    setup([row(0, 'C:\\L\\A\\a.flac')], { noMatches: true })
    expect(screen.queryByText(NO_MATCHES)).toBeNull()
    expect(dataRows()).toHaveLength(1)
  })

  it('the message goes away when rows come back', async () => {
    const view = setup([], { noMatches: true })
    await view.rerender({ rows: [row(0, 'C:\\L\\A\\a.flac')], noMatches: false })
    expect(screen.queryByText(NO_MATCHES)).toBeNull()
    expect(fileColumn()).toEqual(['a.flac'])
  })
})

describe('ResultsTable: selected row not in `rows` (@RF-08)', () => {
  const shown = [row(1, 'C:\\L\\A\\b.flac', { verdict: 'fake' }), row(2, 'C:\\L\\A\\c.flac', { verdict: 'fake' })]
  const hidden = 0 // a.flac, filtered out by the page

  it('highlights nothing', () => {
    setup(shown, { selectedIndex: hidden })
    expect(selectedRows()).toHaveLength(0)
  })

  it('↓ selects the first shown row', async () => {
    const { onselect } = setup(shown, { selectedIndex: hidden })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenCalledWith(1)
  })

  it('Home selects the first shown row and End the last', async () => {
    const { onselect } = setup(shown, { selectedIndex: hidden })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'Home' })
    expect(onselect).toHaveBeenLastCalledWith(1)
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'End' })
    expect(onselect).toHaveBeenLastCalledWith(2)
  })

  it('↑ does nothing', async () => {
    const { onselect } = setup(shown, { selectedIndex: hidden })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowUp' })
    expect(onselect).not.toHaveBeenCalled()
  })

  it('↓ follows the display order of the shown rows', async () => {
    const { onselect } = setup(shown, { selectedIndex: hidden, sort: { column: 'file', direction: 'desc' } })
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenCalledWith(2)
  })

  it('highlights the row again when it is shown again', async () => {
    const view = setup(shown, { selectedIndex: hidden })
    await view.rerender({ rows: [row(0, 'C:\\L\\A\\a.flac'), ...shown] })
    expect(selectedRows().map((r) => cells(r)[1].textContent?.trim())).toEqual(['a.flac'])
  })

  it('the keys do nothing with no shown row', async () => {
    const { onselect } = setup([], { selectedIndex: hidden, noMatches: true })
    const grid = screen.getByRole('grid')
    for (const key of ['ArrowDown', 'ArrowUp', 'Home', 'End']) await fireEvent.keyDown(grid, { key })
    expect(onselect).not.toHaveBeenCalled()
  })
})

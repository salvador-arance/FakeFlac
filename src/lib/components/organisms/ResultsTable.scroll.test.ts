// ResultsTable: the selected row is scrolled into view whatever changed the selection
// (@RF-06, proposed with RF-10: components.md "ResultsTable", scenario "Moving the selection in
// the table keeps the selected row visible"; ui/album-summary.md acceptance A5).
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The scroll container is the role="grid" element; its `scrollTop` is the observable. With
//   `rowHeight` 22 and `viewportHeight` 220 (10 rows), the row at display position p is visible
//   when `scrollTop <= p * rowHeight` and `scrollTop + viewportHeight >= (p + 1) * rowHeight`
//   (the header is sticky inside the grid, so it does not change the bounds), and it is rendered.
// - Scrolling is the least needed: a row already visible does not move the scroll.
import { fireEvent, render, screen } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import type { SortState } from '../../model'
import { makeManyRows } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

const ROW = 22
const VIEWPORT = 220
const byPath: SortState = { column: 'file', direction: 'asc' }
const grid = () => screen.getByRole('grid')

/** Display position `p` is within the visible area and its row is rendered. */
function expectVisible(p: number, selectedIndex: number) {
  const top = grid().scrollTop
  expect(top, `row ${p} above the visible area`).toBeLessThanOrEqual(p * ROW)
  expect(top + VIEWPORT, `row ${p} below the visible area`).toBeGreaterThanOrEqual((p + 1) * ROW)
  const selected = screen.getAllByRole('row').filter((r) => r.getAttribute('aria-selected') === 'true')
  expect(selected).toHaveLength(1)
  expect(selected[0].textContent).toContain(`file-${String(selectedIndex).padStart(4, '0')}.flac`)
}

/** A table whose `selectedIndex` follows the `onselect` events, as the page does. */
function setup(count: number, selectedIndex: number | null, sort: SortState = byPath) {
  const rows = makeManyRows(count)
  const view = render(ResultsTable, {
    props: {
      rows,
      selectedIndex,
      sort,
      rowHeight: ROW,
      viewportHeight: VIEWPORT,
      onselect: (index: number) => {
        void view.rerender({ selectedIndex: index })
      },
    },
  })
  return view
}

describe('ResultsTable: selected row scrolled into view', () => {
  // @RF-06 @RF-10 from a prop change (the page selects a row, A5)
  it('scrolls down to a row selected from outside', async () => {
    const view = setup(500, 0)
    await view.rerender({ selectedIndex: 400 })
    await vi.waitFor(() => expectVisible(400, 400))
  })

  it('scrolls back up to a row selected from outside', async () => {
    const view = setup(500, 0)
    await view.rerender({ selectedIndex: 400 })
    await vi.waitFor(() => expectVisible(400, 400))
    await view.rerender({ selectedIndex: 3 })
    await vi.waitFor(() => expectVisible(3, 3))
  })

  it('does not move the scroll when the row is already visible', async () => {
    const view = setup(500, 0)
    await view.rerender({ selectedIndex: 4 })
    await vi.waitFor(() => expectVisible(4, 4))
    expect(grid().scrollTop).toBe(0)
  })

  it('follows the display order when the table is sorted descending', async () => {
    const view = setup(500, null, { column: 'file', direction: 'desc' })
    // index 100 is at display position 399 in descending order
    await view.rerender({ selectedIndex: 100 })
    await vi.waitFor(() => expectVisible(399, 100))
  })

  // @RF-06 Scenario: Moving the selection in the table keeps the selected row visible
  it('End and Home keep the selected row visible', async () => {
    setup(500, 0)
    await fireEvent.keyDown(grid(), { key: 'End' })
    await vi.waitFor(() => expectVisible(499, 499))
    await fireEvent.keyDown(grid(), { key: 'Home' })
    await vi.waitFor(() => expectVisible(0, 0))
  })

  it('ArrowDown keeps the selected row visible after every key press', async () => {
    setup(500, 0)
    for (let p = 1; p <= 25; p++) {
      await fireEvent.keyDown(grid(), { key: 'ArrowDown' })
      await vi.waitFor(() => expectVisible(p, p))
    }
  })

  it('ArrowUp keeps the selected row visible after every key press', async () => {
    const view = setup(500, 0)
    await view.rerender({ selectedIndex: 40 })
    await vi.waitFor(() => expectVisible(40, 40))
    for (let p = 39; p >= 15; p--) {
      await fireEvent.keyDown(grid(), { key: 'ArrowUp' })
      await vi.waitFor(() => expectVisible(p, p))
    }
  })
})

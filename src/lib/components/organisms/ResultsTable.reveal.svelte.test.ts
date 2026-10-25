// ResultsTable: the reveal request (@RF-10; specs/ui/components.md "ResultsTable", decided at G1
// Q16; specs/ui/album-summary.md "Components" ResultsTable row and acceptance A5; scenario
// "Selecting the current album again brings its first track back into view" of
// features/album-summary-app.feature).
//
// components.md (Q16): `revealRequest?: number` (default 0) is a counter the page increments on
// every album selection; every time its value changes, the table scrolls the least needed so the
// row of `selectedIndex` is visible, even when `selectedIndex` did not change; nothing happens
// when the row is already visible, `selectedIndex` is null or the row is not in `rows`; the initial
// mount does not count as a change of `revealRequest`. Only a change of `selectedIndex` or of
// `revealRequest` scrolls the table: new rows arriving and a re-sort never do (Q13).
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - Same observable as ResultsTable.scroll.test.ts: the role="grid" element is the scroll
//   container and its `scrollTop` is read. With `rowHeight` 22 and `viewportHeight` 220 (10 rows),
//   display position p is visible when `scrollTop <= p * 22` and `scrollTop + 220 >= (p + 1) * 22`.
//   jsdom has no layout (the sticky header measures 0), so "the least needed" is exact: a row above
//   the visible area ends at the top edge (`scrollTop = p * 22`), a row below it at the bottom edge
//   (`scrollTop = (p + 1) * 22 - 220`).
// - "Scrolled away" is the user's scroll: `scrollTop` set on the grid plus a `scroll` event.
// - Two ways a parent passes the props, both covered:
//   * `setup` passes one getter per prop backed by its own `$state` (this file is a `.svelte.test.ts`
//     module so it can use runes), as the page's template does: changing one prop changes only it.
//   * `view.rerender(...)` of @testing-library/svelte replaces the whole props object, so every
//     prop is provided again with the same value except the ones given. A prop provided again with
//     the same value is not a change: such a rerender that only brings new rows, a new sort or the
//     same `revealRequest` must not scroll.
// - The initial mount: mounting with a non-zero `revealRequest` gives the same `scrollTop` as
//   mounting with the default (only the `selectedIndex` rule of Q13 acts on mount).
import { fireEvent, render, screen } from '@testing-library/svelte'
import { flushSync, tick } from 'svelte'
import { describe, expect, it, vi } from 'vitest'
import type { ResultRow, SortState } from '../../model'
import { makeManyRows } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

const ROW = 22
const VIEWPORT = 220
const byPath: SortState = { column: 'file', direction: 'asc' }
const byPathDesc: SortState = { column: 'file', direction: 'desc' }
const grid = () => screen.getByRole('grid')

/** Display position `p` is within the visible area and its row is the one selected row. */
function expectVisible(p: number, selectedIndex: number) {
  const top = grid().scrollTop
  expect(top, `row ${p} above the visible area`).toBeLessThanOrEqual(p * ROW)
  expect(top + VIEWPORT, `row ${p} below the visible area`).toBeGreaterThanOrEqual((p + 1) * ROW)
  const selected = screen
    .getAllByRole('row')
    .filter((r) => r.getAttribute('aria-selected') === 'true')
  expect(selected).toHaveLength(1)
  expect(selected[0].textContent).toContain(`file-${String(selectedIndex).padStart(4, '0')}.flac`)
}

/** The user scrolls the table to `top`. */
async function scrollTo(top: number) {
  grid().scrollTop = top
  await fireEvent.scroll(grid())
  expect(grid().scrollTop).toBe(top)
}

/** Lets effects and a possible scroll run before reading a `scrollTop` that must not change. */
const flush = () => new Promise((r) => setTimeout(r, 20))

type Options = { count?: number; selectedIndex?: number | null; sort?: SortState; reveal?: number }
type Changes = {
  rows?: ResultRow[]
  selectedIndex?: number | null
  sort?: SortState
  revealRequest?: number
}

/** Mounts the table with one reactive getter per prop, as a parent's template passes them. */
function setup({ count = 500, selectedIndex = null, sort = byPath, reveal }: Options = {}) {
  let rows = $state.raw(makeManyRows(count))
  let selected = $state.raw(selectedIndex)
  let order = $state.raw(sort)
  let request = $state.raw(reveal)
  const view = render(ResultsTable, {
    props: {
      get rows() {
        return rows
      },
      get selectedIndex() {
        return selected
      },
      get sort() {
        return order
      },
      get revealRequest() {
        return request
      },
      rowHeight: ROW,
      viewportHeight: VIEWPORT,
    },
  })
  /** Changes only the given props, then flushes. */
  async function set(changes: Changes) {
    flushSync(() => {
      if (changes.rows !== undefined) rows = changes.rows
      if ('selectedIndex' in changes) selected = changes.selectedIndex ?? null
      if (changes.sort !== undefined) order = changes.sort
      if (changes.revealRequest !== undefined) request = changes.revealRequest
    })
    await tick()
  }
  return { ...view, set }
}

/** Mounts the table with a plain props object, for `view.rerender` (whole props replaced). */
function setupPlain({ count = 500, selectedIndex = null, sort = byPath, reveal }: Options = {}) {
  return render(ResultsTable, {
    props: {
      rows: makeManyRows(count),
      selectedIndex,
      sort,
      rowHeight: ROW,
      viewportHeight: VIEWPORT,
      ...(reveal === undefined ? {} : { revealRequest: reveal }),
    },
  })
}

describe('ResultsTable: a change of revealRequest with the selection unchanged', () => {
  // @RF-10 Scenario: Selecting the current album again brings its first track back into view
  it('scrolls back up to the selected row when the table was scrolled below it', async () => {
    const view = setup({ selectedIndex: 0 })
    await scrollTo(300 * ROW)
    await view.set({ revealRequest: 1 })
    await vi.waitFor(() => expectVisible(0, 0))
    expect(grid().scrollTop).toBe(0)
  })

  it('scrolls the least needed: a row above the visible area ends at the top edge', async () => {
    const view = setup({ selectedIndex: 250 })
    await scrollTo(400 * ROW)
    await view.set({ revealRequest: 1 })
    await vi.waitFor(() => expectVisible(250, 250))
    expect(grid().scrollTop).toBe(250 * ROW)
  })

  it('scrolls the least needed: a row below the visible area ends at the bottom edge', async () => {
    const view = setup({ selectedIndex: 250 })
    await scrollTo(0)
    await view.set({ revealRequest: 1 })
    await vi.waitFor(() => expectVisible(250, 250))
    expect(grid().scrollTop).toBe(251 * ROW - VIEWPORT)
  })

  it('reveals again on every increment of the counter', async () => {
    const view = setup({ selectedIndex: 0 })
    for (let request = 1; request <= 3; request++) {
      await scrollTo(200 * ROW)
      await view.set({ revealRequest: request })
      await vi.waitFor(() => expectVisible(0, 0))
    }
  })

  it('uses the display order at the time of the request (after a re-sort)', async () => {
    const view = setup({ selectedIndex: 0 })
    await view.set({ sort: byPathDesc })
    // index 0 is at display position 499 in descending order
    await view.set({ revealRequest: 1 })
    await vi.waitFor(() => expectVisible(499, 0))
    expect(grid().scrollTop).toBe(500 * ROW - VIEWPORT)
  })

  it('a first change after mounting with a non-zero value scrolls too', async () => {
    const view = setup({ selectedIndex: 0, reveal: 7 })
    await scrollTo(300 * ROW)
    await view.set({ revealRequest: 8 })
    await vi.waitFor(() => expectVisible(0, 0))
  })

  it('a change given with the whole props object scrolls too', async () => {
    const view = setupPlain({ selectedIndex: 0 })
    await scrollTo(300 * ROW)
    await view.rerender({ revealRequest: 1 })
    await vi.waitFor(() => expectVisible(0, 0))
    // and the next increment, after the user scrolled away again
    await scrollTo(300 * ROW)
    await view.rerender({ revealRequest: 2 })
    await vi.waitFor(() => expectVisible(0, 0))
  })
})

describe('ResultsTable: a change of revealRequest that does nothing', () => {
  // @RF-10 components.md (Q16): "nothing happens when the row is already visible"
  it('does not move the scroll when the selected row is already visible', async () => {
    const view = setup({ selectedIndex: 3 })
    await scrollTo(1 * ROW)
    await view.set({ revealRequest: 1 })
    await flush()
    expect(grid().scrollTop).toBe(1 * ROW)
  })

  // @RF-10 components.md (Q16): "... `selectedIndex` is null ..."
  it('does not move the scroll when no row is selected', async () => {
    const view = setup({ selectedIndex: null })
    await scrollTo(300 * ROW)
    await view.set({ revealRequest: 1 })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })

  // @RF-10 components.md (Q16): "... or the row is not in `rows`"
  it('does not move the scroll when the selected row is not in rows', async () => {
    const view = setup({ count: 500, selectedIndex: 9999 })
    await scrollTo(300 * ROW)
    await view.set({ revealRequest: 1 })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })
})

describe('ResultsTable: the initial mount is not a change of revealRequest', () => {
  // @RF-10 components.md (Q16): "the initial mount does not count as a change of revealRequest"
  it('mounting with a non-zero value scrolls exactly as mounting with the default', async () => {
    const plain = setup({ selectedIndex: 400 })
    await flush()
    const expected = grid().scrollTop
    // the selectedIndex rule (Q13) already brings the row into view on mount
    expect(expected).toBe(401 * ROW - VIEWPORT)
    plain.unmount()

    setup({ selectedIndex: 400, reveal: 5 })
    await flush()
    expect(grid().scrollTop).toBe(expected)
  })

  it('mounting with a non-zero value and no selection does not scroll', async () => {
    setup({ selectedIndex: null, reveal: 5 })
    await flush()
    expect(grid().scrollTop).toBe(0)
  })

  it('the same value provided again is not a change', async () => {
    const view = setupPlain({ selectedIndex: 0, reveal: 5 })
    await scrollTo(300 * ROW)
    await view.rerender({ revealRequest: 5 })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })
})

describe('ResultsTable: new rows and a re-sort never scroll (Q13, Q16)', () => {
  // @RF-10 components.md (Q16): "Only a change of selectedIndex or of revealRequest scrolls the
  // table: new rows arriving and a re-sort never do"
  it('rows arriving after a reveal leave scrollTop unchanged', async () => {
    const view = setup({ count: 500, selectedIndex: 0 })
    await view.set({ revealRequest: 1 })
    await scrollTo(300 * ROW)
    await view.set({ rows: makeManyRows(600) })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })

  it('a re-sort after a reveal leaves scrollTop unchanged', async () => {
    const view = setup({ selectedIndex: 0 })
    await view.set({ revealRequest: 1 })
    await scrollTo(300 * ROW)
    await view.set({ sort: byPathDesc })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })

  it('the selected row arriving after a request that found no row does not scroll', async () => {
    const view = setup({ count: 100, selectedIndex: 450 })
    await view.set({ revealRequest: 1 })
    await flush()
    expect(grid().scrollTop).toBe(0)
    await view.set({ rows: makeManyRows(500) })
    await flush()
    expect(grid().scrollTop).toBe(0)
  })

  it('rows given with the whole props object leave scrollTop unchanged', async () => {
    const view = setupPlain({ count: 500, selectedIndex: 0 })
    await scrollTo(300 * ROW)
    await view.rerender({ rows: makeManyRows(600) })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })

  it('a sort given with the whole props object leaves scrollTop unchanged', async () => {
    const view = setupPlain({ selectedIndex: 0 })
    await scrollTo(300 * ROW)
    await view.rerender({ sort: byPathDesc })
    await flush()
    expect(grid().scrollTop).toBe(300 * ROW)
  })

  it('the selected row arriving with the whole props object does not scroll', async () => {
    const view = setupPlain({ count: 100, selectedIndex: 450, reveal: 1 })
    await view.rerender({ rows: makeManyRows(500) })
    await flush()
    expect(grid().scrollTop).toBe(0)
  })
})

// AnalysisPage: ten thousand rows (features/sort-filter.feature, scenario "Ten thousand rows";
// @RF-08; specs/ui/sort-filter.md acceptance A1/A2 at size). Only the result is checked; timing is
// the manual check P1. Same harness as the other sort-filter page tests.
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - The table virtualises (the grid reports a 220 px viewport); its whole row order is read by
//   scrolling it; "the first row rendered" is the first data row at scroll position 0.
// - "After End in the table": End is pressed on the focused grid; the selected row is the rendered
//   row with aria-selected="true" (the table scrolls it into view).
import { fireEvent } from '@testing-library/svelte'
import type { MockInstance } from 'vitest'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Verdict } from '../model'
import {
  dataRows,
  finishedAnalysis,
  grid,
  installViewport,
  nameOf,
  pressVerdict,
  readAllRows,
  resetApiMocks,
  scrollGridTo,
  selectedRows,
  sortBy,
  statusText,
  typeSearch,
} from './tests/sortFilterHarness'
import type { RowSpec } from './tests/sortFilterHarness'

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

let viewport: MockInstance<(this: Element) => number>

beforeEach(() => {
  resetApiMocks()
  viewport = installViewport()
})

afterEach(() => {
  viewport.mockRestore()
})

const VERDICT_OF: Verdict[] = ['real', 'suspicious', 'fake', 'error']
const pad = (n: number, width: number) => String(n).padStart(width, '0')
const nameAt = (i: number) => `Lib\\Album ${pad(Math.floor(i / 100), 2)}\\${pad(i, 4)} – Track.flac`

describe('Size (@RF-08)', () => {
  // @RF-08 Scenario: Ten thousand rows
  it('sorts and filters ten thousand rows', async () => {
    const rows: RowSpec[] = Array.from({ length: 10000 }, (_, i) => ({
      name: nameAt(i),
      index: i,
      verdict: VERDICT_OF[i % 4],
      durationMs: (i + 1) * 1000,
    }))
    await finishedAnalysis(rows)
    expect(statusText()).toMatch(/^10000 tracks/)

    await sortBy('State', 'ascending')
    await pressVerdict('suspicious')
    await typeSearch('00')
    await vi.waitFor(() => expect(statusText().endsWith('· 52 shown')).toBe(true), { timeout: 10000 })

    const expected = [
      ...Array.from({ length: 25 }, (_, k) => nameAt(1 + 4 * k)),
      ...[1, 2, 3, 4, 5, 6, 7, 8, 9].flatMap((d) => [1, 5, 9].map((y) => nameAt(d * 1000 + y))),
    ]
    const { order } = await readAllRows()
    expect(order).toEqual(expected)

    await scrollGridTo(0)
    expect(nameOf(dataRows()[0])).toBe('Lib\\Album 00\\0001 – Track.flac')
    grid().focus()
    await fireEvent.keyDown(grid(), { key: 'End' })
    await vi.waitFor(() => expect(selectedRows().map(nameOf)).toEqual(['Lib\\Album 90\\9009 – Track.flac']))
  }, 120000)
})

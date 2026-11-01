// AnalysisPage: verdict filter, name search, no match, empty state and runs
// (features/sort-filter.feature, "Verdict filter", "Name search" and "No match, empty state, runs";
// @RF-08; specs/ui/sort-filter.md sections 4, 8 and 9, acceptance A2). The API client is mocked and
// the progress events are stubbed; steps, row specs and locators are those of
// ./tests/sortFilterHarness.ts (see its header for the interface of the feature).
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - "Hovering corrupt.flac shows the tooltip Corrupt stream": the row (or one of its cells) has a
//   `title` containing the text, as in ResultsTable.test.ts; jsdom shows no tooltip.
// - "Each button shows the icon of its verdict in the State column": the button's SVG drawing equals
//   the one VerdictCell draws for that verdict.
// - Typing: plain ASCII is typed key by key; text with other characters (or leading or trailing
//   spaces) is pasted, which is one change of the field's text with the exact code points.
// - "The selection did not change" (Clear filters): the scenario has no selection step, so a row is
//   clicked before the filter is applied to make the check meaningful.
// - "The table is empty without the no-match message" while scanning: the grid is rendered with no
//   data row and the no-match text is absent.
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import VerdictCell from '../components/molecules/VerdictCell.svelte'
import type { Verdict } from '../model'
import {
  BUTTON_ORDER,
  Feeder,
  HINT,
  LABEL,
  NO_MATCHES,
  cells,
  clearSearchButton,
  dataRows,
  fakeRun,
  filterGroup,
  finishRun,
  finishedAnalysis,
  grid,
  listed,
  mountPage,
  nTracks,
  noMatchMessage,
  openFolder,
  pressCtrlF,
  pressVerdict,
  pressedStates,
  queryGrid,
  replaceSearch,
  resetApiMocks,
  rowByFile,
  searchBox,
  selectedRows,
  settle,
  shownFiles,
  shownNames,
  sortBy,
  startRun,
  statusText,
  summaryOf,
  typeSearch,
  verdictButton,
  verdictButtons,
  fileOf,
} from './tests/sortFilterHarness'

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

const files = (from: number, to: number) =>
  Array.from({ length: to - from + 1 }, (_, k) => `${String(from + k).padStart(2, '0')}.flac`)
const ALL_31 = files(1, 31)
const FINISHED_31 = '31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors'

describe('Verdict filter (@RF-08)', () => {
  // @RF-08 Scenario: One verdict button shows only that verdict
  it('one verdict button shows only that verdict', async () => {
    await finishedAnalysis(nTracks({ real: 28, suspicious: 2, fake: 1 }))
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['31.flac']))
    expect(pressedStates()).toEqual(['true', 'false', 'false', 'false'])
    expect(statusText()).toBe(`${FINISHED_31} · 1 shown`)
  })

  // @RF-08 Scenario: Several verdict buttons show the union
  it('several verdict buttons show the union', async () => {
    await finishedAnalysis(nTracks({ real: 28, suspicious: 2, fake: 1 }))
    await pressVerdict('fake')
    await pressVerdict('suspicious')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['29.flac', '30.flac', '31.flac']))
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['29.flac', '30.flac']))
  })

  // @RF-08 Scenario: No verdict button pressed shows every verdict
  it('no verdict button pressed shows every verdict', async () => {
    await finishedAnalysis(nTracks({ real: 28, suspicious: 2, fake: 1 }))
    await pressVerdict('suspicious')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['29.flac', '30.flac']))
    await pressVerdict('suspicious')
    await vi.waitFor(() => expect(shownFiles()).toEqual(ALL_31))
    expect(pressedStates()).toEqual(['false', 'false', 'false', 'false'])
    expect(statusText()).toBe(FINISHED_31)
  })

  // @RF-08 Scenario: Every verdict button pressed shows every track
  it('every verdict button pressed shows every track', async () => {
    await finishedAnalysis(nTracks({ real: 28, suspicious: 2, fake: 1 }))
    for (const v of BUTTON_ORDER) await pressVerdict(v)
    await vi.waitFor(() => expect(pressedStates()).toEqual(['true', 'true', 'true', 'true']))
    expect(shownFiles()).toEqual(ALL_31)
    expect(statusText().endsWith('· 31 shown')).toBe(true)
  })

  // @RF-08 @RNF-04 Scenario: The Error button shows the files that could not be analysed
  it('the Error button shows the files that could not be analysed', async () => {
    await finishedAnalysis([
      { name: 'A\\ok.flac', verdict: 'real' },
      { name: 'A\\corrupt.flac', verdict: 'error', error: 'corruptStream' },
      { name: 'A\\short.flac', verdict: 'error', error: 'tooShort' },
      { name: 'A\\fake.flac', verdict: 'fake' },
    ])
    await pressVerdict('error')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['corrupt.flac', 'short.flac']))
    const corrupt = rowByFile('corrupt.flac') as HTMLElement
    const selector = '[title*="Corrupt stream"]'
    expect(corrupt.matches(selector) || corrupt.querySelector(selector) !== null).toBe(true)
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['corrupt.flac', 'fake.flac', 'short.flac']))
  })

  // @RF-08 @RNF-08 Scenario: The verdict buttons do not rely on colour alone
  it('the verdict buttons do not rely on colour alone', async () => {
    await finishedAnalysis(nTracks({ real: 3 }))
    const group = filterGroup()
    const buttons = within(group).getAllByRole('button')
    expect(buttons).toHaveLength(4)
    BUTTON_ORDER.forEach((v, i) => {
      expect(buttons[i]).toBe(screen.getByRole('button', { name: LABEL[v] }))
      expect(buttons[i].getAttribute('title')).toBe(LABEL[v])
      expect(buttons[i].hasAttribute('aria-pressed')).toBe(true)
    })
    // the icon of each verdict in the State column (VerdictCell)
    const drawings = new Map<Verdict, string>()
    for (const v of BUTTON_ORDER) {
      const { container, unmount } = render(VerdictCell, { props: { verdict: v } })
      drawings.set(v, container.querySelector('svg')?.innerHTML ?? '')
      unmount()
    }
    BUTTON_ORDER.forEach((v) => {
      const svg = verdictButton(v).querySelector('svg')
      expect(svg?.innerHTML, LABEL[v]).toBe(drawings.get(v))
    })
  })
})

describe('Name search (@RF-08)', () => {
  // @RF-08 Scenario: Search by part of the file name, case-insensitive
  it('searches by part of the file name, case-insensitive', async () => {
    await finishedAnalysis([
      { name: 'Artist\\Album 1\\01 \u2013 Intro.flac' },
      { name: 'Artist\\Album 1\\02 \u2013 Outro.flac' },
      { name: 'Artist\\Album 2\\01 \u2013 Intro.flac' },
    ])
    await typeSearch('INTRO')
    await vi.waitFor(() =>
      expect(shownNames()).toEqual(['Artist\\Album 1\\01 \u2013 Intro.flac', 'Artist\\Album 2\\01 \u2013 Intro.flac']),
    )
    expect(statusText().endsWith('· 2 shown')).toBe(true)
  })

  // @RF-08 Scenario: Search by the album folder, but not by where the analysed folder is
  it('searches by the album folder, but not by where the analysed folder is', async () => {
    await finishedAnalysis(
      [
        { name: 'Artist\\Album 1\\Intro.flac' },
        { name: 'Artist\\Album 10\\Intro.flac' },
        { name: 'Artist\\Album 2\\Intro.flac' },
      ],
      'C:\\Music\\Artist',
    )
    await typeSearch('album 1')
    await vi.waitFor(() =>
      expect(shownNames()).toEqual(['Artist\\Album 1\\Intro.flac', 'Artist\\Album 10\\Intro.flac']),
    )
    await replaceSearch('music')
    await vi.waitFor(() => expect(dataRows()).toHaveLength(0))
    expect(noMatchMessage()).not.toBeNull()
  })

  // @RF-08 Scenario: Every word of the search must appear, in any order
  it('requires every word of the search, in any order', async () => {
    await finishedAnalysis([
      { name: 'The Beatles\\Help!\\01 \u2013 Help!.flac' },
      { name: 'The Beatles\\Abbey Road\\01 \u2013 Come Together.flac' },
    ])
    await typeSearch('help beatles')
    await vi.waitFor(() => expect(shownNames()).toEqual(['The Beatles\\Help!\\01 \u2013 Help!.flac']))
    await replaceSearch('beatles/abbey')
    await vi.waitFor(() =>
      expect(shownNames()).toEqual(['The Beatles\\Abbey Road\\01 \u2013 Come Together.flac']),
    )
  })

  // @RF-08 @RNF-09 Scenario Outline: Search ignores case and accents, not other symbols
  it.each([
    ['cancion', 1],
    ['CANCI\u00d3N', 1],
    ['nandu', 2],
    ['\u00f1and\u00fa', 2],
    ['\u00c7A VA', 1],
    ['ca va', 1],
    ['\u{1F3B5}', 1],
    ['\u2033\u00f1and\u00fa\u2033', 2],
    ['"nandu"', 0],
    ['\u2018x\u2019', 1],
    ["'x'", 0],
  ])('query %j shows %i rows', async (query, count) => {
    await finishedAnalysis([
      { name: '\u00c1lbum \u2033\u00d1and\u00fa\u2033\\01 \u2013 Canci\u00f3n \u2018x\u2019.flac' },
      { name: '\u00c1lbum \u2033\u00d1and\u00fa\u2033\\Disco 2 \u00abÇa va\u00bb \u{1F3B5}\\01.flac' },
    ])
    expect(dataRows()).toHaveLength(2)
    await typeSearch(query)
    await vi.waitFor(() => expect(searchBox().value).toBe(query))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(count))
    await settle()
    expect(dataRows()).toHaveLength(count)
  })

  // @RF-08 @RNF-09 Scenario: The search field keeps the text as typed
  it('keeps the text as typed', async () => {
    await finishedAnalysis(nTracks({ real: 3 }))
    const text = '  \u00c1lbum \u2033\u00d1and\u00fa\u2033 '
    await typeSearch(text)
    await vi.waitFor(() => expect(searchBox().value).toBe(text))
    expect([...searchBox().value].map((c) => c.codePointAt(0))).toEqual([...text].map((c) => c.codePointAt(0)))
  })

  // @RF-08 Scenario: The verdict filter and the search combine
  it('combines the verdict filter and the search', async () => {
    await finishedAnalysis([
      { name: 'Lib\\A\\01.flac', verdict: 'fake' },
      { name: 'Lib\\A\\02.flac', verdict: 'real' },
      { name: 'Lib\\B\\01.flac', verdict: 'fake' },
    ])
    await pressVerdict('fake')
    await typeSearch('lib\\a')
    await vi.waitFor(() => expect(shownNames()).toEqual(['Lib\\A\\01.flac']))
  })

  // @RF-08 Scenario: Escape and the clear button empty the search field
  it('empties the search field with Escape and with the clear button', async () => {
    await finishedAnalysis(nTracks({ real: 31 }))
    await typeSearch('intro')
    await vi.waitFor(() => expect(dataRows()).toHaveLength(0))
    expect(document.activeElement).toBe(searchBox())
    await userEvent.keyboard('{Escape}')
    await vi.waitFor(() => expect(searchBox().value).toBe(''))
    expect(document.activeElement).toBe(searchBox())
    await vi.waitFor(() => expect(shownFiles()).toEqual(ALL_31))

    await typeSearch('intro')
    await vi.waitFor(() => expect(clearSearchButton()).not.toBeNull())
    await userEvent.click(clearSearchButton() as HTMLElement)
    await vi.waitFor(() => expect(searchBox().value).toBe(''))
    expect(document.activeElement).toBe(searchBox())
    expect(clearSearchButton()).toBeNull()
  })

  // @RF-08 Scenario: Ctrl+F focuses the search field
  it('focuses the search field with Ctrl+F and selects its text', async () => {
    await finishedAnalysis(nTracks({ real: 31 }))
    await typeSearch('intro')
    await vi.waitFor(() => expect(searchBox().value).toBe('intro'))
    grid().focus()
    expect(document.activeElement).toBe(grid())
    const prevented = await pressCtrlF(grid())
    await vi.waitFor(() => expect(document.activeElement).toBe(searchBox()))
    expect(searchBox().selectionStart).toBe(0)
    expect(searchBox().selectionEnd).toBe(5)
    expect(prevented).toBe(true)
  })
})

describe('No match, empty state, runs (@RF-08)', () => {
  // @RF-08 Scenario: No track matches the filters
  it('shows the no-match state and clears the filters with its button', async () => {
    await finishedAnalysis(nTracks({ real: 29, suspicious: 2 }))
    await sortBy('Duration', 'descending')
    await userEvent.click(cells(rowByFile('31.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['31.flac']))

    await pressVerdict('fake')
    await vi.waitFor(() => expect(dataRows()).toHaveLength(0))
    expect(screen.getAllByRole('columnheader')).toHaveLength(10)
    expect(grid().contains(screen.getByText(NO_MATCHES))).toBe(true)
    expect(statusText().endsWith('· 0 shown')).toBe(true)

    await userEvent.click(screen.getByRole('button', { name: 'Clear filters' }))
    await vi.waitFor(() => expect(shownFiles()).toEqual([...ALL_31].reverse()))
    expect(pressedStates()).toEqual(['false', 'false', 'false', 'false'])
    expect(searchBox().value).toBe('')
    await vi.waitFor(() => expect(document.activeElement).toBe(searchBox()))
    expect(selectedRows().map(fileOf)).toEqual(['31.flac'])
    expect(noMatchMessage()).toBeNull()
  })

  // @RF-08 Scenario: The filters are disabled in the empty state
  it('disables the filters in the empty state', async () => {
    await mountPage()
    expect(screen.getByText(HINT)).not.toBeNull()
    expect(verdictButtons().map((b) => b.disabled)).toEqual([true, true, true, true])
    expect(pressedStates()).toEqual(['false', 'false', 'false', 'false'])
    expect(searchBox().disabled).toBe(true)
    expect(searchBox().value).toBe('')
    expect(searchBox().placeholder).toBe('Search (Ctrl+F)')
    const prevented = await pressCtrlF(document.body)
    await settle()
    expect(document.activeElement).not.toBe(searchBox())
    expect(prevented).toBe(true)
  })

  // @RF-08 Scenario: A filter kept into the empty state is shown on the disabled controls
  it('shows a filter kept into the empty state on the disabled controls', async () => {
    await mountPage()
    const run = await startRun()
    run.scanned(10)
    await vi.waitFor(() => expect(verdictButton('fake').disabled).toBe(false))
    await pressVerdict('fake')
    await typeSearch('01')
    await vi.waitFor(() => expect(searchBox().value).toBe('01'))
    run.cancelled(summaryOf([], { total: 10, done: 0, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toBe('Cancelled · 0 of 10'))

    const rejected = fakeRun()
    await openFolder()
    rejected.fail('NOT_A_FOLDER')
    await vi.waitFor(() => expect(queryGrid()).toBeNull())
    expect(screen.getByText(HINT)).not.toBeNull()
    expect(verdictButtons().map((b) => b.disabled)).toEqual([true, true, true, true])
    expect(pressedStates()).toEqual(['true', 'false', 'false', 'false'])
    expect(searchBox().disabled).toBe(true)
    expect(searchBox().value).toBe('01')
    expect(clearSearchButton()).toBeNull()

    await startRun()
    await vi.waitFor(() => expect(verdictButton('fake').disabled).toBe(false))
    expect(verdictButton('fake').getAttribute('aria-pressed')).toBe('true')
    expect(searchBox().disabled).toBe(false)
    expect(searchBox().value).toBe('01')
  })

  // @RF-08 Scenario: Filters apply to the rows that arrive during a run
  it('applies the filters to the rows that arrive during a run', async () => {
    const all = listed([
      { name: 'A\\03.flac', verdict: 'fake' },
      { name: 'A\\01.flac', verdict: 'real' },
      { name: 'A\\02.flac', verdict: 'real' },
      { name: 'A\\04.flac', verdict: 'real' },
      { name: 'A\\00.flac', verdict: 'fake' },
    ])
    await mountPage()
    const run = await startRun()
    run.scanned(10)
    await vi.waitFor(() => expect(verdictButton('fake').disabled).toBe(false))
    await pressVerdict('fake')
    const feeder = new Feeder(run, all)
    feeder.emit(all.slice(0, 4))
    await settle()
    await vi.waitFor(() => expect(shownFiles()).toEqual(['03.flac']))
    expect(statusText().endsWith('· 1 shown')).toBe(true)
    feeder.emit(all.slice(4))
    await settle()
    await vi.waitFor(() => expect(shownFiles()).toEqual(['00.flac', '03.flac']))
    expect(statusText().endsWith('· 2 shown')).toBe(true)
  })

  // @RF-08 Scenario: While scanning with a filter, the table shows no no-match message
  it('shows no no-match message while scanning with a filter', async () => {
    await finishedAnalysis(nTracks({ real: 2, fake: 1 }))
    await pressVerdict('fake')
    await vi.waitFor(() => expect(shownFiles()).toEqual(['03.flac']))
    await startRun()
    await vi.waitFor(() => expect(statusText()).toBe('Scanning…'))
    expect(verdictButton('fake').getAttribute('aria-pressed')).toBe('true')
    expect(queryGrid()).not.toBeNull()
    expect(dataRows()).toHaveLength(0)
    expect(noMatchMessage()).toBeNull()
    expect(statusText()).not.toContain('shown')
  })

  // @RF-08 Scenario: The filters are kept for the next run
  it('keeps the filters for the next run', async () => {
    await finishedAnalysis(nTracks({ real: 30, fake: 1 }), 'C:\\Music\\Lib A')
    await pressVerdict('fake')
    await typeSearch('01')
    await vi.waitFor(() => expect(searchBox().value).toBe('01'))
    const run = await startRun('C:\\Music\\Lib B')
    await finishRun(
      run,
      [
        { name: 'Lib B\\01.flac', verdict: 'fake' },
        { name: 'Lib B\\10.flac', verdict: 'fake' },
        { name: 'Lib B\\01b.flac', verdict: 'real' },
      ],
      'C:\\Music\\Lib B',
    )
    expect(verdictButton('fake').getAttribute('aria-pressed')).toBe('true')
    expect(searchBox().value).toBe('01')
    await vi.waitFor(() => expect(shownNames()).toEqual(['Lib B\\01.flac']))
  })

  // @RF-08 Scenario: The shown count after a cancel
  it('shows the shown count after a cancel', async () => {
    const all = listed([
      { name: 'A\\01.flac', verdict: 'real' },
      { name: 'A\\02.flac', verdict: 'fake' },
      { name: 'A\\03.flac', verdict: 'real' },
    ])
    await mountPage()
    const run = await startRun()
    run.scanned(10)
    await vi.waitFor(() => expect(verdictButton('fake').disabled).toBe(false))
    await pressVerdict('fake')
    new Feeder(run, all).emit(all)
    await settle()
    run.cancelled(summaryOf(all, { total: 10, done: 3, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toBe('Cancelled · 3 of 10 · 1 shown'))
    expect(shownFiles()).toEqual(['02.flac'])
  })

  // @RF-08 Scenario: No shown count while the store has no row
  it('shows no shown count while the store has no row', async () => {
    await mountPage()
    const run = await startRun()
    run.scanned(10)
    await vi.waitFor(() => expect(verdictButton('fake').disabled).toBe(false))
    await pressVerdict('fake')
    await vi.waitFor(() => expect(verdictButton('fake').getAttribute('aria-pressed')).toBe('true'))
    run.cancelled(summaryOf([], { total: 10, done: 0, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^Cancelled/))
    await settle()
    expect(statusText()).toBe('Cancelled · 0 of 10')
  })

  // @RF-08 @RNF-04 Scenario: A run that fails unexpectedly keeps the filters
  it('keeps the filters when a run fails unexpectedly', async () => {
    const all = listed([
      { name: 'Album A\\01.flac', verdict: 'real' },
      { name: 'Album A\\02.flac', verdict: 'error', error: 'corruptStream' },
    ])
    await mountPage()
    const run = await startRun('C:\\Music\\Album A')
    run.scanned(2)
    await vi.waitFor(() => expect(verdictButton('error').disabled).toBe(false))
    await pressVerdict('error')
    new Feeder(run, all).emit(all)
    run.fail('INTERNAL')
    await vi.waitFor(() =>
      expect(statusText()).toBe('The analysis stopped because of an unexpected error'),
    )
    await vi.waitFor(() => expect(shownFiles()).toEqual(['02.flac']))
    expect(verdictButton('error').getAttribute('aria-pressed')).toBe('true')
    await settle()
    expect(statusText()).toBe('The analysis stopped because of an unexpected error')
  })
})

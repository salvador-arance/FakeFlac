// Shared steps and locators of the AnalysisPage tests of features/sort-filter.feature (@RF-08).
// Lives under `tests/` so it is a protected test file (scripts/test-paths.mjs).
//
// Each test file that uses it mocks `../api` with `vi.mock` (the factory must sit in the test file);
// the mocked module is the one this harness imports.
//
// Interface of the feature (its header comment), as implemented here:
// - A row spec is the "name" `<album name>\<file name>`; TrackInfo.path is `C:\Music\` + name, and
//   TrackInfo.folder is `C:\Music\` + the album name unless the spec gives `folder`.
// - One album per distinct folder part of the names of the run, `AlbumUpdate.index` in path order
//   (component by component, case-insensitive), `files` = its rows in the run, and each trackDone
//   carries the cumulative counts of its album in emission order.
// - Rows get their index in the order listed unless the spec gives one.
// - "The table shows" reads the File cells (or the names: the Folder cell without `C:\Music\`, a
//   `\`, and the File cell) of the rendered data rows, top to bottom. With a measurable viewport
//   (`installViewport`) the whole order is read by scrolling the grid (`readAllRows`).
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import type { MockInstance } from 'vitest'
import { expect, vi } from 'vitest'
import type { AnalysisSummary, ProgressEvent } from '../../api'
import * as api from '../../api'
import type { TrackAnalysis, TrackError, Verdict } from '../../model'
import { makeAlbum, makeSummary, makeTrack } from '../../testing/fixtures'
import AnalysisPage from '../AnalysisPage.svelte'
import { mockSettings, waitForMainScreen } from './settingsHarness'

export const MUSIC = 'C:\\Music\\'
export const ROW = 22
export const VIEWPORT = 220
export const HINT = 'Drop a folder here or click Open folder'
export const NO_MATCHES = 'No tracks match the filters'

export const LABEL: Record<Verdict, string> = {
  fake: 'Show fake tracks',
  suspicious: 'Show suspicious tracks',
  real: 'Show genuine tracks',
  error: 'Show tracks with errors',
}
export const BUTTON_ORDER: Verdict[] = ['fake', 'suspicious', 'real', 'error']

export type RowSpec = {
  name: string
  verdict?: Verdict
  index?: number
  cutoffHz?: number | null
  /** TrackInfo.folder; '' is the empty string. Default `C:\Music\` + the album name. */
  folder?: string
  error?: TrackError['kind']
  durationMs?: number
}

// ---- api mocks ---------------------------------------------------------------------------------

const unlisten = vi.fn()

export function resetApiMocks() {
  // RF-13: English default settings, locale reset (settingsHarness.ts)
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
}

/** The next `startAnalysis` call: its events and its outcome are driven by the test. */
export function fakeRun() {
  let onEvent: (event: ProgressEvent) => void = () => {}
  let resolve: (summary: AnalysisSummary | null) => void = () => {}
  let reject: (error: unknown) => void = () => {}
  const promise = new Promise<AnalysisSummary | null>((res, rej) => {
    resolve = res
    reject = rej
  })
  promise.catch(() => {})
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
    cancelled: (summary: AnalysisSummary) => {
      onEvent({ kind: 'cancelled', summary })
      resolve(summary)
    },
    fail: (code: string) => reject(new api.ApiError({ code, message: 'backend message' } as never)),
  }
}
export type Run = ReturnType<typeof fakeRun>

// ---- rows and events ---------------------------------------------------------------------------

export const albumOf = (name: string) => name.slice(0, Math.max(name.lastIndexOf('\\'), 0))
export const fileOfName = (name: string) => name.slice(name.lastIndexOf('\\') + 1)

/** Path order of the commands contract: component by component, case-insensitive. */
function pathOrder(a: string, b: string): number {
  const pa = a.toLowerCase().split('\\')
  const pb = b.toLowerCase().split('\\')
  for (let i = 0; i < Math.min(pa.length, pb.length); i++) {
    if (pa[i] !== pb[i]) return pa[i] < pb[i] ? -1 : 1
  }
  return pa.length - pb.length
}

function trackError(kind: TrackError['kind']): TrackError {
  return kind === 'tooShort' ? { kind } : ({ kind, detail: 'stub' } as TrackError)
}

export function trackOf(spec: RowSpec): TrackAnalysis {
  const verdict = spec.verdict ?? 'real'
  const track = makeTrack({
    path: MUSIC + spec.name,
    verdict,
    cutoffHz: spec.cutoffHz === undefined ? (verdict === 'error' ? null : 20000) : spec.cutoffHz,
    bitDepth: verdict === 'error' ? null : 16,
    durationMs: spec.durationMs,
    error: spec.error ? trackError(spec.error) : null,
  })
  track.info.folder = spec.folder ?? MUSIC + albumOf(spec.name)
  return track
}

/** Gives every spec its index: the position in the list unless it has one. */
export type Listed = RowSpec & { index: number }

export function listed(rows: RowSpec[]): Listed[] {
  return rows.map((r, i) => ({ ...r, index: r.index ?? i }))
}

/** Emits trackDone events for the rows of one run, with the album rule of the feature. */
export class Feeder {
  private readonly albums = new Map<string, { index: number; files: number; counted: Verdict[] }>()

  constructor(
    private readonly run: Run,
    all: RowSpec[],
  ) {
    const names = [...new Set(all.map((r) => albumOf(r.name)))].sort(pathOrder)
    names.forEach((name, index) => {
      const files = all.filter((r) => albumOf(r.name) === name).length
      this.albums.set(name, { index, files, counted: [] })
    })
  }

  emit(rows: RowSpec[]) {
    for (const spec of rows) {
      if (spec.index === undefined) throw new Error(`row ${spec.name} has no index`)
      const name = albumOf(spec.name)
      const album = this.albums.get(name)
      if (!album) throw new Error(`unknown album ${name}`)
      const verdict = spec.verdict ?? 'real'
      album.counted.push(verdict)
      const count = (v: Verdict) => album.counted.filter((x) => x === v).length
      this.run.emit({
        kind: 'trackDone',
        index: spec.index,
        analysis: trackOf(spec),
        album: makeAlbum({
          index: album.index,
          name,
          folder: MUSIC + name,
          files: album.files,
          real: count('real'),
          suspicious: count('suspicious'),
          fake: count('fake'),
          errors: count('error'),
        }),
      })
    }
  }
}

export function summaryOf(rows: RowSpec[], o: Partial<AnalysisSummary> = {}): AnalysisSummary {
  const count = (v: Verdict) => rows.filter((r) => (r.verdict ?? 'real') === v).length
  return makeSummary({
    root: 'C:\\Music\\Lib',
    total: rows.length,
    done: rows.length,
    real: count('real'),
    suspicious: count('suspicious'),
    fake: count('fake'),
    errors: count('error'),
    ...o,
  })
}

const pad2 = (n: number) => String(n).padStart(2, '0')

/**
 * "N tracks[: a real, b suspicious, c fake and d errors]": "<album>\01.flac" to "<album>\NN.flac",
 * verdicts in the order real, suspicious, fake, error, duration number x 1000 ms. `first` is the
 * index of "01.flac" (index = number - 1 for the first album of a run).
 */
export function nTracks(
  counts: { real?: number; suspicious?: number; fake?: number; errors?: number },
  album = 'Lib\\A',
  first = 0,
): (RowSpec & { index: number })[] {
  const verdicts: Verdict[] = [
    ...Array<Verdict>(counts.real ?? 0).fill('real'),
    ...Array<Verdict>(counts.suspicious ?? 0).fill('suspicious'),
    ...Array<Verdict>(counts.fake ?? 0).fill('fake'),
    ...Array<Verdict>(counts.errors ?? 0).fill('error'),
  ]
  return verdicts.map((verdict, i) => ({
    name: `${album}\\${pad2(i + 1)}.flac`,
    verdict,
    index: first + i,
    durationMs: (i + 1) * 1000,
  }))
}

// ---- page steps --------------------------------------------------------------------------------

export async function mountPage() {
  const view = render(AnalysisPage)
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled())
  await waitForMainScreen()
  return view
}

export const openButton = () => screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement

/** Clicks Open folder with `path` picked, and waits for the new startAnalysis call. */
export async function openFolder(path = 'C:\\Music\\Lib') {
  const before = vi.mocked(api.startAnalysis).mock.calls.length
  vi.mocked(api.pickFolder).mockResolvedValue(path)
  await userEvent.click(openButton())
  await vi.waitFor(() => expect(vi.mocked(api.startAnalysis).mock.calls.length).toBe(before + 1))
}

/** Starts a run of `root` (the page must be mounted): scanning, no event yet. */
export async function startRun(root = 'C:\\Music\\Lib') {
  const run = fakeRun()
  await openFolder(root)
  return run
}

/**
 * "an analysis [of root] finished with these rows": scanned, one trackDone per row (in the listed
 * order, or the order `reorder` gives), finished; waits for the final status text.
 */
export async function finishRun(
  run: Run,
  rows: RowSpec[],
  root = 'C:\\Music\\Lib',
  reorder: (all: Listed[]) => Listed[] = (all) => all,
) {
  const all = listed(rows)
  run.scanned(all.length)
  new Feeder(run, all).emit(reorder(all))
  run.finish(summaryOf(all, { root }))
  await vi.waitFor(() => expect(statusText()).toMatch(/^\d+ tracks?/))
  return all
}

/** Mounts the page and finishes a run with `rows`. */
export async function finishedAnalysis(
  rows: RowSpec[],
  root = 'C:\\Music\\Lib',
  reorder?: (all: Listed[]) => Listed[],
) {
  await mountPage()
  const run = await startRun(root)
  return finishRun(run, rows, root, reorder)
}

export const settle = () => new Promise((r) => setTimeout(r, 130))

// ---- locators ----------------------------------------------------------------------------------

export const norm = (s: string | null | undefined) => (s ?? '').replace(/\s+/g, ' ').trim()
export const statusText = () => norm(screen.getByRole('status').textContent)
export const grid = () => screen.getByRole('grid')
export const queryGrid = () => screen.queryByRole('grid')
export const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
export const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')
export const fileOf = (row: HTMLElement) => cells(row)[1].textContent?.trim() ?? ''
export const folderCellOf = (row: HTMLElement) => cells(row)[2].textContent?.trim() ?? ''
export const stateOf = (row: HTMLElement) => cells(row)[0].textContent?.trim() ?? ''
/**
 * The name of a row: its Folder cell without `C:\Music\`, a `\`, and its File cell. The Folder cell
 * of a row with an empty TrackInfo.folder shows the path's folder part with `/` separators (the
 * fallback of ResultsTable, unchanged by RF-08), so `/` is read as `\` here.
 */
export const nameOf = (row: HTMLElement) => {
  const folder = folderCellOf(row).replaceAll('/', '\\')
  return `${folder.startsWith(MUSIC) ? folder.slice(MUSIC.length) : folder}\\${fileOf(row)}`
}
export const shownFiles = () => dataRows().map(fileOf)
export const shownNames = () => dataRows().map(nameOf)
export const stateColumn = () => dataRows().map(stateOf)
export const selectedRows = () => dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')
export const rowByFile = (file: string) => dataRows().find((r) => fileOf(r) === file)
export const rowByName = (name: string) => dataRows().find((r) => nameOf(r) === name)

export const filterGroup = () => screen.getByRole('group', { name: 'Filter by verdict' })
export const verdictButton = (v: Verdict) =>
  within(filterGroup()).getByRole('button', { name: LABEL[v] }) as HTMLButtonElement
export const verdictButtons = () => BUTTON_ORDER.map(verdictButton)
export const pressedStates = () => verdictButtons().map((b) => b.getAttribute('aria-pressed'))
export const searchBox = () => screen.getByRole('searchbox', { name: 'Search by name' }) as HTMLInputElement
export const clearSearchButton = () => screen.queryByRole('button', { name: 'Clear search' })
export const noMatchMessage = () => screen.queryByText(NO_MATCHES)

export const items = () => screen.queryAllByRole('option')
export const item = (name: string) => {
  const found = items().find((o) => o.querySelector('[title]')?.textContent?.trim() === name)
  if (!found) throw new Error(`no album item ${name}`)
  return found
}
export const isCurrent = (o: HTMLElement) => o.getAttribute('aria-selected') === 'true'
export const spectrogramLoading = () => screen.queryByTestId('spectrogram-loading')

export const header = (label: string) => screen.getByRole('columnheader', { name: new RegExp(`^${label}`) })
export const headerButton = (label: string) =>
  within(header(label)).queryByRole('button') ?? header(label)

// ---- actions -----------------------------------------------------------------------------------

export async function pressVerdict(v: Verdict) {
  await userEvent.click(verdictButton(v))
}

export async function clickHeader(label: string) {
  await userEvent.click(headerButton(label))
}

/** "I sort the table by <label> <direction>": clicks the header until aria-sort says so. */
export async function sortBy(label: string, direction: 'ascending' | 'descending') {
  for (let i = 0; i < 2 && header(label).getAttribute('aria-sort') !== direction; i++) {
    await clickHeader(label)
  }
  await vi.waitFor(() => expect(header(label).getAttribute('aria-sort')).toBe(direction))
}

const plain = (text: string) => /^[\x20-\x7e]*$/.test(text) && !/[{[]/.test(text) && text.trim() === text

/** Types `text` in the search field (keystrokes for plain ASCII, a paste otherwise). */
export async function typeSearch(text: string) {
  if (text.length === 0) return
  if (plain(text)) {
    await userEvent.type(searchBox(), text)
  } else {
    await userEvent.click(searchBox())
    await userEvent.paste(text)
  }
}

export async function replaceSearch(text: string) {
  await userEvent.clear(searchBox())
  await typeSearch(text)
}

/**
 * Ctrl+F on `target`; resolves to whether its default was prevented (the fireEvent of
 * @testing-library/svelte is async: it resolves to dispatchEvent's result).
 */
export async function pressCtrlF(target: Element | Document | Window) {
  return !(await fireEvent.keyDown(target, { key: 'f', code: 'KeyF', ctrlKey: true }))
}

// ---- virtualised table -------------------------------------------------------------------------

/**
 * jsdom has no layout: the grid reports a `clientHeight` of VIEWPORT px (10 rows of 22 px) and every
 * other element 0, so the table virtualises (as in AnalysisPage.albumReveal.test.ts).
 */
export function installViewport(): MockInstance<(this: Element) => number> {
  return vi.spyOn(Element.prototype, 'clientHeight', 'get').mockImplementation(function (this: Element) {
    return this.getAttribute('role') === 'grid' ? VIEWPORT : 0
  })
}

export async function scrollGridTo(top: number) {
  grid().scrollTop = top
  await fireEvent.scroll(grid())
}

/**
 * Every row the table shows, top to bottom, read by scrolling the grid one viewport at a time and
 * keeping the rows of each rendered window in order; the scroll position is put back afterwards.
 * Also returns the names of the rows marked selected on the way.
 */
export async function readAllRows(read: (row: HTMLElement) => string = nameOf) {
  const start = grid().scrollTop
  const order: string[] = []
  const seen = new Set<string>()
  const selected = new Set<string>()
  let top = 0
  for (let step = 0; step < 100000; step++) {
    await scrollGridTo(top)
    let added = 0
    for (const r of dataRows()) {
      const key = read(r)
      if (r.getAttribute('aria-selected') === 'true') selected.add(key)
      if (!seen.has(key)) {
        seen.add(key)
        order.push(key)
        added++
      }
    }
    if (added === 0 && step > 0) break
    top += VIEWPORT
  }
  await scrollGridTo(start)
  return { order, selected: [...selected] }
}

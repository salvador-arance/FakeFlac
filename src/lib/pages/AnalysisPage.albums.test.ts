// AnalysisPage: the Albums panel (specs/ui/album-summary.md; interface layer of
// features/album-summary-app.feature, @RF-10; acceptance A1, A3, A4, A5).
// The API client is mocked and the events are stubbed: the page is the only component that talks
// to the client, and the store never computes an album count (it shows the latest update).
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - The panel is the `listbox` named "Album summaries" (role "option" items) of AlbumSummaryList;
//   an item is found by the start of its accessible name ("{name}, ...").
// - Albums in the stubs are built with `feed`: the n-th track of an album carries `total` n and
//   the counts by verdict of its first n tracks, as the contract orders them.
// - "Flush" is the store's 100 ms batch timer; most tests wait for it with real timers
//   (`settle`), the batching test uses fake timers.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
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

const HINT = 'Drop a folder here or click Open folder'
const unlisten = vi.fn()

function fakeRun() {
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
type Run = ReturnType<typeof fakeRun>

type AlbumSpec = {
  index: number
  name: string
  files: number
  /** Folder of the tracks and `summary.folder` (default `C:/Music/<name with />`). */
  folder?: string
}

const folderOf = (a: AlbumSpec) => a.folder ?? `C:/Music/${a.name.replaceAll('\\', '/')}`

/**
 * Emits one trackDone per verdict for the album `spec`, with track indexes `first`, `first + 1`,
 * ... and the cumulative summary of the tracks emitted so far for the album (`done` before).
 * Returns the next free track index.
 */
function feed(run: Run, spec: AlbumSpec, first: number, verdicts: Verdict[], done: Verdict[] = []) {
  const counted = [...done]
  verdicts.forEach((verdict, i) => {
    counted.push(verdict)
    const count = (v: Verdict) => counted.filter((x) => x === v).length
    run.emit({
      kind: 'trackDone',
      index: first + i,
      analysis: makeTrack({
        path: `${folderOf(spec)}/${String(first + i).padStart(3, '0')}.flac`,
        verdict,
      }),
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

const settle = () => new Promise((r) => setTimeout(r, 130))
const norm = (s: string | null) => (s ?? '').replace(/\s+/g, ' ').trim()
const statusText = () => norm(screen.getByRole('status').textContent)
const openButton = () => screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement
const dataRows = () =>
  screen.queryAllByRole('row').filter((r) => within(r).queryAllByRole('gridcell').length > 0)
const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')
const fileOf = (row: HTMLElement) => cells(row)[1].textContent?.trim()
const selectedRows = () => dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')
const items = () => screen.queryAllByRole('option')
const panel = () => screen.getByRole('listbox', { name: 'Album summaries' })
const item = (name: string, n = 0) => items().filter((o) => within(o).queryByText(name))[n]
/** The name of each item: its first element with a tooltip (the tooltip is the folder). */
const itemNames = () => items().map((o) => o.querySelector('[title]')?.textContent?.trim() ?? '')
const current = (o: HTMLElement) => o.getAttribute('aria-selected') === 'true'

async function mountPage() {
  const view = render(AnalysisPage)
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled())
  await waitForMainScreen()
  return view
}

async function openFolder(path = 'C:/Music/Album A') {
  vi.mocked(api.pickFolder).mockResolvedValue(path)
  await userEvent.click(openButton())
  await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled())
}

/** Opens a folder, feeds the albums and finishes; the run ended and the batch is flushed. */
async function finishedRun(specs: [AlbumSpec, Verdict[]][], root = 'C:/Music/Lib') {
  const run = fakeRun()
  await mountPage()
  await openFolder(root)
  const total = specs.reduce((n, [, v]) => n + v.length, 0)
  run.scanned(total)
  let next = 0
  for (const [spec, verdicts] of specs) next = feed(run, spec, next, verdicts)
  const counts = (v: Verdict) =>
    specs.reduce((n, [, vs]) => n + vs.filter((x) => x === v).length, 0)
  run.finish(
    makeSummary({
      root,
      total,
      done: total,
      real: counts('real'),
      suspicious: counts('suspicious'),
      fake: counts('fake'),
      errors: counts('error'),
    }),
  )
  await vi.waitFor(() => expect(statusText()).toMatch(/^\d+ tracks?/))
  return run
}

const A: AlbumSpec = { index: 0, name: 'Lib\\A', files: 1 }
const B: AlbumSpec = { index: 1, name: 'Lib\\B', files: 1 }
const C: AlbumSpec = { index: 2, name: 'Lib\\C', files: 1 }

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
})

afterEach(() => {
  vi.useRealTimers()
})

describe('Albums panel: after the analysis', () => {
  // @RF-10 Scenario: Album summaries are shown after the analysis
  it('lists the albums in order with their verdict and counts, and keeps the table per track', async () => {
    const artist: AlbumSpec = { index: 0, name: 'Artist', files: 1 }
    const a1: AlbumSpec = { index: 1, name: 'Artist\\Album 1', files: 12 }
    const a2: AlbumSpec = { index: 2, name: 'Artist\\Album 2', files: 9 }
    await finishedRun([
      [artist, ['real']],
      [a1, [...Array<Verdict>(11).fill('real'), 'fake']],
      [a2, [...Array<Verdict>(7).fill('real'), 'suspicious', 'suspicious']],
    ])
    await vi.waitFor(() => expect(screen.getByText('Albums (3)')).not.toBeNull())
    expect(items()).toHaveLength(3)
    const expected = [
      ['Artist', 'Genuine', '1 track · 1 real'],
      ['Artist\\Album 1', 'Fake', '12 tracks · 11 real · 1 fake'],
      ['Artist\\Album 2', 'Suspicious', '9 tracks · 7 real · 2 suspicious'],
    ]
    expected.forEach(([name, verdict, counts], i) => {
      const o = within(items()[i])
      expect(o.getByText(name)).not.toBeNull()
      expect(o.getByText(verdict)).not.toBeNull()
      expect(o.getByText(counts)).not.toBeNull()
      expect(items()[i].querySelector('svg')).not.toBeNull()
    })
    // the table still has one row per track and is not grouped
    expect(dataRows()).toHaveLength(22)
  })

  // @RF-10 Scenario: The panel's accessible names
  it('names the panel and its items', async () => {
    const a1: AlbumSpec = { index: 0, name: 'Artist\\Album 1', files: 12 }
    await finishedRun([[a1, [...Array<Verdict>(11).fill('real'), 'fake']]])
    expect(panel()).not.toBeNull()
    await vi.waitFor(() =>
      expect(
        screen.getByRole('option', {
          name: 'Artist\\Album 1, Fake, 12 tracks · 11 real · 1 fake',
        }),
      ).not.toBeNull(),
    )
  })

  // @RF-10 @RNF-04 Scenario: A folder whose files all failed
  it('shows an error album with the error icon', async () => {
    const broken: AlbumSpec = { index: 0, name: 'Lib\\Broken', files: 2 }
    await finishedRun([[broken, ['error', 'error']]])
    await vi.waitFor(() => expect(items()).toHaveLength(1))
    const o = within(items()[0])
    expect(o.getByText('Error')).not.toBeNull()
    expect(items()[0].querySelector('svg')).not.toBeNull()
    expect(o.getByText('2 tracks · 2 errors')).not.toBeNull()
  })

  // @RF-10 @RNF-09 Scenario: Unicode folder names in the panel
  it('shows Unicode names exactly and the folder as tooltip', async () => {
    const root: AlbumSpec = {
      index: 0,
      name: 'Álbum ″Ñandú″',
      files: 1,
      folder: 'C:\\Music\\Álbum ″Ñandú″',
    }
    const disc: AlbumSpec = {
      index: 1,
      name: 'Álbum ″Ñandú″\\Disco 2 «Ça va» 🎵',
      files: 1,
      folder: 'C:\\Music\\Álbum ″Ñandú″\\Disco 2 «Ça va» 🎵',
    }
    await finishedRun([
      [root, ['real']],
      [disc, ['real']],
    ])
    await vi.waitFor(() => expect(items()).toHaveLength(2))
    expect(within(items()[0]).getByText('Álbum ″Ñandú″').textContent).toBe('Álbum ″Ñandú″')
    const second = within(items()[1]).getByText('Álbum ″Ñandú″\\Disco 2 «Ça va» 🎵')
    expect(second.getAttribute('title')).toBe('C:\\Music\\Álbum ″Ñandú″\\Disco 2 «Ça va» 🎵')
  })

  // @RF-10 Scenario: A long name is cut with an ellipsis and shown whole in the tooltip
  it('gives a long name the full folder path as tooltip', async () => {
    const name =
      'Artist\\A very long album name that does not fit in the panel (Remastered 2024 Deluxe Edition)'
    const long: AlbumSpec = { index: 0, name, files: 1, folder: `C:\\Music\\${name}` }
    await finishedRun([[long, ['real']]])
    await vi.waitFor(() => expect(items()).toHaveLength(1))
    expect(within(items()[0]).getByText(name).getAttribute('title')).toBe(`C:\\Music\\${name}`)
  })

  // @RF-10 @RNF-09 Scenario: Two albums with the same name stay two items
  it('keeps two albums with the same name apart and selects by index', async () => {
    const same = { name: 'Lib\\Disc \uFFFD', files: 1, folder: 'C:\\Music\\Lib\\Disc \uFFFD' }
    await finishedRun([
      [{ ...same, index: 0 }, ['real']],
      [{ ...same, index: 1 }, ['fake']],
    ])
    await vi.waitFor(() => expect(items()).toHaveLength(2))
    expect(within(items()[0]).getByText('Genuine')).not.toBeNull()
    expect(within(items()[1]).getByText('Fake')).not.toBeNull()
    await userEvent.click(items()[1])
    await vi.waitFor(() => expect(selectedRows()).toHaveLength(1))
    expect(cells(selectedRows()[0])[0].textContent).toContain('Fake')
    expect(items().map(current)).toEqual([false, true])
  })

  // @RF-10 Scenario: Sorting the table does not change the panel
  it('does not change the panel when the table is sorted', async () => {
    await finishedRun([
      [A, ['real']],
      [B, ['fake']],
      [C, ['suspicious']],
    ])
    await vi.waitFor(() => expect(items()).toHaveLength(3))
    const before = items().map((o) => norm(o.textContent))
    await userEvent.click(screen.getByRole('button', { name: /State/ }))
    await userEvent.click(screen.getByRole('button', { name: /Folder/ }))
    await userEvent.click(screen.getByRole('button', { name: /Folder/ }))
    await vi.waitFor(() =>
      expect(screen.getByRole('columnheader', { name: /Folder/ }).getAttribute('aria-sort')).toBe(
        'descending',
      ),
    )
    expect(items().map((o) => norm(o.textContent))).toEqual(before)
    expect(itemNames()).toEqual(['Lib\\A', 'Lib\\B', 'Lib\\C'])
  })
})

describe('Albums panel: while the analysis runs', () => {
  // @RF-10 Scenario: Albums appear in path order, whatever order they arrive in
  it('lists an album that arrives later above the albums already shown', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(2)
    feed(run, { index: 2, name: 'Lib\\B', files: 1 }, 1, ['real'])
    await vi.waitFor(() => expect(items()).toHaveLength(1))
    feed(run, { index: 0, name: 'Lib\\A', files: 1 }, 0, ['real'])
    await vi.waitFor(() => expect(items()).toHaveLength(2))
    expect(itemNames()).toEqual(['Lib\\A', 'Lib\\B'])
  })

  // @RF-10 Scenario: Summaries update during the analysis (and A4: same flush as the table)
  it('updates the item and the table in the same flush, at most once every 100 ms', async () => {
    const run = fakeRun()
    await mountPage()
    vi.useFakeTimers()
    vi.mocked(api.pickFolder).mockResolvedValue('C:/Music/Album A')
    await fireEvent.click(openButton())
    await vi.advanceTimersByTimeAsync(0)
    const album: AlbumSpec = { index: 0, name: 'Album A', files: 12 }
    run.scanned(12)
    await vi.advanceTimersByTimeAsync(0)

    feed(run, album, 0, ['real', 'real', 'real'])
    expect(items()).toHaveLength(0)
    expect(dataRows()).toHaveLength(0)
    await vi.advanceTimersByTimeAsync(100)
    expect(dataRows()).toHaveLength(3)
    expect(items()).toHaveLength(1)
    expect(within(items()[0]).getByText('Genuine')).not.toBeNull()
    expect(within(items()[0]).getByText('3 of 12 tracks · 3 real')).not.toBeNull()

    feed(run, album, 3, ['fake'], ['real', 'real', 'real'])
    await vi.advanceTimersByTimeAsync(10)
    expect(within(items()[0]).getByText('3 of 12 tracks · 3 real')).not.toBeNull()
    expect(dataRows()).toHaveLength(3)
    await vi.advanceTimersByTimeAsync(100)
    expect(dataRows()).toHaveLength(4)
    expect(within(items()[0]).getByText('Fake')).not.toBeNull()
    expect(within(items()[0]).getByText('4 of 12 tracks · 3 real · 1 fake')).not.toBeNull()
  })

  // @RF-10 Scenario: Panel counts always match the table rows (A1, A3)
  it('keeps every item equal to the counts of the rows fed after every flush', async () => {
    vi.useFakeTimers()
    const run = fakeRun()
    render(AnalysisPage)
    await waitForMainScreen()
    vi.mocked(api.pickFolder).mockResolvedValue('C:/Music/Lib')
    await fireEvent.click(openButton())
    await vi.advanceTimersByTimeAsync(0)

    // 7 albums, 200 tracks, events of different albums shuffled (a seeded generator)
    const sizes = [40, 35, 30, 30, 25, 20, 20]
    const specs: AlbumSpec[] = sizes.map((files, i) => ({ index: i, name: `Lib\\Album ${i}`, files }))
    let seed = 12345
    const rand = () => {
      seed = (seed * 1664525 + 1013904223) % 4294967296
      return seed / 4294967296
    }
    const verdicts: Verdict[] = ['real', 'real', 'real', 'suspicious', 'fake', 'error']
    const left = sizes.map((n) => n)
    const counted: Verdict[][] = sizes.map(() => [])
    let next = 0
    run.scanned(200)

    // A1 compares each item with the store's rows of its album: the verdicts fed so far
    // (`counted`), all flushed by the 100 ms tick before each check. In jsdom the table is not
    // virtualised and renders at most 50 rows, so the DOM rows are only a cross-check while the
    // table holds every row.
    const byVerdict = (vs: Verdict[]) =>
      (['real', 'suspicious', 'fake', 'error'] as Verdict[]).map(
        (v) => vs.filter((x) => x === v).length,
      )
    const check = () => {
      const fed = counted.reduce((n, c) => n + c.length, 0)
      const albums = counted.filter((c) => c.length > 0).length
      expect(items()).toHaveLength(albums)
      if (fed <= 50) {
        const byFolder = new Map<string, Verdict[]>()
        for (const row of dataRows()) {
          const folder = cells(row)[2].textContent?.trim() ?? ''
          const text = cells(row)[0].textContent ?? ''
          const verdict: Verdict = text.includes('Genuine')
            ? 'real'
            : text.includes('Suspicious')
              ? 'suspicious'
              : text.includes('Fake')
                ? 'fake'
                : 'error'
          byFolder.set(folder, [...(byFolder.get(folder) ?? []), verdict])
        }
        expect(byFolder.size).toBe(albums)
        specs.forEach((spec, a) => {
          if (counted[a].length === 0) return
          expect(byVerdict(byFolder.get(folderOf(spec)) ?? []), spec.name).toEqual(
            byVerdict(counted[a]),
          )
        })
      }
      for (const [a, spec] of specs.entries()) {
        const vs = counted[a]
        if (vs.length === 0) continue
        const n = (v: Verdict) => vs.filter((x) => x === v).length
        const parts = [
          vs.length === spec.files
            ? vs.length === 1
              ? '1 track'
              : `${vs.length} tracks`
            : `${vs.length} of ${spec.files} tracks`,
          n('real') ? `${n('real')} real` : '',
          n('suspicious') ? `${n('suspicious')} suspicious` : '',
          n('fake') ? `${n('fake')} fake` : '',
          n('error') ? (n('error') === 1 ? '1 error' : `${n('error')} errors`) : '',
        ].filter(Boolean)
        const o = items().find((el) => within(el).queryByText(spec.name))
        expect(o, spec.name).toBeDefined()
        expect(within(o as HTMLElement).getByText(parts.join(' · '))).not.toBeNull()
      }
    }

    while (left.some((n) => n > 0)) {
      for (let k = 0; k < 13 && left.some((n) => n > 0); k++) {
        const open = left.map((n, i) => (n > 0 ? i : -1)).filter((i) => i >= 0)
        const a = open[Math.floor(rand() * open.length)]
        const v = verdicts[Math.floor(rand() * verdicts.length)]
        next = feed(run, specs[a], next, [v], counted[a])
        counted[a].push(v)
        left[a] -= 1
      }
      await vi.advanceTimersByTimeAsync(100)
      check()
    }

    const done = counted.reduce((n, c) => n + c.length, 0)
    expect(done).toBe(200)
    run.finish(makeSummary({ total: 200, done: 200 }))
    await vi.advanceTimersByTimeAsync(150)
    const totals = items().map((o) => Number(/(\d+) tracks/.exec(norm(o.textContent))?.[1]))
    expect(totals.reduce((a, b) => a + b, 0)).toBe(200)
    expect(items().every((o) => !/ of \d+ tracks/.test(norm(o.textContent)))).toBe(true)
  }, 30000)
})

describe('Albums panel: states', () => {
  // @RF-10 Scenario: The panel is not shown in the empty state
  it('is not shown at the start; the hint is', async () => {
    await mountPage()
    expect(screen.queryByRole('listbox')).toBeNull()
    expect(screen.getByText(HINT)).not.toBeNull()
  })

  // @RF-10 Scenario: A cancelled run keeps partial albums
  it('keeps the albums with tracks done after a cancel, as partial', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder('C:/Music/Big')
    run.scanned(24)
    feed(run, { index: 0, name: 'Big\\Album 1', files: 12 }, 0, ['real', 'real', 'real'])
    await vi.waitFor(() => expect(items()).toHaveLength(1))
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(api.cancelAnalysis).toHaveBeenCalledTimes(1)
    run.cancelled(makeSummary({ total: 24, done: 3, real: 3, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toBe('Cancelled · 3 of 24'))
    expect(itemNames()).toEqual(['Big\\Album 1'])
    expect(within(items()[0]).getByText('3 of 12 tracks · 3 real')).not.toBeNull()
  })

  // @RF-10 Scenario: A cancel before any track leaves the panel empty
  it('shows the header "Albums" and no items when cancelled before any track', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(10)
    run.cancelled(makeSummary({ total: 10, done: 0, real: 0, cancelled: true }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^Cancelled/))
    expect(screen.getByText('Albums')).not.toBeNull()
    expect(items()).toHaveLength(0)
  })

  // @RF-10 @RNF-04 Scenario: A run that fails unexpectedly keeps the albums it reported
  it('keeps the albums reported, also the pending ones, when the run fails with INTERNAL', async () => {
    vi.useFakeTimers()
    const run = fakeRun()
    render(AnalysisPage)
    await waitForMainScreen()
    vi.mocked(api.pickFolder).mockResolvedValue('C:/Music/Album A')
    await fireEvent.click(openButton())
    await vi.advanceTimersByTimeAsync(0)
    const album: AlbumSpec = { index: 0, name: 'Album A', files: 5 }
    run.scanned(5)
    feed(run, album, 0, ['real'])
    await vi.advanceTimersByTimeAsync(100)
    feed(run, album, 1, ['real'], ['real']) // the second stays in the pending batch
    run.fail('INTERNAL')
    await vi.advanceTimersByTimeAsync(150)
    expect(statusText()).toMatch(/^The analysis stopped because of an unexpected error/)
    expect(itemNames()).toEqual(['Album A'])
    expect(within(items()[0]).getByText('2 of 5 tracks · 2 real')).not.toBeNull()
    expect(dataRows()).toHaveLength(2)
  })

  // @RF-10 Scenario: A new analysis replaces the album summaries
  it('replaces the albums of the previous run', async () => {
    await finishedRun(
      [
        [{ index: 0, name: 'Lib A\\X', files: 1 }, ['real']],
        [{ index: 1, name: 'Lib A\\Y', files: 1 }, ['fake']],
      ],
      'C:/Music/Lib A',
    )
    await vi.waitFor(() => expect(items()).toHaveLength(2))

    const run2 = fakeRun()
    await openFolder('C:/Music/Lib B')
    expect(screen.getByText('Albums')).not.toBeNull()
    expect(items()).toHaveLength(0)
    run2.scanned(1)
    feed(run2, { index: 0, name: 'Lib B', files: 1 }, 0, ['real'])
    run2.finish(makeSummary({ root: 'C:/Music/Lib B', total: 1, done: 1, real: 1 }))
    await vi.waitFor(() => expect(itemNames()).toEqual(['Lib B']))
  })

  // @RF-10 Scenario Outline: A start that fails before Scanning keeps the albums
  it.each(['NO_AUDIO_FOUND', 'FOLDER_UNREADABLE', 'NOT_A_FOLDER', 'ANALYSIS_RUNNING'])(
    'keeps the albums and the current album when start_analysis rejects with %s',
    async (code) => {
      const two: AlbumSpec[] = [A, B]
      await finishedRun([
        [two[0], ['real']],
        [two[1], ['fake']],
      ])
      await vi.waitFor(() => expect(items()).toHaveLength(2))
      const row = dataRows().find((r) => fileOf(r) === '001.flac') as HTMLElement
      await userEvent.click(cells(row)[1])
      await vi.waitFor(() => expect(items().map(current)).toEqual([false, true]))
      const before = items().map((o) => norm(o.textContent))

      const run2 = fakeRun()
      await openFolder('C:/Docs')
      run2.fail(code)
      await vi.waitFor(() => expect(openButton().disabled).toBe(false))
      await settle()
      expect(items().map((o) => norm(o.textContent))).toEqual(before)
      expect(items().map(current)).toEqual([false, true])
      expect(dataRows()).toHaveLength(2)
    },
  )
})

describe('Albums panel: selection', () => {
  // @RF-10 Scenario: Selecting an album selects its first track (A5)
  it('selects the row with the lowest index of the album, whatever the table sort', async () => {
    const run = fakeRun()
    await mountPage()
    await openFolder()
    run.scanned(10)
    const a: AlbumSpec = { index: 0, name: 'Lib\\A', files: 1 }
    const b: AlbumSpec = { index: 1, name: 'Lib\\B', files: 3 }
    feed(run, a, 0, ['real'])
    // rows of B have the indexes 7, 4 and 9
    feed(run, b, 7, ['real'])
    feed(run, b, 4, ['real'], ['real'])
    feed(run, b, 9, ['real'], ['real', 'real'])
    run.finish(makeSummary({ total: 5, done: 5, real: 5 }))
    await vi.waitFor(() => expect(statusText()).toMatch(/^\d+ tracks?/))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(4))

    await userEvent.click(screen.getByRole('button', { name: /File/ })) // descending
    await vi.waitFor(() =>
      expect(screen.getByRole('columnheader', { name: /File/ }).getAttribute('aria-sort')).toBe(
        'descending',
      ),
    )
    await userEvent.click(item('Lib\\B'))
    await vi.waitFor(() => expect(selectedRows()).toHaveLength(1))
    expect(fileOf(selectedRows()[0])).toBe('004.flac')
    expect(api.getSpectrogram).toHaveBeenCalledWith('C:/Music/Lib/B/004.flac')
    expect(item('Lib\\B') && current(item('Lib\\B'))).toBe(true)
    expect(document.activeElement).not.toBe(screen.getByRole('grid'))
  })

  // @RF-10 Scenario: Selecting a track marks its album as current
  it('marks the album of the selected row as current, also with the keys of the table', async () => {
    await finishedRun([
      [A, ['real']],
      [B, ['real']],
    ])
    await vi.waitFor(() => expect(items()).toHaveLength(2))
    expect(items().map(current)).toEqual([false, false])
    await userEvent.click(cells(dataRows()[1])[1])
    await vi.waitFor(() => expect(items().map(current)).toEqual([false, true]))
    await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowUp' })
    await vi.waitFor(() => expect(items().map(current)).toEqual([true, false]))
  })

  // @RF-10 Scenario: Keyboard in the Albums panel
  it('moves between albums with the keys of the panel, keeping the focus in it', async () => {
    await finishedRun([
      [{ ...A, files: 2 }, ['real', 'real']],
      [{ ...B, files: 2 }, ['real', 'real']],
      [{ ...C, files: 2 }, ['real', 'real']],
    ])
    await vi.waitFor(() => expect(items()).toHaveLength(3))
    expect(selectedRows()).toHaveLength(0)
    const list = panel()
    list.focus()
    expect(document.activeElement).toBe(list)

    await fireEvent.keyDown(list, { key: 'ArrowDown' })
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['000.flac']))
    await fireEvent.keyDown(list, { key: 'End' })
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['004.flac']))
    await fireEvent.keyDown(list, { key: 'ArrowUp' })
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['002.flac']))
    expect(document.activeElement).toBe(list)
    expect(items().map(current)).toEqual([false, true, false])
  })
})

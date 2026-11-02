// @RF-06 @RF-10 @RF-13 resizable-widths.feature: persist independently of language;
// unavailable storage; view/data changes; empty and modal states (W4/W5).
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AnalysisSummary, ProgressEvent } from '../api'
import * as api from '../api'
import { makeAlbum, makeLowPassSpectrogram, makeSummary, makeTrack } from '../testing/fixtures'
import { measurements, pointer, WIDTHS_KEY } from '../tests/resizeHarness'
import { mockSettings, snapshot, switchToSpanish, waitForMainScreen } from './tests/settingsHarness'
import AnalysisPage from './AnalysisPage.svelte'

vi.mock('../api', () => {
  class ApiError extends Error {
    code: string
    constructor(error: { code: string; message: string }) { super(error.message); this.code = error.code }
  }
  return { ApiError, pickFolder: vi.fn(), startAnalysis: vi.fn(), startAnalysisOfDropped: vi.fn(), cancelAnalysis: vi.fn(), exportCsv: vi.fn(), clearCache: vi.fn(), getSpectrogram: vi.fn(), onDragDrop: vi.fn(), minimizeWindow: vi.fn(), toggleMaximizeWindow: vi.fn(), closeWindow: vi.fn(), getSettings: vi.fn(), setSettings: vi.fn() }
})

const file = () => screen.getByRole('separator', { name: 'Resize File column' })
const sidebar = () => screen.getByRole('separator', { name: 'Resize Albums panel' })
const width = (handle: HTMLElement) => Number(handle.getAttribute('aria-valuenow'))
let sizes: ReturnType<typeof measurements>

beforeEach(() => {
  localStorage.clear()
  vi.clearAllMocks()
  sizes = measurements()
  mockSettings()
  vi.mocked(api.onDragDrop).mockResolvedValue(() => {})
  vi.mocked(api.pickFolder).mockResolvedValue('C:/Synthetic/Audio')
  vi.mocked(api.getSpectrogram).mockResolvedValue(makeLowPassSpectrogram(16000))
})
afterEach(() => { localStorage.clear(); vi.restoreAllMocks(); vi.unstubAllGlobals() })

function succeedingRun(count = 3) {
  vi.mocked(api.startAnalysis).mockImplementation(async (_root, event) => {
    event({ kind: 'scanned', total: count })
    for (let index = 0; index < count; index++) {
      const corrupt = index === 1
      event({ kind: 'trackDone', index,
        analysis: makeTrack({ path: `C:/Synthetic/Audio/${index === 0 ? '日本語' : String(index)}.flac`, verdict: corrupt ? 'error' : 'real', error: corrupt ? { kind: 'corruptStream', detail: 'synthetic' } : null }),
        album: makeAlbum({ index: 0, name: 'Synthetic', files: count, real: index + 1 - Number(index > 0), errors: Number(index > 0) }),
      })
    }
    const summary = makeSummary({ total: count, done: count, real: count - Number(count > 1), errors: Number(count > 1) })
    event({ kind: 'finished', summary })
    return summary
  })
}
async function mountPage() {
  const view = render(AnalysisPage)
  await waitForMainScreen()
  return view
}
async function openFolder(name = 'Open folder') {
  await userEvent.click(screen.getByRole('button', { name }))
  await screen.findByRole('grid')
  sizes.set(screen.getByRole('grid'), 1040)
  sizes.set(screen.getByRole('main'), 1280, 500)
}
function persisted() { return JSON.parse(localStorage.getItem(WIDTHS_KEY) ?? 'null') }

describe('@RF-06 @RF-10 widths owned by AnalysisPage', () => {
  it('loads preferences before results appear and does not expose a sidebar handle on the initial empty screen', async () => {
    localStorage.setItem(WIDTHS_KEY, JSON.stringify({ version: 1, sidebarWidth: 320, columns: { file: 230 } }))
    succeedingRun()
    await mountPage()
    expect(screen.queryByRole('separator', { name: 'Resize Albums panel' })).toBeNull()
    await openFolder()
    await vi.waitFor(() => expect(width(file())).toBe(230))
    expect(width(sidebar())).toBe(320)
  })
  it.each(['pointerup', 'pointercancel', 'blur'])('pointer motion stays in memory; %s writes one preference snapshot with no audio metadata', async (end) => {
    succeedingRun()
    await mountPage(); await openFolder()
    const write = vi.spyOn(Storage.prototype, 'setItem')
    await vi.waitFor(() => expect(width(file())).toBe(150))
    await pointer(file(), 'pointerdown', 100)
    await pointer(window, 'pointermove', 180)
    expect(width(file())).toBe(230)
    expect(write.mock.calls.filter(([key]) => key === WIDTHS_KEY)).toHaveLength(0)
    if (end === 'blur') await fireEvent(window, new Event('blur'))
    else await pointer(window, end, 180)
    expect(persisted()).toEqual({ version: 1, columns: { file: 230 } })
    expect(write.mock.calls.filter(([key]) => key === WIDTHS_KEY)).toHaveLength(1)
    expect(localStorage.getItem(WIDTHS_KEY)).not.toMatch(/Synthetic|日本語|flac|verdict|real|error/i)
  })
  it('a no-motion click and automatic window measurement do not write layout preferences', async () => {
    succeedingRun()
    await mountPage(); await openFolder()
    const write = vi.spyOn(Storage.prototype, 'setItem')
    await pointer(file(), 'pointerdown', 100)
    await pointer(window, 'pointerup', 100)
    sizes.set(screen.getByRole('grid'), 1390)
    await vi.waitFor(() => expect(width(file())).toBe(300))
    expect(write.mock.calls.filter(([key]) => key === WIDTHS_KEY)).toHaveLength(0)
  })
  it('relaunch in Spanish restores the same overrides, and individual reset preserves the other preference', async () => {
    succeedingRun()
    const first = await mountPage(); await openFolder()
    await fireEvent.keyDown(sidebar(), { key: 'ArrowRight' })
    await fireEvent.keyDown(file(), { key: 'ArrowRight' })
    expect(persisted()).toEqual({ version: 1, sidebarWidth: 250, columns: { file: 160 } })
    await first.unmount()
    mockSettings(snapshot('es', 'loaded'))
    succeedingRun()
    await mountPage(); await openFolder('Abrir carpeta')
    const f = screen.getByRole('separator', { name: 'Cambiar ancho de la columna Archivo' })
    const s = screen.getByRole('separator', { name: 'Cambiar ancho del panel de álbumes' })
    expect(width(f)).toBe(160)
    expect(width(s)).toBe(250)
    await fireEvent.keyDown(f, { key: 'Enter' })
    expect(persisted()).toEqual({ version: 1, sidebarWidth: 250 })
    await fireEvent.keyDown(s, { key: 'Enter' })
    expect(localStorage.getItem(WIDTHS_KEY)).toBeNull()
  })
  it('language, sorting, filtering, selected Unicode/error rows and a new run retain committed widths', async () => {
    localStorage.setItem(WIDTHS_KEY, JSON.stringify({ version: 1, columns: { file: 230 } }))
    succeedingRun()
    await mountPage(); await openFolder()
    await userEvent.click(screen.getByText('日本語.flac'))
    await userEvent.click(within(screen.getAllByRole('columnheader')[0]).getByRole('button'))
    await userEvent.type(screen.getByRole('searchbox', { name: 'Search by name' }), '日本語')
    expect(width(file())).toBe(230)
    await switchToSpanish()
    expect(width(screen.getByRole('separator', { name: 'Cambiar ancho de la columna Archivo' }))).toBe(230)
    succeedingRun(10000)
    await openFolder('Abrir carpeta')
    await vi.waitFor(() => expect(width(screen.getByRole('separator', { name: 'Cambiar ancho de la columna Archivo' }))).toBe(230))
    expect(persisted()).toEqual({ version: 1, columns: { file: 230 } })
  })
  it('unavailable storage still allows keyboard resizing in memory with no error notice', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked') })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('blocked') })
    succeedingRun()
    await mountPage(); await openFolder()
    await vi.waitFor(() => expect(width(file())).toBe(150))
    await fireEvent.keyDown(file(), { key: 'ArrowRight' })
    expect(width(file())).toBe(160)
    expect(screen.getByRole('status').textContent).not.toMatch(/unexpected|cannot|unavailable|blocked/i)
  })
  it('a new run failing before scanning retains widths and previous results', async () => {
    localStorage.setItem(WIDTHS_KEY, '{"version":1,"columns":{"file":230}}')
    succeedingRun()
    await mountPage(); await openFolder()
    vi.mocked(api.startAnalysis).mockRejectedValue(new api.ApiError({ code: 'NO_AUDIO_FOUND', message: 'synthetic' } as never))
    await userEvent.click(screen.getByRole('button', { name: 'Open folder' }))
    await vi.waitFor(() => expect(screen.getByRole('status').textContent).toContain('No supported audio'))
    expect(width(file())).toBe(230)
    expect(screen.getByText('日本語.flac')).not.toBeNull()
  })
  it('opening Settings during a column drag commits its last value then blocks further resizing', async () => {
    succeedingRun()
    await mountPage(); await openFolder()
    await vi.waitFor(() => expect(width(file())).toBe(150))
    await pointer(file(), 'pointerdown', 100)
    await pointer(window, 'pointermove', 130)
    // Open without a preceding pointer-up, so the modal itself must terminate the drag.
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }))
    await screen.findByRole('dialog')
    expect(persisted()).toEqual({ version: 1, columns: { file: 180 } })
    await pointer(window, 'pointermove', 300)
    expect(width(file())).toBe(180)
    expect(file().closest('[inert]')).not.toBeNull()
  })
  it('column resize completion is available during an active analysis without cancelling it', async () => {
    let deliver: (event: ProgressEvent) => void = () => {}
    let finish: (summary: AnalysisSummary) => void = () => {}
    vi.mocked(api.startAnalysis).mockImplementation((_root, callback) => {
      deliver = callback
      return new Promise(resolve => { finish = resolve })
    })
    await mountPage()
    await userEvent.click(screen.getByRole('button', { name: 'Open folder' }))
    deliver({ kind: 'scanned', total: 10 })
    await screen.findByRole('grid')
    sizes.set(screen.getByRole('grid'), 1040)
    await vi.waitFor(() => expect(width(file())).toBe(150))
    await fireEvent.keyDown(file(), { key: 'ArrowRight' })
    expect(persisted()).toEqual({ version: 1, columns: { file: 160 } })
    expect(api.cancelAnalysis).not.toHaveBeenCalled()
    const summary = makeSummary({ total: 10, done: 0, cancelled: true })
    deliver({ kind: 'cancelled', summary }); finish(summary)
  })
})

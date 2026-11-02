// AnalysisPage: live translation of the mounted screen (@RF-13 @RF-08 @RNF-04 @RNF-09;
// specs/ui/language-settings.md sections 1 and 3, acceptance L4, L5 and L7 (page side)). The API
// client is mocked. Scenarios of features/language-settings.feature, page side:
// - "Existing mounted states translate after a successful Save" (ready, scanning, analysing,
//   finished, cancelled, error)
// - "Language changes name ordering while preserving selection and view state"
// - "Settings can change while progress and a spectrogram are pending"
// - "Existing error and export notices translate without changing user data"
// - "An analysis failure during a settings save remains an analysis failure"
// - "A cache notice already on screen translates"
// - "Language changes do not change exported results or detection identity" (the page passes no
//   language to the export; the CSV bytes are src-tauri/tests/settings_invariance.rs)
// - "Native Windows dialogs retain their system language" (the app-owned trigger and messages; the
//   dialogs themselves are the OS's: manual check M2)
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "All mounted visible texts, tooltips and accessible names use the Spanish catalogue" is
//   checked by an audit: no text node and no `title`, `aria-label` or `placeholder` attribute of the
//   page equals an English catalogue text (from specs/ui/translations.md) whose Spanish text
//   differs; and, after switching back, none equals such a Spanish text.
// - "The export notice uses its Spanish sentence with the identical destination path": the notice
//   shows the file name of the destination (export-csv.md `{file}`), byte-identical.
// - Splitter size unchanged: every inline `height` style of the page is the same.
import { fireEvent, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import translations from '../../../specs/ui/translations.md?raw'
import * as api from '../api'
import type { Spectrogram } from '../model'
import { installCanvasStub, makeSpectrogram } from '../testing/spectrogram'
import type { CanvasStub } from '../testing/spectrogram'
import {
  Feeder,
  cells,
  clickHeader,
  dataRows,
  finishRun,
  finishedAnalysis,
  grid,
  header,
  items,
  listed,
  mountPage,
  norm,
  pressVerdict,
  resetApiMocks,
  rowByFile,
  selectedRows,
  shownFiles,
  sortBy,
  startRun,
  statusText,
  summaryOf,
  typeSearch,
  fileOf,
} from './tests/sortFilterHarness'
import type { RowSpec } from './tests/sortFilterHarness'
import {
  deferred,
  dialog,
  dialogButton,
  documentLanguage,
  languageSelect,
  mockSettings,
  openSettings,
  pendingSetSettings,
  settingsButton,
  snapshot,
  switchToEnglish,
  switchToSpanish,
} from './tests/settingsHarness'

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
    clearCache: vi.fn(),
    onDragDrop: vi.fn(),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    getSettings: vi.fn(),
    setSettings: vi.fn(),
  }
})

let canvas: CanvasStub

beforeEach(() => {
  resetApiMocks()
  mockSettings()
  vi.mocked(api.clearCache).mockReset().mockResolvedValue(undefined)
  canvas = installCanvasStub()
})

afterEach(() => {
  canvas.restore()
  vi.useRealTimers()
})

// ---- the catalogue audit -----------------------------------------------------------------------

type Row = { key: string; en: string; es: string }
const ROWS: Row[] = translations
  .split(/\r?\n/)
  .map((line) => line.match(/^\|\s*`([^`]+)`\s*\|(.*)\|(.*)\|\s*$/))
  .filter((m): m is RegExpMatchArray => m !== null)
  .map((m) => ({ key: m[1], en: m[2].trim(), es: m[3].trim() }))
const plain = ROWS.filter((r) => r.en !== r.es && !/\{\w+\}/.test(r.en + r.es))
const ENGLISH_ONLY = new Set(plain.map((r) => r.en).filter((en) => !ROWS.some((r) => r.es === en)))
const SPANISH_ONLY = new Set(plain.map((r) => r.es).filter((es) => !ROWS.some((r) => r.en === es)))

/** Every text node and every title / aria-label / placeholder of the document. */
function renderedStrings(): string[] {
  const out: string[] = []
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const text = norm(n.textContent)
    if (text) out.push(text)
  }
  for (const el of Array.from(document.body.querySelectorAll('*'))) {
    for (const attr of ['title', 'aria-label', 'placeholder']) {
      const v = el.getAttribute(attr)
      if (v) out.push(v.trim())
    }
  }
  return out
}

function expectNoLeftovers(language: 'en' | 'es') {
  const forbidden = language === 'es' ? ENGLISH_ONLY : SPANISH_ONLY
  const left = renderedStrings().filter((s) => forbidden.has(s))
  expect(left, `untranslated texts in ${language}`).toEqual([])
}

// ---- helpers -----------------------------------------------------------------------------------

const allRows = () =>
  screen
    .queryAllByRole('row', { hidden: true })
    .filter((r) => within(r).queryAllByRole('gridcell', { hidden: true }).length > 0)
const button = (name: string) => screen.getByRole('button', { name }) as HTMLButtonElement
const heights = () =>
  Array.from(document.querySelectorAll<HTMLElement>('[style]'))
    .map((e) => e.style.height)
    .filter(Boolean)
const callCounts = () => ({
  start: vi.mocked(api.startAnalysis).mock.calls.length,
  drop: vi.mocked(api.startAnalysisOfDropped).mock.calls.length,
  cancel: vi.mocked(api.cancelAnalysis).mock.calls.length,
  spectrogram: vi.mocked(api.getSpectrogram).mock.calls.length,
  export: vi.mocked(api.exportCsv).mock.calls.length,
  clear: vi.mocked(api.clearCache).mock.calls.length,
  pick: vi.mocked(api.pickFolder).mock.calls.length,
})

const SPANISH_TOOLBAR = [
  'Abrir carpeta',
  'Cancelar',
  'Exportar CSV',
  'Borrar caché de resultados',
  'Mostrar pistas falsas',
  'Mostrar pistas sospechosas',
  'Mostrar pistas reales',
  'Mostrar pistas con errores',
  'Ajustes',
  'Minimizar',
  'Maximizar',
  'Cerrar',
]

// ================================================================================================

describe('Scenario Outline: Existing mounted states translate after a successful Save (@RF-13)', () => {
  type Phase = 'ready' | 'scanning' | 'analysing' | 'finished' | 'cancelled' | 'error'
  const ROWS3: RowSpec[] = [
    { name: 'A\\a.flac', verdict: 'real' },
    { name: 'A\\b.flac', verdict: 'suspicious', cutoffHz: 19000 },
    { name: 'A\\Canción – 日本語.flac', verdict: 'error', error: 'corruptStream' },
  ]

  /** Brings the English page to `phase`; returns the run (if any) and the expected status texts. */
  async function reach(phase: Phase) {
    await mountPage()
    if (phase === 'ready') return { en: 'Ready', es: 'Listo', run: null, all: [] }
    const run = await startRun()
    if (phase === 'scanning') return { en: 'Scanning…', es: 'Buscando archivos…', run, all: [] }
    const all = listed(ROWS3)
    if (phase === 'finished') {
      await finishRun(run, ROWS3)
      return {
        en: '3 tracks · 1 real · 1 suspicious · 0 fake · 1 error',
        es: '3 pistas · 1 real · 1 sospechosa · 0 falsas · 1 error',
        run,
        all,
      }
    }
    run.scanned(3)
    const feeder = new Feeder(run, all)
    feeder.emit(all.slice(0, 2))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(2))
    if (phase === 'analysing') {
      await vi.waitFor(() => expect(statusText()).toBe('2/3 · 1 suspicious · 0 fake'))
      return { en: '2/3 · 1 suspicious · 0 fake', es: '2/3 · 1 sospechosa · 0 falsas', run, all, feeder }
    }
    if (phase === 'cancelled') {
      run.cancelled(summaryOf(all.slice(0, 2), { total: 3, done: 2, cancelled: true }))
      await vi.waitFor(() => expect(statusText()).toBe('Cancelled · 2 of 3'))
      return { en: 'Cancelled · 2 of 3', es: 'Cancelado · 2 de 3', run, all }
    }
    run.fail('INTERNAL')
    const en = 'The analysis stopped because of an unexpected error'
    await vi.waitFor(() => expect(statusText()).toBe(en))
    return { en, es: 'El análisis se ha detenido debido a un error inesperado', run, all }
  }

  it.each<Phase>(['ready', 'scanning', 'analysing', 'finished', 'cancelled', 'error'])(
    '%s: English -> Spanish -> English, with the run, rows, counts and phase preserved',
    async (phase) => {
      const state = await reach(phase)
      expect(settingsButton().disabled).toBe(false)
      const files = shownFiles()
      const cancelEnabled = !button('Cancel').disabled
      const before = callCounts()

      await switchToSpanish()
      await vi.waitFor(() => expect(statusText()).toBe(state.es))
      expect(shownFiles()).toEqual(files)
      expect(!button('Cancelar').disabled).toBe(cancelEnabled)
      expect(screen.getByRole('toolbar', { name: 'Barra de herramientas principal' })).toBeTruthy()
      for (const name of SPANISH_TOOLBAR) expect(screen.getByRole('button', { name })).toBeTruthy()
      const search = screen.getByRole('searchbox', { name: 'Buscar por nombre' }) as HTMLInputElement
      expect(search.placeholder).toBe('Buscar (Ctrl+F)')
      if (phase === 'ready') {
        expect(screen.getByText('Arrastra una carpeta aquí o pulsa Abrir carpeta')).toBeTruthy()
      } else {
        expect(screen.getByRole('columnheader', { name: /^Archivo/ })).toBeTruthy()
        expect(screen.getByRole('listbox', { name: 'Resúmenes de álbumes' })).toBeTruthy()
      }
      if (phase === 'finished') {
        expect(screen.getByText('Álbumes (1)')).toBeTruthy()
        expect(within(items()[0]).getByText('3 pistas · 1 real · 1 sospechosa · 1 error')).toBeTruthy()
        expect(rowByFile('Canción – 日本語.flac')?.getAttribute('title')).toBe('Flujo de audio dañado')
      }
      expectNoLeftovers('es')
      expect(callCounts()).toEqual(before)

      // subsequent render cycles use the active language
      if (phase === 'analysing' && state.run && 'feeder' in state && state.feeder) {
        state.feeder.emit([state.all[2]])
        state.run.finish(summaryOf(state.all))
        await vi.waitFor(() =>
          expect(statusText()).toBe('3 pistas · 1 real · 1 sospechosa · 0 falsas · 1 error'),
        )
      }

      await switchToEnglish()
      if (phase !== 'analysing') await vi.waitFor(() => expect(statusText()).toBe(state.en))
      expect(shownFiles()).toEqual(phase === 'analysing' ? [...files, 'Canción – 日本語.flac'] : files)
      expect(button('Open folder')).toBeTruthy()
      expectNoLeftovers('en')
    },
  )
})

// ================================================================================================

describe('Scenario: Language changes name ordering while preserving selection and view state (@RF-13 @RF-08 @RNF-09)', () => {
  it('re-sorts nube, ñandú, nora by the Spanish collator; the selection, filters and layout stay', async () => {
    await finishedAnalysis([
      { name: 'A\\nube.flac' },
      { name: 'A\\ñandú.flac' },
      { name: 'A\\nora.flac' },
      { name: 'B\\x.flac' },
    ])
    await pressVerdict('real')
    await typeSearch('n')
    await vi.waitFor(() => expect(shownFiles()).toHaveLength(3))
    await sortBy('File', 'ascending')
    expect(shownFiles()).toEqual(['ñandú.flac', 'nora.flac', 'nube.flac'])
    await userEvent.click(cells(rowByFile('ñandú.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['ñandú.flac']))
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledTimes(1))

    const top = grid().scrollTop
    // album names are user data, in backend index order
    const albumNames = () => items().map((o) => norm(o.querySelector('[title]')?.textContent))
    const albums = albumNames()
    expect(albums).toEqual(['A', 'B'])
    const layout = heights()
    const pressed = screen.getByRole('button', { name: 'Show genuine tracks' }).getAttribute('aria-pressed')
    const stored = localStorage.getItem('fakeflac.layout.detailHeight')

    await switchToSpanish()
    await vi.waitFor(() => expect(shownFiles()).toEqual(['nora.flac', 'nube.flac', 'ñandú.flac']))
    expect(selectedRows().map(fileOf)).toEqual(['ñandú.flac'])
    expect(grid().scrollTop).toBe(top)
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1)
    expect((screen.getByRole('searchbox', { name: 'Buscar por nombre' }) as HTMLInputElement).value).toBe('n')
    expect(screen.getByRole('button', { name: 'Mostrar pistas reales' }).getAttribute('aria-pressed')).toBe(pressed)
    expect(header('Archivo').getAttribute('aria-sort')).toBe('ascending')
    expect(albumNames()).toEqual(albums)
    expect(heights()).toEqual(layout)
    expect(localStorage.getItem('fakeflac.layout.detailHeight')).toBe(stored)

    // the next arrow navigation uses the new visible order
    grid().focus()
    await fireEvent.keyDown(grid(), { key: 'ArrowUp' })
    await vi.waitFor(() => expect(selectedRows().map(fileOf)).toEqual(['nube.flac']))
    await clickHeader('Archivo')
    expect(header('Archivo').getAttribute('aria-sort')).toBe('descending')
    expect(shownFiles()).toEqual(['ñandú.flac', 'nube.flac', 'nora.flac'])
  })
})

// ================================================================================================

describe('Scenario: Settings can change while progress and a spectrogram are pending (@RF-13)', () => {
  it('keeps applying events, accepts the same spectrogram reply and makes no extra request', async () => {
    await mountPage()
    const spectrogram = deferred<Spectrogram>()
    vi.mocked(api.getSpectrogram).mockReset().mockReturnValue(spectrogram.promise)
    const run = await startRun()
    const all = listed([{ name: 'A\\a.flac' }, { name: 'A\\b.flac' }, { name: 'A\\c.flac' }, { name: 'A\\d.flac' }])
    run.scanned(4)
    const feeder = new Feeder(run, all)
    feeder.emit(all.slice(0, 2))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(2))
    await userEvent.click(cells(rowByFile('a.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledTimes(1))
    const before = callCounts()

    const saves = pendingSetSettings()
    await openSettings()
    await userEvent.selectOptions(languageSelect(), 'es')
    await userEvent.click(dialogButton('Save'))
    await vi.waitFor(() => expect(api.setSettings).toHaveBeenCalledTimes(1))
    feeder.emit([all[2]])
    await vi.waitFor(() => expect(allRows()).toHaveLength(3))
    saves[0].resolve(snapshot('es', 'loaded'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(documentLanguage()).toBe('es')

    feeder.emit([all[3]])
    spectrogram.resolve(makeSpectrogram())
    await vi.waitFor(() => expect(screen.getByRole('img', { name: 'Espectrograma' })).toBeTruthy())
    expect(selectedRows().map(fileOf)).toEqual(['a.flac'])
    run.finish(summaryOf(all))
    await vi.waitFor(() =>
      expect(statusText()).toBe('4 pistas · 4 reales · 0 sospechosas · 0 falsas · 0 errores'),
    )
    expect(dataRows()).toHaveLength(4)
    expect(callCounts()).toEqual(before)
  })
})

// ================================================================================================

describe('Scenario: Existing error and export notices translate without changing user data (@RF-13 @RNF-04 @RNF-09)', () => {
  it('translates the friendly error and the export sentence; names, paths and data unchanged', async () => {
    await finishedAnalysis([
      { name: 'A\\Canción – 日本語.flac', verdict: 'error', error: 'corruptStream' },
      { name: 'A\\b.flac' },
      { name: 'A\\c.flac', verdict: 'fake', cutoffHz: 16000 },
    ])
    vi.mocked(api.exportCsv).mockResolvedValue({ path: 'C:\\Out\\Álbum 日本語.csv', rows: 3 })
    await userEvent.click(button('Export CSV'))
    await vi.waitFor(() =>
      expect(statusText()).toBe(
        '3 tracks · 1 real · 0 suspicious · 1 fake · 1 error · Saved 3 rows to Álbum 日本語.csv',
      ),
    )
    await userEvent.click(cells(rowByFile('Canción – 日本語.flac') as HTMLElement)[1])
    await vi.waitFor(() => expect(screen.getAllByText('Corrupt stream').length).toBeGreaterThan(0))
    const diagnostics = document.body.innerHTML.split('stub').length
    const before = callCounts()

    await switchToSpanish()
    await vi.waitFor(() =>
      expect(statusText()).toBe(
        '3 pistas · 1 real · 0 sospechosas · 1 falsa · 1 error · Se han guardado 3 filas en Álbum 日本語.csv',
      ),
    )
    expect(screen.getAllByText('Flujo de audio dañado').length).toBeGreaterThan(0)
    expect(rowByFile('Canción – 日本語.flac')?.getAttribute('title')).toBe('Flujo de audio dañado')
    expect(screen.queryByText('Corrupt stream')).toBeNull()
    expect(shownFiles()).toContain('Canción – 日本語.flac')
    expect(document.body.innerHTML.split('stub').length).toBe(diagnostics)
    expect(callCounts()).toEqual(before)
  })

  it('a contract error notice of the export translates too', async () => {
    await finishedAnalysis([{ name: 'A\\a.flac' }])
    vi.mocked(api.exportCsv).mockRejectedValue(new api.ApiError({ code: 'EXPORT_FAILED', message: 'os' } as never))
    await userEvent.click(button('Export CSV'))
    await vi.waitFor(() => expect(statusText()).toMatch(/ · Cannot save the CSV file$/))
    await switchToSpanish()
    await vi.waitFor(() => expect(statusText()).toMatch(/ · No se puede guardar el archivo CSV$/))
    expect(statusText()).toMatch(/^1 pista · 1 real · /)
  })
})

// ================================================================================================

describe('Scenario: An analysis failure during a settings save remains an analysis failure (@RF-13 @RNF-04)', () => {
  it('keeps the rows, the error phase and its Spanish text; the saved settings are Spanish', async () => {
    await mountPage()
    const run = await startRun()
    const all = listed([{ name: 'A\\a.flac' }, { name: 'A\\b.flac' }, { name: 'A\\c.flac' }])
    run.scanned(3)
    new Feeder(run, all).emit(all.slice(0, 2))
    await vi.waitFor(() => expect(dataRows()).toHaveLength(2))

    const saves = pendingSetSettings()
    await openSettings()
    await userEvent.selectOptions(languageSelect(), 'es')
    await userEvent.click(dialogButton('Save'))
    await vi.waitFor(() => expect(api.setSettings).toHaveBeenCalledTimes(1))
    run.fail('INTERNAL')
    await vi.waitFor(() =>
      expect(norm(screen.getByRole('status', { hidden: true }).textContent)).toBe(
        'The analysis stopped because of an unexpected error',
      ),
    )
    saves[0].resolve(snapshot('es', 'loaded'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(statusText()).toBe('El análisis se ha detenido debido a un error inesperado')
    expect(dataRows()).toHaveLength(2)
    expect(api.setSettings).toHaveBeenCalledWith({ language: 'es' })
    expect(documentLanguage()).toBe('es')
    expect(button('Abrir carpeta').disabled).toBe(false)
  })
})

// ================================================================================================

describe('Scenario: A cache notice already on screen translates (@RF-13)', () => {
  it('reads "Caché de resultados borrada" with no additional clear_cache command', async () => {
    await mountPage()
    await userEvent.click(button('Clear results cache'))
    await vi.waitFor(() => expect(statusText()).toBe('Ready · Results cache cleared'))
    await switchToSpanish()
    await vi.waitFor(() => expect(statusText()).toBe('Listo · Caché de resultados borrada'))
    expect(api.clearCache).toHaveBeenCalledTimes(1)
  })
})

// ================================================================================================

describe('Scenario: Language changes do not change exported results or detection identity (@RF-13)', () => {
  it('exports in English and in Spanish through the same command, with no language argument', async () => {
    await finishedAnalysis([
      { name: 'A\\real.flac' },
      { name: 'A\\sus.flac', verdict: 'suspicious' },
      { name: 'A\\fake.flac', verdict: 'fake' },
      { name: 'A\\Canción – 日本語.flac', verdict: 'error', error: 'corruptStream' },
    ])
    vi.mocked(api.exportCsv).mockResolvedValue({ path: 'C:\\Out\\a.csv', rows: 4 })
    await userEvent.click(button('Export CSV'))
    await vi.waitFor(() => expect(statusText()).toMatch(/Saved 4 rows to a\.csv$/))
    await switchToSpanish()
    await userEvent.click(button('Exportar CSV'))
    await vi.waitFor(() => expect(api.exportCsv).toHaveBeenCalledTimes(2))
    expect(vi.mocked(api.exportCsv).mock.calls).toEqual([[], []])
    await vi.waitFor(() => expect(statusText()).toMatch(/Se han guardado 4 filas en a\.csv$/))
    expect(vi.mocked(api.startAnalysis).mock.calls).toHaveLength(1)
  })
})

// ================================================================================================

describe('Scenario: Native Windows dialogs retain their system language (@RF-13)', () => {
  it('the app-owned trigger and result messages are Spanish; the dialogs get no language', async () => {
    mockSettings(snapshot('es', 'loaded'))
    await mountPage()
    expect(documentLanguage()).toBe('es')
    vi.mocked(api.pickFolder).mockResolvedValue(null)
    await userEvent.click(button('Abrir carpeta'))
    await vi.waitFor(() => expect(api.pickFolder).toHaveBeenCalledTimes(1))
    expect(vi.mocked(api.pickFolder).mock.calls[0]).toEqual([])
    expect(statusText()).toBe('Listo')
  })
})

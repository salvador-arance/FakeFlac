// ResultsTable in both languages (@RF-13 @RF-08 @RNF-09; specs/ui/translations.md section 3,
// specs/ui/language-settings.md section 3 and acceptance L4/L5). Column headers, the real bitrate
// tooltip, verdict texts, reason and error tooltips, full band and the no-match state translate;
// a locale change re-sorts text columns with the new `Intl.Collator` locale like a re-sort: no
// selection change and no automatic scroll. Scenario "Language changes name ordering while
// preserving selection and view state" (table side).
import { render, screen, within } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import type { ResultRow, SortState } from '../../model'
import { makeRow } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

afterEach(() => setLocale('en'))

const byFile: SortState = { column: 'file', direction: 'asc' }
const dataRows = () =>
  screen.queryAllByRole('row').filter((row) => within(row).queryAllByRole('gridcell').length > 0)
const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')
const fileColumn = () => dataRows().map((row) => cells(row)[1].textContent?.trim())
const headers = () => screen.getAllByRole('columnheader').map((h) => h.textContent?.trim())

function setup(rows: ResultRow[], extra: Record<string, unknown> = {}) {
  const onselect = vi.fn()
  const onsort = vi.fn()
  const view = render(ResultsTable, {
    props: { rows, selectedIndex: null, sort: byFile, onselect, onsort, ...extra },
  })
  return { onselect, onsort, ...view }
}

const SPANISH_HEADERS = [
  'Estado',
  'Archivo',
  'Carpeta',
  'Formato',
  'Frecuencia de muestreo',
  'Bits',
  'Duración',
  'Corte',
  'Tasa de bits',
  'Tasa de bits real',
]
const ENGLISH_HEADERS = [
  'State',
  'File',
  'Folder',
  'Format',
  'Samplerate',
  'Bits',
  'Duration',
  'Cutoff',
  'Bitrate',
  'Real bitrate',
]
const HELP_ES =
  'La tasa de bits real se estima a partir del corte, según el códec del propio archivo. Es posible que no se detecten los archivos procedentes de fuentes con pocos agudos, de codificadores sin filtro de paso bajo o recodificados desde una tasa de bits cercana.'

const MIXED = [
  makeRow(0, { path: 'C:/Music/A/a.flac', verdict: 'real', cutoffHz: 22050 }),
  makeRow(1, { path: 'C:/Music/A/b.flac', verdict: 'suspicious', cutoffHz: 19000, reasons: ['gradualRolloff', 'highBandGaps'] }),
  makeRow(2, { path: 'C:/Music/A/c.flac', verdict: 'fake', cutoffHz: 16000 }),
  makeRow(3, {
    path: 'C:/Music/A/Canción – 日本語.flac',
    verdict: 'error',
    cutoffHz: null,
    bitDepth: null,
    error: { kind: 'corruptStream', detail: 'invalid frame header at byte 4096' },
  }),
]

describe('ResultsTable: Spanish (@RF-13)', () => {
  it('translates the headers and the real bitrate tooltip', () => {
    setLocale('es')
    setup([])
    expect(headers()).toEqual(SPANISH_HEADERS)
    const help = screen.getAllByRole('columnheader')[9]
    expect(help.getAttribute('title')).toBe(HELP_ES)
  })

  it('translates verdicts, reasons, the error tooltip and full band; data unchanged', () => {
    setLocale('es')
    setup(MIXED)
    const rows = dataRows()
    expect(rows.map((r) => cells(r)[0].textContent?.trim())).toEqual(['Real', 'Sospechoso', 'Falso', 'Error'])
    expect(cells(rows[1])[0].querySelector('[title]')?.getAttribute('title')).toBe(
      'Las frecuencias altas se atenúan gradualmente\nHuecos frecuentes por encima de 16 kHz, como en un MP3',
    )
    expect(cells(rows[0])[7].textContent?.trim()).toBe('Banda completa')
    expect(cells(rows[2])[7].textContent?.trim()).toBe('16.0 kHz')
    const errorRow = rows[3]
    const titles = [errorRow, ...cells(errorRow)].map((e) => e.getAttribute('title'))
    expect(titles).toContain('Flujo de audio dañado')
    expect(cells(errorRow)[1].textContent?.trim()).toBe('Canción – 日本語.flac')
    expect(errorRow.textContent).not.toContain('Corrupt stream')
  })

  it('translates the no-match state', () => {
    setLocale('es')
    setup([], { noMatches: true })
    expect(screen.getByText('Ninguna pista coincide con los filtros')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Quitar filtros' })).toBeTruthy()
  })
})

describe('ResultsTable: live locale change (@RF-13)', () => {
  it('re-renders headers and cells while mounted, English -> Spanish -> English', () => {
    setup(MIXED)
    expect(headers()).toEqual(ENGLISH_HEADERS)
    setLocale('es')
    flushSync()
    expect(headers()).toEqual(SPANISH_HEADERS)
    expect(cells(dataRows()[0])[0].textContent?.trim()).toBe('Real')
    expect(cells(dataRows()[0])[7].textContent?.trim()).toBe('Banda completa')
    setLocale('en')
    flushSync()
    expect(headers()).toEqual(ENGLISH_HEADERS)
    expect(cells(dataRows()[0])[0].textContent?.trim()).toBe('Genuine')
  })

  // @RF-13 @RF-08 the order follows the collator of the active language; selection is kept
  it('re-sorts nube, ñandú, nora with the new collator, keeping the selection and the scroll', () => {
    const rows = [
      makeRow(0, { path: 'C:/Music/A/nube.flac' }),
      makeRow(1, { path: 'C:/Music/A/ñandú.flac' }),
      makeRow(2, { path: 'C:/Music/A/nora.flac' }),
    ]
    const { onselect, onsort } = setup(rows, { selectedIndex: 1 })
    expect(fileColumn()).toEqual(['ñandú.flac', 'nora.flac', 'nube.flac'])
    const grid = screen.getByRole('grid')
    const top = grid.scrollTop
    setLocale('es')
    flushSync()
    expect(fileColumn()).toEqual(['nora.flac', 'nube.flac', 'ñandú.flac'])
    const selected = dataRows().filter((r) => r.getAttribute('aria-selected') === 'true')
    expect(selected.map((r) => cells(r)[1].textContent?.trim())).toEqual(['ñandú.flac'])
    expect(grid.scrollTop).toBe(top)
    expect(onselect).not.toHaveBeenCalled()
    expect(onsort).not.toHaveBeenCalled()
    setLocale('en')
    flushSync()
    expect(fileColumn()).toEqual(['ñandú.flac', 'nora.flac', 'nube.flac'])
  })
})

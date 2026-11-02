// i18n texts of sort and filter (specs/ui/sort-filter.md section 12, @RF-08).
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale, t } from './index'

afterEach(() => setLocale('en'))

const TEXTS: [string, string][] = [
  ['filter.label', 'Filter by verdict'],
  ['filter.fake', 'Show fake tracks'],
  ['filter.suspicious', 'Show suspicious tracks'],
  ['filter.real', 'Show genuine tracks'],
  ['filter.error', 'Show tracks with errors'],
  ['search.label', 'Search by name'],
  ['search.placeholder', 'Search (Ctrl+F)'],
  ['search.clear', 'Clear search'],
  ['table.noMatches', 'No tracks match the filters'],
  ['table.clearFilters', 'Clear filters'],
]

describe('sort and filter texts (@RF-08)', () => {
  it.each(TEXTS)('en: %s', (key, text) => {
    expect(t(key as never)).toBe(text)
  })

  it('en: status.shown fills n', () => {
    expect(t('status.shown' as never, { n: 3 })).toBe('3 shown')
  })

  it('en: status.shownOne', () => {
    expect(t('status.shownOne' as never)).toBe('1 shown')
  })

  // @RF-13 translations.md sections 4 and 5: explicit Spanish texts
  it.each([
    ['filter.label', 'Filtrar por veredicto'],
    ['filter.fake', 'Mostrar pistas falsas'],
    ['filter.suspicious', 'Mostrar pistas sospechosas'],
    ['filter.real', 'Mostrar pistas reales'],
    ['filter.error', 'Mostrar pistas con errores'],
    ['search.label', 'Buscar por nombre'],
    ['search.placeholder', 'Buscar (Ctrl+F)'],
    ['search.clear', 'Borrar búsqueda'],
    ['table.noMatches', 'Ninguna pista coincide con los filtros'],
    ['table.clearFilters', 'Quitar filtros'],
  ])('es: %s', (key, text) => {
    setLocale('es')
    expect(t(key as never)).toBe(text)
  })

  it('es: status.shown and status.shownOne', () => {
    setLocale('es')
    expect(t('status.shown' as never, { n: 0 })).toBe('0 mostradas')
    expect(t('status.shown' as never, { n: 3 })).toBe('3 mostradas')
    expect(t('status.shownOne' as never)).toBe('1 mostrada')
  })
})

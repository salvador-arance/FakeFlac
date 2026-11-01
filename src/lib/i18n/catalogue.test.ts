// The complete English and Spanish catalogue (@RF-13 @RNF-10; specs/ui/translations.md, acceptance
// L1 of specs/ui/language-settings.md). Scenario "The two catalogues cover all user-facing states
// explicitly" of features/language-settings.feature.
//
// The normative key tables are read from specs/ui/translations.md itself, so every row of the
// spec is checked: `| \`key\` | English | Spanish |`.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "Explicit" is observed through `t()`: in Spanish every key gives its Spanish cell, so no key
//   whose texts differ can come from the English fallback. Keys whose two texts are identical
//   (`app.title`, `settings.english`, ...) cannot be told apart from a fallback by behaviour.
// - The placeholder multiset is checked by interpolating a distinct marker per parameter: each
//   marker must appear exactly as often as its `{name}` in the spec cell, and no brace remains.
// - "Substituted literally once per occurrence" includes replacement-pattern characters (`$&`,
//   `$1`) and a value that itself looks like a placeholder.
import { afterEach, describe, expect, it } from 'vitest'
import translations from '../../../specs/ui/translations.md?raw'
import { getLocale, setLocale, t } from './index'

afterEach(() => setLocale('en'))

type Row = { key: string; en: string; es: string }

const ROWS: Row[] = translations
  .split(/\r?\n/)
  .map((line) => line.match(/^\|\s*`([^`]+)`\s*\|(.*)\|(.*)\|\s*$/))
  .filter((m): m is RegExpMatchArray => m !== null)
  .map((m) => ({ key: m[1], en: m[2].trim(), es: m[3].trim() }))

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort()
const marker = (name: string) => `⟦${name}⟧`
const paramsOf = (row: Row) =>
  Object.fromEntries([...new Set(placeholders(row.en))].map((p) => [p, marker(p)]))
const filled = (text: string, params: Record<string, string>) =>
  text.replace(/\{(\w+)\}/g, (whole, name: string) => params[name] ?? whole)

describe('the normative tables of translations.md', () => {
  it('are parsed: every section, unique keys', () => {
    expect(ROWS.length).toBeGreaterThan(100)
    const keys = ROWS.map((r) => r.key)
    expect(new Set(keys).size).toBe(keys.length)
    for (const key of [
      'app.title',
      'settings.saveFailed',
      'error.SETTINGS_READ_ONLY',
      'column.state',
      'albums.errorsOne',
      'status.exportedOne',
    ]) {
      expect(keys).toContain(key)
    }
    for (const row of ROWS) {
      expect(row.en, row.key).not.toBe('')
      expect(row.es, row.key).not.toBe('')
    }
  })

  // "both texts have the specified identical placeholder multiset"
  it.each(ROWS.map((r) => [r.key, r] as const))('%s: English and Spanish share the placeholders', (_key, row) => {
    expect(placeholders(row.es)).toEqual(placeholders(row.en))
  })
})

describe('the English catalogue (@RF-13)', () => {
  it.each(ROWS.map((r) => [r.key, r] as const))('en: %s', (key, row) => {
    setLocale('en')
    const params = paramsOf(row)
    expect(t(key as never, params)).toBe(filled(row.en, params))
  })
})

describe('the Spanish catalogue (@RF-13)', () => {
  // "each key has explicit English and Spanish text"; "no supported Spanish key depends on the
  // English fallback"
  it.each(ROWS.map((r) => [r.key, r] as const))('es: %s', (key, row) => {
    setLocale('es')
    expect(getLocale()).toBe('es')
    const params = paramsOf(row)
    expect(t(key as never, params)).toBe(filled(row.es, params))
  })
})

describe('interpolation', () => {
  it('substitutes every occurrence literally, once, in both languages', () => {
    const file = 'C:\\Out\\{rows} $& $1 $$ <b>Álbum 日本語</b>.csv'
    setLocale('en')
    expect(t('status.exported' as never, { rows: 3, file })).toBe(`Saved 3 rows to ${file}`)
    expect(t('status.exportedOne' as never, { file })).toBe(`Saved 1 row to ${file}`)
    setLocale('es')
    expect(t('status.exported' as never, { rows: 3, file })).toBe(`Se han guardado 3 filas en ${file}`)
    expect(t('status.exportedOne' as never, { file })).toBe(`Se ha guardado 1 fila en ${file}`)
  })

  it('fills several placeholders of one text', () => {
    setLocale('es')
    expect(t('status.cancelled' as never, { done: 1, total: 3 })).toBe('Cancelado · 1 de 3')
    expect(t('albums.partial' as never, { done: 4, files: 12 })).toBe('4 de 12 pistas')
  })

  it('keeps plain digits and the decimal point in Spanish (translations.md section 3)', () => {
    setLocale('es')
    expect(t('detail.cutoff' as never, { khz: '16.5' })).toBe('Corte 16.5 kHz')
    expect(t('status.tracks' as never, { n: 10000 })).toBe('10000 pistas')
  })
})

describe('unknown keys (L1: the existing last-resort behaviour)', () => {
  it.each(['en', 'es'] as const)('%s: an unknown key is shown as the key', (locale) => {
    setLocale(locale)
    expect(t('no.such.key' as never)).toBe('no.such.key')
  })
})

describe('locale switching', () => {
  it('switches English -> Spanish -> English', () => {
    expect(getLocale()).toBe('en')
    expect(t('toolbar.settings' as never)).toBe('Settings')
    setLocale('es')
    expect(t('toolbar.settings' as never)).toBe('Ajustes')
    setLocale('en')
    expect(t('toolbar.settings' as never)).toBe('Settings')
  })
})

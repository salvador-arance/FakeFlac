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

  // Spanish falls back to English until RF-13
  it('es falls back to English', () => {
    setLocale('es')
    for (const [key, text] of TEXTS) expect(t(key as never)).toBe(text)
    expect(t('status.shown' as never, { n: 0 })).toBe('0 shown')
  })
})

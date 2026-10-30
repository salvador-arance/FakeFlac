// i18n texts of the result cache (specs/ui/result-cache.md section 5, RF-11). English is the
// default; Spanish falls back to English until RF-13.
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale, t } from './index'

afterEach(() => setLocale('en'))

describe('result cache texts (@RF-11)', () => {
  it.each([
    ['toolbar.clearCache', 'Clear results cache'],
    ['status.cacheDegraded', 'Results cache error: some results were not saved'],
    ['status.cacheUnavailable', 'Results cache unavailable'],
    ['status.cacheCleared', 'Results cache cleared'],
    ['error.CACHE_FAILED', 'Cannot clear the results cache'],
  ])('en: %s', (key, text) => {
    expect(t(key as never)).toBe(text)
  })

  // No plural form: "1 from cache" reads correctly (section 5)
  it.each([
    [0, '0 from cache'],
    [1, '1 from cache'],
    [31, '31 from cache'],
  ])('en: status.fromCache with n = %i', (n, text) => {
    expect(t('status.fromCache' as never, { n })).toBe(text)
  })

  it('es falls back to English', () => {
    setLocale('es')
    expect(t('toolbar.clearCache' as never)).toBe('Clear results cache')
    expect(t('status.fromCache' as never, { n: 2 })).toBe('2 from cache')
    expect(t('error.CACHE_FAILED' as never)).toBe('Cannot clear the results cache')
  })
})

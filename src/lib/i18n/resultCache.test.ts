// i18n texts of the result cache (specs/ui/result-cache.md section 5, RF-11). English is the
// default; the Spanish texts come from specs/ui/translations.md (RF-13).
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

  // @RF-13 translations.md sections 1, 2 and 5
  it.each([
    ['toolbar.clearCache', 'Borrar caché de resultados'],
    ['status.cacheDegraded', 'Error en la caché de resultados: algunos resultados no se han guardado'],
    ['status.cacheUnavailable', 'Caché de resultados no disponible'],
    ['status.cacheCleared', 'Caché de resultados borrada'],
    ['error.CACHE_FAILED', 'No se puede borrar la caché de resultados'],
  ])('es: %s', (key, text) => {
    setLocale('es')
    expect(t(key as never)).toBe(text)
  })

  // No singular key for status.fromCache in either language (translations.md section 5)
  it.each([
    [0, '0 de la caché'],
    [1, '1 de la caché'],
    [31, '31 de la caché'],
  ])('es: status.fromCache with n = %i', (n, text) => {
    setLocale('es')
    expect(t('status.fromCache' as never, { n })).toBe(text)
  })
})

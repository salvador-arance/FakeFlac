// i18n texts of the CSV export (specs/ui/export-csv.md "Texts", RF-09). The Spanish texts come
// from specs/ui/translations.md (RF-13), which replaces the English fallback.
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale, t } from './index'

afterEach(() => setLocale('en'))

describe('export CSV texts', () => {
  it.each([
    ['toolbar.export', 'Export CSV'],
    ['error.EXPORT_FAILED', 'Cannot save the CSV file'],
    ['error.NOT_ANALYZED', 'There are no results to export'],
  ])('en: %s', (key, text) => {
    expect(t(key as never)).toBe(text)
  })

  it('en: status.exported fills rows and file', () => {
    expect(t('status.exported' as never, { rows: 31, file: 'FakeFlac - Album A.csv' })).toBe(
      'Saved 31 rows to FakeFlac - Album A.csv',
    )
  })

  it('en: status.exportedOne fills the file', () => {
    expect(t('status.exportedOne' as never, { file: 'one.csv' })).toBe('Saved 1 row to one.csv')
  })

  // @RF-13 translations.md sections 1, 2 and 5: explicit Spanish texts
  it.each([
    ['toolbar.export', 'Exportar CSV'],
    ['error.EXPORT_FAILED', 'No se puede guardar el archivo CSV'],
    ['error.NOT_ANALYZED', 'No hay resultados para exportar'],
  ])('es: %s', (key, text) => {
    setLocale('es')
    expect(t(key as never)).toBe(text)
  })

  it('es: status.exported and status.exportedOne', () => {
    setLocale('es')
    expect(t('status.exported' as never, { rows: 31, file: 'FakeFlac - Album A.csv' })).toBe(
      'Se han guardado 31 filas en FakeFlac - Album A.csv',
    )
    expect(t('status.exportedOne' as never, { file: 'one.csv' })).toBe('Se ha guardado 1 fila en one.csv')
  })
})

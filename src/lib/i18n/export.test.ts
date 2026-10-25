// i18n texts of the CSV export (specs/ui/export-csv.md "Texts", RF-09).
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

  // Spanish falls back to English until RF-13
  it('es falls back to English', () => {
    setLocale('es')
    expect(t('toolbar.export' as never)).toBe('Export CSV')
    expect(t('error.EXPORT_FAILED' as never)).toBe('Cannot save the CSV file')
  })
})

// The reason text of rule L6a (specs/ui/lossy-formats.md, "Streaming sample rate (RF-16)";
// specs/features/detection-streaming-source.feature, @RF-16). The Spanish text comes from
// specs/ui/translations.md section 2 (RF-13), which replaces the English fallback of Q28.
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale, t } from './index'

afterEach(() => setLocale('en'))

const STREAMING_TEXT =
  'Full band at 48 kHz: possibly a streaming rip; genuine 48 kHz files look the same.'

describe('i18n: reason.streamingSampleRate', () => {
  // @RF-16 @RF-06 @RF-07 the decided text, exactly (lossy-source.md Q28, 82 characters)
  it('has the decided English text', () => {
    expect(t('reason.streamingSampleRate')).toBe(STREAMING_TEXT)
    expect(STREAMING_TEXT).toHaveLength(82)
  })

  // @RF-13 the explicit Spanish text (translations.md section 2)
  it('has the Spanish text in Spanish', () => {
    setLocale('es')
    expect(t('reason.streamingSampleRate')).toBe(
      'Banda completa a 48 kHz: posible ripeo de streaming; los archivos auténticos a 48 kHz se ven igual.',
    )
  })
})

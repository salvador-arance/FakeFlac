// The reason text of rule L6a (specs/ui/lossy-formats.md, "Streaming sample rate (RF-16)";
// specs/features/detection-streaming-source.feature, @RF-16). English only: the Spanish catalogue
// is not extended before RF-13, so it falls back to English.
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

  // @RF-16 the Spanish catalogue falls back to English for this reason (Q28)
  it('falls back to English in Spanish', () => {
    setLocale('es')
    expect(t('reason.streamingSampleRate')).toBe(STREAMING_TEXT)
  })
})

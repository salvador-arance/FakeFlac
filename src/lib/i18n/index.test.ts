// i18n texts (specs/ui/main-screen.md "Texts"; docs/architecture/frontend.md "Verdict tokens").
// English is the H1 default; Spanish is only required for the verdict texts the docs give.
import { afterEach, describe, expect, it } from 'vitest'
import { getLocale, setLocale, t } from './index'

afterEach(() => setLocale('en'))

const EN: Record<string, string> = {
  'app.title': 'FakeFlac',
  'toolbar.open': 'Open folder',
  'toolbar.cancel': 'Cancel',
  'empty.hint': 'Drop a folder here or click Open folder',
  'status.ready': 'Ready',
  'status.scanning': 'Scanning…',
  'verdict.real': 'Genuine',
  'verdict.suspicious': 'Suspicious',
  'verdict.fake': 'Fake',
  'verdict.error': 'Error',
  'reason.lowHighFrequencyContent': 'Little high-frequency content: cannot confirm',
  'reason.gradualRolloff': 'High frequencies fade out gradually',
  'reason.ambiguousCutoff': 'Sharp cutoff at a high frequency',
  'reason.highBandGaps': 'Frequent gaps above 16 kHz, as in an MP3',
  'error.tooShort': 'Too short to analyse',
  'error.corruptStream': 'Corrupt stream',
  'error.unreadable': 'Cannot read the file',
  'error.unsupportedFormat': 'Unsupported audio format',
  'column.bitrate': 'Bitrate',
  'column.realBitrate': 'Real bitrate',
  'bitrate.kbps': '{kbps} kbps',
  'bitrate.vbr': 'VBR',
  'bitrate.abr': 'ABR',
  'bitrate.approx': '≈{kbps} kbps',
  'bitrate.none': '—',
  'format.m4aAlac': 'M4A (ALAC)',
  'detail.declared': '{kbps} kbps',
  'detail.source': '≈{kbps} kbps source',
  'reason.cutoffBelowDeclared': 'Cutoff lower than expected for its bitrate',
  'help.realBitrate':
    "The real bitrate is an estimate from the cutoff, in terms of the file's own codec. Files from sources with little treble, from encoders without a low-pass, or re-encoded from a close bitrate may not be detected.",
  'error.NO_AUDIO_FOUND': 'No supported audio files in this folder',
  'error.FOLDER_UNREADABLE': 'Cannot open this folder',
  'error.ANALYSIS_RUNNING': 'An analysis is already running',
  'error.INTERNAL': 'The analysis stopped because of an unexpected error',
}

describe('i18n', () => {
  it('defaults to English', () => {
    expect(getLocale()).toBe('en')
  })

  // @RF-06 @RNF-10 every text of main-screen.md exists in English
  it.each(Object.entries(EN))('en: %s', (key, text) => {
    expect(t(key)).toBe(text)
  })

  // frontend.md verdict table (es / en): Real, Sospechoso, Falso, Error
  it.each([
    ['verdict.real', 'Real'],
    ['verdict.suspicious', 'Sospechoso'],
    ['verdict.fake', 'Falso'],
    ['verdict.error', 'Error'],
  ])('es: %s', (key, text) => {
    setLocale('es')
    expect(getLocale()).toBe('es')
    expect(t(key)).toBe(text)
  })

  it('switches back to English', () => {
    setLocale('es')
    setLocale('en')
    expect(t('verdict.real')).toBe('Genuine')
  })
})

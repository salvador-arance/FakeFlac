// i18n: English is the default locale (specs/ui/main-screen.md, "Texts").
export type Locale = 'en' | 'es'
export type MessageKey = string

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
  'error.NO_AUDIO_FOUND': 'No WAV or FLAC files in this folder',
  'error.FOLDER_UNREADABLE': 'Cannot open this folder',
  'error.ANALYSIS_RUNNING': 'An analysis is already running',
  'error.INTERNAL': 'The analysis stopped because of an unexpected error',
}

const ES: Record<string, string> = {
  ...EN,
  'verdict.real': 'Real',
  'verdict.suspicious': 'Sospechoso',
  'verdict.fake': 'Falso',
  'verdict.error': 'Error',
}

const catalogs: Record<Locale, Record<string, string>> = {
  en: EN,
  es: ES,
}

let currentLocale: Locale = 'en'

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const dict = catalogs[currentLocale] ?? EN
  let val = dict[key] ?? EN[key] ?? key
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      val = val.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v))
    }
  }
  return val
}

export function setLocale(locale: Locale): void {
  currentLocale = locale
}

export function getLocale(): Locale {
  return currentLocale
}

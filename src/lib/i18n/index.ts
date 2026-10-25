// i18n: English is the default locale (specs/ui/main-screen.md, "Texts").
export type Locale = 'en' | 'es'
export type MessageKey = string

const EN: Record<string, string> = {
  'app.title': 'FakeFlac',
  'toolbar.open': 'Open folder',
  'toolbar.cancel': 'Cancel',
  'toolbar.export': 'Export CSV',
  'empty.hint': 'Drop a folder here or click Open folder',
  'status.ready': 'Ready',
  'status.scanning': 'Scanning…',
  'status.exported': 'Saved {rows} rows to {file}',
  'status.exportedOne': 'Saved 1 row to {file}',
  'verdict.real': 'Genuine',
  'verdict.suspicious': 'Suspicious',
  'verdict.fake': 'Fake',
  'verdict.error': 'Error',
  'reason.lowHighFrequencyContent': 'Little high-frequency content: cannot confirm',
  'reason.gradualRolloff': 'High frequencies fade out gradually',
  'reason.ambiguousCutoff': 'Sharp cutoff at a high frequency',
  'reason.highBandGaps': 'Frequent gaps above 16 kHz, as in an MP3',
  'reason.cutoffBelowDeclared': 'Cutoff lower than expected for its bitrate',
  'error.tooShort': 'Too short to analyse',
  'error.corruptStream': 'Corrupt stream',
  'error.unreadable': 'Cannot read the file',
  'error.unsupportedFormat': 'Unsupported audio format',
  'error.NO_AUDIO_FOUND': 'No supported audio files in this folder',
  'error.FOLDER_UNREADABLE': 'Cannot open this folder',
  'error.ANALYSIS_RUNNING': 'An analysis is already running',
  'error.EXPORT_FAILED': 'Cannot save the CSV file',
  'error.NOT_ANALYZED': 'There are no results to export',
  'error.INTERNAL': 'The analysis stopped because of an unexpected error',
  'detail.cutoff': 'Cutoff {khz} kHz',
  'detail.declared': '{kbps} kbps',
  'detail.source': '≈{kbps} kbps source',
  // specs/ui/lossy-formats.md, "Texts"
  'column.bitrate': 'Bitrate',
  'column.realBitrate': 'Real bitrate',
  'bitrate.kbps': '{kbps} kbps',
  'bitrate.vbr': 'VBR',
  'bitrate.abr': 'ABR',
  'bitrate.approx': '≈{kbps} kbps',
  'bitrate.none': '—',
  'bitrate.noneLabel': 'No bitrate',
  'format.m4aAlac': 'M4A (ALAC)',
  'help.realBitrate':
    "The real bitrate is an estimate from the cutoff, in terms of the file's own codec. Files from sources with little treble, from encoders without a low-pass, or re-encoded from a close bitrate may not be detected.",
  // specs/ui/album-summary.md, "Texts"
  'albums.title': 'Albums',
  'albums.titleCount': 'Albums ({count})',
  'albums.label': 'Album summaries',
  'albums.tracks': '{n} tracks',
  'albums.tracksOne': '1 track',
  'albums.partial': '{done} of {files} tracks',
  'albums.real': '{n} real',
  'albums.suspicious': '{n} suspicious',
  'albums.fake': '{n} fake',
  'albums.errors': '{n} errors',
  'albums.errorsOne': '1 error',
  'spectrogram.label': 'Spectrogram',
  'spectrogram.loading': 'Loading spectrogram…',
  'spectrogram.unavailable': 'Cannot show the spectrogram of this file',
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

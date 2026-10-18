// i18n: English is the default locale (specs/ui/main-screen.md, "Texts").
export type Locale = 'en' | 'es'
export type MessageKey = string

export function t(_key: MessageKey, _params?: Record<string, string | number>): string {
  throw new Error('not implemented')
}

export function setLocale(_locale: Locale): void {
  throw new Error('not implemented')
}

export function getLocale(): Locale {
  throw new Error('not implemented')
}

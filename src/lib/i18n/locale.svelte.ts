// The reactive locale (specs/ui/language-settings.md section 1): one `$state` drives every mounted
// text. `t()` and `getLocale()` read it, so any template, `$derived` or `$effect` that calls them
// re-renders when `setLocale()` changes the language; no remount is needed.
import { EN, ES } from './messages'
import type { Locale } from './messages'

export type { Locale } from './messages'
export type MessageKey = string
export type MessageParams = Record<string, string | number>

const catalogs: Record<Locale, Record<string, string>> = { en: EN, es: ES }

// English is the default until the page applies the settings snapshot.
let current = $state<Locale>('en')

const PLACEHOLDER = /\{(\w+)\}/g

/**
 * The text of `key` in the active locale, with each `{name}` replaced by `params[name]`. The
 * substitution is literal and happens once per occurrence: a value is never parsed again, so `$&`
 * or `{rows}` inside a value stay as they are (translations.md). Unknown keys fall back to English,
 * then to the key itself (a defensive development behaviour).
 */
export function t(key: MessageKey, params?: MessageParams): string {
  const text = catalogs[current][key] ?? EN[key] ?? key
  if (!params) return text
  return text.replace(PLACEHOLDER, (whole, name: string) =>
    Object.hasOwn(params, name) ? String(params[name]) : whole,
  )
}

export function setLocale(locale: Locale): void {
  current = locale
}

export function getLocale(): Locale {
  return current
}

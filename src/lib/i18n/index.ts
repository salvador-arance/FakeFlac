// i18n (specs/ui/translations.md, specs/ui/language-settings.md): English is the default locale
// until the settings load; the page switches it with `setLocale`. The locale is reactive (runes),
// so it lives in a `.svelte.ts` module and is re-exported here.
export { getLocale, setLocale, t } from './locale.svelte'
export type { Locale, MessageKey, MessageParams } from './locale.svelte'

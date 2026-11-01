// Shared settings steps and locators of the AnalysisPage tests (@RF-13; specs/ui/language-settings.md).
// Lives under `tests/` so it is a protected test file (scripts/test-paths.mjs).
//
// Each test file that uses it mocks `../api` with `vi.mock` (the factory must sit in the test file)
// and lists `getSettings` and `setSettings` in it; the mocked module is the one imported here.
//
// Interface of the page assumed here (specs/ui/language-settings.md sections 1, 2 and 4):
// - On mount the page calls `getSettings()` once before enabling folder-open/drop, export,
//   clear-cache or Settings. While it is pending it shows the title bar and a busy placeholder: an
//   element with `aria-busy="true"` whose accessible name is "FakeFlac" (`aria-label`, or
//   `aria-labelledby` pointing at an element whose text is "FakeFlac").
// - "Initialized" means: that placeholder is gone and the toolbar is rendered.
// - The document language is `document.documentElement.lang`.
// - Locale isolation: `mockSettings()` (called in `beforeEach`) puts the i18n locale back to
//   English and clears the document language, so no test depends on the order of the others.
import { screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { expect, vi } from 'vitest'
import type { Language, SettingsLoadStatus, SettingsSnapshot } from '../../api'
import * as api from '../../api'
import { setLocale } from '../../i18n'

export const snapshot = (language: Language, loadStatus: SettingsLoadStatus): SettingsSnapshot => ({
  settings: { language },
  loadStatus,
})

/** No settings file and an English (or unsupported) Windows: the silent first-launch default. */
export const EN_DEFAULT = snapshot('en', 'default')

export const WARNING = {
  invalid: 'Saved settings could not be read. English is in use. Save to replace them.',
  unavailable: 'Settings are unavailable. English is in use. You can try saving again.',
  newerVersion:
    'These settings were saved by a newer app version. English is in use; these settings will be kept.',
} as const

export const WARNING_ES = {
  invalid:
    'No se han podido interpretar los ajustes guardados. Se está usando el inglés. Guarda para sustituirlos.',
  unavailable:
    'Los ajustes no están disponibles. Se está usando el inglés. Puedes volver a intentar guardarlos.',
  newerVersion:
    'Estos ajustes se guardaron con una versión más reciente de la aplicación. Se está usando el inglés y se conservarán los ajustes.',
} as const

export const SAVE_FAILED = 'Cannot save the language preference. Try again.'
export const READ_ONLY = 'These settings belong to a newer app version and cannot be replaced.'

/** Resets the locale, the document language and both settings mocks (getSettings answers `initial`). */
export function mockSettings(initial: SettingsSnapshot = EN_DEFAULT) {
  setLocale('en')
  document.documentElement.removeAttribute('lang')
  vi.mocked(api.getSettings).mockReset().mockResolvedValue(initial)
  vi.mocked(api.setSettings)
    .mockReset()
    .mockImplementation(async (settings) => snapshot(settings.language, 'loaded'))
}

/** A controllable promise for `getSettings` or `setSettings`. */
export function deferred<T>() {
  let resolve: (value: T) => void = () => {}
  let reject: (error: unknown) => void = () => {}
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  promise.catch(() => {})
  return { promise, resolve, reject }
}

/** The next `getSettings` call stays pending until the test settles it. */
export function pendingGetSettings() {
  const d = deferred<SettingsSnapshot>()
  vi.mocked(api.getSettings).mockReset().mockReturnValue(d.promise)
  return d
}

/** The next `setSettings` calls stay pending until the test settles them (one deferred per call). */
export function pendingSetSettings() {
  const calls: ReturnType<typeof deferred<SettingsSnapshot>>[] = []
  vi.mocked(api.setSettings)
    .mockReset()
    .mockImplementation(() => {
      const d = deferred<SettingsSnapshot>()
      calls.push(d)
      return d.promise
    })
  return calls
}

const textOf = (id: string) => document.getElementById(id)?.textContent?.trim() ?? ''

/** The language-neutral startup placeholder, or null. */
export function busyPlaceholder(): HTMLElement | null {
  for (const el of Array.from(document.querySelectorAll<HTMLElement>('[aria-busy="true"]'))) {
    const label = el.getAttribute('aria-label')
    const labelledBy = el.getAttribute('aria-labelledby')
    const name = label ?? (labelledBy ? labelledBy.split(/\s+/).map(textOf).join(' ') : '')
    if (name.trim() === 'FakeFlac') return el
  }
  return null
}

/**
 * Waits until the ordinary main screen is shown: no startup placeholder, the toolbar rendered.
 * It does not require the `getSettings` call itself (AnalysisPage.settings.test.ts asserts it), so
 * the earlier page tests that only need an initialized screen do not depend on it.
 */
export async function waitForMainScreen() {
  await vi.waitFor(() => {
    expect(busyPlaceholder()).toBeNull()
    expect(screen.queryByRole('toolbar')).not.toBeNull()
  })
}

export const documentLanguage = () => document.documentElement.getAttribute('lang')

// ---- the panel ---------------------------------------------------------------------------------

export const settingsButton = (name = 'Settings') =>
  screen.getByRole('button', { name }) as HTMLButtonElement
export const dialog = () => screen.queryByRole('dialog')
export const openDialog = () => {
  const d = dialog()
  if (!d) throw new Error('the Settings dialog is not open')
  return d
}
export const languageSelect = () => within(openDialog()).getByRole('combobox') as HTMLSelectElement
export const dialogButton = (name: string) =>
  within(openDialog()).getByRole('button', { name }) as HTMLButtonElement
export const dialogBusy = () => {
  const d = openDialog()
  return d.getAttribute('aria-busy') === 'true' || d.querySelector('[aria-busy="true"]') !== null
}

/** Opens Settings with the toolbar button (named in the active language). */
export async function openSettings(buttonName = 'Settings') {
  await userEvent.click(settingsButton(buttonName))
  await vi.waitFor(() => expect(dialog()).not.toBeNull())
}

/** Opens Settings, selects `language` and presses Save (button names in the active language). */
export async function saveLanguage(
  language: Language,
  names: { settings: string; save: string } = { settings: 'Settings', save: 'Save' },
) {
  await openSettings(names.settings)
  await userEvent.selectOptions(languageSelect(), language)
  await userEvent.click(dialogButton(names.save))
}

/** Saves Spanish from an English screen and waits until the Spanish screen is applied. */
export async function switchToSpanish() {
  const before = vi.mocked(api.setSettings).mock.calls.length
  await saveLanguage('es')
  await vi.waitFor(() => expect(vi.mocked(api.setSettings).mock.calls.length).toBe(before + 1))
  await vi.waitFor(() => {
    expect(dialog()).toBeNull()
    expect(documentLanguage()).toBe('es')
  })
}

/** Saves English from a Spanish screen and waits until the English screen is applied. */
export async function switchToEnglish() {
  const before = vi.mocked(api.setSettings).mock.calls.length
  await saveLanguage('en', { settings: 'Ajustes', save: 'Guardar' })
  await vi.waitFor(() => expect(vi.mocked(api.setSettings).mock.calls.length).toBe(before + 1))
  await vi.waitFor(() => {
    expect(dialog()).toBeNull()
    expect(documentLanguage()).toBe('en')
  })
}

/** True when `el` cannot receive user input: inside an `inert` subtree. */
export const isInert = (el: Element) => el.closest('[inert]') !== null

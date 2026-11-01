// AnalysisPage: language settings (@RF-13 @RNF-04 @RNF-08; specs/ui/language-settings.md sections 1,
// 2 and 4, acceptance L3 and L6). The API client is mocked: the page is the only component that
// talks to it. Scenarios of features/language-settings.feature, page side:
// - "First launch detects Windows language without creating a file" (the snapshot the backend
//   returns: the main screen and the document language use it)
// - "Restore a saved language before enabling the main screen"
// - "An explicit Save survives a new app process"
// - "Dismissing the draft does not persist it"
// - "A save already in progress cannot be duplicated or dismissed"
// - "A failed save preserves the old preference and permits retry"
// - "Invalid persisted settings fall back without modifying the file"
// - "An unreadable configuration does not prevent analysis"
// - "A newer settings version is preserved"
// - "A newer file appearing after startup is not overwritten"
// - "The language modal supports keyboard-only use"
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - Startup interface: see tests/settingsHarness.ts (busy placeholder named "FakeFlac"). "No
//   English main-screen flash" is checked on the visible text: neither "Ready" nor the drop hint
//   (nor their Spanish texts) is shown before the read settles; any analysis/Settings button that
//   is already rendered is disabled.
// - The settings warning of invalid/unavailable/newer is a status-bar notice: the status text
//   contains the catalogue sentence. A successful Save removes it.
// - Background suppression: a background control that is not inside an `inert` subtree must not
//   react to a click (jsdom does not implement `inert`, so an inert control is accepted as is);
//   Ctrl+F and folder drops are dispatched as usual and must do nothing. Queries that may reach a
//   background hidden from the accessibility tree use `{ hidden: true }`.
// - "A new app process reads ..." is the page side: a new mount whose `getSettings` answers the
//   snapshot the save returned shows that language.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Language } from '../api'
import * as api from '../api'
import AnalysisPage from './AnalysisPage.svelte'
import {
  Feeder,
  HINT,
  listed,
  mountPage,
  norm,
  resetApiMocks,
  startRun,
  statusText,
} from './tests/sortFilterHarness'
import {
  READ_ONLY,
  SAVE_FAILED,
  WARNING,
  WARNING_ES,
  busyPlaceholder,
  dialog,
  dialogBusy,
  dialogButton,
  documentLanguage,
  isInert,
  languageSelect,
  mockSettings,
  openDialog,
  openSettings,
  pendingGetSettings,
  pendingSetSettings,
  saveLanguage,
  settingsButton,
  snapshot,
  waitForMainScreen,
} from './tests/settingsHarness'

vi.mock('../api', () => {
  class ApiError extends Error {
    code: string
    constructor(error: { code: string; message: string }) {
      super(error.message)
      this.name = 'ApiError'
      this.code = error.code
    }
  }
  return {
    ApiError,
    pickFolder: vi.fn(),
    startAnalysis: vi.fn(),
    startAnalysisOfDropped: vi.fn(),
    cancelAnalysis: vi.fn(),
    getSpectrogram: vi.fn(),
    exportCsv: vi.fn(),
    clearCache: vi.fn(),
    onDragDrop: vi.fn(),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    getSettings: vi.fn(),
    setSettings: vi.fn(),
  }
})

let dragHandler: (event: api.DragDropEvent) => void = () => {}

beforeEach(() => {
  resetApiMocks()
  mockSettings()
  vi.mocked(api.clearCache).mockReset().mockResolvedValue(undefined)
  dragHandler = () => {}
  vi.mocked(api.onDragDrop)
    .mockReset()
    .mockImplementation(async (handler) => {
      dragHandler = handler
      return () => {}
    })
})

afterEach(() => {
  vi.useRealTimers()
})

const apiError = (code: string) => new api.ApiError({ code, message: 'technical detail C:\\x' } as never)

/** Mounts with `getSettings` answering `language`/`status` and waits for the main screen. */
async function mountWith(language: Language, status: 'default' | 'loaded' | 'invalid' | 'unavailable' | 'newerVersion') {
  vi.mocked(api.getSettings).mockReset().mockResolvedValue(snapshot(language, status))
  return mountPage()
}

const NAMES = {
  en: { open: 'Open folder', settings: 'Settings', save: 'Save', ready: 'Ready', hint: HINT },
  es: {
    open: 'Abrir carpeta',
    settings: 'Ajustes',
    save: 'Guardar',
    ready: 'Listo',
    hint: 'Arrastra una carpeta aquí o pulsa Abrir carpeta',
  },
} as const

const queryButton = (name: string) =>
  screen.queryByRole('button', { name, hidden: true }) as HTMLButtonElement | null
const button = (name: string) => screen.getByRole('button', { name, hidden: true }) as HTMLButtonElement
const anyStatusText = () => norm(screen.getByRole('status', { hidden: true }).textContent)
const allRows = () =>
  screen
    .queryAllByRole('row', { hidden: true })
    .filter((r) => within(r).queryAllByRole('gridcell', { hidden: true }).length > 0)

function expectMainScreen(language: Language) {
  const n = NAMES[language]
  expect(button(n.open).disabled).toBe(false)
  expect(settingsButton(n.settings).disabled).toBe(false)
  expect(statusText()).toBe(n.ready)
  expect(screen.getByText(n.hint)).toBeTruthy()
  expect(documentLanguage()).toBe(language)
}

// ================================================================================================

describe('startup (@RF-13)', () => {
  // Scenario Outline: First launch detects Windows language without creating a file (page side)
  it.each(['en', 'es'] as const)('a default %s snapshot gives that main screen, silently', async (language) => {
    await mountWith(language, 'default')
    expectMainScreen(language)
    for (const warning of [...Object.values(WARNING), ...Object.values(WARNING_ES)]) {
      expect(document.body.textContent).not.toContain(warning)
    }
    expect(api.getSettings).toHaveBeenCalledTimes(1)
    expect(api.setSettings).not.toHaveBeenCalled()
  })

  // Scenario: Restore a saved language before enabling the main screen
  it('holds a language-neutral busy placeholder while the read is pending, then shows Spanish', async () => {
    const read = pendingGetSettings()
    render(AnalysisPage)
    await vi.waitFor(() => expect(api.getSettings).toHaveBeenCalledTimes(1))

    // "analysis and Settings actions are disabled" / "the language-neutral busy placeholder"
    expect(busyPlaceholder()).not.toBeNull()
    for (const name of ['Open folder', 'Abrir carpeta', 'Settings', 'Ajustes', 'Export CSV', 'Clear results cache']) {
      const b = queryButton(name)
      if (b) expect(b.disabled, name).toBe(true)
    }
    const visible = document.body.textContent ?? ''
    for (const text of ['Ready', 'Listo', HINT, NAMES.es.hint]) expect(visible).not.toContain(text)

    // nothing starts early: drops, Ctrl+F and clicks do nothing
    dragHandler({ type: 'drop', paths: ['C:\\Music\\Album A'] })
    await fireEvent.keyDown(window, { key: 'f', code: 'KeyF', ctrlKey: true })
    const open = queryButton('Open folder')
    if (open) await userEvent.click(open)
    expect(api.pickFolder).not.toHaveBeenCalled()
    expect(api.startAnalysis).not.toHaveBeenCalled()
    expect(api.startAnalysisOfDropped).not.toHaveBeenCalled()

    // window controls stay usable with the provisional English labels
    await userEvent.click(button('Minimize'))
    expect(api.minimizeWindow).toHaveBeenCalledTimes(1)
    expect(screen.getByText('FakeFlac')).toBeTruthy()

    read.resolve(snapshot('es', 'loaded'))
    await waitForMainScreen()
    expectMainScreen('es')
    expect(api.getSettings).toHaveBeenCalledTimes(1)
  })

  it.each([
    ['an ApiError', () => apiError('INTERNAL')],
    ['a plain rejection', () => 'ipc failed'],
  ])('an unexpected getSettings failure (%s) gives English with the unavailable warning', async (_label, make) => {
    vi.mocked(api.getSettings).mockReset().mockRejectedValue(make())
    await mountPage()
    expect(documentLanguage()).toBe('en')
    expect(button('Open folder').disabled).toBe(false)
    expect(statusText()).toContain(WARNING.unavailable)
    expect(statusText()).not.toContain('The analysis stopped because of an unexpected error')
  })
})

// ================================================================================================

describe('Scenario Outline: An explicit Save survives a new app process (@RF-13)', () => {
  it.each<[Language, Language]>([
    ['en', 'es'],
    ['es', 'en'],
    ['en', 'en'],
  ])('%s -> %s', async (before, after) => {
    const view = await mountWith(before, 'loaded')
    await openSettings(NAMES[before].settings)
    await userEvent.selectOptions(languageSelect(), after)
    // "Then the active interface still uses <before>"
    expect(documentLanguage()).toBe(before)
    expect(button(NAMES[before].open)).toBeTruthy()

    await userEvent.click(dialogButton(NAMES[before].save))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(api.setSettings).toHaveBeenCalledTimes(1)
    expect(api.setSettings).toHaveBeenCalledWith({ language: after })
    expectMainScreen(after)
    // "the dialog closes with focus on the translated Settings button"
    expect(document.activeElement).toBe(settingsButton(NAMES[after].settings))

    // "a new app process reads language <after> with loadStatus loaded"
    const saved = await vi.mocked(api.setSettings).mock.results[0].value
    expect(saved).toEqual(snapshot(after, 'loaded'))
    view.unmount()
    mockSettings(saved)
    await mountPage()
    expectMainScreen(after)
  })
})

// ================================================================================================

describe('Scenario Outline: Dismissing the draft does not persist it (@RF-13)', () => {
  it.each(['Cancel', 'Escape'] as const)('%s', async (action) => {
    await mountWith('en', 'loaded')
    await openSettings()
    await userEvent.selectOptions(languageSelect(), 'es')
    if (action === 'Cancel') await userEvent.click(dialogButton('Cancel'))
    else await userEvent.keyboard('{Escape}')
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(api.setSettings).not.toHaveBeenCalled()
    expectMainScreen('en')
    expect(document.activeElement).toBe(settingsButton())

    // the next opening starts again from the active language
    await openSettings()
    expect(languageSelect().value).toBe('en')
  })
})

// ================================================================================================

describe('Scenario: A save already in progress cannot be duplicated or dismissed (@RF-13)', () => {
  it('keeps one request, disables the select and both actions, stays open and busy', async () => {
    await mountWith('en', 'loaded')
    const saves = pendingSetSettings()
    await saveLanguage('es')
    await vi.waitFor(() => expect(api.setSettings).toHaveBeenCalledTimes(1))

    await userEvent.click(dialogButton('Saving…'))
    await userEvent.keyboard('{Escape}')
    await userEvent.click(document.body)
    expect(api.setSettings).toHaveBeenCalledTimes(1)
    expect(languageSelect().disabled).toBe(true)
    expect(dialogButton('Saving…').disabled).toBe(true)
    expect(dialogButton('Cancel').disabled).toBe(true)
    expect(dialog()).not.toBeNull()
    expect(dialogBusy()).toBe(true)
    expect(documentLanguage()).toBe('en')

    saves[0].resolve(snapshot('es', 'loaded'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expectMainScreen('es')
  })
})

// ================================================================================================

describe('Scenario Outline: A failed save preserves the old preference and permits retry (@RF-13 @RNF-04)', () => {
  // Every <operation> of the outline reaches the page as SETTINGS_WRITE_FAILED (the backend side
  // is src-tauri/tests/settings_store.rs); an unexpected failure uses settings.saveFailed.
  it.each([
    ['SETTINGS_WRITE_FAILED', () => apiError('SETTINGS_WRITE_FAILED')],
    ['an unexpected INTERNAL', () => apiError('INTERNAL')],
    ['a plain rejection', () => new Error('ipc failed')],
  ])('%s: the Spanish draft, the localized failure, English kept; the retry saves Spanish', async (_label, make) => {
    await mountWith('en', 'loaded')
    vi.mocked(api.setSettings).mockReset().mockRejectedValueOnce(make())
    await saveLanguage('es')
    const alert = await vi.waitFor(() => within(openDialog()).getByRole('alert'))
    expect(alert.textContent?.trim()).toBe(SAVE_FAILED)
    expect(languageSelect().value).toBe('es')
    expect(languageSelect().disabled).toBe(false)
    expect(documentLanguage()).toBe('en')
    expect(button('Open folder')).toBeTruthy()
    await vi.waitFor(() => expect(document.activeElement).toBe(dialogButton('Save')))
    expect(anyStatusText()).not.toContain('The analysis stopped because of an unexpected error')

    // "When I retry and the save succeeds"
    vi.mocked(api.setSettings).mockResolvedValueOnce(snapshot('es', 'loaded'))
    await userEvent.click(dialogButton('Save'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(api.setSettings).toHaveBeenCalledTimes(2)
    expect(vi.mocked(api.setSettings).mock.calls).toEqual([[{ language: 'es' }], [{ language: 'es' }]])
    expectMainScreen('es')
  })
})

// ================================================================================================

describe('Scenario Outline: Invalid persisted settings fall back without modifying the file (@RF-13 @RNF-04)', () => {
  it('English with the warning; Settings explains it; an explicit Save of Spanish clears it', async () => {
    await mountWith('en', 'invalid')
    expect(documentLanguage()).toBe('en')
    expect(button('Open folder').disabled).toBe(false)
    expect(statusText()).toContain(WARNING.invalid)
    expect(api.setSettings).not.toHaveBeenCalled()

    await openSettings()
    expect(openDialog().textContent).toContain(WARNING.invalid)
    expect(languageSelect().disabled).toBe(false)
    expect(dialogButton('Save').disabled).toBe(false)
    await userEvent.selectOptions(languageSelect(), 'es')
    await userEvent.click(dialogButton('Save'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(api.setSettings).toHaveBeenCalledWith({ language: 'es' })
    expect(documentLanguage()).toBe('es')
    expect(statusText()).toBe('Listo')
  })
})

// ================================================================================================

describe('Scenario Outline: An unreadable configuration does not prevent analysis (@RF-13 @RNF-04)', () => {
  it('English with the warning; folder analysis and its normal errors work; Settings offers Save', async () => {
    await mountWith('en', 'unavailable')
    expect(statusText()).toContain(WARNING.unavailable)

    const run = await startRun()
    expect(api.startAnalysis).toHaveBeenCalledTimes(1)
    run.fail('NO_AUDIO_FOUND')
    // the warning never replaces an analysis error
    await vi.waitFor(() => expect(statusText()).toContain('No supported audio files in this folder'))

    await openSettings()
    expect(openDialog().textContent).toContain(WARNING.unavailable)
    expect(dialogButton('Save').disabled).toBe(false)
    await userEvent.click(dialogButton('Save'))
    await vi.waitFor(() => expect(api.setSettings).toHaveBeenCalledWith({ language: 'en' }))
  })
})

// ================================================================================================

describe('Scenario: A newer settings version is preserved (@RF-13)', () => {
  it('English with the warning; the select and Save disabled; Close enabled and focused', async () => {
    await mountWith('en', 'newerVersion')
    expect(statusText()).toContain(WARNING.newerVersion)
    await openSettings()
    expect(openDialog().textContent).toContain(WARNING.newerVersion)
    expect(languageSelect().disabled).toBe(true)
    expect(dialogButton('Save').disabled).toBe(true)
    expect(dialogButton('Close').disabled).toBe(false)
    await vi.waitFor(() => expect(document.activeElement).toBe(dialogButton('Close')))
    await userEvent.click(dialogButton('Close'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(document.activeElement).toBe(settingsButton())
    expect(api.setSettings).not.toHaveBeenCalled()
  })
})

// ================================================================================================

describe('Scenario: A newer file appearing after startup is not overwritten (@RF-13)', () => {
  it('shows the read-only error, keeps English, disables Save and permits Close', async () => {
    await mountWith('en', 'loaded')
    vi.mocked(api.setSettings).mockReset().mockRejectedValue(apiError('SETTINGS_READ_ONLY'))
    await saveLanguage('es')
    const alert = await vi.waitFor(() => within(openDialog()).getByRole('alert'))
    expect(alert.textContent?.trim()).toBe(READ_ONLY)
    expect(dialogButton('Save').disabled).toBe(true)
    expect(dialogButton('Close').disabled).toBe(false)
    await vi.waitFor(() => expect(document.activeElement).toBe(dialogButton('Close')))
    expect(documentLanguage()).toBe('en')
    await userEvent.click(dialogButton('Save'))
    expect(api.setSettings).toHaveBeenCalledTimes(1)
    await userEvent.click(dialogButton('Close'))
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expectMainScreen('en')
  })
})

// ================================================================================================

describe('Scenario: The language modal supports keyboard-only use (@RF-13 @RNF-08)', () => {
  it('opens labelled with the focus on the select; Tab stays inside; the background is inert', async () => {
    await mountPage()
    const run = await startRun()
    run.scanned(3)
    const all = listed([{ name: 'A\\a.flac' }, { name: 'A\\b.flac' }, { name: 'A\\c.flac' }])
    const feeder = new Feeder(run, all)
    feeder.emit([all[0]])
    await vi.waitFor(() => expect(allRows()).toHaveLength(1))

    // "Given the initialized main screen has keyboard focus on Settings / When I activate Settings"
    settingsButton().focus()
    await userEvent.keyboard('{Enter}')
    await vi.waitFor(() => expect(dialog()).not.toBeNull())
    const d = screen.getByRole('dialog', { name: 'Settings' })
    expect(d.getAttribute('aria-modal')).toBe('true')
    expect(document.activeElement).toBe(languageSelect())

    // Tab and Shift+Tab remain within its controls
    const user = userEvent.setup()
    await user.tab()
    expect(document.activeElement).toBe(dialogButton('Save'))
    await user.tab()
    expect(document.activeElement).toBe(dialogButton('Cancel'))
    await user.tab()
    expect(document.activeElement).toBe(languageSelect())
    await user.tab({ shift: true })
    expect(document.activeElement).toBe(dialogButton('Cancel'))
    languageSelect().focus()

    // English and Español have their language annotations
    const options = within(languageSelect()).getAllByRole('option') as HTMLOptionElement[]
    expect(options.map((o) => [o.textContent?.trim(), o.getAttribute('lang')])).toEqual([
      ['English', 'en'],
      ['Español', 'es'],
    ])

    // background Ctrl+F, clicks and folder drops have no effect
    const search = screen.getByRole('searchbox', { hidden: true })
    await fireEvent.keyDown(document.activeElement as Element, { key: 'f', code: 'KeyF', ctrlKey: true })
    expect(document.activeElement).not.toBe(search)
    expect(d.contains(document.activeElement)).toBe(true)
    const cancel = dialogButton('Cancel')
    const toolbarCancel = screen
      .getAllByRole('button', { name: 'Cancel', hidden: true })
      .find((b) => !d.contains(b)) as HTMLButtonElement
    expect(toolbarCancel).not.toBe(cancel)
    if (!isInert(toolbarCancel)) await userEvent.click(toolbarCancel)
    expect(api.cancelAnalysis).not.toHaveBeenCalled()
    const verdict = screen.getByRole('button', { name: 'Show fake tracks', hidden: true })
    const pressedBefore = verdict.getAttribute('aria-pressed')
    if (!isInert(verdict)) await userEvent.click(verdict)
    expect(verdict.getAttribute('aria-pressed')).toBe(pressedBefore)
    // a background click does not take the focus out of the modal either
    expect(d.contains(document.activeElement), 'the focus stays in the modal').toBe(true)
    dragHandler({ type: 'drop', paths: ['C:\\Music\\Other'] })
    expect(api.startAnalysisOfDropped).not.toHaveBeenCalled()
    expect(dialog()).not.toBeNull()

    // analysis progress can still update the background view
    feeder.emit([all[1], all[2]])
    await vi.waitFor(() => expect(allRows()).toHaveLength(3))
    expect(dialog()).not.toBeNull()

    // "When I press Escape while not saving / Then the modal closes and focus returns to Settings"
    await userEvent.keyboard('{Escape}')
    await vi.waitFor(() => expect(dialog()).toBeNull())
    expect(document.activeElement).toBe(settingsButton())
    expect(api.setSettings).not.toHaveBeenCalled()
    expect(anyStatusText()).toBe('3/3 · 0 suspicious · 0 fake')
  })

  it('a backdrop click does not dismiss the modal', async () => {
    await mountPage()
    await openSettings()
    const d = openDialog()
    if (d.parentElement && !isInert(d.parentElement)) await userEvent.click(d.parentElement)
    await userEvent.click(document.body)
    expect(dialog()).not.toBeNull()
    expect(api.setSettings).not.toHaveBeenCalled()
  })

  it('Settings is enabled while scanning and while analysing', async () => {
    await mountPage()
    const run = await startRun()
    expect(settingsButton().disabled).toBe(false)
    run.scanned(2)
    await vi.waitFor(() => expect(statusText()).toMatch(/^0\/2/))
    expect(settingsButton().disabled).toBe(false)
  })
})

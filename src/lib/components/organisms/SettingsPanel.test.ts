// SettingsPanel (@RF-13 @RNF-08 @RNF-04; specs/ui/language-settings.md sections 2 and 4, acceptance
// L6). Props `language`, `loadStatus`, `saving`, `error: 'writeFailed' | 'readOnly' | 'internal' |
// null`; events `onsave(language)`, `onclose()`. The draft belongs to the panel and starts at
// `language` on each opening (mount). The panel owns the modal keyboard and focus behaviour and
// makes no API call. Scenarios of features/language-settings.feature, component side: "Dismissing
// the draft does not persist it", "A save already in progress cannot be duplicated or dismissed",
// "A failed save preserves the old preference and permits retry", "Invalid persisted settings fall
// back ...", "An unreadable configuration ...", "A newer settings version is preserved", "A newer
// file appearing after startup is not overwritten" and "The language modal supports keyboard-only
// use".
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The dialog is the element with role "dialog", `aria-modal="true"`, named by its heading
//   through `aria-labelledby`. "The dialog container (tabindex="-1")" that takes the focus while
//   saving is that element or an element inside it with tabindex -1 that is not a control.
// - "Mark the panel busy" / "its loading state is announced": `aria-busy="true"` on the dialog or
//   inside it while `saving`.
// - With the `readOnly` error the dismissal button is Close (`settings.close`), as with
//   `newerVersion`; Cancel is not shown then.
// - The warning for invalid/unavailable/newer is the catalogue sentence shown as text in the panel;
//   only the save error is an alert.
// - Rerenders pass the whole props object (as ResultsTable.reveal.svelte.test.ts notes).
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import SettingsPanel from './SettingsPanel.svelte'

const TEXT = {
  invalid: 'Saved settings could not be read. English is in use. Save to replace them.',
  unavailable: 'Settings are unavailable. English is in use. You can try saving again.',
  newerVersion:
    'These settings were saved by a newer app version. English is in use; these settings will be kept.',
  writeFailed: 'Cannot save the language preference. Try again.',
  readOnly: 'These settings belong to a newer app version and cannot be replaced.',
  internal: 'Cannot save the language preference. Try again.',
}

// The contract types (settings.md section 2), written out: presentational tests do not import the
// API client (lint-arch).
type Language = 'en' | 'es'
type SettingsLoadStatus = 'default' | 'loaded' | 'invalid' | 'unavailable' | 'newerVersion'
type PanelError = 'writeFailed' | 'readOnly' | 'internal' | null
type Props = {
  language: Language
  loadStatus: SettingsLoadStatus
  saving: boolean
  error: PanelError
  onsave: (language: Language) => void
  onclose: () => void
}

let outside: HTMLButtonElement

beforeEach(() => {
  // A focusable element outside the panel, so that a missing focus trap is observable.
  outside = document.createElement('button')
  outside.textContent = 'outside'
  document.body.appendChild(outside)
})

afterEach(() => {
  outside.remove()
  setLocale('en')
})

function setup(o: Partial<Props> = {}) {
  const props: Props = {
    language: 'en',
    loadStatus: 'loaded',
    saving: false,
    error: null,
    onsave: vi.fn(),
    onclose: vi.fn(),
    ...o,
  }
  const view = render(SettingsPanel, { props })
  const update = (next: Partial<Props>) => {
    Object.assign(props, next)
    view.rerender({ ...props })
    flushSync()
  }
  return { props, view, update }
}

const dialog = (name = 'Settings') => screen.getByRole('dialog', { name })
const anyDialog = () => screen.getByRole('dialog')
const select = (name = 'Language') => within(anyDialog()).getByRole('combobox', { name }) as HTMLSelectElement
const button = (name: string) => within(anyDialog()).getByRole('button', { name }) as HTMLButtonElement
const queryButton = (name: string) => within(anyDialog()).queryByRole('button', { name })
const busy = () => {
  const d = dialog()
  return d.getAttribute('aria-busy') === 'true' || d.querySelector('[aria-busy="true"]') !== null
}
const focusIsOnDialogContainer = () => {
  const active = document.activeElement as HTMLElement | null
  if (!active) return false
  const d = dialog()
  const inside = active === d || d.contains(active)
  const control = ['BUTTON', 'SELECT', 'INPUT', 'TEXTAREA', 'A'].includes(active.tagName)
  return inside && !control && active.getAttribute('tabindex') === '-1'
}

describe('SettingsPanel: structure (@RF-13 @RNF-08)', () => {
  it('is a labelled modal dialog with the heading, the select and Save and Cancel, in order', () => {
    setup()
    const d = dialog()
    expect(d.getAttribute('aria-modal')).toBe('true')
    const heading = within(d).getByRole('heading', { name: 'Settings' })
    expect(heading.id).not.toBe('')
    expect((d.getAttribute('aria-labelledby') ?? '').split(/\s+/)).toContain(heading.id)
    const order = [heading, select(), button('Save'), button('Cancel')]
    for (let i = 0; i < order.length - 1; i++) {
      expect(order[i].compareDocumentPosition(order[i + 1]) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    }
  })

  it('offers English and Español with their language annotations', () => {
    setup()
    const options = within(select()).getAllByRole('option') as HTMLOptionElement[]
    expect(options.map((o) => [o.textContent?.trim(), o.value, o.getAttribute('lang')])).toEqual([
      ['English', 'en', 'en'],
      ['Español', 'es', 'es'],
    ])
  })

  it.each(['en', 'es'] as const)('the draft starts at the active language (%s)', (language) => {
    setup({ language })
    expect(select().value).toBe(language)
  })

  it('puts the initial focus on the language select', () => {
    setup()
    expect(document.activeElement).toBe(select())
  })

  it.each<SettingsLoadStatus>(['default', 'loaded'])('%s: no warning and no alert', (loadStatus) => {
    setup({ loadStatus })
    const text = dialog().textContent ?? ''
    for (const warning of [TEXT.invalid, TEXT.unavailable, TEXT.newerVersion]) {
      expect(text).not.toContain(warning)
    }
    expect(within(dialog()).queryByRole('alert')).toBeNull()
  })
})

describe('SettingsPanel: draft, Save and dismissal (@RF-13)', () => {
  it('changing the select updates only the draft', async () => {
    const { props } = setup()
    await userEvent.selectOptions(select(), 'es')
    expect(select().value).toBe('es')
    expect(props.onsave).not.toHaveBeenCalled()
    expect(props.onclose).not.toHaveBeenCalled()
  })

  it('Save emits the draft once', async () => {
    const { props } = setup()
    await userEvent.selectOptions(select(), 'es')
    await userEvent.click(button('Save'))
    expect(props.onsave).toHaveBeenCalledTimes(1)
    expect(props.onsave).toHaveBeenCalledWith('es')
  })

  it('Save is enabled for an unchanged language', async () => {
    const { props } = setup()
    expect(button('Save').disabled).toBe(false)
    await userEvent.click(button('Save'))
    expect(props.onsave).toHaveBeenCalledWith('en')
  })

  // Scenario Outline: Dismissing the draft does not persist it | Cancel |
  it('Cancel closes without saving', async () => {
    const { props } = setup()
    await userEvent.selectOptions(select(), 'es')
    await userEvent.click(button('Cancel'))
    expect(props.onclose).toHaveBeenCalledTimes(1)
    expect(props.onsave).not.toHaveBeenCalled()
  })

  // Scenario Outline: Dismissing the draft does not persist it | Escape |
  it('Escape closes without saving', async () => {
    const { props } = setup()
    await userEvent.selectOptions(select(), 'es')
    select().focus()
    await userEvent.keyboard('{Escape}')
    expect(props.onclose).toHaveBeenCalledTimes(1)
    expect(props.onsave).not.toHaveBeenCalled()
  })

  it('a click outside the dialog does not dismiss it', async () => {
    const { props } = setup()
    const backdrop = dialog().parentElement as HTMLElement
    await userEvent.click(backdrop)
    await userEvent.click(document.body)
    expect(props.onclose).not.toHaveBeenCalled()
    expect(props.onsave).not.toHaveBeenCalled()
  })

  it('the draft starts again at the active language on each opening', async () => {
    const first = setup()
    await userEvent.selectOptions(select(), 'es')
    first.view.unmount()
    setup({ language: 'en' })
    expect(select().value).toBe('en')
  })
})

describe('SettingsPanel: keyboard (@RF-13 @RNF-08)', () => {
  it('Tab and Shift+Tab stay within its controls', async () => {
    setup()
    const user = userEvent.setup()
    expect(document.activeElement).toBe(select())
    await user.tab()
    expect(document.activeElement).toBe(button('Save'))
    await user.tab()
    expect(document.activeElement).toBe(button('Cancel'))
    await user.tab()
    expect(document.activeElement).toBe(select())
    await user.tab({ shift: true })
    expect(document.activeElement).toBe(button('Cancel'))
    expect(document.activeElement).not.toBe(outside)
  })
})

describe('SettingsPanel: saving (@RF-13)', () => {
  // Scenario: A save already in progress cannot be duplicated or dismissed
  it('disables the select and both actions, says "Saving…" and is busy', async () => {
    const { props, update } = setup()
    await userEvent.selectOptions(select(), 'es')
    await userEvent.click(button('Save'))
    update({ saving: true })
    expect(select().disabled).toBe(true)
    expect(button('Saving…').disabled).toBe(true)
    expect(button('Cancel').disabled).toBe(true)
    expect(busy()).toBe(true)
    expect(select().value).toBe('es')
    expect(props.onsave).toHaveBeenCalledTimes(1)
  })

  it('moves the focus to the dialog container and keeps it there', async () => {
    const { update } = setup()
    await userEvent.click(button('Save'))
    update({ saving: true })
    await vi.waitFor(() => expect(focusIsOnDialogContainer()).toBe(true))
    const user = userEvent.setup()
    await user.tab()
    expect(focusIsOnDialogContainer()).toBe(true)
    await user.tab({ shift: true })
    expect(focusIsOnDialogContainer()).toBe(true)
  })

  it('ignores Save, Escape and outside clicks while saving', async () => {
    const { props, update } = setup()
    await userEvent.click(button('Save'))
    update({ saving: true })
    await userEvent.click(button('Saving…'))
    await userEvent.keyboard('{Escape}')
    await userEvent.click(document.body)
    expect(props.onsave).toHaveBeenCalledTimes(1)
    expect(props.onclose).not.toHaveBeenCalled()
    expect(dialog()).toBeTruthy()
  })
})

describe('SettingsPanel: save errors (@RF-13 @RNF-04)', () => {
  it.each([
    ['writeFailed', TEXT.writeFailed],
    ['internal', TEXT.internal],
  ] as const)('%s: an alert, the draft kept, controls enabled, focus on Save, retry', async (error, text) => {
    const { props, update } = setup()
    await userEvent.selectOptions(select(), 'es')
    await userEvent.click(button('Save'))
    update({ saving: true })
    update({ saving: false, error })
    expect(within(dialog()).getByRole('alert').textContent?.trim()).toBe(text)
    expect(select().value).toBe('es')
    expect(select().disabled).toBe(false)
    expect(button('Save').disabled).toBe(false)
    expect(button('Cancel').disabled).toBe(false)
    await vi.waitFor(() => expect(document.activeElement).toBe(button('Save')))
    expect(busy()).toBe(false)
    await userEvent.click(button('Save'))
    expect(props.onsave).toHaveBeenCalledTimes(2)
    expect(props.onsave).toHaveBeenLastCalledWith('es')
  })

  // Scenario: A newer file appearing after startup is not overwritten
  it('readOnly: the read-only alert, Save disabled, Close enabled and focused', async () => {
    const { props, update } = setup()
    await userEvent.selectOptions(select(), 'es')
    await userEvent.click(button('Save'))
    update({ saving: true })
    update({ saving: false, error: 'readOnly' })
    expect(within(dialog()).getByRole('alert').textContent?.trim()).toBe(TEXT.readOnly)
    expect(button('Save').disabled).toBe(true)
    expect(button('Close').disabled).toBe(false)
    expect(queryButton('Cancel')).toBeNull()
    await vi.waitFor(() => expect(document.activeElement).toBe(button('Close')))
    await userEvent.click(button('Save'))
    expect(props.onsave).toHaveBeenCalledTimes(1)
    await userEvent.click(button('Close'))
    expect(props.onclose).toHaveBeenCalledTimes(1)
  })
})

describe('SettingsPanel: load warnings (@RF-13 @RNF-04)', () => {
  // Scenario Outline: Invalid persisted settings fall back ... "Settings explains the fallback"
  it('invalid: explains the fallback; the select and Save stay enabled', () => {
    setup({ loadStatus: 'invalid' })
    expect(dialog().textContent).toContain(TEXT.invalid)
    expect(select().disabled).toBe(false)
    expect(button('Save').disabled).toBe(false)
  })

  // Scenario Outline: An unreadable configuration ... "opening Settings offers an explicit Save retry"
  it('unavailable: explains it and offers Save', async () => {
    const { props } = setup({ loadStatus: 'unavailable' })
    expect(dialog().textContent).toContain(TEXT.unavailable)
    await userEvent.selectOptions(select(), 'es')
    await userEvent.click(button('Save'))
    expect(props.onsave).toHaveBeenCalledWith('es')
  })

  // Scenario: A newer settings version is preserved
  it('newerVersion: explains it, select and Save disabled, Close enabled with the focus', async () => {
    const { props } = setup({ loadStatus: 'newerVersion' })
    expect(dialog().textContent).toContain(TEXT.newerVersion)
    expect(select().disabled).toBe(true)
    expect(button('Save').disabled).toBe(true)
    expect(button('Close').disabled).toBe(false)
    expect(queryButton('Cancel')).toBeNull()
    expect(document.activeElement).toBe(button('Close'))
    await userEvent.click(button('Close'))
    expect(props.onclose).toHaveBeenCalledTimes(1)
    expect(props.onsave).not.toHaveBeenCalled()
  })

  it('newerVersion: Escape closes', async () => {
    const { props } = setup({ loadStatus: 'newerVersion' })
    await userEvent.keyboard('{Escape}')
    expect(props.onclose).toHaveBeenCalledTimes(1)
  })
})

describe('SettingsPanel: Spanish (@RF-13)', () => {
  it('renders every text of the panel in Spanish', async () => {
    setLocale('es')
    const { update } = setup({ language: 'es', loadStatus: 'invalid' })
    expect(within(dialog('Ajustes')).getByRole('heading', { name: 'Ajustes' })).toBeTruthy()
    expect(select('Idioma')).toBeTruthy()
    expect(dialog('Ajustes').textContent).toContain(
      'No se han podido interpretar los ajustes guardados. Se está usando el inglés. Guarda para sustituirlos.',
    )
    expect(within(dialog('Ajustes')).getByRole('button', { name: 'Guardar' })).toBeTruthy()
    expect(within(dialog('Ajustes')).getByRole('button', { name: 'Cancelar' })).toBeTruthy()
    update({ saving: true })
    expect(within(dialog('Ajustes')).getByRole('button', { name: 'Guardando…' })).toBeTruthy()
    update({ saving: false, error: 'readOnly' })
    expect(within(dialog('Ajustes')).getByRole('alert').textContent?.trim()).toBe(
      'Estos ajustes pertenecen a una versión más reciente de la aplicación y no se pueden sustituir.',
    )
    expect(within(dialog('Ajustes')).getByRole('button', { name: 'Cerrar' })).toBeTruthy()
  })

  it.each([
    ['unavailable', 'Los ajustes no están disponibles. Se está usando el inglés. Puedes volver a intentar guardarlos.'],
    [
      'newerVersion',
      'Estos ajustes se guardaron con una versión más reciente de la aplicación. Se está usando el inglés y se conservarán los ajustes.',
    ],
  ] as const)('es: the %s warning', (loadStatus, text) => {
    setLocale('es')
    setup({ loadStatus })
    expect(dialog('Ajustes').textContent).toContain(text)
  })

  it('es: an unexpected failure uses settings.saveFailed', () => {
    setLocale('es')
    setup({ error: 'internal' })
    expect(within(dialog('Ajustes')).getByRole('alert').textContent?.trim()).toBe(
      'No se puede guardar la preferencia de idioma. Vuelve a intentarlo.',
    )
  })
})

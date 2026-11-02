// Toolbar "Settings" (@RF-13 @RNF-08; specs/ui/language-settings.md sections 2 and 4, `Toolbar` row):
// a gear IconButton after the four verdict filter buttons and before Search, with
// `toolbar.settings` as tooltip and accessible name; props `canSettings: boolean` and
// `onsettings()`. Enabled in every initialized phase (the page decides), disabled at startup.
// Scenario "The language modal supports keyboard-only use" (toolbar side) and the toolbar part of
// "Existing mounted states translate after a successful Save".
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - No separator is added around the gear (the two existing separators stay).
// - No keyboard shortcut is added, so no key opens Settings by itself.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import Toolbar from './Toolbar.svelte'

afterEach(() => setLocale('en'))

const LABELS = ['Show fake tracks', 'Show suspicious tracks', 'Show genuine tracks', 'Show tracks with errors']
const button = (name: string) => screen.getByRole('button', { name }) as HTMLButtonElement
const settingsButton = () => button('Settings')
const searchBox = () => screen.getByRole('searchbox') as HTMLInputElement
const follows = (a: Node, b: Node) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING)

type Extra = Partial<{
  canOpen: boolean
  canCancel: boolean
  canExport: boolean
  canClearCache: boolean
  canFilter: boolean
  canSettings: boolean
  query: string
  onsettings: () => void
}>

function setup(extra: Extra = {}) {
  return render(Toolbar, {
    props: {
      canOpen: true,
      canCancel: false,
      canExport: true,
      canClearCache: true,
      canFilter: true,
      canSettings: true,
      verdictFilter: [],
      query: '',
      ...extra,
    },
  })
}

describe('Toolbar: Settings (@RF-13)', () => {
  it('has the gear button with the accessible name and tooltip "Settings"', () => {
    setup()
    expect(settingsButton().getAttribute('title')).toBe('Settings')
    expect(settingsButton().querySelector('svg')).not.toBeNull()
  })

  it('places it after the four verdict buttons and before Search, adding no separator', () => {
    setup()
    const toolbar = screen.getByRole('toolbar')
    expect(within(toolbar).getAllByRole('separator')).toHaveLength(2)
    const group = screen.getByRole('group', { name: 'Filter by verdict' })
    expect(follows(group, settingsButton())).toBe(true)
    expect(group.contains(settingsButton())).toBe(false)
    expect(follows(settingsButton(), searchBox())).toBe(true)
    const buttons = Array.from(toolbar.querySelectorAll('button'))
    expect(buttons.indexOf(settingsButton())).toBe(buttons.indexOf(button(LABELS[3])) + 1)
  })

  it('emits settings on click', async () => {
    const onsettings = vi.fn()
    setup({ onsettings })
    await userEvent.click(settingsButton())
    expect(onsettings).toHaveBeenCalledTimes(1)
  })

  it('opens with the keyboard (Enter and Space)', async () => {
    const onsettings = vi.fn()
    setup({ onsettings })
    settingsButton().focus()
    await userEvent.keyboard('{Enter}')
    await userEvent.keyboard(' ')
    expect(onsettings).toHaveBeenCalledTimes(2)
  })

  it('startup-disabled: no event, not a tab stop', async () => {
    const onsettings = vi.fn()
    setup({ canOpen: false, canExport: false, canClearCache: false, canFilter: false, canSettings: false, onsettings })
    expect(settingsButton().disabled).toBe(true)
    await userEvent.click(settingsButton())
    expect(onsettings).not.toHaveBeenCalled()
  })

  it('stays enabled while an analysis runs (scanning or analysing)', () => {
    setup({ canOpen: false, canCancel: true, canExport: false, canClearCache: false, canSettings: true })
    expect(settingsButton().disabled).toBe(false)
    expect(button('Open folder').disabled).toBe(true)
  })

  it('adds no keyboard shortcut', async () => {
    const onsettings = vi.fn()
    setup({ onsettings })
    for (const key of ['s', ',', 'F1']) {
      await fireEvent.keyDown(window, { key, ctrlKey: true })
      await fireEvent.keyDown(document.body, { key })
    }
    expect(onsettings).not.toHaveBeenCalled()
  })

  it('tab order: open, cancel, export, clear, the verdict buttons, Settings, search', async () => {
    setup({ canCancel: true })
    const user = userEvent.setup()
    const expected = [
      button('Open folder'),
      button('Cancel'),
      button('Export CSV'),
      button('Clear results cache'),
      ...LABELS.map(button),
      settingsButton(),
      searchBox(),
    ]
    for (const element of expected) {
      await user.tab()
      expect(document.activeElement).toBe(element)
    }
  })
})

describe('Toolbar: Spanish (@RF-13)', () => {
  const SPANISH = [
    'Abrir carpeta',
    'Cancelar',
    'Exportar CSV',
    'Borrar caché de resultados',
    'Mostrar pistas falsas',
    'Mostrar pistas sospechosas',
    'Mostrar pistas reales',
    'Mostrar pistas con errores',
    'Ajustes',
    'Borrar búsqueda',
  ]

  it('translates every name and tooltip while mounted, without a remount', async () => {
    setup({ query: 'abc' })
    expect(screen.getByRole('toolbar', { name: 'Main toolbar' })).toBeTruthy()
    setLocale('es')
    flushSync()
    expect(screen.getByRole('toolbar', { name: 'Barra de herramientas principal' })).toBeTruthy()
    expect(screen.getByRole('group', { name: 'Filtrar por veredicto' })).toBeTruthy()
    for (const name of SPANISH) {
      const b = button(name)
      if (b.hasAttribute('title')) expect(b.getAttribute('title')).toBe(name)
    }
    const search = screen.getByRole('searchbox', { name: 'Buscar por nombre' }) as HTMLInputElement
    expect(search.placeholder).toBe('Buscar (Ctrl+F)')
    expect(search.value).toBe('abc')

    setLocale('en')
    flushSync()
    expect(settingsButton().getAttribute('title')).toBe('Settings')
    expect(screen.getByRole('toolbar', { name: 'Main toolbar' })).toBeTruthy()
  })
})

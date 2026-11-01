// Toolbar "Clear results cache" (specs/ui/result-cache.md sections 2, 4 `Toolbar` row and 6;
// @RF-11 @RNF-08): an IconButton right after Export CSV in the export group, the optional prop
// `canClearCache` (default false) and the event `onclearcache`.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "in the same group" is checked as: after Export CSV and before the second separator.
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import Toolbar from './Toolbar.svelte'

const LABELS = ['Show fake tracks', 'Show suspicious tracks', 'Show genuine tracks', 'Show tracks with errors']
const button = (name: string) => screen.getByRole('button', { name }) as HTMLButtonElement
const clearButton = () => button('Clear results cache')
const follows = (a: Node, b: Node) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING)

type Extra = Partial<{
  canOpen: boolean
  canCancel: boolean
  canExport: boolean
  canClearCache: boolean
  canFilter: boolean
  onclearcache: () => void
}>

function setup(extra: Extra = {}) {
  return render(Toolbar, {
    props: {
      canOpen: true,
      canCancel: false,
      canExport: true,
      canFilter: true,
      canSettings: false,
      verdictFilter: [],
      query: '',
      ...extra,
    },
  })
}

describe('Toolbar: Clear results cache (@RF-11)', () => {
  it('has the button with the accessible name and tooltip "Clear results cache"', () => {
    setup({ canClearCache: true })
    expect(clearButton().getAttribute('title')).toBe('Clear results cache')
    expect(clearButton().querySelector('svg')).not.toBeNull()
  })

  it('places it right after Export CSV, before the second separator, adding no separator', () => {
    setup({ canClearCache: true })
    const toolbar = screen.getByRole('toolbar')
    const separators = within(toolbar).getAllByRole('separator')
    expect(separators).toHaveLength(2)
    const exportButton = button('Export CSV')
    expect(follows(exportButton, clearButton())).toBe(true)
    expect(follows(clearButton(), separators[1])).toBe(true)
    const buttons = Array.from(toolbar.querySelectorAll('button'))
    expect(buttons.indexOf(clearButton())).toBe(buttons.indexOf(exportButton) + 1)
  })

  it('is disabled by default (canClearCache is optional)', () => {
    setup()
    expect(clearButton().disabled).toBe(true)
  })

  it('idle with clear enabled: emits clearCache on click', async () => {
    const onclearcache = vi.fn()
    setup({ canClearCache: true, onclearcache })
    expect(clearButton().disabled).toBe(false)
    await userEvent.click(clearButton())
    expect(onclearcache).toHaveBeenCalledTimes(1)
  })

  it('clear disabled while clearing: Open folder disabled too, no event', async () => {
    const onclearcache = vi.fn()
    setup({ canOpen: false, canClearCache: false, onclearcache })
    expect(clearButton().disabled).toBe(true)
    expect(button('Open folder').disabled).toBe(true)
    await userEvent.click(clearButton())
    expect(onclearcache).not.toHaveBeenCalled()
  })

  it('running: clear disabled', () => {
    setup({ canOpen: false, canCancel: true, canExport: false, canClearCache: false })
    expect(clearButton().disabled).toBe(true)
    expect(button('Cancel').disabled).toBe(false)
  })

  // Section 6: open, cancel, export, clear, the verdict buttons, the search field
  it('tab order with every toolbar button enabled', async () => {
    setup({ canCancel: true, canClearCache: true })
    const user = userEvent.setup()
    const expected = [
      button('Open folder'),
      button('Cancel'),
      button('Export CSV'),
      clearButton(),
      ...LABELS.map(button),
      screen.getByRole('searchbox', { name: 'Search by name' }),
    ]
    for (const element of expected) {
      await user.tab()
      expect(document.activeElement).toBe(element)
    }
  })

  it('when disabled it is not a tab stop', async () => {
    setup({ canCancel: true, canClearCache: false })
    const user = userEvent.setup()
    await user.tab()
    await user.tab()
    await user.tab()
    expect(document.activeElement).toBe(button('Export CSV'))
    await user.tab()
    expect(document.activeElement).toBe(button(LABELS[0]))
  })
})

// Toolbar with the verdict filter and the search field (specs/ui/sort-filter.md sections 2, 8, 10
// `Toolbar` row, and 13; @RF-08): a separator and `VerdictFilter` after the export group, and
// `SearchField` (with `shortcut`) at the right end; both disabled when `canFilter` is false, still
// showing `verdictFilter` and `query`.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "Right-aligned" is CSS; what is tested is that the search field comes last in the toolbar.
// - Tab order (section 13): open, cancel, export, the four verdict buttons, the search field.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Verdict } from '../../model'
import Toolbar from './Toolbar.svelte'

const LABELS = ['Show fake tracks', 'Show suspicious tracks', 'Show genuine tracks', 'Show tracks with errors']
const toolbar = () => screen.getByRole('toolbar')
const group = () => screen.getByRole('group', { name: 'Filter by verdict' })
const verdictButtons = () => LABELS.map((name) => screen.getByRole('button', { name }) as HTMLButtonElement)
const box = () => screen.getByRole('searchbox', { name: 'Search by name' }) as HTMLInputElement
const clearButton = () => screen.queryByRole('button', { name: 'Clear search' })
const pressed = () => verdictButtons().map((b) => b.getAttribute('aria-pressed'))

type Extra = Partial<{
  canOpen: boolean
  canCancel: boolean
  canExport: boolean
  canFilter: boolean
  verdictFilter: Verdict[]
  query: string
  searchFocusRequest: number
  ontoggleverdict: (v: Verdict) => void
  onquery: (q: string) => void
}>

function setup(extra: Extra = {}) {
  return render(Toolbar, {
    props: {
      canOpen: true,
      canCancel: false,
      canExport: true,
      canFilter: true,
      verdictFilter: [],
      query: '',
      ...extra,
    },
  })
}

const follows = (a: Node, b: Node) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING)

describe('Toolbar: filters layout (@RF-08)', () => {
  it('adds a separator and the verdict group after Export CSV, and the search field last', () => {
    setup()
    const exportButton = screen.getByRole('button', { name: 'Export CSV' })
    const separators = within(toolbar()).getAllByRole('separator')
    expect(separators).toHaveLength(2)
    expect(follows(exportButton, separators[1])).toBe(true)
    expect(follows(separators[1], group())).toBe(true)
    expect(toolbar().contains(group())).toBe(true)
    expect(follows(group(), box())).toBe(true)
    const focusables = Array.from(toolbar().querySelectorAll('button, input'))
    expect(focusables[focusables.length - 1] === box() || focusables[focusables.length - 2] === box()).toBe(true)
    expect(within(group()).getAllByRole('button')).toEqual(verdictButtons())
  })

  it('tab order: open, cancel (when enabled), export, the four verdict buttons, the search field', async () => {
    setup({ canCancel: true })
    const user = userEvent.setup()
    const expected = [
      screen.getByRole('button', { name: 'Open folder' }),
      screen.getByRole('button', { name: 'Cancel' }),
      screen.getByRole('button', { name: 'Export CSV' }),
      ...verdictButtons(),
      box(),
    ]
    for (const element of expected) {
      await user.tab()
      expect(document.activeElement).toBe(element)
    }
  })
})

describe('Toolbar: filter states (@RF-08)', () => {
  it('idle with filters: enabled, nothing pressed, empty field', () => {
    setup()
    expect(verdictButtons().map((b) => b.disabled)).toEqual([false, false, false, false])
    expect(pressed()).toEqual(['false', 'false', 'false', 'false'])
    expect(box().disabled).toBe(false)
    expect(box().value).toBe('')
  })

  it('filters pressed and query', () => {
    setup({ verdictFilter: ['fake', 'error'], query: 'intro' })
    expect(pressed()).toEqual(['true', 'false', 'false', 'true'])
    expect(box().value).toBe('intro')
    expect(clearButton()).not.toBeNull()
  })

  it('empty (filters disabled)', () => {
    setup({ canFilter: false, canExport: false })
    expect(verdictButtons().map((b) => b.disabled)).toEqual([true, true, true, true])
    expect(pressed()).toEqual(['false', 'false', 'false', 'false'])
    expect(box().disabled).toBe(true)
    expect(box().placeholder).toBe('Search (Ctrl+F)')
  })

  it('empty with a kept filter: disabled, pressed, text, no clear button', () => {
    setup({ canFilter: false, canExport: false, verdictFilter: ['fake'], query: '01' })
    expect(verdictButtons().map((b) => b.disabled)).toEqual([true, true, true, true])
    expect(pressed()).toEqual(['true', 'false', 'false', 'false'])
    expect(box().disabled).toBe(true)
    expect(box().value).toBe('01')
    expect(clearButton()).toBeNull()
  })

  it('running with filters: enabled while Open is disabled', () => {
    setup({ canOpen: false, canCancel: true, canExport: false, verdictFilter: ['suspicious'], query: 'a' })
    expect((screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement).disabled).toBe(true)
    expect(verdictButtons().map((b) => b.disabled)).toEqual([false, false, false, false])
    expect(pressed()).toEqual(['false', 'true', 'false', 'false'])
    expect(box().disabled).toBe(false)
  })
})

describe('Toolbar: filter events (@RF-08)', () => {
  it('emits toggleVerdict(verdict) for each verdict button', async () => {
    const ontoggleverdict = vi.fn()
    setup({ ontoggleverdict })
    const order: Verdict[] = ['fake', 'suspicious', 'real', 'error']
    for (const [i, b] of verdictButtons().entries()) {
      await userEvent.click(b)
      expect(ontoggleverdict).toHaveBeenLastCalledWith(order[i])
    }
  })

  it('emits query(text) when the search text changes', async () => {
    const onquery = vi.fn()
    setup({ onquery })
    await userEvent.type(box(), 'a')
    expect(onquery).toHaveBeenLastCalledWith('a')
  })

  it('emits nothing from the disabled filters', async () => {
    const ontoggleverdict = vi.fn()
    const onquery = vi.fn()
    setup({ canFilter: false, ontoggleverdict, onquery })
    for (const b of verdictButtons()) await userEvent.click(b)
    await userEvent.type(box(), 'a')
    expect(ontoggleverdict).not.toHaveBeenCalled()
    expect(onquery).not.toHaveBeenCalled()
  })

  it('the search field has the Ctrl+F shortcut', async () => {
    setup({ query: 'intro' })
    // the fireEvent of @testing-library/svelte is async: it resolves to dispatchEvent's result
    const prevented = !(await fireEvent.keyDown(document.body, { key: 'f', code: 'KeyF', ctrlKey: true }))
    expect(prevented).toBe(true)
    expect(document.activeElement).toBe(box())
  })

  it('searchFocusRequest focuses the search field on each change, not on mount', async () => {
    const view = setup({ query: 'x', searchFocusRequest: 2 })
    expect(document.activeElement).not.toBe(box())
    await view.rerender({ searchFocusRequest: 3 })
    expect(document.activeElement).toBe(box())
  })
})

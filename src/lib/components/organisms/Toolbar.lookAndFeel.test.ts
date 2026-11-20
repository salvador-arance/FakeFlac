// Toolbar as a whole: every command is an icon-only button whose tooltip is its label (@RNF-10;
// specs/ui/look-and-feel.md element L2, specs/ui/components.md `IconButton` and `Toolbar`).
// Scenario S3 "Every toolbar command is an icon button with its label as tooltip" of
// specs/features/look-and-feel.feature. `IconButton.test.ts` checks one button alone; this file
// checks every button the toolbar renders, so a command added later as a text button is caught.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "Every button enabled": every `can*` prop is true. The query is not empty, so the search
//   field's own clear button is rendered and the exclusion "outside the search field" is exercised.
// - "The search field" is the element that holds the searchbox (its parent), clear button included.
// - "Shows an icon": the button holds an `svg`. "No visible text": the button's text content is
//   empty once whitespace is removed.
// - "The tooltip is its accessible name": the `title` is not empty and the button is found by role
//   with exactly that name.
import { render, screen, within } from '@testing-library/svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import Toolbar from './Toolbar.svelte'

afterEach(() => setLocale('en'))

function renderEveryButtonEnabled() {
  setLocale('en')
  render(Toolbar, {
    props: {
      canOpen: true,
      canCancel: true,
      canExport: true,
      canClearCache: true,
      canFilter: true,
      canSettings: true,
      verdictFilter: [],
      query: 'synthetic',
    },
  })
  const toolbar = screen.getByRole('toolbar', { name: 'Main toolbar' })
  const searchField = within(toolbar).getByRole('searchbox').parentElement!
  const all = Array.from(toolbar.querySelectorAll('button'))
  const commands = all.filter((b) => !searchField.contains(b))
  return { toolbar, searchField, all, commands }
}

describe('@RNF-10 Scenario: Every toolbar command is an icon button with its label as tooltip', () => {
  it('renders the commands enabled and the search field with its own button left out', () => {
    const { searchField, all, commands } = renderEveryButtonEnabled()
    // not vacuous: the toolbar has commands, and the search field's button is the one left out
    expect(commands.length).toBeGreaterThan(0)
    expect(all.length).toBeGreaterThan(commands.length)
    expect(searchField.querySelectorAll('button').length).toBe(all.length - commands.length)
    // the verdict filter toggles are commands of the toolbar too (nested group)
    expect(commands.some((b) => b.hasAttribute('aria-pressed'))).toBe(true)
    for (const button of commands) expect(button.disabled, button.outerHTML).toBe(false)
  })

  it('shows an icon and no visible text in every button outside the search field', () => {
    const { commands } = renderEveryButtonEnabled()
    for (const button of commands) {
      const name = button.getAttribute('aria-label') ?? button.outerHTML
      expect(button.querySelector('svg'), name).not.toBeNull()
      expect((button.textContent ?? '').replace(/\s+/g, ''), name).toBe('')
    }
  })

  it('gives each of those buttons its accessible name as tooltip', () => {
    const { toolbar, commands } = renderEveryButtonEnabled()
    for (const button of commands) {
      const tooltip = button.getAttribute('title') ?? ''
      expect(tooltip.trim(), button.outerHTML).not.toBe('')
      expect(within(toolbar).getAllByRole('button', { name: tooltip })).toContain(button)
    }
  })
})

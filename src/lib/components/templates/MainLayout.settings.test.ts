// MainLayout: the optional `settings` snippet (@RF-13; specs/ui/language-settings.md section 4,
// `MainLayout` row: "Slot for modal; no settings/store/API knowledge").
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The modal slot is rendered inside the layout, after the regular regions, and outside the
//   title bar, toolbar, table, detail and status bar regions; without it nothing extra renders.
import { render, screen } from '@testing-library/svelte'
import { createRawSnippet } from 'svelte'
import { describe, expect, it } from 'vitest'
import MainLayout from './MainLayout.svelte'

const snippet = (text: string) =>
  createRawSnippet(() => ({ render: () => `<div data-testid="${text}">${text}</div>` }))

const slots = {
  titleBar: snippet('title-bar'),
  toolbar: snippet('toolbar'),
  table: snippet('table'),
  detail: snippet('detail'),
  statusBar: snippet('status-bar'),
}

describe('MainLayout: settings slot (@RF-13)', () => {
  it('renders the settings snippet when given, outside the other regions', () => {
    const { container } = render(MainLayout, { props: { ...slots, settings: snippet('settings') } })
    const settings = screen.getByTestId('settings')
    expect(container.contains(settings)).toBe(true)
    for (const id of ['title-bar', 'toolbar', 'table', 'detail', 'status-bar']) {
      const region = screen.getByTestId(id)
      expect(region.contains(settings)).toBe(false)
      expect(region.parentElement?.contains(settings)).toBe(false)
    }
  })

  it('renders nothing extra without it', () => {
    render(MainLayout, { props: slots })
    expect(screen.queryByTestId('settings')).toBeNull()
    for (const id of ['title-bar', 'toolbar', 'table', 'detail', 'status-bar']) {
      expect(screen.getByTestId(id)).toBeTruthy()
    }
  })

  it('keeps every other region rendered while the settings slot is present', () => {
    render(MainLayout, { props: { ...slots, settings: snippet('settings') } })
    for (const id of ['title-bar', 'toolbar', 'table', 'detail', 'status-bar']) {
      expect(screen.getByTestId(id)).toBeTruthy()
    }
  })
})

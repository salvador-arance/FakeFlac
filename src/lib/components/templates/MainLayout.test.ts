// MainLayout (specs/ui/components.md, Templates): slots titleBar, toolbar, table, detail, statusBar.
// `detail` is optional until RF-07. Svelte 5 slots are snippet props.
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
  statusBar: snippet('status-bar'),
}

describe('MainLayout', () => {
  // main-screen.md "Layout": title bar, toolbar, table, (detail), status bar, top to bottom
  it('renders the slots in layout order', () => {
    render(MainLayout, { props: { ...slots, detail: snippet('detail') } })
    const order = ['title-bar', 'toolbar', 'table', 'detail', 'status-bar'].map((id) =>
      screen.getByTestId(id),
    )
    for (let i = 0; i < order.length - 1; i++) {
      expect(
        order[i].compareDocumentPosition(order[i + 1]) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy()
    }
  })

  // RF-07 is out of this feature: the detail slot is optional
  it('renders without a detail slot', () => {
    render(MainLayout, { props: slots })
    for (const id of ['title-bar', 'toolbar', 'table', 'status-bar']) {
      expect(screen.getByTestId(id)).not.toBeNull()
    }
    expect(screen.queryByTestId('detail')).toBeNull()
  })
})

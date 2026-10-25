// MainLayout, optional `sidebar` snippet (specs/ui/album-summary.md "Component changes"; @RF-10):
// rendered at the left of the content area, full content height, width `var(--sidebar-width)`,
// with a 1 px right border. Without it the layout is as before. CSS values are not testable in
// jsdom; what is tested is the structure: the sidebar is inside the content area (`main`), before
// the table, and the table and detail stay together on its right.
import { render, screen } from '@testing-library/svelte'
import { createRawSnippet } from 'svelte'
import { describe, expect, it } from 'vitest'
import tokens from '../../tokens/tokens.css?raw'
import MainLayout from './MainLayout.svelte'

const snippet = (text: string) =>
  createRawSnippet(() => ({ render: () => `<div data-testid="${text}">${text}</div>` }))

const slots = {
  titleBar: snippet('title-bar'),
  toolbar: snippet('toolbar'),
  table: snippet('table'),
  statusBar: snippet('status-bar'),
}

const follows = (a: Node, b: Node) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING)

describe('MainLayout: sidebar', () => {
  // @RF-10 with sidebar
  it('renders the sidebar inside the content area, before the table and the detail', () => {
    render(MainLayout, { props: { ...slots, detail: snippet('detail'), sidebar: snippet('sidebar') } })
    const main = screen.getByRole('main')
    const sidebar = screen.getByTestId('sidebar')
    expect(main.contains(sidebar)).toBe(true)
    expect(follows(screen.getByTestId('toolbar'), sidebar)).toBe(true)
    expect(follows(sidebar, screen.getByTestId('table'))).toBe(true)
    expect(follows(sidebar, screen.getByTestId('detail'))).toBe(true)
    expect(follows(screen.getByTestId('detail'), screen.getByTestId('status-bar'))).toBe(true)
  })

  // @RF-10 the sidebar spans the whole content height: it is not a child of the table or detail wrappers
  it('does not put the sidebar inside the table or the detail wrapper', () => {
    render(MainLayout, { props: { ...slots, detail: snippet('detail'), sidebar: snippet('sidebar') } })
    const sidebar = screen.getByTestId('sidebar')
    expect(screen.getByTestId('table').parentElement?.contains(sidebar)).toBe(false)
    expect(screen.getByTestId('detail').parentElement?.contains(sidebar)).toBe(false)
  })

  // @RF-10 the table and the detail panel keep the splitter between them
  it('keeps the splitter between the table and the detail', () => {
    render(MainLayout, { props: { ...slots, detail: snippet('detail'), sidebar: snippet('sidebar') } })
    const sep = screen.getByRole('separator')
    expect(follows(screen.getByTestId('table'), sep)).toBe(true)
    expect(follows(sep, screen.getByTestId('detail'))).toBe(true)
  })

  // @RF-10 without sidebar
  it('renders as before without a sidebar', () => {
    render(MainLayout, { props: { ...slots, detail: snippet('detail') } })
    expect(screen.queryByTestId('sidebar')).toBeNull()
    for (const id of ['title-bar', 'toolbar', 'table', 'detail', 'status-bar']) {
      expect(screen.getByTestId(id)).not.toBeNull()
    }
  })

  it('renders a sidebar without a detail slot', () => {
    render(MainLayout, { props: { ...slots, sidebar: snippet('sidebar') } })
    expect(screen.getByTestId('sidebar')).not.toBeNull()
    expect(screen.queryByRole('separator')).toBeNull()
  })
})

describe('layout token', () => {
  // @RF-10 "a new layout token --sidebar-width: 240px in src/lib/tokens/tokens.css"
  it('defines --sidebar-width as 240px', () => {
    expect(tokens).toMatch(/--sidebar-width:\s*240px\s*;/)
  })
})

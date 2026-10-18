// FolderDropZone (specs/ui/components.md, Organisms; main-screen.md "Empty" and "Dragging over").
// Readings chosen: the root carries data-testid="folder-drop-zone" and data-active="true|false";
// it is rendered when `empty` or `active`. The drop itself is handled by the page.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import FolderDropZone from './FolderDropZone.svelte'

const HINT = 'Drop a folder here or click Open folder'

describe('FolderDropZone', () => {
  // @RF-01 state: hint (empty start screen)
  it('shows the hint when empty', () => {
    render(FolderDropZone, { props: { active: false, empty: true } })
    expect(screen.getByText(HINT)).not.toBeNull()
    expect(screen.getByTestId('folder-drop-zone').getAttribute('data-active')).toBe('false')
  })

  // @RF-01 state: drag-over highlights the whole window
  it('is highlighted while dragging over', () => {
    render(FolderDropZone, { props: { active: true, empty: true } })
    expect(screen.getByTestId('folder-drop-zone').getAttribute('data-active')).toBe('true')
  })

  it('highlights while dragging even when the table has rows', () => {
    render(FolderDropZone, { props: { active: true, empty: false } })
    expect(screen.getByTestId('folder-drop-zone').getAttribute('data-active')).toBe('true')
    // "the 'drop here' hint only in the empty state" (frontend.md)
    expect(screen.queryByText(HINT)).toBeNull()
  })

  it('shows no hint when not empty', () => {
    render(FolderDropZone, { props: { active: false, empty: false } })
    expect(screen.queryByText(HINT)).toBeNull()
  })
})

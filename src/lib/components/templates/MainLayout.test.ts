// MainLayout (specs/ui/components.md, Templates): slots titleBar, toolbar, table, detail, statusBar.
// `detail` is optional until RF-07. Svelte 5 slots are snippet props.
import { fireEvent, render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { createRawSnippet } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
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

  // RF-07 splitter. Contract chosen by these tests:
  //  - the detail height is the inline style `height` (px) of the detail wrapper (parent of the slot content)
  //  - localStorage key `fakeflac.layout.detailHeight`, value = integer px as a string
  //  - the layout subscribes to the Splitter's resize(delta): height = height - delta, clamped
  // The table min of 120 px is tested by stubbing clientHeight; the CSS min-heights are checked visually.
  describe('splitter (RF-07)', () => {
    const KEY = 'fakeflac.layout.detailHeight'
    const detailHeight = () =>
      (screen.getByTestId('detail').parentElement as HTMLElement).style.height
    const renderWithDetail = () =>
      render(MainLayout, { props: { ...slots, detail: snippet('detail') } })
    const pointer = (el: Element, type: string, clientY: number) =>
      fireEvent(el, new MouseEvent(type, { bubbles: true, cancelable: true, clientY }))

    afterEach(() => {
      vi.restoreAllMocks()
      localStorage.clear()
    })

    it('places a separator between the table and the detail', () => {
      renderWithDetail()
      const sep = screen.getByRole('separator')
      const follows = (a: Node, b: Node) =>
        a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING
      expect(follows(screen.getByTestId('table'), sep)).toBeTruthy()
      expect(follows(sep, screen.getByTestId('detail'))).toBeTruthy()
    })

    it('has no separator without a detail slot', () => {
      render(MainLayout, { props: slots })
      expect(screen.queryByRole('separator')).toBeNull()
    })

    it('defaults the detail height to 260 px with empty storage', () => {
      localStorage.clear()
      renderWithDetail()
      expect(detailHeight()).toBe('260px')
    })

    it('moving the splitter up grows the detail, down shrinks it', async () => {
      renderWithDetail()
      const user = userEvent.setup()
      await user.tab()
      await user.keyboard('{ArrowUp}')
      expect(detailHeight()).toBe('270px')
      await user.keyboard('{ArrowDown}{ArrowDown}')
      expect(detailHeight()).toBe('250px')
    })

    it('a pointer drag resizes the detail by the pointer delta', async () => {
      renderWithDetail()
      const sep = screen.getByRole('separator')
      await pointer(sep, 'pointerdown', 300)
      await pointer(sep, 'pointermove', 250)
      await pointer(sep, 'pointerup', 250)
      expect(detailHeight()).toBe('310px')
    })

    it('clamps the detail to at least 160 px', async () => {
      renderWithDetail()
      const user = userEvent.setup()
      await user.tab()
      await user.keyboard('{ArrowDown}'.repeat(15))
      expect(detailHeight()).toBe('160px')
    })

    it('clamps the detail so the table keeps at least 120 px', async () => {
      vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(400)
      renderWithDetail()
      const user = userEvent.setup()
      await user.tab()
      await user.keyboard('{ArrowUp}'.repeat(30))
      const h = parseInt(detailHeight(), 10)
      expect(h).toBeLessThanOrEqual(280)
      expect(h).toBeGreaterThanOrEqual(160)
    })

    it('saves the position to localStorage', async () => {
      renderWithDetail()
      const user = userEvent.setup()
      await user.tab()
      await user.keyboard('{ArrowUp}')
      expect(localStorage.getItem(KEY)).toBe('270')
    })

    it('restores the saved position on the next render', () => {
      localStorage.setItem(KEY, '300')
      renderWithDetail()
      expect(detailHeight()).toBe('300px')
    })

    it('falls back to 260 px when storage holds garbage', () => {
      for (const bad of ['abc', '', 'NaN', '-5px', 'null']) {
        localStorage.setItem(KEY, bad)
        const { unmount } = renderWithDetail()
        expect(detailHeight()).toBe('260px')
        unmount()
      }
    })

    it('falls back to 260 px when localStorage throws', async () => {
      vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
        throw new Error('denied')
      })
      vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
        throw new Error('denied')
      })
      renderWithDetail()
      expect(detailHeight()).toBe('260px')
      const user = userEvent.setup()
      await user.tab()
      await user.keyboard('{ArrowUp}')
      expect(detailHeight()).toBe('270px')
    })
  })
})

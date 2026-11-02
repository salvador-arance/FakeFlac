// @RF-10 resizable-widths.feature: sidebar dragging/reset, bounds, modal and hidden state (W1/W3).
import { fireEvent, render, screen } from '@testing-library/svelte'
import { createRawSnippet } from 'svelte'
import type { ComponentProps } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import { measurements, pointer } from '../../tests/resizeHarness'
import MainLayout from './MainLayout.svelte'

const snippet = (name: string) => createRawSnippet(() => ({ render: () => `<div data-testid="${name}">${name}</div>` }))
const slots = { titleBar: snippet('title'), toolbar: snippet('toolbar'), table: snippet('table'), sidebar: snippet('sidebar'), statusBar: snippet('status') }
const handle = () => screen.getByRole('separator', { name: 'Resize Albums panel' })
const value = () => Number(handle().getAttribute('aria-valuenow'))
afterEach(() => { vi.unstubAllGlobals(); setLocale('en') })
function setup(extra: Record<string, unknown> = {}) {
  setLocale('en')
  const sizes = measurements()
  const onsidebarresize = vi.fn(), onsidebarresizeend = vi.fn(), onresetsidebar = vi.fn()
  const view = render(MainLayout, { props: { ...slots, onsidebarresize, onsidebarresizeend, onresetsidebar, ...extra } as unknown as ComponentProps<typeof MainLayout> })
  sizes.set(screen.getByRole('main'), 900, 400)
  return { ...view, sizes, onsidebarresize, onsidebarresizeend, onresetsidebar }
}

describe('@RF-10 adjustable Albums sidebar', () => {
  it('starts at 240 and dragging 80 increases only the sidebar width to 320', async () => {
    const view = setup()
    expect(value()).toBe(240)
    await pointer(handle(), 'pointerdown', 240)
    await pointer(window, 'pointermove', 320)
    expect(view.onsidebarresize).toHaveBeenLastCalledWith(320)
    expect(view.onsidebarresizeend).not.toHaveBeenCalled()
    await pointer(window, 'pointerup', 320)
    expect(view.onsidebarresizeend).toHaveBeenCalledTimes(1)
    expect(localStorage.getItem('fakeflac.layout.widths')).toBeNull()
  })
  it('clamps at 160/480 and reverses immediately after excess movement', async () => {
    const view = setup()
    await pointer(handle(), 'pointerdown', 240)
    await pointer(window, 'pointermove', 1000)
    expect(view.onsidebarresize).toHaveBeenLastCalledWith(480)
    await pointer(window, 'pointermove', 990)
    expect(view.onsidebarresize).toHaveBeenLastCalledWith(470)
    await pointer(window, 'pointermove', -1000)
    expect(view.onsidebarresize).toHaveBeenLastCalledWith(160)
    await pointer(window, 'pointerup', -1000)
  })
  it.each(['keyboard', 'pointer'])('W1: measured clamping does not save; %s starts from displayed width, not stored preference', async (input) => {
    const view = setup({ sidebarWidth: 480 })
    view.sizes.set(screen.getByRole('main'), 764, 400)
    await vi.waitFor(() => expect(value()).toBe(400))
    expect(view.onsidebarresize).not.toHaveBeenCalled()
    expect(view.onsidebarresizeend).not.toHaveBeenCalled()
    if (input === 'keyboard') await fireEvent.keyDown(handle(), { key: 'ArrowLeft' })
    else { await pointer(handle(), 'pointerdown', 400); await pointer(window, 'pointermove', 390); await pointer(window, 'pointerup', 390) }
    expect(view.onsidebarresize).toHaveBeenLastCalledWith(390)
    expect(view.onsidebarresizeend).toHaveBeenCalledTimes(1)
  })
  it('restores the preferred width when more space returns without a user change', async () => {
    const view = setup({ sidebarWidth: 480 })
    view.sizes.set(screen.getByRole('main'), 764, 400)
    await vi.waitFor(() => expect(value()).toBe(400))
    view.sizes.set(screen.getByRole('main'), 900, 400)
    await vi.waitFor(() => expect(value()).toBe(480))
    expect(view.onsidebarresize).not.toHaveBeenCalled()
  })
  it('double-click and Enter reset without changing detail-height storage', async () => {
    localStorage.setItem('fakeflac.layout.detailHeight', '300')
    const view = setup({ sidebarWidth: 320 })
    await fireEvent.doubleClick(handle())
    await fireEvent.keyDown(handle(), { key: 'Enter' })
    expect(view.onresetsidebar).toHaveBeenCalledTimes(2)
    expect(view.onsidebarresize).not.toHaveBeenCalled()
    expect(localStorage.getItem('fakeflac.layout.detailHeight')).toBe('300')
    localStorage.removeItem('fakeflac.layout.detailHeight')
  })
  it('hiding the sidebar removes only its separator and restores the supplied preference when shown', async () => {
    const view = setup({ sidebarWidth: 320, detail: snippet('detail') })
    const horizontal = () => screen.getAllByRole('separator').filter(s => s.getAttribute('aria-orientation') === 'horizontal')
    expect(horizontal()).toHaveLength(1)
    await view.rerender({ sidebar: undefined })
    expect(screen.queryByRole('separator', { name: 'Resize Albums panel' })).toBeNull()
    expect(horizontal()).toHaveLength(1)
    await view.rerender({ sidebar: slots.sidebar })
    expect(value()).toBe(320)
    expect(view.onsidebarresize).not.toHaveBeenCalled()
  })
  it('opening Settings ends an active drag and ignores later pointer motion', async () => {
    const view = setup()
    await pointer(handle(), 'pointerdown', 240)
    await pointer(window, 'pointermove', 280)
    await view.rerender({ settings: snippet('settings') })
    expect(view.onsidebarresizeend).toHaveBeenCalledTimes(1)
    expect(handle().closest('[inert]')).not.toBeNull()
    view.onsidebarresize.mockClear()
    await pointer(window, 'pointermove', 400)
    expect(view.onsidebarresize).not.toHaveBeenCalled()
  })
})

// TitleBar (specs/ui/components.md, Organisms; main-screen.md "Interaction"):
// title, data-tauri-drag-region, double-click to maximise, the three buttons.
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import TitleBar from './TitleBar.svelte'

describe('TitleBar', () => {
  it('shows the title', () => {
    render(TitleBar, { props: { title: 'FakeFlac' } })
    expect(screen.getByText('FakeFlac')).not.toBeNull()
  })

  // "draggable": Tauri needs data-tauri-drag-region on the area that drags the window
  it('has a drag region that contains the title', () => {
    const { container } = render(TitleBar, { props: { title: 'FakeFlac' } })
    const region = container.querySelector('[data-tauri-drag-region]')
    expect(region).not.toBeNull()
    expect(region?.contains(screen.getByText('FakeFlac'))).toBe(true)
  })

  // "double-click to maximise"
  it('emits toggleMaximize on double click of the title', async () => {
    const ontoggleMaximize = vi.fn()
    render(TitleBar, { props: { title: 'FakeFlac', ontoggleMaximize } })
    await userEvent.dblClick(screen.getByText('FakeFlac'))
    expect(ontoggleMaximize).toHaveBeenCalledTimes(1)
  })

  it('forwards the window control events', async () => {
    const onminimize = vi.fn()
    const ontoggleMaximize = vi.fn()
    const onclose = vi.fn()
    render(TitleBar, { props: { title: 'FakeFlac', onminimize, ontoggleMaximize, onclose } })
    await userEvent.click(screen.getByRole('button', { name: 'Minimize' }))
    await userEvent.click(screen.getByRole('button', { name: 'Maximize' }))
    await userEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(onminimize).toHaveBeenCalledTimes(1)
    expect(ontoggleMaximize).toHaveBeenCalledTimes(1)
    expect(onclose).toHaveBeenCalledTimes(1)
  })
})

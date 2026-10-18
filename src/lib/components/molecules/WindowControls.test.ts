// WindowControls (specs/ui/components.md, Molecules): minimise, toggle maximise, close.
// The spec gives no texts for the three buttons; chosen accessible names: Minimize, Maximize, Close.
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import WindowControls from './WindowControls.svelte'

describe('WindowControls', () => {
  it('renders the three buttons', () => {
    render(WindowControls)
    expect(screen.getAllByRole('button')).toHaveLength(3)
    expect(screen.getByRole('button', { name: 'Minimize' })).not.toBeNull()
    expect(screen.getByRole('button', { name: 'Maximize' })).not.toBeNull()
    expect(screen.getByRole('button', { name: 'Close' })).not.toBeNull()
  })

  // "The window title bar supports ... the three buttons" (main-screen.md, Interaction)
  it('emits minimize', async () => {
    const onminimize = vi.fn()
    const ontoggleMaximize = vi.fn()
    const onclose = vi.fn()
    render(WindowControls, { props: { onminimize, ontoggleMaximize, onclose } })
    await userEvent.click(screen.getByRole('button', { name: 'Minimize' }))
    expect(onminimize).toHaveBeenCalledTimes(1)
    expect(ontoggleMaximize).not.toHaveBeenCalled()
    expect(onclose).not.toHaveBeenCalled()
  })

  it('emits toggleMaximize', async () => {
    const onminimize = vi.fn()
    const ontoggleMaximize = vi.fn()
    const onclose = vi.fn()
    render(WindowControls, { props: { onminimize, ontoggleMaximize, onclose } })
    await userEvent.click(screen.getByRole('button', { name: 'Maximize' }))
    expect(ontoggleMaximize).toHaveBeenCalledTimes(1)
    expect(onminimize).not.toHaveBeenCalled()
    expect(onclose).not.toHaveBeenCalled()
  })

  it('emits close', async () => {
    const onminimize = vi.fn()
    const ontoggleMaximize = vi.fn()
    const onclose = vi.fn()
    render(WindowControls, { props: { onminimize, ontoggleMaximize, onclose } })
    await userEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(onclose).toHaveBeenCalledTimes(1)
    expect(onminimize).not.toHaveBeenCalled()
    expect(ontoggleMaximize).not.toHaveBeenCalled()
  })
})

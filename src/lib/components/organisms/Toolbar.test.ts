// Toolbar (specs/ui/components.md, Organisms; main-screen.md "Screen states", Toolbar column).
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import Toolbar from './Toolbar.svelte'

const open = () => screen.getByRole('button', { name: 'Open folder' }) as HTMLButtonElement
const cancel = () => screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement

describe('Toolbar', () => {
  // state: idle -> Open enabled, Cancel disabled
  it('idle: Open enabled, Cancel disabled', () => {
    render(Toolbar, { props: { canOpen: true, canCancel: false } })
    expect(open().disabled).toBe(false)
    expect(cancel().disabled).toBe(true)
  })

  // state: running -> Open disabled, Cancel enabled (@RF-05 only one analysis at a time)
  it('running: Open disabled, Cancel enabled', () => {
    render(Toolbar, { props: { canOpen: false, canCancel: true } })
    expect(open().disabled).toBe(true)
    expect(cancel().disabled).toBe(false)
  })

  // @RF-01 Choose a folder with the dialog
  it('emits open when Open folder is clicked', async () => {
    const onopen = vi.fn()
    const oncancel = vi.fn()
    render(Toolbar, { props: { canOpen: true, canCancel: false, onopen, oncancel } })
    await userEvent.click(open())
    expect(onopen).toHaveBeenCalledTimes(1)
    expect(oncancel).not.toHaveBeenCalled()
  })

  // @RF-05 Cancel a running analysis
  it('emits cancel when Cancel is clicked', async () => {
    const onopen = vi.fn()
    const oncancel = vi.fn()
    render(Toolbar, { props: { canOpen: false, canCancel: true, onopen, oncancel } })
    await userEvent.click(cancel())
    expect(oncancel).toHaveBeenCalledTimes(1)
    expect(onopen).not.toHaveBeenCalled()
  })

  it('emits nothing from disabled buttons', async () => {
    const onopen = vi.fn()
    const oncancel = vi.fn()
    render(Toolbar, { props: { canOpen: false, canCancel: false, onopen, oncancel } })
    await userEvent.click(open())
    await userEvent.click(cancel())
    expect(onopen).not.toHaveBeenCalled()
    expect(oncancel).not.toHaveBeenCalled()
  })
})

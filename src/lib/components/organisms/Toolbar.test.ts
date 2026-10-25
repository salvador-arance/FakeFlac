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

// RF-09 specs/ui/export-csv.md "Toolbar": a second group after a separator
const exportButton = () => screen.getByRole('button', { name: 'Export CSV' }) as HTMLButtonElement

describe('Toolbar: Export CSV', () => {
  it('idle with export enabled', () => {
    render(Toolbar, { props: { canOpen: true, canCancel: false, canExport: true } })
    expect(exportButton().disabled).toBe(false)
  })

  it('idle with export disabled', () => {
    render(Toolbar, { props: { canOpen: true, canCancel: false, canExport: false } })
    expect(exportButton().disabled).toBe(true)
  })

  it('running: export disabled', () => {
    render(Toolbar, { props: { canOpen: false, canCancel: true, canExport: false } })
    expect(exportButton().disabled).toBe(true)
  })

  // Reading chosen: the separator is an element with role "separator", just before the export button
  it('has a separator between the open/cancel group and the export group', () => {
    render(Toolbar, { props: { canOpen: true, canCancel: false, canExport: true } })
    const separator = screen.getByRole('separator')
    const order = Array.from(
      separator.parentElement?.querySelectorAll('button, [role="separator"]') ?? [],
    )
    expect(order.indexOf(separator)).toBe(2)
    expect(order[3]).toBe(exportButton())
  })

  it('emits export when clicked, and nothing else', async () => {
    const onexport = vi.fn()
    const onopen = vi.fn()
    render(Toolbar, { props: { canOpen: true, canCancel: false, canExport: true, onexport, onopen } })
    await userEvent.click(exportButton())
    expect(onexport).toHaveBeenCalledTimes(1)
    expect(onopen).not.toHaveBeenCalled()
  })

  it('emits nothing from a disabled export button', async () => {
    const onexport = vi.fn()
    render(Toolbar, { props: { canOpen: true, canCancel: false, canExport: false, onexport } })
    await userEvent.click(exportButton())
    expect(onexport).not.toHaveBeenCalled()
  })
})

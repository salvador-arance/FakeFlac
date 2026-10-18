// IconButton (specs/ui/components.md, Atoms): label as accessible name and tooltip, click, disabled.
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import IconButton from './IconButton.svelte'

describe('IconButton', () => {
  // @RF-01 the toolbar button is named by its label
  it('is a button named by its label, with the label as tooltip', () => {
    render(IconButton, { props: { icon: 'folder-open', label: 'Open folder' } })
    const button = screen.getByRole('button', { name: 'Open folder' })
    expect(button.getAttribute('title')).toBe('Open folder')
  })

  // @RNF-08 the icon inside is decorative: the label already names the button
  it('renders its icon as decorative', () => {
    const { container } = render(IconButton, { props: { icon: 'cancel', label: 'Cancel' } })
    expect(container.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true')
  })

  // state: normal -> click event
  it('calls onclick when clicked', async () => {
    const onclick = vi.fn()
    render(IconButton, { props: { icon: 'cancel', label: 'Cancel', onclick } })
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onclick).toHaveBeenCalledTimes(1)
  })

  // state: disabled
  it('does not call onclick when disabled', async () => {
    const onclick = vi.fn()
    render(IconButton, { props: { icon: 'cancel', label: 'Cancel', disabled: true, onclick } })
    const button = screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement
    expect(button.disabled).toBe(true)
    await userEvent.click(button)
    expect(onclick).not.toHaveBeenCalled()
  })

  it('is enabled by default', () => {
    render(IconButton, { props: { icon: 'cancel', label: 'Cancel' } })
    expect((screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(false)
  })

  // keyboard: focus-visible is CSS, but the button must be reachable and activatable by keyboard
  it('is activated with the keyboard', async () => {
    const onclick = vi.fn()
    render(IconButton, { props: { icon: 'cancel', label: 'Cancel', onclick } })
    const user = userEvent.setup()
    await user.tab()
    await user.keyboard('{Enter}')
    expect(onclick).toHaveBeenCalledTimes(1)
  })
})

// @RF-06 resizable-widths.feature: independent handles, non-resize input, keyboard/reset (W3).
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import type { ComponentProps } from 'svelte'
import { describe, expect, it, vi } from 'vitest'
import ColumnHeader from './ColumnHeader.svelte'
import { pointer } from '../../tests/resizeHarness'

const resize = { width: 230, min: 100, max: 800, label: 'Resize File column', description: 'Resize help', valueText: '230 pixels' }
function setup(tooltip?: string) {
  const onsort = vi.fn(), onresize = vi.fn(), onresizeend = vi.fn(), onresetwidth = vi.fn()
  render(ColumnHeader, { props: { label: 'File', sort: 'asc', resize, tooltip, onsort, onresize, onresizeend, onresetwidth } as unknown as ComponentProps<typeof ColumnHeader> })
  return { onsort, onresize, onresizeend, onresetwidth }
}

describe('@RF-06 resizable ColumnHeader', () => {
  it('places a named resize separator beside, not inside, the sort button', () => {
    setup()
    const button = screen.getByRole('button', { name: 'File' })
    const handle = screen.getByRole('separator', { name: resize.label })
    expect(button.contains(handle)).toBe(false)
    expect(screen.getByRole('columnheader').contains(handle)).toBe(true)
    expect(screen.getByRole('columnheader').getAttribute('aria-sort')).toBe('ascending')
    expect(handle.getAttribute('aria-valuenow')).toBe('230')
  })
  it('sort and handle remain distinct keyboard stops', async () => {
    const { onsort, onresetwidth } = setup()
    const user = userEvent.setup()
    await user.tab()
    expect(document.activeElement).toBe(screen.getByRole('button'))
    await user.keyboard('{Enter}')
    expect(onsort).toHaveBeenCalledTimes(1)
    await user.tab()
    expect(document.activeElement).toBe(screen.getByRole('separator'))
    await user.keyboard('{Enter}')
    expect(onresetwidth).toHaveBeenCalledTimes(1)
    expect(onsort).toHaveBeenCalledTimes(1)
  })
  it('resize, its click and double-click never sort', async () => {
    const { onsort, onresize, onresizeend, onresetwidth } = setup()
    const handle = screen.getByRole('separator')
    await pointer(handle, 'pointerdown', 100)
    await pointer(window, 'pointermove', 140)
    await pointer(window, 'pointerup', 140)
    await fireEvent.click(handle)
    await fireEvent.doubleClick(handle)
    expect(onresize).toHaveBeenCalledWith(40)
    expect(onresizeend).toHaveBeenCalledTimes(1)
    expect(onresetwidth).toHaveBeenCalledTimes(1)
    expect(onsort).not.toHaveBeenCalled()
  })
  it('preserves full hover/accessible labels and existing help', () => {
    const help = 'The source bitrate is an estimate.'
    setup(help)
    const header = screen.getByRole('columnheader')
    expect(header.getAttribute('title')).toContain('File')
    expect(header.getAttribute('title')).toContain(help)
    const button = within(header).getByRole('button', { name: 'File' })
    const description = (button.getAttribute('aria-describedby') ?? '').split(/\s+/).map(id => document.getElementById(id)?.textContent ?? '').join(' ') || button.getAttribute('aria-description')
    expect(description).toContain(help)
  })
})

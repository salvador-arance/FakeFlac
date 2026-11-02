// @RF-06 @RF-10 resizable-widths.feature: Bounds and keyboard operation;
// Finishing an interrupted drag; Non-resize input retains its meaning (W3).
import { fireEvent, render, screen } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import type { ComponentProps } from 'svelte'
import Splitter from './Splitter.svelte'
import { pointer } from '../../tests/resizeHarness'

const base = {
  orientation: 'vertical', label: 'Resize File column', description: 'Resize help',
  value: 230, min: 100, max: 800, valueText: '230 pixels', variant: 'column',
} as const
const props = (extra = {}) => ({ ...base, ...extra } as unknown as ComponentProps<typeof Splitter>)

describe('@RF-06 @RF-10 vertical Splitter', () => {
  it('exposes its localized accessible name, description, width and bounds', () => {
    render(Splitter, { props: props() })
    const handle = screen.getByRole('separator', { name: base.label })
    expect(handle.getAttribute('aria-orientation')).toBe('vertical')
    expect(handle.getAttribute('aria-valuenow')).toBe('230')
    expect(handle.getAttribute('aria-valuemin')).toBe('100')
    expect(handle.getAttribute('aria-valuemax')).toBe('800')
    expect(handle.getAttribute('aria-valuetext')).toBe('230 pixels')
    expect(handle.getAttribute('tabindex')).toBe('0')
    const description = (handle.getAttribute('aria-describedby') ?? '').split(/\s+/).map(id => document.getElementById(id)?.textContent ?? '').join(' ') || handle.getAttribute('aria-description')
    expect(description).toContain('Resize help')
  })
  it('emits horizontal incremental deltas outside the handle and preserves fractions', async () => {
    const onresize = vi.fn(), onresizeend = vi.fn()
    render(Splitter, { props: props({ onresize, onresizeend }) })
    await pointer(screen.getByRole('separator'), 'pointerdown', 100)
    await pointer(window, 'pointermove', 130.25)
    await pointer(window, 'pointermove', 120.5)
    expect(onresize.mock.calls).toEqual([[30.25], [-9.75]])
    await pointer(window, 'pointerup', 120.5)
    expect(onresizeend).toHaveBeenCalledTimes(1)
  })
  it.each(['pointerup', 'pointercancel', 'blur'])(
    '%s commits the changed gesture once and stops subsequent motion', async (event) => {
      const onresize = vi.fn(), onresizeend = vi.fn()
      render(Splitter, { props: props({ onresize, onresizeend }) })
      await pointer(screen.getByRole('separator'), 'pointerdown', 100)
      await pointer(window, 'pointermove', 120)
      if (event === 'blur') await fireEvent(window, new Event('blur'))
      else await pointer(window, event, 120)
      expect(onresizeend).toHaveBeenCalledTimes(1)
      onresize.mockClear()
      await pointer(window, 'pointermove', 200)
      expect(onresize).not.toHaveBeenCalled()
    },
  )
  it('ignores other pointers during a gesture, including their completion', async () => {
    const onresize = vi.fn(), onresizeend = vi.fn()
    render(Splitter, { props: props({ onresize, onresizeend }) })
    await pointer(screen.getByRole('separator'), 'pointerdown', 100, { id: 1 })
    await pointer(window, 'pointermove', 150, { id: 2 })
    await pointer(window, 'pointerup', 150, { id: 2 })
    expect(onresize).not.toHaveBeenCalled()
    expect(onresizeend).not.toHaveBeenCalled()
    await pointer(window, 'pointermove', 110, { id: 1 })
    expect(onresize).toHaveBeenLastCalledWith(10)
    await pointer(window, 'pointerup', 110, { id: 1 })
  })
  it.each([{ button: 2 }, { primary: false }])('does not start a disallowed pointer %o', async (options) => {
    const onresize = vi.fn()
    render(Splitter, { props: props({ onresize }) })
    await pointer(screen.getByRole('separator'), 'pointerdown', 100, options)
    await pointer(window, 'pointermove', 150)
    expect(onresize).not.toHaveBeenCalled()
  })
  it('a click without movement creates no resize completion', async () => {
    const onresize = vi.fn(), onresizeend = vi.fn()
    render(Splitter, { props: props({ onresize, onresizeend }) })
    const handle = screen.getByRole('separator')
    await pointer(handle, 'pointerdown', 100)
    await pointer(window, 'pointerup', 100)
    expect(onresize).not.toHaveBeenCalled()
    expect(onresizeend).not.toHaveBeenCalled()
  })
  it('Left/Right emit ten-pixel steps and commit; Enter and double-click emit only reset', async () => {
    const onresize = vi.fn(), onresizeend = vi.fn(), onreset = vi.fn()
    render(Splitter, { props: props({ onresize, onresizeend, onreset }) })
    const handle = screen.getByRole('separator')
    await fireEvent.keyDown(handle, { key: 'ArrowLeft' })
    await fireEvent.keyDown(handle, { key: 'ArrowRight' })
    expect(onresize.mock.calls).toEqual([[-10], [10]])
    expect(onresizeend).toHaveBeenCalledTimes(2)
    onresize.mockClear(); onresizeend.mockClear()
    await fireEvent.keyDown(handle, { key: 'Enter' })
    await fireEvent.doubleClick(handle)
    expect(onreset).toHaveBeenCalledTimes(2)
    expect(onresize).not.toHaveBeenCalled()
    expect(onresizeend).not.toHaveBeenCalled()
  })
  it('cleans up an active drag on unmount', async () => {
    const onresize = vi.fn()
    const view = render(Splitter, { props: props({ onresize }) })
    await pointer(screen.getByRole('separator'), 'pointerdown', 100)
    await view.unmount()
    await pointer(window, 'pointermove', 150)
    expect(onresize).not.toHaveBeenCalled()
  })
})

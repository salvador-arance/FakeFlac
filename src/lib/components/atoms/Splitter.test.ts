// Splitter (specs/ui/components.md, Atoms): props orientation 'horizontal'; event resize(delta);
// states idle, dragging, focus; keyboard: ArrowUp/ArrowDown move 10 px. @RF-07
// Contract chosen by these tests (the implementation must match):
//  - element: focusable role="separator", aria-orientation="horizontal", tabindex 0
//  - event: `onresize(delta)` callback prop; delta in px, positive = downward
//  - dragging state: attribute data-state="dragging" while a drag is active, "idle" otherwise
//  - pointer events are dispatched on the separator and bubble (listeners on it or on window both work)
import { fireEvent, render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import Splitter from './Splitter.svelte'

// jsdom may lack PointerEvent: a MouseEvent with a pointer type carries clientY just the same
const pointer = (el: Element, type: string, clientY: number) =>
  fireEvent(el, new MouseEvent(type, { bubbles: true, cancelable: true, clientY }))

describe('Splitter', () => {
  it('is a focusable horizontal separator', () => {
    render(Splitter, { props: { orientation: 'horizontal' } })
    const sep = screen.getByRole('separator')
    expect(sep.getAttribute('aria-orientation')).toBe('horizontal')
    expect(sep.getAttribute('tabindex')).toBe('0')
  })

  it('is reachable with the keyboard', async () => {
    render(Splitter, { props: { orientation: 'horizontal' } })
    await userEvent.setup().tab()
    expect(document.activeElement).toBe(screen.getByRole('separator'))
  })

  // the emitted delta is incremental (since the previous event): the sum of deltas is the total movement
  it('emits the vertical pointer delta while dragging, positive downward', async () => {
    const onresize = vi.fn()
    render(Splitter, { props: { orientation: 'horizontal', onresize } })
    const sep = screen.getByRole('separator')
    await pointer(sep, 'pointerdown', 100)
    await pointer(sep, 'pointermove', 130)
    expect(onresize).toHaveBeenLastCalledWith(30)
    await pointer(sep, 'pointermove', 110)
    expect(onresize).toHaveBeenLastCalledWith(-20)
    await pointer(sep, 'pointerup', 110)
  })

  it('emits nothing on pointer moves without a pointerdown', async () => {
    const onresize = vi.fn()
    render(Splitter, { props: { orientation: 'horizontal', onresize } })
    await pointer(screen.getByRole('separator'), 'pointermove', 50)
    expect(onresize).not.toHaveBeenCalled()
  })

  it('stops emitting after pointerup', async () => {
    const onresize = vi.fn()
    render(Splitter, { props: { orientation: 'horizontal', onresize } })
    const sep = screen.getByRole('separator')
    await pointer(sep, 'pointerdown', 100)
    await pointer(sep, 'pointerup', 100)
    onresize.mockClear()
    await pointer(sep, 'pointermove', 160)
    expect(onresize).not.toHaveBeenCalled()
  })

  // state: idle -> dragging -> idle
  it('exposes the dragging state via data-state', async () => {
    render(Splitter, { props: { orientation: 'horizontal' } })
    const sep = screen.getByRole('separator')
    expect(sep.getAttribute('data-state')).toBe('idle')
    await pointer(sep, 'pointerdown', 100)
    expect(sep.getAttribute('data-state')).toBe('dragging')
    await pointer(sep, 'pointerup', 100)
    expect(sep.getAttribute('data-state')).toBe('idle')
  })

  // keyboard: arrows move 10 px
  it('emits -10 on ArrowUp and +10 on ArrowDown', async () => {
    const onresize = vi.fn()
    render(Splitter, { props: { orientation: 'horizontal', onresize } })
    const user = userEvent.setup()
    await user.tab()
    await user.keyboard('{ArrowUp}')
    expect(onresize).toHaveBeenLastCalledWith(-10)
    await user.keyboard('{ArrowDown}')
    expect(onresize).toHaveBeenLastCalledWith(10)
    expect(onresize).toHaveBeenCalledTimes(2)
  })

  it('emits nothing for other keys', async () => {
    const onresize = vi.fn()
    render(Splitter, { props: { orientation: 'horizontal', onresize } })
    const user = userEvent.setup()
    await user.tab()
    await user.keyboard('{ArrowLeft}{ArrowRight}{Enter}a{Home}{End}')
    expect(onresize).not.toHaveBeenCalled()
  })
})

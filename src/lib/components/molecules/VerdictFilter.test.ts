// VerdictFilter (specs/ui/sort-filter.md section 10, `VerdictFilter` row; sections 4.1 and 13;
// @RF-08 @RNF-08): a group named "Filter by verdict" with four toggle IconButtons (size toolbar,
// `pressed`, `tone`), in the order fake, suspicious, real, error, with the `verdict-*` icons and the
// labels as tooltip and accessible name. It keeps no state.
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Verdict } from '../../model'
import Icon from '../atoms/Icon.svelte'
import VerdictFilter from './VerdictFilter.svelte'

const ORDER: [Verdict, string][] = [
  ['fake', 'Show fake tracks'],
  ['suspicious', 'Show suspicious tracks'],
  ['real', 'Show genuine tracks'],
  ['error', 'Show tracks with errors'],
]

const group = () => screen.getByRole('group', { name: 'Filter by verdict' })
const buttons = () => within(group()).getAllByRole('button') as HTMLButtonElement[]
const pressed = () => buttons().map((b) => b.getAttribute('aria-pressed'))

/** The inner drawing of the `verdict-*` icon. */
function iconDrawing(verdict: Verdict): string {
  const { container, unmount } = render(Icon, { props: { name: `verdict-${verdict}` as const } })
  const inner = container.querySelector('svg')?.innerHTML ?? ''
  unmount()
  return inner
}

describe('VerdictFilter (@RF-08)', () => {
  it('is a group named "Filter by verdict" with the four buttons in order', () => {
    render(VerdictFilter, { props: { selected: [] } })
    expect(buttons().map((b) => b.getAttribute('aria-label') ?? b.textContent?.trim())).toEqual(
      ORDER.map(([, label]) => label),
    )
    ORDER.forEach(([, label], i) => {
      expect(screen.getByRole('button', { name: label })).toBe(buttons()[i])
      expect(buttons()[i].getAttribute('title')).toBe(label)
    })
  })

  // @RNF-08 the verdict is shown by the icon's shape and the name, not by colour alone
  it('shows the icon of each verdict, decorative, at toolbar size', () => {
    const drawings = ORDER.map(([verdict]) => iconDrawing(verdict))
    render(VerdictFilter, { props: { selected: [] } })
    buttons().forEach((b, i) => {
      const svg = b.querySelector('svg')
      expect(svg?.getAttribute('aria-hidden')).toBe('true')
      expect(svg?.getAttribute('width')).toBe('24')
      expect(svg?.innerHTML).toBe(drawings[i])
    })
  })

  it('none pressed', () => {
    render(VerdictFilter, { props: { selected: [] } })
    expect(pressed()).toEqual(['false', 'false', 'false', 'false'])
  })

  it('one pressed', () => {
    render(VerdictFilter, { props: { selected: ['suspicious'] } })
    expect(pressed()).toEqual(['false', 'true', 'false', 'false'])
  })

  it('several pressed (whatever the order of `selected`)', () => {
    render(VerdictFilter, { props: { selected: ['error', 'fake'] } })
    expect(pressed()).toEqual(['true', 'false', 'false', 'true'])
  })

  it('all pressed', () => {
    render(VerdictFilter, { props: { selected: ['fake', 'suspicious', 'real', 'error'] } })
    expect(pressed()).toEqual(['true', 'true', 'true', 'true'])
  })

  it('disabled: every button disabled', () => {
    render(VerdictFilter, { props: { selected: [], disabled: true } })
    expect(buttons().map((b) => b.disabled)).toEqual([true, true, true, true])
    expect(pressed()).toEqual(['false', 'false', 'false', 'false'])
  })

  it('disabled with one pressed: the pressed state stays visible', () => {
    render(VerdictFilter, { props: { selected: ['fake'], disabled: true } })
    expect(buttons().map((b) => b.disabled)).toEqual([true, true, true, true])
    expect(pressed()).toEqual(['true', 'false', 'false', 'false'])
  })

  it('emits toggle(verdict) for the clicked button and keeps no state', async () => {
    const ontoggle = vi.fn()
    render(VerdictFilter, { props: { selected: ['real'], ontoggle } })
    for (const [verdict, label] of ORDER) {
      await userEvent.click(screen.getByRole('button', { name: label }))
      expect(ontoggle).toHaveBeenLastCalledWith(verdict)
    }
    expect(ontoggle).toHaveBeenCalledTimes(4)
    expect(pressed()).toEqual(['false', 'false', 'true', 'false'])
  })

  it('follows `selected`', async () => {
    const view = render(VerdictFilter, { props: { selected: [] } })
    await view.rerender({ selected: ['fake', 'real'] })
    expect(pressed()).toEqual(['true', 'false', 'true', 'false'])
  })

  it('emits nothing when disabled', async () => {
    const ontoggle = vi.fn()
    render(VerdictFilter, { props: { selected: [], disabled: true, ontoggle } })
    for (const b of buttons()) await userEvent.click(b)
    expect(ontoggle).not.toHaveBeenCalled()
  })

  it('toggles with Space on the focused button', async () => {
    const ontoggle = vi.fn()
    render(VerdictFilter, { props: { selected: [], ontoggle } })
    const user = userEvent.setup()
    await user.tab()
    await user.tab()
    expect(document.activeElement).toBe(buttons()[1])
    await user.keyboard(' ')
    expect(ontoggle).toHaveBeenCalledWith('suspicious')
  })
})

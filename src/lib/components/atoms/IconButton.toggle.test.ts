// IconButton as a toggle and with a verdict tone (specs/ui/sort-filter.md section 10, `IconButton`
// row; section 13; @RF-08 @RNF-08).
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The pressed look, the hover and focus-visible looks and the verdict colour are CSS, which jsdom
//   does not compute; they are covered by the stories and the manual check M3. What is tested is
//   the markup: `aria-pressed` only when `pressed` is given, the disabled state keeping it, and
//   `tone` changing the rendered markup (a class or style on the button or the icon) without
//   changing the name, the tooltip or the decorative icon.
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Verdict } from '../../model'
import IconButton from './IconButton.svelte'

const button = (name = 'Show fake tracks') => screen.getByRole('button', { name }) as HTMLButtonElement
const base = { icon: 'verdict-fake' as const, label: 'Show fake tracks' }

describe('IconButton: toggle (@RF-08)', () => {
  it('is a plain button without aria-pressed when `pressed` is absent', () => {
    render(IconButton, { props: base })
    expect(button().hasAttribute('aria-pressed')).toBe(false)
  })

  it('toggle not pressed: aria-pressed="false"', () => {
    render(IconButton, { props: { ...base, pressed: false } })
    expect(button().getAttribute('aria-pressed')).toBe('false')
  })

  it('toggle pressed: aria-pressed="true"', () => {
    render(IconButton, { props: { ...base, pressed: true } })
    expect(button().getAttribute('aria-pressed')).toBe('true')
  })

  it('follows the `pressed` prop', async () => {
    const view = render(IconButton, { props: { ...base, pressed: false } })
    await view.rerender({ pressed: true })
    expect(button().getAttribute('aria-pressed')).toBe('true')
    await view.rerender({ pressed: false })
    expect(button().getAttribute('aria-pressed')).toBe('false')
  })

  it('toggle disabled: disabled, not pressed', () => {
    render(IconButton, { props: { ...base, pressed: false, disabled: true } })
    expect(button().disabled).toBe(true)
    expect(button().getAttribute('aria-pressed')).toBe('false')
  })

  it('toggle disabled and pressed: disabled, still aria-pressed="true"', () => {
    render(IconButton, { props: { ...base, pressed: true, disabled: true } })
    expect(button().disabled).toBe(true)
    expect(button().getAttribute('aria-pressed')).toBe('true')
  })

  it('keeps no state: a click emits onclick and leaves aria-pressed to the prop', async () => {
    const onclick = vi.fn()
    render(IconButton, { props: { ...base, pressed: false, onclick } })
    await userEvent.click(button())
    expect(onclick).toHaveBeenCalledTimes(1)
    expect(button().getAttribute('aria-pressed')).toBe('false')
  })

  it('is activated with Space and Enter', async () => {
    const onclick = vi.fn()
    render(IconButton, { props: { ...base, pressed: true, onclick } })
    const user = userEvent.setup()
    await user.tab()
    expect(document.activeElement).toBe(button())
    await user.keyboard(' ')
    await user.keyboard('{Enter}')
    expect(onclick).toHaveBeenCalledTimes(2)
  })

  it('keeps the label as tooltip and accessible name, and the icon decorative', () => {
    const { container } = render(IconButton, { props: { ...base, pressed: true, tone: 'fake' } })
    expect(button().getAttribute('title')).toBe('Show fake tracks')
    expect(container.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true')
  })
})

describe('IconButton: tone (@RF-08 @RNF-08)', () => {
  const markup = (props: Record<string, unknown>) => {
    const { container, unmount } = render(IconButton, { props: { ...base, ...props } })
    const html = container.innerHTML
    unmount()
    return html
  }

  it('changes the rendered markup for each verdict, released and pressed', () => {
    const verdicts: Verdict[] = ['fake', 'suspicious', 'real', 'error']
    for (const pressed of [false, true]) {
      const plain = markup({ pressed })
      const toned = verdicts.map((tone) => markup({ pressed, tone }))
      for (const html of toned) expect(html).not.toBe(plain)
      expect(new Set(toned).size).toBe(4)
    }
  })

  it('a plain button without tone renders as before', () => {
    expect(markup({})).toBe(markup({ tone: undefined }))
  })
})

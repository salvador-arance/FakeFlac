// Icon `settings` (@RF-13; specs/ui/language-settings.md section 4, `Icon` row): `IconName` gains
// `settings`, an original gear SVG in the 16x16 view box, decorative inside the labelled
// IconButton, distinct from the other toolbar icons.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import Icon from './Icon.svelte'
import type { IconName } from './icons'

const DRAWN = 'path, circle, ellipse, line, rect, polyline, polygon'

describe('Icon settings (@RF-13)', () => {
  it('draws the settings icon decorative', () => {
    const { container } = render(Icon, { props: { name: 'settings' } })
    const svg = container.querySelector('svg')
    expect(svg?.getAttribute('viewBox')).toBe('0 0 16 16')
    expect(svg?.getAttribute('aria-hidden')).toBe('true')
    expect(svg?.querySelector(DRAWN)).not.toBeNull()
  })

  it('draws the settings icon labelled', () => {
    const { container } = render(Icon, { props: { name: 'settings', label: 'Settings' } })
    const svg = container.querySelector('svg')
    expect(screen.getByLabelText('Settings')).toBe(svg)
  })

  it('is its own drawing, not one of the other toolbar icons', () => {
    const draw = (name: IconName) => {
      const { container, unmount } = render(Icon, { props: { name } })
      const inner = container.querySelector('svg')?.innerHTML ?? ''
      unmount()
      return inner
    }
    const gear = draw('settings')
    expect(gear).toMatch(/<(path|circle|ellipse|line|rect|polyline|polygon)\b/)
    for (const other of ['clear-cache', 'export', 'cancel', 'close', 'folder-open', 'search'] as IconName[]) {
      expect(gear).not.toBe(draw(other))
    }
  })
})

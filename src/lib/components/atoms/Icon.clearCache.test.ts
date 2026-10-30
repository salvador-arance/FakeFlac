// Icon `clear-cache` (specs/ui/result-cache.md section 4, `Icon` row; @RF-11): a new IconName,
// its own SVG in a 16x16 view box (a database cylinder with a small diagonal cross), decorative
// and labelled, distinct from `export`.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import Icon from './Icon.svelte'

const DRAWN = 'path, circle, ellipse, line, rect, polyline, polygon'

describe('Icon clear-cache (@RF-11)', () => {
  it('draws the clear-cache icon decorative', () => {
    const { container } = render(Icon, { props: { name: 'clear-cache' } })
    const svg = container.querySelector('svg')
    expect(svg?.getAttribute('viewBox')).toBe('0 0 16 16')
    expect(svg?.getAttribute('aria-hidden')).toBe('true')
    expect(svg?.querySelector(DRAWN)).not.toBeNull()
  })

  it('draws the clear-cache icon labelled', () => {
    const { container } = render(Icon, { props: { name: 'clear-cache', label: 'Clear results cache' } })
    const svg = container.querySelector('svg')
    expect(screen.getByLabelText('Clear results cache')).toBe(svg)
    expect(svg?.querySelector(DRAWN)).not.toBeNull()
  })

  it('is its own drawing, not one of the other toolbar icons', () => {
    const draw = (name: 'clear-cache' | 'export' | 'cancel' | 'close' | 'folder-open') => {
      const { container, unmount } = render(Icon, { props: { name } })
      const inner = container.querySelector('svg')?.innerHTML ?? ''
      unmount()
      return inner
    }
    const clear = draw('clear-cache')
    expect(clear).toMatch(/<(path|circle|ellipse|line|rect|polyline|polygon)\b/)
    for (const other of ['export', 'cancel', 'close', 'folder-open'] as const) {
      expect(clear).not.toBe(draw(other))
    }
  })
})

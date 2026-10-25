// Icon `search` (specs/ui/sort-filter.md section 10, `Icon` row; @RF-08): new IconName, own SVG in
// a 16x16 view box (a magnifier); decorative and labelled.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import Icon from './Icon.svelte'

describe('Icon search (@RF-08)', () => {
  it('draws the search icon decorative', () => {
    const { container } = render(Icon, { props: { name: 'search' } })
    const svg = container.querySelector('svg')
    expect(svg?.getAttribute('viewBox')).toBe('0 0 16 16')
    expect(svg?.getAttribute('aria-hidden')).toBe('true')
    expect(svg?.querySelector('path, circle, line')).not.toBeNull()
  })

  it('draws the search icon labelled', () => {
    const { container } = render(Icon, { props: { name: 'search', label: 'Search by name' } })
    const svg = container.querySelector('svg')
    expect(screen.getByLabelText('Search by name')).toBe(svg)
    expect(svg?.querySelector('path, circle, line')).not.toBeNull()
  })

  it('is its own drawing, not one of the other icons', () => {
    const draw = (name: 'search' | 'close' | 'cancel' | 'export') => {
      const { container, unmount } = render(Icon, { props: { name } })
      const inner = container.querySelector('svg')?.innerHTML ?? ''
      unmount()
      return inner
    }
    const search = draw('search')
    expect(search.length).toBeGreaterThan(0)
    for (const other of ['close', 'cancel', 'export'] as const) expect(search).not.toBe(draw(other))
  })
})

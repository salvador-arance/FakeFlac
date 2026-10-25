// Icon (specs/ui/components.md, Atoms): `label` -> aria-label; without it the icon is decorative.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import Icon from './Icon.svelte'

describe('Icon', () => {
  // @RNF-08 decorative icon: hidden from assistive technology
  it('is aria-hidden when it has no label', () => {
    const { container } = render(Icon, { props: { name: 'folder-open' } })
    const svg = container.querySelector('svg')
    expect(svg).not.toBeNull()
    expect(svg?.getAttribute('aria-hidden')).toBe('true')
    expect(svg?.hasAttribute('aria-label')).toBe(false)
  })

  // @RNF-08 labelled icon: exposed with its label
  it('exposes the label as aria-label', () => {
    const { container } = render(Icon, { props: { name: 'folder-open', label: 'Open' } })
    const svg = container.querySelector('svg')
    expect(svg?.getAttribute('aria-label')).toBe('Open')
    expect(svg?.getAttribute('aria-hidden')).not.toBe('true')
    expect(screen.getByLabelText('Open')).toBe(svg)
  })

  // @RF-06 size variants 16 | 24 | 32
  it.each([16, 24, 32] as const)('renders at size %i', (size) => {
    const { container } = render(Icon, { props: { name: 'close', size } })
    const svg = container.querySelector('svg')
    expect(svg?.getAttribute('width')).toBe(String(size))
    expect(svg?.getAttribute('height')).toBe(String(size))
  })
})

describe('Icon export (RF-09)', () => {
  // specs/ui/export-csv.md: new IconName export, own SVG, 16x16 view box; decorative and labelled
  it('draws the export icon decorative', () => {
    const { container } = render(Icon, { props: { name: 'export' } })
    const svg = container.querySelector('svg')
    expect(svg?.getAttribute('viewBox')).toBe('0 0 16 16')
    expect(svg?.getAttribute('aria-hidden')).toBe('true')
    expect(svg?.querySelector('path')).not.toBeNull()
  })

  it('draws the export icon labelled', () => {
    const { container } = render(Icon, { props: { name: 'export', label: 'Export CSV' } })
    const svg = container.querySelector('svg')
    expect(screen.getByLabelText('Export CSV')).toBe(svg)
    expect(svg?.querySelector('path')).not.toBeNull()
  })
})

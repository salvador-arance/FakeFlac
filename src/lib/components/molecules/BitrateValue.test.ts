// BitrateValue (specs/ui/lossy-formats.md, "Molecule BitrateValue"): "320 kbps", "245 kbps VBR",
// "192 kbps ABR", "≈128 kbps", "—" with the accessible label "No bitrate". Plain text, no colour of its own.
// Reading: the accessible name is the `aria-label` when there is one, else the text content.
import { render } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import type { BitrateMode } from '../../model'
import BitrateValue from './BitrateValue.svelte'

const text = (c: HTMLElement) => (c.textContent ?? '').replace(/\s+/g, ' ').trim()
const accessibleName = (c: HTMLElement) => {
  const labelled = c.querySelector('[aria-label]')
  return labelled ? labelled.getAttribute('aria-label') : text(c)
}

describe('BitrateValue', () => {
  // @RF-06 @RF-15 Scenario Outline: BitrateValue states
  it.each([
    [320, 'cbr', false, '320 kbps', '320 kbps'],
    [245, 'vbr', false, '245 kbps VBR', '245 kbps VBR'],
    [192, 'abr', false, '192 kbps ABR', '192 kbps ABR'],
    [128, null, true, '≈128 kbps', '≈128 kbps'],
    [null, null, false, '—', 'No bitrate'],
  ] as [number | null, BitrateMode | null, boolean, string, string][])(
    'kbps %s, mode %s, approx %s shows "%s" named "%s"',
    (kbps, mode, approx, shown, name) => {
      const { container } = render(BitrateValue, { props: { kbps, mode, approx } })
      expect(text(container)).toBe(shown)
      expect(accessibleName(container)).toBe(name)
    },
  )

  // @RF-06 @RF-15 BitrateValue states: mode is optional
  it('works without a mode and without approx', () => {
    const { container } = render(BitrateValue, { props: { kbps: 128 } })
    expect(text(container)).toBe('128 kbps')
  })

  // @RF-06 @RF-15 BitrateValue states: an approximate VBR value keeps the approximation text
  it('an approximate value shows "≈" and no mode suffix for a null mode', () => {
    const { container } = render(BitrateValue, { props: { kbps: 192, mode: null, approx: true } })
    expect(text(container)).toBe('≈192 kbps')
  })

  // @RNF-08 no colour of its own: the verdict carries the colour
  it('sets no colour of its own', () => {
    const { container } = render(BitrateValue, { props: { kbps: 320, mode: 'cbr' } })
    expect(container.querySelector('[style*="color"]')).toBeNull()
  })
})

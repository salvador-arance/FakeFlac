// ProgressBar (specs/ui/components.md, Atoms): role="progressbar", aria-valuenow, indeterminate.
// Reading chosen for aria-valuenow: a percentage (value 0..1 -> 0..100, aria-valuemin 0, aria-valuemax 100).
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import ProgressBar from './ProgressBar.svelte'

describe('ProgressBar', () => {
  // @RF-05 determinate: "the progress bar shows 4 of 4"
  it.each([
    [0.5, '50'],
    [1, '100'],
    [0, '0'],
  ])('determinate value %f -> aria-valuenow %s', (value, expected) => {
    render(ProgressBar, { props: { value } })
    const bar = screen.getByRole('progressbar')
    expect(bar.getAttribute('aria-valuenow')).toBe(expected)
  })

  // @RF-05 indeterminate while scanning: no value
  it('is indeterminate without a value', () => {
    render(ProgressBar)
    const bar = screen.getByRole('progressbar')
    expect(bar.hasAttribute('aria-valuenow')).toBe(false)
  })
})

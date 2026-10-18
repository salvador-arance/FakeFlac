// CutoffValue (specs/ui/components.md, Molecules): "16.0 kHz"; hz == nyquistHz -> "Full band".
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import CutoffValue from './CutoffValue.svelte'

describe('CutoffValue', () => {
  // @RF-06 "cutoff 16.0 kHz"
  it('formats a value in kHz with one decimal', () => {
    render(CutoffValue, { props: { hz: 16000, nyquistHz: 22050 } })
    expect(screen.getByText('16.0 kHz')).not.toBeNull()
  })

  it('keeps the decimal of a non-round value', () => {
    render(CutoffValue, { props: { hz: 19500, nyquistHz: 22050 } })
    expect(screen.getByText('19.5 kHz')).not.toBeNull()
  })

  // @RF-06 state: full band
  it('shows "Full band" when the cutoff is the nyquist frequency', () => {
    render(CutoffValue, { props: { hz: 22050, nyquistHz: 22050 } })
    expect(screen.getByText('Full band')).not.toBeNull()
    expect(screen.queryByText(/kHz/)).toBeNull()
  })

  // @RF-06 state: none (no cutoff found)
  it('shows nothing for a null cutoff', () => {
    const { container } = render(CutoffValue, { props: { hz: null, nyquistHz: 22050 } })
    expect((container.textContent ?? '').trim()).toBe('')
  })
})

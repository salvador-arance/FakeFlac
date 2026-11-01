// CutoffValue in Spanish (@RF-13; specs/ui/translations.md section 3: `cutoff.fullBand` Full band /
// Banda completa; numeric presentation keeps plain digits, the decimal point and kHz).
import { render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import CutoffValue from './CutoffValue.svelte'

afterEach(() => setLocale('en'))

const text = (c: HTMLElement) => (c.textContent ?? '').trim()

describe('CutoffValue: Spanish (@RF-13)', () => {
  it('full band reads "Banda completa"', () => {
    setLocale('es')
    const { container } = render(CutoffValue, { props: { hz: 22050, nyquistHz: 22050 } })
    expect(text(container)).toBe('Banda completa')
  })

  it('a cutoff keeps the decimal point and kHz', () => {
    setLocale('es')
    const { container } = render(CutoffValue, { props: { hz: 16500, nyquistHz: 22050 } })
    expect(text(container)).toBe('16.5 kHz')
  })

  it('re-renders "Full band" while mounted', () => {
    const { container } = render(CutoffValue, { props: { hz: 24000, nyquistHz: 24000 } })
    expect(text(container)).toBe('Full band')
    setLocale('es')
    flushSync()
    expect(text(container)).toBe('Banda completa')
  })
})

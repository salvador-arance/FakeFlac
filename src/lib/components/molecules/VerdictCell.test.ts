// VerdictCell (specs/ui/components.md, Molecules; frontend.md "Verdict tokens").
// RNF-08: the verdict is never conveyed by colour alone: icon + text.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import type { FakeEvidence, Verdict } from '../../model'
import VerdictCell from './VerdictCell.svelte'

const TEXT: Record<Verdict, string> = {
  real: 'Genuine',
  suspicious: 'Suspicious',
  fake: 'Fake',
  error: 'Error',
}

describe('VerdictCell', () => {
  // @RNF-08 @RF-06 "each state is shown with an icon, a text and a colour"
  it.each(Object.entries(TEXT) as [Verdict, string][])(
    'shows an icon and the text for %s',
    (verdict, text) => {
      const { container } = render(VerdictCell, { props: { verdict } })
      expect(screen.getByText(text)).not.toBeNull()
      expect(container.querySelector('svg')).not.toBeNull()
    },
  )

  // @RNF-08 the four verdicts have four different icons (not only a different colour)
  it('uses a different icon for every verdict', () => {
    const icons = (Object.keys(TEXT) as Verdict[]).map((verdict) => {
      const { container, unmount } = render(VerdictCell, { props: { verdict } })
      const svg = container.querySelector('svg')?.outerHTML ?? ''
      unmount()
      return svg
    })
    expect(icons.every((svg) => svg.length > 0)).toBe(true)
    expect(new Set(icons).size).toBe(4)
  })

  // @RNF-08 the icon is decorative: the text carries the meaning for assistive technology
  it('hides the icon from assistive technology', () => {
    const { container } = render(VerdictCell, { props: { verdict: 'fake' } })
    expect(container.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true')
  })

  // @RF-06 reasons go in the tooltip
  it('lists the reasons in the tooltip', () => {
    const { container } = render(VerdictCell, {
      props: { verdict: 'suspicious', reasons: ['gradualRolloff', 'highBandGaps'] },
    })
    const tooltip = container.querySelector('[title]')?.getAttribute('title') ?? ''
    expect(tooltip).toContain('High frequencies fade out gradually')
    expect(tooltip).toContain('Frequent gaps above 16 kHz, as in an MP3')
  })

  it('maps every reason to its text', () => {
    const expected = {
      lowHighFrequencyContent: 'Little high-frequency content: cannot confirm',
      gradualRolloff: 'High frequencies fade out gradually',
      ambiguousCutoff: 'Sharp cutoff at a high frequency',
      highBandGaps: 'Frequent gaps above 16 kHz, as in an MP3',
    } as const
    for (const [reason, text] of Object.entries(expected)) {
      const { container, unmount } = render(VerdictCell, {
        props: { verdict: 'suspicious', reasons: [reason as keyof typeof expected] },
      })
      expect(container.querySelector('[title]')?.getAttribute('title')).toContain(text)
      unmount()
    }
  })
})

describe('VerdictCell: lossy reason', () => {
  // @RF-07 @RF-15 reason.cutoffBelowDeclared is shown in the tooltip like every reason
  it('lists cutoffBelowDeclared in the tooltip', () => {
    const { container } = render(VerdictCell, {
      props: { verdict: 'suspicious', reasons: ['cutoffBelowDeclared'] },
    })
    expect(container.querySelector('[title]')?.getAttribute('title')).toContain(
      'Cutoff lower than expected for its bitrate',
    )
  })
})

// @RF-17 @RF-06 The State cell's tooltip of a fake row shows its evidence.
describe('VerdictCell fake evidence', () => {
  const gap: FakeEvidence = { kind: 'gapStep', value: 12, threshold: 6, margin: 6 }
  const step: FakeEvidence = { kind: 'cutoffStep', value: 24.7, threshold: 10, margin: 14.7 }
  const shortfall: FakeEvidence = { kind: 'shortfall', value: 3500, threshold: 2500, margin: 1000 }
  const gapLine = 'Gaps above 16 kHz, as in an MP3: gap step 12.0 dB, 6.0 dB over the Fake threshold (6.0 dB)'
  const stepLine = 'Sharp cutoff where lossy encoders cut: step 24.7 dB, 14.7 dB over the Fake threshold (10.0 dB)'
  const shortfallLine = 'Content ends 3.5 kHz below what its bitrate keeps: 1.0 kHz over the Fake threshold (2.5 kHz)'

  const examples: [string, FakeEvidence[], string[]][] = [
    ['gap only', [gap], [gapLine]],
    ['gap and cutoff', [gap, step], [gapLine, stepLine]],
    ['shortfall and cutoff', [shortfall, step], [shortfallLine, stepLine]],
  ]
  it.each(examples)('%s includes exactly the evidence item sentences in order', (_case, evidence, lines) => {
    const { container } = render(VerdictCell, { props: { verdict: 'fake', evidence } })
    expect(screen.getByText('Fake')).not.toBeNull()
    expect(container.querySelector('[title]')?.getAttribute('title')).toBe(lines.join('\n'))
    expect(container.querySelector('svg')).not.toBeNull()
  })
})

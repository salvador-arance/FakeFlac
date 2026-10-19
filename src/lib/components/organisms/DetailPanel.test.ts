// DetailPanel (specs/ui/components.md, Organisms; inspect-track.feature @RF-07).
// States: nothing selected · loading · spectrogram · error.
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - Text line "Cutoff 16.0 kHz · Fake" then " · <reason>" with the existing i18n wording of the
//   reasons (`reason.*`); the verdict text is the `verdict.*` text. Tested on the normalised text
//   content of the panel, so the markup is free.
// - Error tracks show the i18n text of the error kind (`error.corruptStream`, ...) and no
//   spectrogram.
// - Props: track, spectrogram, loading (components.md). SpectrogramView's testids are reused.
import { render, screen } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { makeTrack } from '../../testing/fixtures'
import { installCanvasStub, makeSpectrogram } from '../../testing/spectrogram'
import type { CanvasStub } from '../../testing/spectrogram'
import DetailPanel from './DetailPanel.svelte'

let stub: CanvasStub
beforeEach(() => {
  stub = installCanvasStub()
})
afterEach(() => stub.restore())

const text = (container: HTMLElement) => (container.textContent ?? '').replace(/\s+/g, ' ').trim()
const image = () => screen.queryByRole('img', { name: /spectrogram/i })

describe('DetailPanel: nothing selected', () => {
  it('shows no spectrogram, no loading and no cutoff text', () => {
    const { container } = render(DetailPanel, {
      props: { track: null, spectrogram: null, loading: false },
    })
    expect(image()).toBeNull()
    expect(screen.queryByTestId('spectrogram-loading')).toBeNull()
    expect(text(container)).not.toMatch(/Cutoff/)
  })
})

describe('DetailPanel: loading', () => {
  it('shows the loading indicator and no image', () => {
    render(DetailPanel, {
      props: {
        track: makeTrack({ verdict: 'fake', cutoffHz: 16000 }),
        spectrogram: null,
        loading: true,
      },
    })
    expect(screen.getByTestId('spectrogram-loading')).not.toBeNull()
    expect(image()).toBeNull()
  })
})

describe('DetailPanel: spectrogram', () => {
  // @RF-07 Selecting a row shows its spectrogram
  it('shows the spectrogram, the cutoff line and "Cutoff 16.0 kHz · Fake"', () => {
    const { container } = render(DetailPanel, {
      props: {
        track: makeTrack({ path: 'C:/Album A/lp16k.flac', verdict: 'fake', cutoffHz: 16000 }),
        spectrogram: makeSpectrogram({ width: 8, height: 4 }),
        loading: false,
      },
    })
    expect(image()).not.toBeNull()
    expect(screen.queryByTestId('spectrogram-loading')).toBeNull()
    expect(screen.getByTestId('cutoff-line').getAttribute('data-cutoff-hz')).toBe('16000')
    expect(text(container)).toContain('Cutoff 16.0 kHz · Fake')
  })

  // @RF-07 Scenario: A Suspicious track shows its reason in plain words (existing wording)
  it.each([
    ['gradualRolloff', 'High frequencies fade out gradually'],
    ['ambiguousCutoff', 'Sharp cutoff at a high frequency'],
    ['highBandGaps', 'Frequent gaps above 16 kHz, as in an MP3'],
    ['lowHighFrequencyContent', 'Little high-frequency content: cannot confirm'],
  ] as const)('adds the reason %s in plain words', (reason, words) => {
    const { container } = render(DetailPanel, {
      props: {
        track: makeTrack({ verdict: 'suspicious', cutoffHz: 17000, reasons: [reason] }),
        spectrogram: makeSpectrogram(),
        loading: false,
      },
    })
    expect(text(container)).toContain(`Cutoff 17.0 kHz · Suspicious · ${words}`)
  })

  it('shows no cutoff line when the track has no cutoff', () => {
    render(DetailPanel, {
      props: {
        track: makeTrack({ verdict: 'suspicious', cutoffHz: null }),
        spectrogram: makeSpectrogram(),
        loading: false,
      },
    })
    expect(image()).not.toBeNull()
    expect(screen.queryByTestId('cutoff-line')).toBeNull()
  })
})

describe('DetailPanel: error', () => {
  // @RF-07 Track with an error
  it.each([
    [{ kind: 'corruptStream', detail: 'x' }, 'Corrupt stream'],
    [{ kind: 'tooShort' }, 'Too short to analyse'],
    [{ kind: 'unreadable', detail: 'x' }, 'Cannot read the file'],
  ] as const)('shows the reason of %j instead of a spectrogram', (error, words) => {
    const { container } = render(DetailPanel, {
      props: {
        track: makeTrack({ path: 'C:/Mixed/corrupt.flac', verdict: 'error', error: { ...error } }),
        spectrogram: null,
        loading: false,
      },
    })
    expect(text(container)).toContain(words)
    expect(image()).toBeNull()
    expect(screen.queryByTestId('spectrogram-loading')).toBeNull()
    expect(text(container)).not.toMatch(/Cutoff/)
  })
})

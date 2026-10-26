// DetailPanel for a track flagged by rule L6a (specs/features/detection-streaming-source.feature,
// @RF-16 @RF-07; specs/ui/lossy-formats.md, "Streaming sample rate (RF-16)"). The analysis is
// stubbed (no audio): no "≈… kbps source" part, because the estimate is null.
import { render } from '@testing-library/svelte'
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

describe('DetailPanel: a track flagged by its streaming sample rate', () => {
  // @RF-16 @RF-07 Scenario: Detail line of a flagged track
  it('shows the line with the reason text and no source estimate', () => {
    const { container } = render(DetailPanel, {
      props: {
        track: makeTrack({
          path: 'C:/A/a.mp3',
          format: 'mp3',
          codec: 'mp3',
          sampleRate: 48000,
          bitDepth: null,
          declaredBitrateKbps: 320,
          bitrateMode: 'cbr',
          estimatedSourceBitrateKbps: null,
          cutoffHz: 20100,
          verdict: 'suspicious',
          reasons: ['streamingSampleRate'],
        }),
        spectrogram: makeSpectrogram({ width: 8, height: 4 }),
        loading: false,
      },
    })
    expect(text(container)).toContain(
      'Cutoff 20.1 kHz · 320 kbps · Suspicious · Full band at 48 kHz: possibly a streaming rip; genuine 48 kHz files look the same.',
    )
    expect(text(container)).not.toMatch(/kbps source/)
  })
})

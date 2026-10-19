// SpectrogramView (specs/ui/components.md, Organisms; inspect-track.feature @RF-07).
// States: loading · image · none. jsdom has no canvas, so `getContext` is stubbed
// (src/lib/testing/spectrogram.ts).
//
// Readings chosen where the specs are open (reported in the test-stage report):
// - The image is a <canvas role="img" aria-label containing "spectrogram"> whose width/height
//   attributes equal the spectrogram's, painted with one `putImageData` of an image made by
//   `createImageData(width, height)`. Time is the x axis, frequency the y axis with 0 Hz at the
//   BOTTOM row (data[t * height + f] -> pixel x = t, y = height - 1 - f).
// - Loading shows an element with data-testid="spectrogram-loading" (a spinner) and no canvas.
// - The cutoff line is a DOM element data-testid="cutoff-line" with data-cutoff-hz and
//   style `bottom: <cutoffHz / maxFrequencyHz * 100>%`; absent when cutoffHz is null.
import { render, screen } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { installCanvasStub, makeSpectrogram } from '../../testing/spectrogram'
import type { CanvasStub } from '../../testing/spectrogram'
import SpectrogramView from './SpectrogramView.svelte'

let stub: CanvasStub
beforeEach(() => {
  stub = installCanvasStub()
})
afterEach(() => stub.restore())

const canvas = () => screen.getByRole('img', { name: /spectrogram/i }) as HTMLCanvasElement

describe('SpectrogramView: none', () => {
  it('shows neither an image nor the loading indicator', () => {
    render(SpectrogramView, { props: { spectrogram: null, cutoffHz: null, loading: false } })
    expect(screen.queryByRole('img', { name: /spectrogram/i })).toBeNull()
    expect(screen.queryByTestId('spectrogram-loading')).toBeNull()
    expect(screen.queryByTestId('cutoff-line')).toBeNull()
  })
})

describe('SpectrogramView: loading', () => {
  it('shows the loading indicator and no image', () => {
    render(SpectrogramView, { props: { spectrogram: null, cutoffHz: 16000, loading: true } })
    expect(screen.getByTestId('spectrogram-loading')).not.toBeNull()
    expect(screen.queryByRole('img', { name: /spectrogram/i })).toBeNull()
  })
})

describe('SpectrogramView: image', () => {
  // @RF-07 Selecting a row shows its spectrogram
  it('draws the spectrogram on a canvas of its size', () => {
    const spectrogram = makeSpectrogram({ width: 8, height: 4 })
    render(SpectrogramView, { props: { spectrogram, cutoffHz: null, loading: false } })
    expect(canvas().width).toBe(8)
    expect(canvas().height).toBe(4)
    expect(stub.putImageData).toHaveBeenCalledTimes(1)
    expect(screen.queryByTestId('spectrogram-loading')).toBeNull()
  })

  // ADR-0009: row-major by time, 0 Hz at row 0 of the data = bottom of the picture
  it('puts time on x and low frequencies at the bottom', () => {
    const spectrogram = makeSpectrogram({ width: 2, height: 4 })
    spectrogram.data[1 * 4 + 0] = 255 // t = 1, f = 0
    render(SpectrogramView, { props: { spectrogram, cutoffHz: null, loading: false } })
    const image = stub.putImageData.mock.calls[0][0] as {
      width: number
      height: number
      data: Uint8ClampedArray
    }
    expect([image.width, image.height]).toEqual([2, 4])
    const px = (x: number, y: number) =>
      Array.from(image.data.slice((y * 2 + x) * 4, (y * 2 + x) * 4 + 4))
    const bright = px(1, 3)
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 2; x++) {
        expect(px(x, y)[3], `alpha at ${x},${y}`).toBe(255)
        if (x === 1 && y === 3) continue
        expect(px(x, y), `pixel ${x},${y}`).not.toEqual(bright)
        expect(px(x, y)).toEqual(px(0, 0))
      }
    }
  })

  it('maps different bytes to different colours', () => {
    const spectrogram = makeSpectrogram({ width: 1, height: 3 })
    spectrogram.data = [0, 128, 255]
    render(SpectrogramView, { props: { spectrogram, cutoffHz: null, loading: false } })
    const d = (stub.putImageData.mock.calls[0][0] as { data: Uint8ClampedArray }).data
    const colour = (y: number) => Array.from(d.slice(y * 4, y * 4 + 3)).join(',')
    expect(new Set([colour(0), colour(1), colour(2)]).size).toBe(3)
  })

  it('labels the frequency axis in kHz', () => {
    const { container } = render(SpectrogramView, {
      props: { spectrogram: makeSpectrogram(), cutoffHz: null, loading: false },
    })
    expect(container.textContent).toMatch(/kHz/)
  })

  // @RF-07 a horizontal line marks the estimated cutoff
  it('marks the cutoff with a line at its height', () => {
    render(SpectrogramView, {
      props: {
        spectrogram: makeSpectrogram({ maxFrequencyHz: 22050 }),
        cutoffHz: 16000,
        loading: false,
      },
    })
    const line = screen.getByTestId('cutoff-line')
    expect(line.getAttribute('data-cutoff-hz')).toBe('16000')
    expect(parseFloat(line.style.bottom)).toBeCloseTo((16000 / 22050) * 100, 0)
  })

  it('shows no cutoff line without a cutoff', () => {
    render(SpectrogramView, {
      props: { spectrogram: makeSpectrogram(), cutoffHz: null, loading: false },
    })
    expect(screen.queryByTestId('cutoff-line')).toBeNull()
  })

  it('places the line against the spectrogram own maximum frequency', () => {
    render(SpectrogramView, {
      props: {
        spectrogram: makeSpectrogram({ sampleRate: 96000, maxFrequencyHz: 48000 }),
        cutoffHz: 16000,
        loading: false,
      },
    })
    expect(parseFloat(screen.getByTestId('cutoff-line').style.bottom)).toBeCloseTo(100 / 3, 0)
  })
})

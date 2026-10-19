// Synthetic spectrograms and a fake canvas context for component tests (RF-07).
// jsdom has no canvas: tests stub `HTMLCanvasElement.prototype.getContext` with `installCanvasStub`.
import { vi } from 'vitest'
import type { Spectrogram } from '../model'

/** A `width` x `height` spectrogram of one flat byte (row-major by time: `data[t * height + f]`). */
export function makeSpectrogram(o: Partial<Spectrogram> & { fill?: number } = {}): Spectrogram {
  const width = o.width ?? 8
  const height = o.height ?? 4
  const { fill, ...rest } = o
  return {
    width,
    height,
    sampleRate: 44100,
    maxFrequencyHz: 22050,
    dbRange: [-120, 0],
    data: new Array<number>(width * height).fill(fill ?? 0),
    ...rest,
  }
}

export type CanvasStub = {
  /** Arguments of every `putImageData(imageData, dx, dy)` call, in order. */
  putImageData: ReturnType<typeof vi.fn>
  restore: () => void
}

/**
 * Replaces `getContext('2d')` by a fake context. `createImageData(w, h)` returns a plain
 * `{ width, height, data }` (jsdom has no `ImageData`); any other context method is a no-op mock.
 */
export function installCanvasStub(): CanvasStub {
  const putImageData = vi.fn()
  const target: Record<string | symbol, unknown> = {
    createImageData: vi.fn((width: number, height: number) => ({
      width,
      height,
      data: new Uint8ClampedArray(width * height * 4),
    })),
    putImageData,
  }
  const ctx = new Proxy(target, {
    get(t, prop) {
      if (!(prop in t)) t[prop] = vi.fn()
      return t[prop]
    },
  })
  const spy = vi
    .spyOn(HTMLCanvasElement.prototype, 'getContext')
    .mockImplementation((() => ctx) as never)
  return { putImageData, restore: () => spy.mockRestore() }
}

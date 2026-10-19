// API client: getSpectrogram (RF-07, specs/contracts/commands.md `get_spectrogram`).
// Against a mocked Tauri runtime; the only place that may call `invoke()`.
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => {
  class Channel {
    onmessage: (message: unknown) => void = () => {}
  }
  return { invoke, Channel }
})

import { ApiError, getSpectrogram } from './index'

const spectrogram = {
  width: 2,
  height: 3,
  sampleRate: 44100,
  maxFrequencyHz: 22050,
  dbRange: [-120, 0] as [number, number],
  data: [0, 1, 2, 253, 254, 255],
}

beforeEach(() => {
  invoke.mockReset()
})

describe('getSpectrogram', () => {
  // @RF-07 Selecting a row shows its spectrogram: the client asks for the track's path
  it('invokes get_spectrogram with the path and returns the spectrogram', async () => {
    invoke.mockResolvedValue(spectrogram)
    const result = await getSpectrogram('C:/Music/Album A/lp16k.flac')
    expect(invoke).toHaveBeenCalledWith('get_spectrogram', { path: 'C:/Music/Album A/lp16k.flac' })
    expect(result).toEqual(spectrogram)
  })

  it('passes a Unicode path exactly', async () => {
    invoke.mockResolvedValue(spectrogram)
    const path = 'C:/Música/Álbum ″Ñandú″/01 – Canción ‘x’.flac'
    await getSpectrogram(path)
    expect(invoke).toHaveBeenCalledWith('get_spectrogram', { path })
  })

  // Contract: INTERNAL if the file cannot be decoded
  it('rejects with an ApiError carrying INTERNAL', async () => {
    invoke.mockRejectedValue({ code: 'INTERNAL', message: 'cannot decode' })
    const error = await getSpectrogram('C:/A/bad.flac').catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('INTERNAL')
    expect((error as ApiError).message).toBe('cannot decode')
  })
})

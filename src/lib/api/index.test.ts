// Tests of the API client (RF-01, RF-02, RF-05) against a mocked Tauri runtime.
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invoke, channels } = vi.hoisted(() => ({
  invoke: vi.fn(),
  channels: [] as { onmessage: (message: unknown) => void }[],
}))

vi.mock('@tauri-apps/api/core', () => {
  class Channel {
    onmessage: (message: unknown) => void = () => {}
    constructor() {
      channels.push(this)
    }
  }
  return { invoke, Channel }
})

import { ApiError, cancelAnalysis, pickFolder, startAnalysis, startAnalysisOfDropped } from './index'
import type { AnalysisSummary, ProgressEvent } from './index'

const summary: AnalysisSummary = {
  root: 'C:/A',
  total: 1,
  done: 1,
  real: 1,
  suspicious: 0,
  fake: 0,
  errors: 0,
  cancelled: false,
  elapsedMs: 10,
}

beforeEach(() => {
  invoke.mockReset()
  channels.length = 0
})

describe('pickFolder', () => {
  // @RF-01 Choose a folder with the dialog
  it('returns the chosen path', async () => {
    invoke.mockResolvedValue('C:/Music')
    expect(await pickFolder()).toBe('C:/Music')
    expect(invoke).toHaveBeenCalledWith('pick_folder')
  })

  // @RF-01 Cancel the folder dialog
  it('returns null when the dialog is cancelled', async () => {
    invoke.mockResolvedValue(null)
    expect(await pickFolder()).toBeNull()
  })
})

describe('startAnalysis', () => {
  // @RF-05 progress events reach the caller in order
  it('passes a channel to the command and forwards events in order', async () => {
    const events: ProgressEvent[] = [
      { kind: 'scanned', total: 1 },
      { kind: 'trackStarted', index: 0, path: 'C:/A/a.wav' },
      { kind: 'finished', summary },
    ]
    invoke.mockImplementation(async () => {
      for (const e of events) channels[0].onmessage(e)
      return summary
    })
    const received: ProgressEvent[] = []
    const result = await startAnalysis('C:/A', (e) => received.push(e))

    expect(invoke).toHaveBeenCalledWith('start_analysis', {
      root: 'C:/A',
      onProgress: channels[0],
    })
    expect(received).toEqual(events)
    expect(result).toEqual(summary)
  })

  // @RF-05 Only one analysis at a time
  it('rejects with an ApiError carrying the contract code', async () => {
    invoke.mockRejectedValue({ code: 'ANALYSIS_RUNNING', message: 'busy' })
    const error = await startAnalysis('C:/A', () => {}).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('ANALYSIS_RUNNING')
    expect((error as ApiError).message).toBe('busy')
  })

  // @RF-02 Folder without supported audio
  it('rejects with NO_AUDIO_FOUND', async () => {
    invoke.mockRejectedValue({ code: 'NO_AUDIO_FOUND', message: 'nothing here' })
    const error = await startAnalysis('C:/A', () => {}).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('NO_AUDIO_FOUND')
  })

  it('rethrows a JS Error as is', async () => {
    const boom = new Error('boom')
    invoke.mockRejectedValue(boom)
    const error = await startAnalysis('C:/A', () => {}).catch((e: unknown) => e)
    expect(error).toBe(boom)
    expect(error).not.toBeInstanceOf(ApiError)
  })
})

describe('startAnalysis ordering and errors', () => {
  // Channel messages and the command reply travel on different paths: the promise must not
  // resolve before the terminal event has reached the caller (@RF-05).
  it('resolves only after the terminal event was delivered', async () => {
    invoke.mockImplementation(async () => {
      const channel = channels[0]
      setTimeout(() => {
        channel.onmessage({ kind: 'scanned', total: 1 })
        channel.onmessage({ kind: 'finished', summary })
      }, 20)
      return summary
    })
    const received: ProgressEvent[] = []
    const result = await startAnalysis('C:/A', (e) => received.push(e))
    expect(result).toEqual(summary)
    expect(received.map((e) => e.kind)).toEqual(['scanned', 'finished'])
  })

  it('also waits for a cancelled terminal event', async () => {
    const cancelled = { ...summary, cancelled: true, done: 0 }
    invoke.mockImplementation(async () => {
      const channel = channels[0]
      setTimeout(() => channel.onmessage({ kind: 'cancelled', summary: cancelled }), 20)
      return cancelled
    })
    const received: ProgressEvent[] = []
    await startAnalysis('C:/A', (e) => received.push(e))
    expect(received.map((e) => e.kind)).toEqual(['cancelled'])
  })

  // Tauri rejects with a plain string for IPC-level failures (e.g. invalid arguments).
  it('wraps a non-contract rejection as INTERNAL', async () => {
    invoke.mockRejectedValue('invalid args `root` for command `start_analysis`')
    const error = await startAnalysis('C:/A', () => {}).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('INTERNAL')
    expect((error as ApiError).message).toContain('invalid args')
  })
})

describe('startAnalysisOfDropped', () => {
  // Fake backend: paths ending in an audio/image extension are files; anything else is a folder.
  function backend(folderError?: { code: string; message: string }) {
    invoke.mockImplementation(async (_cmd: string, args: { root: string }) => {
      if (/\.(flac|wav|jpg)$/.test(args.root)) {
        throw { code: 'NOT_A_FOLDER', message: 'a file' }
      }
      if (folderError) throw folderError
      const channel = channels[channels.length - 1]
      channel.onmessage({ kind: 'scanned', total: 1 })
      channel.onmessage({ kind: 'finished', summary: { ...summary, root: args.root } })
      return { ...summary, root: args.root }
    })
  }

  // @RF-01 Drop a folder on the window
  it('analyses a dropped folder', async () => {
    backend()
    const result = await startAnalysisOfDropped(['C:/Album A'], () => {})
    expect(result?.root).toBe('C:/Album A')
    expect(invoke).toHaveBeenCalledTimes(1)
  })

  // @RF-01 Drop files and a folder together
  it('skips files and analyses the first folder', async () => {
    backend()
    const received: ProgressEvent[] = []
    const result = await startAnalysisOfDropped(['C:/a.flac', 'C:/Album A', 'C:/Album B'], (e) =>
      received.push(e),
    )
    expect(result?.root).toBe('C:/Album A')
    expect(invoke).toHaveBeenCalledTimes(2)
    expect(invoke).toHaveBeenLastCalledWith('start_analysis', expect.objectContaining({ root: 'C:/Album A' }))
    expect(received.map((e) => e.kind)).toEqual(['scanned', 'finished'])
  })

  // @RF-01 Dropped files are ignored: no analysis and no error
  it('resolves with null when every dropped path is a file', async () => {
    backend()
    await expect(startAnalysisOfDropped(['C:/cover.jpg', 'C:/a.flac'], () => {})).resolves.toBeNull()
  })

  it('stops at the first error that is not NOT_A_FOLDER', async () => {
    invoke.mockRejectedValue({ code: 'FOLDER_UNREADABLE', message: 'locked' })
    const error = await startAnalysisOfDropped(['C:/Locked', 'C:/Album A'], () => {}).catch(
      (e: unknown) => e,
    )
    expect((error as ApiError).code).toBe('FOLDER_UNREADABLE')
    expect(invoke).toHaveBeenCalledTimes(1)
  })

  it('resolves with null for an empty drop without calling the backend', async () => {
    backend()
    await expect(startAnalysisOfDropped([], () => {})).resolves.toBeNull()
    expect(invoke).not.toHaveBeenCalled()
  })

  // Any other error is shown as usual (contract v2)
  it('rejects with other contract errors', async () => {
    backend({ code: 'NO_AUDIO_FOUND', message: 'nothing here' })
    const error = await startAnalysisOfDropped(['C:/a.flac', 'C:/Docs'], () => {}).catch(
      (e: unknown) => e,
    )
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('NO_AUDIO_FOUND')
  })
})

describe('cancelAnalysis', () => {
  // @RF-05 cancel
  it('invokes cancel_analysis', async () => {
    invoke.mockResolvedValue(undefined)
    await cancelAnalysis()
    expect(invoke).toHaveBeenCalledWith('cancel_analysis')
  })
})

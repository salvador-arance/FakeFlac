// API client: clearCache (RF-11, specs/contracts/commands.md `clear_cache`, version 7;
// specs/contracts/result-cache.md section 10.4). Against a mocked Tauri runtime; the only place
// that may call `invoke()`. It rejects with `ApiError` like `exportCsv`.
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => {
  class Channel {
    onmessage: (message: unknown) => void = () => {}
  }
  return { invoke, Channel }
})

import { ApiError, clearCache } from './index'

beforeEach(() => {
  invoke.mockReset()
})

describe('clearCache (@RF-11)', () => {
  it('invokes clear_cache without arguments and resolves with nothing', async () => {
    invoke.mockResolvedValue(null)
    await expect(clearCache()).resolves.toBeUndefined()
    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke.mock.calls[0][0]).toBe('clear_cache')
    expect(invoke.mock.calls[0][1] ?? {}).toEqual({})
  })

  // Contract errors: ANALYSIS_RUNNING, CACHE_FAILED, INTERNAL
  it.each(['ANALYSIS_RUNNING', 'CACHE_FAILED', 'INTERNAL'])(
    'rejects with an ApiError carrying %s',
    async (code) => {
      invoke.mockRejectedValue({ code, message: 'technical message' })
      const error = await clearCache().catch((e: unknown) => e)
      expect(error).toBeInstanceOf(ApiError)
      expect((error as ApiError).code).toBe(code)
      expect((error as ApiError).message).toBe('technical message')
    },
  )

  it('turns an IPC-level failure (a plain string) into INTERNAL', async () => {
    invoke.mockRejectedValue('ipc failed')
    const error = await clearCache().catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('INTERNAL')
  })
})

// API client: exportCsv (RF-09, specs/contracts/commands.md `export_csv`, version 4).
// Against a mocked Tauri runtime; the only place that may call `invoke()`.
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => {
  class Channel {
    onmessage: (message: unknown) => void = () => {}
  }
  return { invoke, Channel }
})

import { ApiError, exportCsv } from './index'

beforeEach(() => {
  invoke.mockReset()
})

describe('exportCsv', () => {
  // The destination never comes from the frontend (csv-export.md Q7): no argument at all
  it('invokes export_csv without a destination and returns the result', async () => {
    invoke.mockResolvedValue({ path: 'C:\\Out\\FakeFlac - Album A.csv', rows: 31 })
    const result = await exportCsv()
    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke.mock.calls[0][0]).toBe('export_csv')
    expect(invoke.mock.calls[0][1] ?? {}).toEqual({})
    expect(result).toEqual({ path: 'C:\\Out\\FakeFlac - Album A.csv', rows: 31 })
  })

  // @RF-09 Scenario: Cancelling the save dialog writes nothing: resolves with null
  it('resolves with null when the dialog is cancelled', async () => {
    invoke.mockResolvedValue(null)
    expect(await exportCsv()).toBeNull()
  })

  // Contract errors: NOT_ANALYZED, ANALYSIS_RUNNING, EXPORT_FAILED, INTERNAL
  it.each(['NOT_ANALYZED', 'ANALYSIS_RUNNING', 'EXPORT_FAILED', 'INTERNAL'])(
    'rejects with an ApiError carrying %s',
    async (code) => {
      invoke.mockRejectedValue({ code, message: 'technical message' })
      const error = await exportCsv().catch((e: unknown) => e)
      expect(error).toBeInstanceOf(ApiError)
      expect((error as ApiError).code).toBe(code)
      expect((error as ApiError).message).toBe('technical message')
    },
  )
})

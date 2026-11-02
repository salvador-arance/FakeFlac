// API client: getSettings / setSettings (@RF-13; specs/contracts/settings.md section 2, contract
// version 8). Against a mocked Tauri runtime; the client is the only place that may call
// `invoke()`. Contract errors throw `ApiError`; unexpected rejections become `INTERNAL`.
// Scenario "An unreadable configuration does not prevent analysis" (example "unexpected IPC
// rejection", client side) and the client half of "A failed save preserves the old preference".
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => {
  class Channel {
    onmessage: (message: unknown) => void = () => {}
  }
  return { invoke, Channel }
})

import { ApiError, getSettings, setSettings } from './index'
import type { Language, Settings, SettingsLoadStatus, SettingsSnapshot } from './index'

beforeEach(() => {
  invoke.mockReset()
})

const snapshot = (language: Language, loadStatus: SettingsLoadStatus): SettingsSnapshot => ({
  settings: { language },
  loadStatus,
})

describe('getSettings (@RF-13)', () => {
  it.each<SettingsLoadStatus>(['default', 'loaded', 'invalid', 'unavailable', 'newerVersion'])(
    'invokes get_settings without arguments and resolves with the %s snapshot',
    async (status) => {
      invoke.mockResolvedValue(snapshot('es', status))
      await expect(getSettings()).resolves.toEqual(snapshot('es', status))
      expect(invoke).toHaveBeenCalledTimes(1)
      expect(invoke.mock.calls[0][0]).toBe('get_settings')
      expect(invoke.mock.calls[0][1] ?? {}).toEqual({})
    },
  )

  // Examples: | unexpected IPC rejection |
  it('turns an IPC-level failure (a plain string) into an INTERNAL ApiError', async () => {
    invoke.mockRejectedValue('ipc failed')
    const error = await getSettings().catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('INTERNAL')
  })

  it('rejects with an ApiError carrying INTERNAL for a contract error', async () => {
    invoke.mockRejectedValue({ code: 'INTERNAL', message: 'technical message' })
    const error = await getSettings().catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('INTERNAL')
  })
})

describe('setSettings (@RF-13)', () => {
  it.each<Language>(['en', 'es'])(
    'invokes set_settings with { settings: { language: %s } } and resolves with the snapshot',
    async (language) => {
      invoke.mockResolvedValue(snapshot(language, 'loaded'))
      const settings: Settings = { language }
      await expect(setSettings(settings)).resolves.toEqual(snapshot(language, 'loaded'))
      expect(invoke).toHaveBeenCalledTimes(1)
      expect(invoke).toHaveBeenCalledWith('set_settings', { settings: { language } })
    },
  )

  // Contract errors: SETTINGS_WRITE_FAILED, SETTINGS_READ_ONLY, INTERNAL
  it.each(['SETTINGS_WRITE_FAILED', 'SETTINGS_READ_ONLY', 'INTERNAL'])(
    'rejects with an ApiError carrying %s',
    async (code) => {
      invoke.mockRejectedValue({ code, message: 'technical message' })
      const error = await setSettings({ language: 'es' }).catch((e: unknown) => e)
      expect(error).toBeInstanceOf(ApiError)
      expect((error as ApiError).code).toBe(code)
      expect((error as ApiError).message).toBe('technical message')
    },
  )

  it('turns an IPC-level failure (a plain string) into INTERNAL', async () => {
    invoke.mockRejectedValue('ipc failed')
    const error = await setSettings({ language: 'es' }).catch((e: unknown) => e)
    expect(error).toBeInstanceOf(ApiError)
    expect((error as ApiError).code).toBe('INTERNAL')
  })
})

// @RF-06 @RF-10 resizable-widths.feature: persistence/recovery, bounds and independent defaults (W1/W2/W4).
// API reading: the helper accepts an optional Storage-like dependency; preferences exclude version.
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { SortColumn } from './model'

type Preferences = { sidebarWidth?: number; columns?: Partial<Record<SortColumn, number>> }
type StorageLike = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>
type WidthsApi = {
  readLayoutWidths(storage?: StorageLike): Preferences
  writeLayoutWidths(value: Preferences, storage?: StorageLike): void
  defaultColumnWidths(viewportWidth: number): Record<SortColumn, number>
  clampColumnWidth(column: SortColumn, width: number): number
  clampSidebarWidth(width: number, contentWidth: number): number
}

// The approved helper does not exist at the red stage. Collect all assertions without a
// production stub or missing-import compilation error; Vite discovers the module once added.
const modules = import.meta.glob('./layout-widths.ts', { eager: true })
function api(): WidthsApi {
  const module = modules['./layout-widths.ts'] as WidthsApi | undefined
  expect(module, 'approved layout-widths helper exports are not implemented').toBeDefined()
  return module!
}
const KEY = 'fakeflac.layout.widths'
const storage = (raw: string | null = null): StorageLike => ({ getItem: vi.fn(() => raw), setItem: vi.fn(), removeItem: vi.fn() })
afterEach(() => vi.restoreAllMocks())

describe('@RF-06 @RF-10 local width preferences: storage recovery table', () => {
  it.each([null, '{', 'null', '[]', 'true', '1', '{"version":"1"}', '{"version":2}', '{}'])(
    'defaults without repairing storage: %s', (raw) => {
      const store = storage(raw)
      const value = api().readLayoutWidths(store)
      expect(value.sidebarWidth).toBeUndefined()
      expect(value.columns ?? {}).toEqual({})
      expect(store.setItem).not.toHaveBeenCalled()
      expect(store.removeItem).not.toHaveBeenCalled()
    },
  )
  it('restores only stable known keys, independently of unknown fields and invalid widths', () => {
    const store = storage(JSON.stringify({ version: 1, sidebarWidth: 320, columns: { file: 230, folder: 'bad', bits: 99, state: true }, extra: 'ignored' }))
    expect(api().readLayoutWidths(store)).toMatchObject({ sidebarWidth: 320, columns: { file: 230 } })
    const result = api().readLayoutWidths(store)
    expect(result.columns).not.toHaveProperty('folder')
    expect(result.columns).not.toHaveProperty('bits')
    expect(result.columns).not.toHaveProperty('state')
    expect(result).not.toHaveProperty('extra')
  })
  it.each(['null', 'true', '"bad"', '[]'])(
    'invalid columns recover independently of a valid sidebar: %s', (columns) => {
      const result = api().readLayoutWidths(storage(`{"version":1,"sidebarWidth":320,"columns":${columns}}`))
      expect(result.sidebarWidth).toBe(320)
      expect(result.columns ?? {}).toEqual({})
    },
  )
  it('rounds then clamps finite values using each user bound', () => {
    expect(api().readLayoutWidths(storage('{"version":1,"sidebarWidth":1000,"columns":{"file":99.6,"folder":900,"state":-1,"bitDepth":50.6}}'))).toMatchObject({
      sidebarWidth: 480, columns: { file: 100, folder: 800, state: 120, bitDepth: 51 },
    })
  })
  it('ignores non-finite and invalid sidebar values without discarding valid column widths', () => {
    for (const sidebarWidth of ['"320"', 'null', 'true', '1e999']) {
      const result = api().readLayoutWidths(storage(`{"version":1,"sidebarWidth":${sidebarWidth},"columns":{"file":230}}`))
      expect(result.sidebarWidth).toBeUndefined()
      expect(result.columns).toEqual({ file: 230 })
    }
    expect(api().readLayoutWidths(storage('{"version":1,"columns":{"file":1e999}}')).columns ?? {}).toEqual({})
  })
  it('reading an unavailable storage falls back without throwing', () => {
    const store = storage()
    vi.mocked(store.getItem).mockImplementation(() => { throw new Error('blocked') })
    const result = api().readLayoutWidths(store)
    expect(result.sidebarWidth).toBeUndefined()
    expect(result.columns ?? {}).toEqual({})
  })
  it('also catches a throwing localStorage getter when no injected storage is supplied', () => {
    const helper = api()
    vi.spyOn(window, 'localStorage', 'get').mockImplementation(() => { throw new Error('unavailable getter') })
    const result = helper.readLayoutWidths()
    expect(result.sidebarWidth).toBeUndefined()
    expect(result.columns ?? {}).toEqual({})
    expect(() => helper.writeLayoutWidths({ columns: { file: 230 } })).not.toThrow()
  })
  it('writes a normalized version-one snapshot and no unrelated state', () => {
    const store = storage()
    api().writeLayoutWidths({ sidebarWidth: 320, columns: { file: 230 } }, store)
    expect(store.setItem).toHaveBeenCalledTimes(1)
    const [key, raw] = vi.mocked(store.setItem).mock.calls[0]
    expect(key).toBe(KEY)
    expect(JSON.parse(raw)).toEqual({ version: 1, sidebarWidth: 320, columns: { file: 230 } })
  })
  it('removes the record when every target is in default mode', () => {
    const store = storage()
    api().writeLayoutWidths({ columns: {} }, store)
    expect(store.removeItem).toHaveBeenCalledWith(KEY)
    expect(store.setItem).not.toHaveBeenCalled()
  })
  it('does not leak write/remove failures into analysis', () => {
    const store = storage()
    vi.mocked(store.setItem).mockImplementation(() => { throw new Error('quota') })
    vi.mocked(store.removeItem).mockImplementation(() => { throw new Error('blocked') })
    const helper = api()
    expect(() => helper.writeLayoutWidths({ columns: { file: 230 } }, store)).not.toThrow()
    expect(() => helper.writeLayoutWidths({}, store)).not.toThrow()
  })
})

describe('@RF-06 @RF-10 width design bounds and responsive defaults (W1/W2)', () => {
  it.each([[0, 140, 140], [970, 140, 140], [1040, 150, 200], [1390, 300, 400], [10000, 800, 800]])(
    'viewport %s gives File %s and Folder %s', (viewport, file, folder) => {
      expect(api().defaultColumnWidths(viewport)).toEqual({ state: 120, file, folder, format: 70, sampleRate: 80, bitDepth: 50, duration: 70, cutoff: 90, bitrate: 110, realBitrate: 100 })
    },
  )
  it.each(Object.entries({ state: 120, file: 100, folder: 100, format: 70, sampleRate: 80, bitDepth: 50, duration: 70, cutoff: 90, bitrate: 110, realBitrate: 100 }))(
    '%s has its specified lower bound and upper bound 800', (column, min) => {
      expect(api().clampColumnWidth(column as SortColumn, -10)).toBe(min)
      expect(api().clampColumnWidth(column as SortColumn, 900)).toBe(800)
      expect(api().clampColumnWidth(column as SortColumn, min + 1.25)).toBe(min + 1.25)
    },
  )
  it('clamps the displayed sidebar without replacing a preference', () => {
    expect(api().clampSidebarWidth(480, 764)).toBe(400)
    expect(api().clampSidebarWidth(480, 900)).toBe(480)
    expect(api().clampSidebarWidth(320, 0)).toBe(320)
    expect(api().clampSidebarWidth(100, 900)).toBe(160)
  })
})

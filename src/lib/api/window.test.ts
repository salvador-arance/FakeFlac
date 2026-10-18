// API client: window controls and drag-drop (RF-01), against mocked Tauri window/webview modules.
// Components never import @tauri-apps/*: the client is the only place that does.
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { win, webview, unlisten, dragState } = vi.hoisted(() => ({
  win: {
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
  },
  webview: { onDragDropEvent: vi.fn() },
  unlisten: vi.fn(),
  dragState: { callback: null as null | ((event: { payload: unknown }) => void) },
}))

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), Channel: class {} }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => win }))
vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => webview }))

import { closeWindow, minimizeWindow, onDragDrop, toggleMaximizeWindow } from './index'
import type { DragDropEvent } from './index'

beforeEach(() => {
  win.minimize.mockReset().mockResolvedValue(undefined)
  win.toggleMaximize.mockReset().mockResolvedValue(undefined)
  win.close.mockReset().mockResolvedValue(undefined)
  unlisten.mockReset()
  dragState.callback = null
  webview.onDragDropEvent.mockReset().mockImplementation(async (callback) => {
    dragState.callback = callback
    return unlisten
  })
})

describe('window controls', () => {
  // The custom title bar (main-screen.md, Interaction)
  it('minimizeWindow minimises the current window', async () => {
    await minimizeWindow()
    expect(win.minimize).toHaveBeenCalledTimes(1)
  })

  it('toggleMaximizeWindow toggles the maximised state', async () => {
    await toggleMaximizeWindow()
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1)
  })

  it('closeWindow closes the current window', async () => {
    await closeWindow()
    expect(win.close).toHaveBeenCalledTimes(1)
  })
})

describe('onDragDrop', () => {
  const position = { x: 1, y: 2 }

  // @RF-01 Drop a folder on the window
  it('forwards a drop with the dropped paths', async () => {
    const handler = vi.fn<(event: DragDropEvent) => void>()
    await onDragDrop(handler)
    dragState.callback?.({ payload: { type: 'drop', paths: ['C:/Album A', 'C:/a.flac'], position } })
    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'drop', paths: ['C:/Album A', 'C:/a.flac'] }),
    )
  })

  // main-screen.md "Dragging over": enter/leave drive the overlay
  it.each(['enter', 'over', 'leave'] as const)('forwards %s', async (type) => {
    const handler = vi.fn<(event: DragDropEvent) => void>()
    await onDragDrop(handler)
    dragState.callback?.({ payload: { type, paths: [], position } })
    expect(handler).toHaveBeenCalledWith(expect.objectContaining({ type }))
  })

  it('resolves with a function that stops listening', async () => {
    const stop = await onDragDrop(() => {})
    expect(unlisten).not.toHaveBeenCalled()
    stop()
    expect(unlisten).toHaveBeenCalledTimes(1)
  })
})

// Controlled browser inputs for resizable-widths.feature (@RF-06 @RF-10).
// Geometry is supplied as ResizeObserver would report it; real layout is M1, not jsdom.
import { fireEvent } from '@testing-library/svelte'
import { vi } from 'vitest'

export function pointer(target: Element | Window, type: string, x: number, options: { id?: number; button?: number; primary?: boolean } = {}) {
  const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, button: options.button ?? 0 })
  Object.defineProperties(event, {
    pointerId: { value: options.id ?? 1 },
    isPrimary: { value: options.primary ?? true },
    pointerType: { value: 'mouse' },
  })
  return fireEvent(target, event)
}

const observers = new Set<{ callback: ResizeObserverCallback; targets: Set<Element> }>()
export function measurements() {
  class Observer {
    targets = new Set<Element>()
    constructor(public callback: ResizeObserverCallback) { observers.add(this) }
    observe(element: Element) { this.targets.add(element) }
    unobserve(element: Element) { this.targets.delete(element) }
    disconnect() { this.targets.clear(); observers.delete(this) }
  }
  vi.stubGlobal('ResizeObserver', Observer)
  return {
    set(element: HTMLElement, width: number, height = 220) {
      Object.defineProperties(element, {
        clientWidth: { configurable: true, value: width },
        clientHeight: { configurable: true, value: height },
        offsetWidth: { configurable: true, value: width },
        offsetHeight: { configurable: true, value: height },
      })
      element.getBoundingClientRect = () => new DOMRect(0, 0, width, height)
      const entry = {
        target: element,
        contentRect: new DOMRect(0, 0, width, height),
        contentBoxSize: [{ inlineSize: width, blockSize: height }],
        borderBoxSize: [{ inlineSize: width, blockSize: height }],
        devicePixelContentBoxSize: [{ inlineSize: width, blockSize: height }],
      } as ResizeObserverEntry
      for (const observer of observers) if (observer.targets.has(element)) observer.callback([entry], observer as unknown as ResizeObserver)
      window.dispatchEvent(new Event('resize'))
    },
  }
}

export const WIDTHS_KEY = 'fakeflac.layout.widths'
export const columns = ['state', 'file', 'folder', 'format', 'sampleRate', 'bitDepth', 'duration', 'cutoff', 'bitrate', 'realBitrate'] as const
export const labels = ['State', 'File', 'Folder', 'Format', 'Samplerate', 'Bits', 'Duration', 'Cutoff', 'Bitrate', 'Real bitrate'] as const

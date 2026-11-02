// @RF-06 @RF-10 resizable-widths.feature: every handle; independent responsive widths; W2/W3/W5.
// Width behavior is asserted via the separator's numeric accessible value. Real pixel alignment is M1.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import type { ComponentProps } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import type { SortColumn, SortState } from '../../model'
import { makeManyRows, makeRow } from '../../testing/fixtures'
import { columns, labels, measurements, pointer } from '../../tests/resizeHarness'
import ResultsTable from './ResultsTable.svelte'

const sort: SortState = { column: 'file', direction: 'asc' }
const value = (handle: HTMLElement) => Number(handle.getAttribute('aria-valuenow'))
const handle = (name = 'File') => screen.getByRole('separator', { name: `Resize ${name} column` })
afterEach(() => { setLocale('en'); vi.unstubAllGlobals() })

function setup(extra: Record<string, unknown> = {}) {
  setLocale('en')
  const sizes = measurements()
  const oncolumnresize = vi.fn(), oncolumnresizeend = vi.fn(), oncolumnreset = vi.fn(), onsort = vi.fn(), onselect = vi.fn()
  const props = { rows: makeManyRows(3), selectedIndex: null, sort, oncolumnresize, oncolumnresizeend, oncolumnreset, onsort, onselect, ...extra }
  const view = render(ResultsTable, { props: props as unknown as ComponentProps<typeof ResultsTable> })
  sizes.set(screen.getByRole('grid'), 1040)
  return { ...view, sizes, oncolumnresize, oncolumnresizeend, oncolumnreset, onsort, onselect }
}

describe('@RF-06 independent column widths', () => {
  it.each(columns.map((id, i) => [id, labels[i]] as const))('%s has a reachable independent handle', async (_id, name) => {
    setup()
    const separator = handle(name)
    expect(separator.getAttribute('aria-orientation')).toBe('vertical')
    expect(separator.getAttribute('aria-valuemax')).toBe('800')
    expect(separator.getAttribute('tabindex')).toBe('0')
    expect(Number(separator.getAttribute('aria-valuemin'))).toBeGreaterThan(0)
  })
  it('W2: viewport defaults are computed independently of an override and reset is target-specific', async () => {
    const view = setup()
    await vi.waitFor(() => expect(value(handle('File'))).toBe(150))
    expect(value(handle('Folder'))).toBe(200)
    await pointer(handle(), 'pointerdown', 100)
    await pointer(window, 'pointermove', 180)
    expect(view.oncolumnresize).toHaveBeenLastCalledWith('file', 230)
    expect(value(handle('Folder'))).toBe(200)
    await pointer(window, 'pointerup', 180)
    await view.rerender({ columnWidths: { file: 230 } } as unknown as Partial<ComponentProps<typeof ResultsTable>>)
    view.sizes.set(screen.getByRole('grid'), 1390)
    await vi.waitFor(() => expect(value(handle('Folder'))).toBe(400))
    expect(value(handle())).toBe(230)
    await fireEvent.keyDown(handle(), { key: 'Enter' })
    expect(view.oncolumnreset).toHaveBeenCalledWith('file')
    await view.rerender({ columnWidths: {} } as unknown as Partial<ComponentProps<typeof ResultsTable>>)
    await vi.waitFor(() => expect(value(handle())).toBe(300))
    expect(value(handle('Folder'))).toBe(400)
  })
  it('retains a live pointer baseline across parent rerenders', async () => {
    const view = setup()
    await vi.waitFor(() => expect(value(handle())).toBe(150))
    await pointer(handle(), 'pointerdown', 100)
    await pointer(window, 'pointermove', 130)
    expect(view.oncolumnresize).toHaveBeenLastCalledWith('file', 180)
    await view.rerender({ columnWidths: { file: 180 } } as unknown as Partial<ComponentProps<typeof ResultsTable>>)
    await pointer(window, 'pointermove', 150)
    expect(view.oncolumnresize).toHaveBeenLastCalledWith('file', 200)
    await pointer(window, 'pointerup', 150)
  })
  it('discards excess motion at a bound so reversing moves immediately', async () => {
    const view = setup({ columnWidths: { file: 790 } })
    await pointer(handle(), 'pointerdown', 100)
    await pointer(window, 'pointermove', 1000)
    expect(view.oncolumnresize).toHaveBeenLastCalledWith('file', 800)
    await pointer(window, 'pointermove', 990)
    expect(view.oncolumnresize).toHaveBeenLastCalledWith('file', 790)
    await pointer(window, 'pointerup', 990)
  })
  it.each(columns.map((id, i) => [id, labels[i]] as const))('%s clamps keyboard changes at both user bounds', async (column, name) => {
    const view = setup({ columnWidths: { [column]: 800 } })
    await fireEvent.keyDown(handle(name), { key: 'ArrowRight' })
    expect(value(handle(name))).toBe(800)
    await fireEvent.keyDown(handle(name), { key: 'ArrowLeft' })
    expect(view.oncolumnresize).toHaveBeenLastCalledWith(column, 790)
    await view.rerender({ columnWidths: { [column]: Number(handle(name).getAttribute('aria-valuemin')) } } as unknown as Partial<ComponentProps<typeof ResultsTable>>)
    await fireEvent.keyDown(handle(name), { key: 'ArrowLeft' })
    expect(value(handle(name))).toBe(Number(handle(name).getAttribute('aria-valuemin')))
  })
  it('rounds the final changed width only on completion and signals commit', async () => {
    const view = setup()
    await vi.waitFor(() => expect(value(handle())).toBe(150))
    await pointer(handle(), 'pointerdown', 100)
    await pointer(window, 'pointermove', 100.4)
    expect(view.oncolumnresize.mock.calls.at(-1)?.[1]).toBeCloseTo(150.4)
    expect(view.oncolumnresizeend).not.toHaveBeenCalled()
    await pointer(window, 'pointermove', 100.8)
    await pointer(window, 'pointerup', 100.8)
    expect(view.oncolumnresize).toHaveBeenLastCalledWith('file', 151)
    expect(view.oncolumnresizeend).toHaveBeenCalledTimes(1)
  })
  it('a no-motion handle click does not resize, reset, sort or select', async () => {
    const view = setup()
    await pointer(handle(), 'pointerdown', 100)
    await pointer(window, 'pointerup', 100)
    await fireEvent.click(handle())
    expect(view.oncolumnresize).not.toHaveBeenCalled()
    expect(view.oncolumnresizeend).not.toHaveBeenCalled()
    expect(view.oncolumnreset).not.toHaveBeenCalled()
    expect(view.onsort).not.toHaveBeenCalled()
    expect(view.onselect).not.toHaveBeenCalled()
  })
  it('handle keys do not invoke table navigation; normal header activation still sorts', async () => {
    const view = setup({ selectedIndex: 0 })
    await fireEvent.keyDown(handle(), { key: 'ArrowRight' })
    expect(view.onselect).not.toHaveBeenCalled()
    expect(view.onsort).not.toHaveBeenCalled()
    await fireEvent.click(within(screen.getAllByRole('columnheader')[1]).getByRole('button'))
    expect(view.onsort).toHaveBeenCalledWith('file')
  })
  it('W5: overrides survive large result replacement, errors, Unicode names, sort and language changes', async () => {
    const view = setup({ columnWidths: { file: 230, folder: 310 }, selectedIndex: 0 })
    const rows = makeManyRows(10000)
    rows[0] = makeRow(0, { path: 'C:/Synthetic/日本語.flac', verdict: 'error', error: { kind: 'corruptStream', detail: 'synthetic' } })
    await view.rerender({ rows, sort: { column: 'state' as SortColumn, direction: 'desc' }, columnWidths: { file: 230, folder: 310 } } as unknown as Partial<ComponentProps<typeof ResultsTable>>)
    setLocale('es')
    await vi.waitFor(() => expect(value(screen.getByRole('separator', { name: 'Cambiar ancho de la columna Archivo' }))).toBe(230))
    expect(value(screen.getByRole('separator', { name: 'Cambiar ancho de la columna Carpeta' }))).toBe(310)
    expect(view.oncolumnresize).not.toHaveBeenCalled()
    expect(view.onselect).not.toHaveBeenCalled()
    await view.rerender({ rows: [], noMatches: true } as Partial<ComponentProps<typeof ResultsTable>>)
    expect(screen.getAllByRole('separator')).toHaveLength(10)
    expect(value(screen.getByRole('separator', { name: 'Cambiar ancho de la columna Archivo' }))).toBe(230)
  })
})

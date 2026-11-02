import type { LayoutWidths, SortColumn } from './model'
import { COLUMN_MIN, COLUMN_MAX, RESPONSIVE_MIN, FILE_WEIGHT, SIDEBAR_MIN, SIDEBAR_MAX, MAIN_MIN, SEPARATOR_WIDTH } from './tokens/layout'

const KEY = 'fakeflac.layout.widths'
type StorageLike = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>
export function clampColumnWidth(column: SortColumn, width: number): number {
  return Math.max(COLUMN_MIN[column], Math.min(COLUMN_MAX, width))
}
export function clampSidebarWidth(width: number, contentWidth: number): number {
  const max = contentWidth > 0 ? Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, contentWidth - MAIN_MIN - SEPARATOR_WIDTH)) : SIDEBAR_MAX
  return Math.max(SIDEBAR_MIN, Math.min(max, width))
}
export function defaultColumnWidths(viewportWidth: number): Record<SortColumn, number> {
  const fixed = Object.entries(COLUMN_MIN).filter(([id]) => id !== 'file' && id !== 'folder').reduce((sum, [, width]) => sum + width, 0)
  const flexible = Math.max(RESPONSIVE_MIN * 2, viewportWidth - fixed)
  const file = Math.max(RESPONSIVE_MIN, FILE_WEIGHT * flexible)
  return { ...COLUMN_MIN, file: Math.min(COLUMN_MAX, file), folder: Math.min(COLUMN_MAX, flexible - file) }
}
const object = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)
const finite = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value)
function normalize(value: Record<string, unknown>): LayoutWidths {
  const result: LayoutWidths = {}
  if (finite(value.sidebarWidth)) result.sidebarWidth = clampSidebarWidth(Math.round(value.sidebarWidth), 0)
  if (object(value.columns)) {
    const columns: Partial<Record<SortColumn, number>> = {}
    for (const column of Object.keys(COLUMN_MIN) as SortColumn[]) {
      const width = value.columns[column]
      if (finite(width)) columns[column] = clampColumnWidth(column, Math.round(width))
    }
    if (Object.keys(columns).length) result.columns = columns
  }
  return result
}
export function readLayoutWidths(storage?: StorageLike): LayoutWidths {
  try {
    const raw: unknown = JSON.parse((storage ?? localStorage).getItem(KEY) ?? 'null')
    return object(raw) && raw.version === 1 ? normalize(raw) : {}
  } catch { return {} }
}
export function writeLayoutWidths(value: LayoutWidths, storage?: StorageLike): void {
  try {
    const prefs = normalize(value)
    const target = storage ?? localStorage
    if (Object.keys(prefs).length) target.setItem(KEY, JSON.stringify({ version: 1, ...prefs }))
    else target.removeItem(KEY)
  } catch { /* Layout preferences are convenience-only. */ }
}

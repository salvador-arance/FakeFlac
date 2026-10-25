// The table's view rules: sort order and filters (specs/ui/sort-filter.md sections 3, 4 and 10,
// "Model"). Pure module: no store, no API, no DOM.
//
// The expensive parts are built once (spec section 10, implementation note): one collator per
// locale, the sort keys and the folded searched text once per row (cached by the row object), the
// folded terms once per query.
import { t } from './i18n'
import type { Locale } from './i18n'
import type { ResultRow, RowFilter, SortState, Verdict } from './model'

/** Search folding of section 4.2: lower case, NFD, nonspacing marks removed, NFC. */
export function foldForSearch(text: string): string {
  return text.toLowerCase().normalize('NFD').replace(/\p{Mn}/gu, '').normalize('NFC')
}

let lastQuery: string | undefined
let lastTerms: string[] = []

/** The folded terms of a query (section 4.2), in the order they appear. */
export function searchTerms(query: string): string[] {
  if (query !== lastQuery) {
    lastTerms = query
      .replace(/\//g, '\\')
      .split(/\s+/)
      .map(foldForSearch)
      .filter((term) => term !== '')
    lastQuery = query
  }
  return lastTerms
}

/** At least one verdict pressed or one search term. */
export function isFilterActive(filter: RowFilter): boolean {
  return filter.verdicts.length > 0 || searchTerms(filter.query).length > 0
}

function fileNameOf(path: string): string {
  const cut = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return cut >= 0 ? path.slice(cut + 1) : path
}

function folderPartOf(path: string): string {
  const cut = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return cut >= 0 ? path.slice(0, cut) : ''
}

type SearchEntry = { albumName: string | undefined; text: string }
const searchCache = new WeakMap<ResultRow, SearchEntry>()

function searchedText(row: ResultRow, albumName: string | undefined): string {
  const cached = searchCache.get(row)
  if (cached && cached.albumName === albumName) return cached.text
  const path = row.analysis.info.path
  const text = foldForSearch(albumName === undefined ? path : `${albumName}\\${fileNameOf(path)}`)
  searchCache.set(row, { albumName, text })
  return text
}

/** Whether `row` passes the verdict filter and the name search (section 4). */
export function rowMatches(row: ResultRow, filter: RowFilter, albumName: string | undefined): boolean {
  if (filter.verdicts.length > 0 && !filter.verdicts.includes(row.analysis.verdict)) return false
  const terms = searchTerms(filter.query)
  if (terms.length === 0) return true
  const text = searchedText(row, albumName)
  return terms.every((term) => text.includes(term))
}

const collators = new Map<Locale, Intl.Collator>()

function collatorFor(locale: Locale): Intl.Collator {
  let collator = collators.get(locale)
  if (!collator) {
    collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'accent' })
    collators.set(locale, collator)
  }
  return collator
}

/** Name comparison of 3.2: component by component (split at `\` and `/`), a prefix comes first. */
function compareComponents(a: string[], b: string[], collator: Intl.Collator): number {
  const common = Math.min(a.length, b.length)
  for (let i = 0; i < common; i++) {
    const c = collator.compare(a[i], b[i])
    if (c !== 0) return c
  }
  return a.length - b.length
}

const components = (text: string): string[] => text.split(/[\\/]/)

type Keys = { path: string[]; folder: string[]; format: string[] }
const keyCache = new WeakMap<ResultRow, Keys>()

function keysOf(row: ResultRow): Keys {
  let keys = keyCache.get(row)
  if (!keys) {
    const info = row.analysis.info
    const format =
      info.format === 'm4a' && info.codec === 'alac'
        ? t('format.m4aAlac')
        : (info.format?.toUpperCase() ?? '')
    keys = {
      path: components(info.path),
      folder: components(info.folder || folderPartOf(info.path)),
      format: components(format),
    }
    keyCache.set(row, keys)
  }
  return keys
}

// What needs attention first (3.1)
const VERDICT_RANK: Record<Verdict, number> = { fake: 0, suspicious: 1, error: 2, real: 3 }

function numericKey(row: ResultRow, column: SortState['column']): number | null {
  const a = row.analysis
  switch (column) {
    case 'sampleRate':
      return a.info.sampleRate ?? null
    case 'bitDepth':
      return a.info.bitDepth ?? null
    case 'duration':
      return a.info.durationMs ?? null
    case 'cutoff':
      return a.cutoff?.frequencyHz ?? null
    case 'bitrate':
      return a.info.declaredBitrateKbps ?? null
    case 'realBitrate':
      return a.estimatedSourceBitrateKbps ?? null
    default:
      return null
  }
}

/** Display order of two rows under `sort` (section 3), ties included. */
export function compareRows(a: ResultRow, b: ResultRow, sort: SortState, locale: Locale): number {
  const collator = collatorFor(locale)
  const dir = sort.direction === 'desc' ? -1 : 1
  let result = 0
  switch (sort.column) {
    case 'state':
      result = dir * (VERDICT_RANK[a.analysis.verdict] - VERDICT_RANK[b.analysis.verdict])
      break
    case 'file':
      result = dir * compareComponents(keysOf(a).path, keysOf(b).path, collator)
      break
    case 'folder':
      result = dir * compareComponents(keysOf(a).folder, keysOf(b).folder, collator)
      break
    case 'format':
      result = dir * compareComponents(keysOf(a).format, keysOf(b).format, collator)
      break
    default: {
      // Empty cells go last in both directions
      const x = numericKey(a, sort.column)
      const y = numericKey(b, sort.column)
      if (x === null || y === null) {
        result = x === y ? 0 : x === null ? 1 : -1
      } else {
        result = dir * (x - y)
      }
    }
  }
  if (result !== 0) return result
  // Ties: File order ascending, then index ascending, in both directions (3.3)
  result = compareComponents(keysOf(a).path, keysOf(b).path, collator)
  return result !== 0 ? result : a.index - b.index
}

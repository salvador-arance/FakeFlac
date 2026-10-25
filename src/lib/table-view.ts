// The table's view rules: sort order and filters (specs/ui/sort-filter.md sections 3, 4 and 10,
// "Model"). Pure module: no store, no API, no DOM.
//
// RF-08 compile-only stub (stage `test`): the signatures of the spec; the implementer replaces the
// bodies.
import type { Locale } from './i18n'
import type { ResultRow, RowFilter, SortState } from './model'

/** Search folding of section 4.2: lower case, NFD, nonspacing marks removed, NFC. */
export function foldForSearch(text: string): string {
  void text
  throw new Error('not implemented')
}

/** The folded terms of a query (section 4.2), in the order they appear. */
export function searchTerms(query: string): string[] {
  void query
  throw new Error('not implemented')
}

/** Whether `row` passes the verdict filter and the name search (section 4). */
export function rowMatches(row: ResultRow, filter: RowFilter, albumName: string | undefined): boolean {
  void row
  void filter
  void albumName
  throw new Error('not implemented')
}

/** Display order of two rows under `sort` (section 3), ties included. */
export function compareRows(a: ResultRow, b: ResultRow, sort: SortState, locale: Locale): number {
  void a
  void b
  void sort
  void locale
  throw new Error('not implemented')
}

/** At least one verdict pressed or one search term. */
export function isFilterActive(filter: RowFilter): boolean {
  void filter
  throw new Error('not implemented')
}

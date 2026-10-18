<script lang="ts">
  import type { ResultRow, SortColumn, SortState } from '../../model'
  type Props = {
    rows: ResultRow[]
    selectedIndex: number | null
    sort: SortState
    /** Row height in px; defaults to the `--row-height` token (22). */
    rowHeight?: number
    /** Visible height of the grid in px. Overrides the measured height (jsdom has no layout). */
    viewportHeight?: number
    onselect?: (index: number) => void
    onsort?: (column: SortColumn) => void
  }
  let { rows, selectedIndex, sort, rowHeight, viewportHeight, onselect, onsort }: Props = $props()
</script>

<span data-stub="ResultsTable" data-rows={rows.length} data-sel={selectedIndex} data-sort={sort.column} data-h={rowHeight} data-vh={viewportHeight} data-a={!!onselect} data-b={!!onsort}></span>

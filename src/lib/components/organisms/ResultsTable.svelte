<script lang="ts">
  import { untrack } from 'svelte'
  import BitrateValue from '../molecules/BitrateValue.svelte'
  import ColumnHeader from '../molecules/ColumnHeader.svelte'
  import CutoffValue from '../molecules/CutoffValue.svelte'
  import VerdictCell from '../molecules/VerdictCell.svelte'
  import { t } from '../../i18n'
  import type { ResultRow, SortColumn, SortState, TrackAnalysis } from '../../model'

  type Props = {
    rows: ResultRow[]
    selectedIndex: number | null
    sort: SortState
    rowHeight?: number
    viewportHeight?: number
    onselect?: (index: number) => void
    onsort?: (column: SortColumn) => void
  }

  let {
    rows,
    selectedIndex,
    sort,
    rowHeight = 22,
    viewportHeight,
    onselect,
    onsort,
  }: Props = $props()

  let scrollTop = $state(0)
  let gridElement: HTMLElement | undefined = $state()
  let headerElement: HTMLElement | undefined = $state()

  function getFileName(p: string): string {
    const norm = p.replace(/\\/g, '/')
    const idx = norm.lastIndexOf('/')
    return idx !== -1 ? norm.slice(idx + 1) : norm
  }

  function getFolderName(p: string): string {
    const norm = p.replace(/\\/g, '/')
    const idx = norm.lastIndexOf('/')
    return idx !== -1 ? norm.slice(0, idx) : ''
  }

  function formatDuration(ms?: number | null): string {
    if (ms === null || ms === undefined) return ''
    const totalSec = Math.floor(ms / 1000)
    const m = Math.floor(totalSec / 60)
    const s = totalSec % 60
    return `${m}:${s.toString().padStart(2, '0')}`
  }

  /** Format cell: `format` in upper case; an M4A with codec `alac` is "M4A (ALAC)" (lossy-formats.md). */
  function formatText(a: TrackAnalysis): string {
    if (a.info.format === 'm4a' && a.info.codec === 'alac') return t('format.m4aAlac')
    return a.info.format?.toUpperCase() ?? ''
  }

  /** Real bitrate is approximate when the estimate is lower than the declared bitrate. */
  function isApprox(a: TrackAnalysis): boolean {
    const declared = a.info.declaredBitrateKbps
    const estimate = a.estimatedSourceBitrateKbps
    return declared !== null && estimate !== null && estimate < declared
  }

  /** Numeric order in `dir`; null ("—") cells go last in both directions. */
  function compareNullable(a: number | null, b: number | null, dir: number): number {
    if (a === null && b === null) return 0
    if (a === null) return 1
    if (b === null) return -1
    return dir * (a - b)
  }

  let sortedRows = $derived.by(() => {
    const copy = [...rows]
    const dir = sort.direction === 'desc' ? -1 : 1

    copy.sort((a, b) => {
      switch (sort.column) {
        case 'file':
          return dir * a.analysis.info.path.localeCompare(b.analysis.info.path)
        case 'folder':
          return dir * (a.analysis.info.folder || getFolderName(a.analysis.info.path)).localeCompare(
            b.analysis.info.folder || getFolderName(b.analysis.info.path),
          )
        case 'state':
          return dir * a.analysis.verdict.localeCompare(b.analysis.verdict)
        case 'format':
          return dir * formatText(a.analysis).localeCompare(formatText(b.analysis))
        case 'sampleRate':
          return dir * ((a.analysis.info.sampleRate ?? 0) - (b.analysis.info.sampleRate ?? 0))
        case 'bitDepth':
          return dir * ((a.analysis.info.bitDepth ?? 0) - (b.analysis.info.bitDepth ?? 0))
        case 'duration':
          return dir * ((a.analysis.info.durationMs ?? 0) - (b.analysis.info.durationMs ?? 0))
        case 'cutoff':
          return dir * ((a.analysis.cutoff?.frequencyHz ?? 0) - (b.analysis.cutoff?.frequencyHz ?? 0))
        case 'bitrate':
          return compareNullable(
            a.analysis.info.declaredBitrateKbps,
            b.analysis.info.declaredBitrateKbps,
            dir,
          )
        case 'realBitrate':
          return compareNullable(
            a.analysis.estimatedSourceBitrateKbps,
            b.analysis.estimatedSourceBitrateKbps,
            dir,
          )
        default:
          return 0
      }
    })
    return copy
  })

  let isVirtualised = $derived(
    viewportHeight !== undefined || (gridElement && gridElement.clientHeight > 0),
  )

  let range = $derived.by(() => {
    const total = sortedRows.length
    if (total === 0) return { start: 0, end: 0 }

    if (!isVirtualised) {
      if (total <= 50) return { start: 0, end: total }
      return { start: 0, end: 50 }
    }

    const effectiveHeight = viewportHeight || (gridElement ? gridElement.clientHeight : 400)
    const visibleCount = Math.ceil(effectiveHeight / rowHeight)
    const startIdx = Math.max(0, Math.floor(scrollTop / rowHeight) - 5)
    const endIdx = Math.min(total, startIdx + visibleCount + 10)
    return { start: startIdx, end: endIdx }
  })

  let visibleRows = $derived(sortedRows.slice(range.start, range.end))

  function handleScroll(e: Event) {
    const target = e.currentTarget as HTMLElement
    scrollTop = target.scrollTop
  }

  // components.md "ResultsTable": a selection change (prop or key) scrolls the least needed so the
  // selected row is visible. Only `selectedIndex` triggers it: new rows or a re-sort never move
  // the scroll by themselves.
  $effect(() => {
    const target = selectedIndex
    if (target === null || !gridElement) return
    untrack(() => {
      const position = sortedRows.findIndex((r) => r.index === target)
      if (position < 0 || !gridElement) return
      const viewport = viewportHeight ?? gridElement.clientHeight
      const header = headerElement?.offsetHeight ?? 0
      const top = position * rowHeight
      const bottom = header + (position + 1) * rowHeight
      let next = gridElement.scrollTop
      if (next > top) next = top
      else if (viewport > 0 && next + viewport < bottom) next = bottom - viewport
      if (next !== gridElement.scrollTop) {
        gridElement.scrollTop = next
        scrollTop = next
      }
    })
  })

  function handleKeyDown(e: KeyboardEvent) {
    if (sortedRows.length === 0) return

    if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (selectedIndex === null) {
        onselect?.(sortedRows[0].index)
      } else {
        const curr = sortedRows.findIndex((r) => r.index === selectedIndex)
        if (curr >= 0 && curr < sortedRows.length - 1) {
          onselect?.(sortedRows[curr + 1].index)
        }
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (selectedIndex !== null) {
        const curr = sortedRows.findIndex((r) => r.index === selectedIndex)
        if (curr > 0) {
          onselect?.(sortedRows[curr - 1].index)
        }
      }
    } else if (e.key === 'Home') {
      e.preventDefault()
      onselect?.(sortedRows[0].index)
    } else if (e.key === 'End') {
      e.preventDefault()
      onselect?.(sortedRows[sortedRows.length - 1].index)
    }
  }

  function getRowErrorTitle(row: ResultRow): string | undefined {
    if (row.analysis.error) {
      return t(`error.${row.analysis.error.kind}`)
    }
    return undefined
  }
</script>

<div
  bind:this={gridElement}
  class="results-table-grid"
  role="grid"
  tabindex="0"
  onscroll={handleScroll}
  onkeydown={handleKeyDown}
  style:--row-height="{rowHeight}px"
>
  <div class="table-header" role="rowgroup" bind:this={headerElement}>
    <div class="row header-row" role="row">
      <ColumnHeader
        label="State"
        sort={sort.column === 'state' ? sort.direction : null}
        onsort={() => onsort?.('state')}
      />
      <ColumnHeader
        label="File"
        sort={sort.column === 'file' ? sort.direction : null}
        onsort={() => onsort?.('file')}
      />
      <ColumnHeader
        label="Folder"
        sort={sort.column === 'folder' ? sort.direction : null}
        onsort={() => onsort?.('folder')}
      />
      <ColumnHeader
        label="Format"
        sort={sort.column === 'format' ? sort.direction : null}
        onsort={() => onsort?.('format')}
      />
      <ColumnHeader
        label="Samplerate"
        sort={sort.column === 'sampleRate' ? sort.direction : null}
        onsort={() => onsort?.('sampleRate')}
      />
      <ColumnHeader
        label="Bits"
        sort={sort.column === 'bitDepth' ? sort.direction : null}
        onsort={() => onsort?.('bitDepth')}
      />
      <ColumnHeader
        label="Duration"
        sort={sort.column === 'duration' ? sort.direction : null}
        onsort={() => onsort?.('duration')}
      />
      <ColumnHeader
        label="Cutoff"
        sort={sort.column === 'cutoff' ? sort.direction : null}
        onsort={() => onsort?.('cutoff')}
      />
      <ColumnHeader
        label={t('column.bitrate')}
        sort={sort.column === 'bitrate' ? sort.direction : null}
        onsort={() => onsort?.('bitrate')}
      />
      <ColumnHeader
        label={t('column.realBitrate')}
        tooltip={t('help.realBitrate')}
        sort={sort.column === 'realBitrate' ? sort.direction : null}
        onsort={() => onsort?.('realBitrate')}
      />
    </div>
  </div>

  <div
    class="table-body"
    role="rowgroup"
    style:padding-top="{range.start * rowHeight}px"
    style:padding-bottom="{Math.max(0, (sortedRows.length - range.end) * rowHeight)}px"
  >
    {#each visibleRows as row (row.index)}
      {@const nyquist = (row.analysis.info.sampleRate ?? 44100) / 2}
      {@const errTitle = getRowErrorTitle(row)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_interactive_supports_focus -->
      <div
        class="row data-row"
        class:selected={row.index === selectedIndex}
        role="row"
        tabindex="-1"
        aria-selected={row.index === selectedIndex ? 'true' : 'false'}
        title={errTitle}
        onclick={() => onselect?.(row.index)}
      >
        <div class="cell state-cell" role="gridcell">
          <VerdictCell
            verdict={row.analysis.verdict}
            reasons={row.analysis.reasons}
          />
        </div>
        <div class="cell file-cell" role="gridcell" title={getFileName(row.analysis.info.path)}>
          {getFileName(row.analysis.info.path)}
        </div>
        <div class="cell folder-cell" role="gridcell" title={row.analysis.info.folder || getFolderName(row.analysis.info.path)}>
          {row.analysis.info.folder || getFolderName(row.analysis.info.path)}
        </div>
        <div class="cell format-cell" role="gridcell">
          {formatText(row.analysis)}
        </div>
        <div class="cell rate-cell" role="gridcell">
          {row.analysis.info.sampleRate ?? ''}
        </div>
        <div class="cell bits-cell" role="gridcell">
          {row.analysis.info.bitDepth ?? ''}
        </div>
        <div class="cell duration-cell" role="gridcell">
          {formatDuration(row.analysis.info.durationMs)}
        </div>
        <div class="cell cutoff-cell" role="gridcell">
          <CutoffValue hz={row.analysis.cutoff?.frequencyHz ?? null} nyquistHz={nyquist} />
        </div>
        <div class="cell bitrate-cell" role="gridcell">
          <BitrateValue
            kbps={row.analysis.info.declaredBitrateKbps}
            mode={row.analysis.info.bitrateMode}
          />
        </div>
        <div class="cell real-bitrate-cell" role="gridcell">
          <BitrateValue
            kbps={row.analysis.estimatedSourceBitrateKbps}
            approx={isApprox(row.analysis)}
          />
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .results-table-grid {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow-y: auto;
    overflow-x: auto;
    background: var(--color-bg);
    outline: none;
    font-family: var(--font-ui);
    font-size: var(--font-size);
    box-sizing: border-box;
  }

  .table-header {
    position: sticky;
    top: 0;
    z-index: 10;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
  }

  .row {
    display: grid;
    grid-template-columns: 120px minmax(140px, 1.5fr) minmax(140px, 2fr) 70px 80px 50px 70px 90px 110px 100px;
    align-items: center;
    height: var(--row-height);
    box-sizing: border-box;
  }

  .data-row {
    cursor: pointer;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .data-row:hover {
    background: var(--color-surface-raised);
  }

  .data-row.selected {
    background: var(--color-row-selected);
  }

  .cell {
    padding: 0 var(--space-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    box-sizing: border-box;
  }

  .rate-cell,
  .bits-cell,
  .duration-cell,
  .cutoff-cell,
  .bitrate-cell,
  .real-bitrate-cell {
    text-align: right;
  }
</style>

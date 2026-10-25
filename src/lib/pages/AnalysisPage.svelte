<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import type { Spectrogram } from '../model'
  import {
    ApiError,
    cancelAnalysis,
    closeWindow,
    exportCsv,
    getSpectrogram,
    minimizeWindow,
    onDragDrop,
    pickFolder,
    startAnalysis,
    startAnalysisOfDropped,
    toggleMaximizeWindow,
  } from '../api'
  import AlbumSummaryList from '../components/organisms/AlbumSummaryList.svelte'
  import DetailPanel from '../components/organisms/DetailPanel.svelte'
  import FolderDropZone from '../components/organisms/FolderDropZone.svelte'
  import ResultsTable from '../components/organisms/ResultsTable.svelte'
  import StatusBar from '../components/organisms/StatusBar.svelte'
  import TitleBar from '../components/organisms/TitleBar.svelte'
  import Toolbar from '../components/organisms/Toolbar.svelte'
  import MainLayout from '../components/templates/MainLayout.svelte'
  import { t } from '../i18n'
  import { createAnalysisStore } from '../stores/analysis.svelte'

  const store = createAnalysisStore()

  let isDraggingOver = $state(false)
  let unlistenDragDrop: (() => void) | undefined

  onMount(async () => {
    unlistenDragDrop = await onDragDrop((event) => {
      if (event.type === 'enter') {
        isDraggingOver = true
      } else if (event.type === 'leave') {
        isDraggingOver = false
      } else if (event.type === 'drop') {
        isDraggingOver = false
        handleDrop(event.paths)
      }
    })
  })

  onDestroy(() => {
    unlistenDragDrop?.()
  })

  // RF-07: spectrogram of the selected row. `spectrogramRequest` identifies the latest request;
  // a response or failure for an older one (a row no longer selected) is discarded.
  let spectrogram = $state.raw<Spectrogram | null>(null)
  let spectrogramLoading = $state(false)
  let spectrogramUnavailable = $state(false)
  let spectrogramRequest = 0
  // RF-10 (Q16): bumped on every album selection so the table brings the selected row into view.
  let revealRequest = $state(0)

  // RF-10: the current album is the album of the selected row.
  let currentAlbum = $derived(
    store.rows.find((row) => row.index === store.selectedIndex)?.albumIndex ?? null,
  )

  let selectedTrack = $derived(
    store.rows.find((row) => row.index === store.selectedIndex)?.analysis ?? null,
  )

  function clearSpectrogram() {
    spectrogramRequest += 1
    spectrogram = null
    spectrogramLoading = false
    spectrogramUnavailable = false
  }

  async function handleSelect(index: number) {
    if (index === store.selectedIndex) return
    store.select(index)
    await showSpectrogram(index)
  }

  // RF-10: selecting an album selects its row with the lowest index (also when it is the album of
  // the current row), through the same flow as a click on that row.
  async function handleSelectAlbum(albumIndex: number) {
    revealRequest += 1
    const before = store.selectedIndex
    store.selectAlbum(albumIndex)
    const after = store.selectedIndex
    if (after === null || after === before) return
    await showSpectrogram(after)
  }

  async function showSpectrogram(index: number) {
    clearSpectrogram()
    const track = store.rows.find((row) => row.index === index)?.analysis
    if (!track || track.verdict === 'error') return

    const request = spectrogramRequest
    spectrogramLoading = true
    let result: Spectrogram | null = null
    try {
      result = await getSpectrogram(track.info.path)
    } catch {
      // The panel keeps the verdict line and shows `spectrogram.unavailable` (main-screen.md).
      result = null
    }
    if (request !== spectrogramRequest) return
    spectrogram = result
    spectrogramLoading = false
    spectrogramUnavailable = result === null
  }

  // RF-09: the notice of the last export, shown in the status bar until a start attempt.
  let notice = $state<string | undefined>(undefined)
  let exporting = $state(false)

  type Saved = { notice: string | undefined; spectrogram: typeof spectrogram }

  function beginRun(): Saved {
    const saved = { notice, spectrogram }
    clearSpectrogram()
    notice = undefined
    store.begin()
    return saved
  }

  /** A start that ended before any event: the previous results stay on screen (export-csv.md). */
  function undoStart(saved: Saved) {
    store.restore()
    spectrogram = saved.spectrogram
    notice = saved.notice
  }

  function handleError(err: unknown, eventsSeen: boolean, saved: Saved) {
    if (!eventsSeen) {
      store.restore()
      spectrogram = saved.spectrogram
    }
    if (err instanceof ApiError) {
      if (err.code === 'NOT_A_FOLDER') {
        store.reset()
        return
      }
      store.fail(t(`error.${err.code}`))
    } else {
      store.fail(t('error.INTERNAL'))
    }
  }

  async function handleOpenFolder() {
    if (store.phase === 'scanning' || store.phase === 'analysing') return
    const path = await pickFolder()
    if (!path) return

    const saved = beginRun()
    let eventsSeen = false
    try {
      await startAnalysis(path, (ev) => {
        eventsSeen = true
        store.apply(ev)
      })
    } catch (err) {
      handleError(err, eventsSeen, saved)
    }
  }

  async function handleDrop(paths: string[]) {
    if (store.phase === 'scanning' || store.phase === 'analysing') return
    if (!paths || paths.length === 0) return

    const saved = beginRun()
    let eventsSeen = false
    try {
      const summary = await startAnalysisOfDropped(paths, (ev) => {
        eventsSeen = true
        store.apply(ev)
      })
      if (summary === null) undoStart(saved)
    } catch (err) {
      handleError(err, eventsSeen, saved)
    }
  }

  async function handleExport() {
    if (!canExport) return
    exporting = true
    try {
      const result = await exportCsv()
      if (result !== null) {
        const file = result.path.split(/[\\/]/).pop() ?? result.path
        notice =
          result.rows === 1
            ? t('status.exportedOne', { file })
            : t('status.exported', { rows: result.rows, file })
      }
    } catch (err) {
      const code = err instanceof ApiError ? err.code : 'INTERNAL'
      notice =
        code === 'NOT_ANALYZED' || code === 'ANALYSIS_RUNNING'
          ? t(`error.${code}`)
          : t('error.EXPORT_FAILED')
    } finally {
      exporting = false
    }
  }

  async function handleCancel() {
    await cancelAnalysis()
  }

  let canOpen = $derived(store.phase !== 'scanning' && store.phase !== 'analysing')
  let canExport = $derived(
    store.rows.length > 0 &&
      store.phase !== 'scanning' &&
      store.phase !== 'analysing' &&
      !exporting,
  )
  let canCancel = $derived(store.phase === 'scanning' || store.phase === 'analysing')
  let isEmpty = $derived(store.phase === 'ready' && store.rows.length === 0)
  // main-screen.md "Screen states": the detail panel is hidden until there are rows.
  let showDetail = $derived(store.rows.length > 0)
</script>

{#snippet detailPanel()}
  <DetailPanel
    track={selectedTrack}
    {spectrogram}
    loading={spectrogramLoading}
    unavailable={spectrogramUnavailable}
  />
{/snippet}

{#snippet albumsPanel()}
  <AlbumSummaryList
    albums={store.albums}
    currentIndex={currentAlbum}
    onselect={handleSelectAlbum}
  />
{/snippet}

<MainLayout detail={showDetail ? detailPanel : undefined} sidebar={isEmpty ? undefined : albumsPanel}>
  {#snippet titleBar()}
    <TitleBar
      title={t('app.title')}
      onminimize={minimizeWindow}
      ontoggleMaximize={toggleMaximizeWindow}
      onclose={closeWindow}
    />
  {/snippet}

  {#snippet toolbar()}
    <Toolbar
      {canOpen}
      {canCancel}
      {canExport}
      onopen={handleOpenFolder}
      oncancel={handleCancel}
      onexport={handleExport}
    />
  {/snippet}

  {#snippet table()}
    <div class="table-container">
      <FolderDropZone active={isDraggingOver} empty={isEmpty} />
      {#if !isEmpty}
        <ResultsTable
          rows={store.rows}
          selectedIndex={store.selectedIndex}
          sort={store.sort}
          {revealRequest}
          onselect={handleSelect}
          onsort={(column) => store.toggleSort(column)}
        />
      {/if}
    </div>
  {/snippet}

  {#snippet statusBar()}
    <StatusBar
      phase={store.phase}
      summary={store.summary}
      message={store.message}
      {notice}
    />
  {/snippet}
</MainLayout>

<style>
  .table-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
</style>

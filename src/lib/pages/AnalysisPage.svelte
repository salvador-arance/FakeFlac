<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import type { Spectrogram } from '../model'
  import {
    ApiError,
    cancelAnalysis,
    closeWindow,
    getSpectrogram,
    minimizeWindow,
    onDragDrop,
    pickFolder,
    startAnalysis,
    startAnalysisOfDropped,
    toggleMaximizeWindow,
  } from '../api'
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

  function handleError(err: unknown) {
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

  // RF-07: spectrogram of the selected row. `spectrogramRequest` identifies the latest request;
  // a response or failure for an older one (a row no longer selected) is discarded.
  let spectrogram = $state.raw<Spectrogram | null>(null)
  let spectrogramLoading = $state(false)
  let spectrogramRequest = 0

  let selectedTrack = $derived(
    store.rows.find((row) => row.index === store.selectedIndex)?.analysis ?? null,
  )

  function clearSpectrogram() {
    spectrogramRequest += 1
    spectrogram = null
    spectrogramLoading = false
  }

  async function handleSelect(index: number) {
    if (index === store.selectedIndex) return
    store.select(index)
    clearSpectrogram()
    const track = store.rows.find((row) => row.index === index)?.analysis
    if (!track || track.verdict === 'error') return

    const request = spectrogramRequest
    spectrogramLoading = true
    let result: Spectrogram | null = null
    try {
      result = await getSpectrogram(track.info.path)
    } catch {
      // No text for a failed spectrogram in H1: the panel keeps the verdict line, without image.
      result = null
    }
    if (request !== spectrogramRequest) return
    spectrogram = result
    spectrogramLoading = false
  }

  function beginRun() {
    clearSpectrogram()
    store.begin()
  }

  async function handleOpenFolder() {
    if (store.phase === 'scanning' || store.phase === 'analysing') return
    const path = await pickFolder()
    if (!path) return

    beginRun()
    try {
      await startAnalysis(path, (ev) => store.apply(ev))
    } catch (err) {
      handleError(err)
    }
  }

  async function handleDrop(paths: string[]) {
    if (store.phase === 'scanning' || store.phase === 'analysing') return
    if (!paths || paths.length === 0) return

    beginRun()
    try {
      const summary = await startAnalysisOfDropped(paths, (ev) => store.apply(ev))
      if (summary === null) {
        store.reset()
      }
    } catch (err) {
      handleError(err)
    }
  }

  async function handleCancel() {
    await cancelAnalysis()
  }

  let canOpen = $derived(store.phase !== 'scanning' && store.phase !== 'analysing')
  let canCancel = $derived(store.phase === 'scanning' || store.phase === 'analysing')
  let isEmpty = $derived(store.phase === 'ready' && store.rows.length === 0)
  // main-screen.md "Screen states": the detail panel is hidden until there are rows.
  let showDetail = $derived(store.rows.length > 0)
</script>

{#snippet detailPanel()}
  <DetailPanel track={selectedTrack} {spectrogram} loading={spectrogramLoading} />
{/snippet}

<MainLayout detail={showDetail ? detailPanel : undefined}>
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
      onopen={handleOpenFolder}
      oncancel={handleCancel}
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

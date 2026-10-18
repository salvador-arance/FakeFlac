<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import {
    ApiError,
    cancelAnalysis,
    closeWindow,
    minimizeWindow,
    onDragDrop,
    pickFolder,
    startAnalysis,
    startAnalysisOfDropped,
    toggleMaximizeWindow,
  } from '../api'
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

  async function handleOpenFolder() {
    if (store.phase === 'scanning' || store.phase === 'analysing') return
    const path = await pickFolder()
    if (!path) return

    store.begin()
    try {
      await startAnalysis(path, (ev) => store.apply(ev))
    } catch (err) {
      handleError(err)
    }
  }

  async function handleDrop(paths: string[]) {
    if (store.phase === 'scanning' || store.phase === 'analysing') return
    if (!paths || paths.length === 0) return

    store.begin()
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
</script>

<MainLayout>
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
          onselect={(index) => store.select(index)}
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

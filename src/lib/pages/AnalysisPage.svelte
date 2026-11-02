<script lang="ts">
  import { flushSync, onDestroy, onMount } from 'svelte'
  import type { Language, SettingsError, SettingsLoadStatus, Spectrogram } from '../model'
  import {
    ApiError,
    cancelAnalysis,
    clearCache,
    closeWindow,
    exportCsv,
    getSettings,
    getSpectrogram,
    minimizeWindow,
    onDragDrop,
    pickFolder,
    setSettings,
    startAnalysis,
    startAnalysisOfDropped,
    toggleMaximizeWindow,
  } from '../api'
  import AlbumSummaryList from '../components/organisms/AlbumSummaryList.svelte'
  import DetailPanel from '../components/organisms/DetailPanel.svelte'
  import FolderDropZone from '../components/organisms/FolderDropZone.svelte'
  import ResultsTable from '../components/organisms/ResultsTable.svelte'
  import SettingsPanel from '../components/organisms/SettingsPanel.svelte'
  import StatusBar from '../components/organisms/StatusBar.svelte'
  import TitleBar from '../components/organisms/TitleBar.svelte'
  import Toolbar from '../components/organisms/Toolbar.svelte'
  import MainLayout from '../components/templates/MainLayout.svelte'
  import { getLocale, setLocale, t } from '../i18n'
  import type { MessageKey, MessageParams } from '../i18n'
  import { createAnalysisStore } from '../stores/analysis.svelte'
  import { isFilterActive } from '../table-view'

  const store = createAnalysisStore()

  let isDraggingOver = $state(false)
  let unlistenDragDrop: (() => void) | undefined
  let destroyed = false

  // RF-13 (language-settings.md section 1): the main screen and its actions wait for the settings.
  let settingsReady = $state(false)
  let loadStatus = $state<SettingsLoadStatus>('default')
  // RF-13 (section 2): the Settings modal and its save request.
  let settingsOpen = $state(false)
  let settingsSaving = $state(false)
  let settingsError = $state<SettingsError | null>(null)
  /** The element focused when Settings opened (the toolbar button); it gets the focus back. */
  let settingsTrigger: HTMLElement | null = null

  /** Folder input and toolbar actions: only on the initialized screen, never behind the modal. */
  let acceptsInput = $derived(settingsReady && !settingsOpen)

  onMount(async () => {
    void loadSettings()
    const unlisten = await onDragDrop((event) => {
      if (event.type === 'enter') {
        isDraggingOver = acceptsInput
      } else if (event.type === 'leave') {
        isDraggingOver = false
      } else if (event.type === 'drop') {
        isDraggingOver = false
        handleDrop(event.paths)
      }
    })
    if (destroyed) unlisten()
    else unlistenDragDrop = unlisten
  })

  onDestroy(() => {
    destroyed = true
    unlistenDragDrop?.()
  })

  function applyLanguage(language: Language) {
    setLocale(language)
    document.documentElement.lang = language
  }

  /** Invalid, unavailable and newer settings show their warning as a status-bar notice. */
  function warningOf(status: SettingsLoadStatus): Notice | undefined {
    if (status === 'invalid' || status === 'unavailable' || status === 'newerVersion') {
      return { key: `settings.${status}`, settingsWarning: true }
    }
    return undefined
  }

  // No timeout: a hung read keeps the busy placeholder (language-settings.md section 1).
  async function loadSettings() {
    let language: Language = 'en'
    let status: SettingsLoadStatus = 'unavailable'
    try {
      const snapshot = await getSettings()
      language = snapshot.settings.language
      status = snapshot.loadStatus
    } catch {
      // An unexpected failure is English with the unavailable warning, never an analysis error.
    }
    if (destroyed) return
    applyLanguage(language)
    loadStatus = status
    notice = warningOf(status)
    settingsReady = true
  }

  function openSettings() {
    if (!acceptsInput) return
    settingsTrigger = document.activeElement instanceof HTMLElement ? document.activeElement : null
    settingsError = null
    settingsOpen = true
  }

  function closeSettings() {
    if (settingsSaving) return
    settingsOpen = false
    settingsError = null
    // The background stops being inert in this flush, so the trigger can take the focus back.
    flushSync()
    const trigger = settingsTrigger
    settingsTrigger = null
    if (trigger?.isConnected) trigger.focus()
  }

  async function saveSettings(language: Language) {
    if (!settingsOpen || settingsSaving) return
    if (loadStatus === 'newerVersion' || settingsError === 'readOnly') return
    settingsSaving = true
    settingsError = null
    try {
      const snapshot = await setSettings({ language })
      if (destroyed) return
      // One render cycle: the new language, the cleared warning and the closed panel.
      applyLanguage(snapshot.settings.language)
      loadStatus = snapshot.loadStatus
      if (notice?.settingsWarning) notice = warningOf(snapshot.loadStatus)
      settingsSaving = false
      closeSettings()
    } catch (err) {
      if (destroyed) return
      const code = err instanceof ApiError ? err.code : 'INTERNAL'
      settingsError =
        code === 'SETTINGS_WRITE_FAILED'
          ? 'writeFailed'
          : code === 'SETTINGS_READ_ONLY'
            ? 'readOnly'
            : 'internal'
      settingsSaving = false
    }
  }

  // Background key commands are suppressed while the modal is open: Ctrl+F (handled on the window
  // by the search field) is stopped before it gets there, and the WebView find bar stays closed.
  function onWindowKeydownCapture(event: KeyboardEvent) {
    if (!settingsOpen) return
    const ctrlF =
      event.ctrlKey && (event.key === 'f' || event.key === 'F' || event.code === 'KeyF')
    if (!ctrlF) return
    event.preventDefault()
    event.stopPropagation()
  }

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

  /**
   * A status-bar notice kept as a catalogue key plus its data, so it re-renders in the active
   * language (language-settings.md section 1). `settingsWarning` marks the RF-13 load warning.
   */
  type Notice = { key: MessageKey; params?: MessageParams; settingsWarning?: boolean }

  // RF-09: the notice of the last export, shown in the status bar until a start attempt. RF-13: the
  // settings warning stays until another notice replaces it or a Save succeeds.
  let notice = $state<Notice | undefined>(undefined)
  let noticeText = $derived(notice ? t(notice.key, notice.params) : undefined)
  let exporting = $state(false)
  // RF-11 (result-cache.md section 4): true while the clearCache() call is pending
  let clearing = $state(false)

  type Saved = { notice: Notice | undefined; spectrogram: typeof spectrogram }

  function beginRun(): Saved {
    const saved = { notice, spectrogram }
    clearSpectrogram()
    // RF-13: a start attempt is not a notice, so the settings warning stays.
    if (!notice?.settingsWarning) notice = undefined
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
      // RF-13: the store keeps the catalogue key; the status bar translates it on every render.
      store.fail(`error.${err.code}`)
    } else {
      store.fail('error.INTERNAL')
    }
  }

  async function handleOpenFolder() {
    if (!acceptsInput) return
    if (store.phase === 'scanning' || store.phase === 'analysing' || clearing) return
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
    if (!acceptsInput) return
    if (store.phase === 'scanning' || store.phase === 'analysing' || clearing) return
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
    if (!acceptsInput || !canExport) return
    exporting = true
    try {
      const result = await exportCsv()
      if (result !== null) {
        const file = result.path.split(/[\\/]/).pop() ?? result.path
        notice =
          result.rows === 1
            ? { key: 'status.exportedOne', params: { file } }
            : { key: 'status.exported', params: { rows: result.rows, file } }
      }
    } catch (err) {
      const code = err instanceof ApiError ? err.code : 'INTERNAL'
      notice = {
        key:
          code === 'NOT_ANALYZED' || code === 'ANALYSIS_RUNNING'
            ? `error.${code}`
            : 'error.EXPORT_FAILED',
      }
    } finally {
      exporting = false
    }
  }

  // RF-11 (result-cache.md section 2.2): no confirmation; only the notice changes
  async function handleClearCache() {
    if (!acceptsInput || !canClearCache) return
    clearing = true
    try {
      await clearCache()
      notice = { key: 'status.cacheCleared' }
    } catch (err) {
      const code = err instanceof ApiError ? err.code : 'INTERNAL'
      notice = { key: code === 'ANALYSIS_RUNNING' ? 'error.ANALYSIS_RUNNING' : 'error.CACHE_FAILED' }
    } finally {
      clearing = false
    }
  }

  async function handleCancel() {
    if (!acceptsInput) return
    await cancelAnalysis()
  }

  // RF-08 (sort-filter.md 4.3): the Clear filters button disappears with the no-match state, so
  // the focus moves to the search field.
  let searchFocusRequest = $state(0)

  function handleClearFilters() {
    store.clearFilter()
    searchFocusRequest += 1
  }

  let running = $derived(store.phase === 'scanning' || store.phase === 'analysing')
  let canOpen = $derived(!running && !clearing)
  // RF-11 (result-cache.md section 2.1): the export does not use the cache, so exporting does not disable it
  let canClearCache = $derived(!running && !clearing)
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

{#snippet settingsPanel()}
  <SettingsPanel
    language={getLocale()}
    {loadStatus}
    saving={settingsSaving}
    error={settingsError}
    onsave={saveSettings}
    onclose={closeSettings}
  />
{/snippet}

{#snippet titleBarContent()}
  <TitleBar
    title={t('app.title')}
    onminimize={minimizeWindow}
    ontoggleMaximize={toggleMaximizeWindow}
    onclose={closeWindow}
  />
{/snippet}

<svelte:window onkeydowncapture={onWindowKeydownCapture} />

{#if !settingsReady}
  <!-- RF-13: no main screen before the settings read settles; the window controls stay usable -->
  <div class="startup">
    {@render titleBarContent()}
    <main class="startup-body" aria-busy="true" aria-label={t('app.title')}></main>
  </div>
{:else}
  <MainLayout
    detail={showDetail ? detailPanel : undefined}
    sidebar={isEmpty ? undefined : albumsPanel}
    settings={settingsOpen ? settingsPanel : undefined}
  >
    {#snippet titleBar()}
      {@render titleBarContent()}
    {/snippet}

    {#snippet toolbar()}
      <Toolbar
        {canOpen}
        {canCancel}
        {canExport}
        {canClearCache}
        canFilter={!isEmpty}
        canSettings={settingsReady}
        verdictFilter={store.filter.verdicts}
        query={store.filter.query}
        {searchFocusRequest}
        onopen={handleOpenFolder}
        oncancel={handleCancel}
        onexport={handleExport}
        onclearcache={handleClearCache}
        onsettings={openSettings}
        ontoggleverdict={(verdict) => {
          if (acceptsInput) store.toggleVerdictFilter(verdict)
        }}
        onquery={(text) => {
          if (acceptsInput) store.setQuery(text)
        }}
      />
    {/snippet}

    {#snippet table()}
      <div class="table-container">
        <FolderDropZone active={isDraggingOver} empty={isEmpty} />
        {#if !isEmpty}
          <ResultsTable
            rows={store.visibleRows}
            selectedIndex={store.selectedIndex}
            sort={store.sort}
            {revealRequest}
            noMatches={store.rows.length > 0 && store.visibleRows.length === 0}
            onselect={handleSelect}
            onsort={(column) => store.toggleSort(column)}
            onclearfilters={handleClearFilters}
          />
        {/if}
      </div>
    {/snippet}

    {#snippet statusBar()}
      <StatusBar
        phase={store.phase}
        summary={store.summary}
        message={store.message === undefined ? undefined : t(store.message)}
        shown={isFilterActive(store.filter) && store.rows.length > 0
          ? store.visibleRows.length
          : undefined}
        cache={store.cache ?? undefined}
        notice={noticeText}
      />
    {/snippet}
  </MainLayout>
{/if}

<style>
  .table-container {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .startup {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text);
  }

  .startup-body {
    flex: 1;
  }
</style>

<script lang="ts">
  import type { Snippet } from 'svelte'
  import Splitter from '../atoms/Splitter.svelte'

  type Props = {
    titleBar: Snippet
    toolbar: Snippet
    table: Snippet
    detail?: Snippet
    sidebar?: Snippet
    statusBar: Snippet
  }

  let { titleBar, toolbar, table, detail, sidebar, statusBar }: Props = $props()

  // main-screen.md "Layout": detail default 260 px (min 160), table min 120 px
  const DEFAULT_DETAIL_HEIGHT = 260
  const MIN_DETAIL_HEIGHT = 160
  const MIN_TABLE_HEIGHT = 120
  // matches the Splitter bar height (--space-1)
  const SPLITTER_HEIGHT = 4
  const STORAGE_KEY = 'fakeflac.layout.detailHeight'

  let content: HTMLElement | undefined = $state()
  let detailHeight = $state(loadDetailHeight())

  // storage may be missing, blocked or hold anything: fall back to the default
  function loadDetailHeight(): number {
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      if (saved !== null && /^\d+$/.test(saved)) {
        return Math.max(MIN_DETAIL_HEIGHT, Number(saved))
      }
    } catch {
      // ignored: default below
    }
    return DEFAULT_DETAIL_HEIGHT
  }

  function saveDetailHeight(height: number) {
    try {
      localStorage.setItem(STORAGE_KEY, String(height))
    } catch {
      // ignored: the position is a convenience, resizing still works
    }
  }

  function clamp(height: number): number {
    // clientHeight is 0 before layout: then only the detail minimum applies
    const available = content?.clientHeight ?? 0
    const max = available > 0 ? available - MIN_TABLE_HEIGHT - SPLITTER_HEIGHT : Infinity
    return Math.max(MIN_DETAIL_HEIGHT, Math.min(max, height))
  }

  // the splitter delta is positive downward: moving it down shrinks the detail.
  // The height stays fractional (sub-pixel deltas under display scaling); it is rounded only for
  // display and storage. Clamping first drops what a smaller window has already hidden.
  function resize(delta: number) {
    detailHeight = clamp(clamp(detailHeight) - delta)
    saveDetailHeight(Math.round(detailHeight))
  }
</script>

<div class="main-layout">
  <div class="layout-titlebar">
    {@render titleBar()}
  </div>
  <div class="layout-toolbar">
    {@render toolbar()}
  </div>
  <main class="layout-content">
    {#if sidebar}
      <aside class="layout-sidebar">
        {@render sidebar()}
      </aside>
    {/if}
    <div class="layout-main" bind:this={content}>
      <div class="layout-table">
        {@render table()}
      </div>
      {#if detail}
        <Splitter orientation="horizontal" onresize={resize} />
        <div class="layout-detail" style:height="{Math.round(detailHeight)}px">
          {@render detail()}
        </div>
      {/if}
    </div>
  </main>
  <div class="layout-statusbar">
    {@render statusBar()}
  </div>
</div>

<style>
  .main-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text);
  }

  .layout-titlebar {
    flex-shrink: 0;
  }

  .layout-toolbar {
    flex-shrink: 0;
  }

  .layout-content {
    display: flex;
    flex-direction: row;
    flex: 1;
    min-height: 0;
    position: relative;
    overflow: hidden;
  }

  .layout-sidebar {
    flex: 0 0 var(--sidebar-width);
    width: var(--sidebar-width);
    min-height: 0;
    overflow-y: auto;
    box-sizing: border-box;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
  }

  .layout-main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    position: relative;
    overflow: hidden;
  }

  .layout-table {
    flex: 1;
    min-height: 120px;
    overflow: hidden;
    position: relative;
  }

  .layout-detail {
    /* the height comes from the splitter; it shrinks to the minimum when the window gets smaller */
    flex-shrink: 1;
    min-height: 160px;
    background: var(--color-surface);
  }

  .layout-statusbar {
    flex-shrink: 0;
  }
</style>

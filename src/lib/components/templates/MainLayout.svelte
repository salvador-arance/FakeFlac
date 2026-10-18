<script lang="ts">
  import type { Snippet } from 'svelte'

  type Props = {
    titleBar: Snippet
    toolbar: Snippet
    table: Snippet
    detail?: Snippet
    statusBar: Snippet
  }

  let { titleBar, toolbar, table, detail, statusBar }: Props = $props()
</script>

<div class="main-layout">
  <div class="layout-titlebar">
    {@render titleBar()}
  </div>
  <div class="layout-toolbar">
    {@render toolbar()}
  </div>
  <main class="layout-content">
    <div class="layout-table">
      {@render table()}
    </div>
    {#if detail}
      <div class="layout-detail">
        {@render detail()}
      </div>
    {/if}
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
    flex-direction: column;
    flex: 1;
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
    flex-shrink: 0;
    min-height: 160px;
    border-top: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .layout-statusbar {
    flex-shrink: 0;
  }
</style>

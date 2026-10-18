<script lang="ts">
  import WindowControls from '../molecules/WindowControls.svelte'

  type Props = {
    title: string
    onminimize?: () => void
    ontoggleMaximize?: () => void
    onclose?: () => void
  }

  let { title, onminimize, ontoggleMaximize, onclose }: Props = $props()
</script>

<header class="title-bar">
  <div
    class="drag-region"
    data-tauri-drag-region
    ondblclick={ontoggleMaximize}
    role="presentation"
  >
    <span class="app-title">{title}</span>
  </div>
  <WindowControls {onminimize} {ontoggleMaximize} {onclose} />
</header>

<style>
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    padding: 0 var(--space-2);
    box-sizing: border-box;
    user-select: none;
  }

  .drag-region {
    display: flex;
    align-items: center;
    flex: 1;
    height: 100%;
    cursor: default;
  }

  .app-title {
    font-weight: 600;
    font-size: var(--font-size);
    color: var(--color-text);
  }
</style>

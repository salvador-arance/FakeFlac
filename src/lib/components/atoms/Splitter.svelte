<script lang="ts">
  type Props = {
    orientation: 'horizontal'
    onresize?: (delta: number) => void
  }

  // components.md "Atoms": keyboard ↑/↓ move 10 px
  const KEY_STEP = 10

  let { orientation, onresize }: Props = $props()

  let dragging = $state(false)
  let lastY = 0

  function startDrag(e: PointerEvent) {
    if (e.button !== 0) return
    // keep the text from being selected while dragging; focus is lost with preventDefault, so set it
    e.preventDefault()
    ;(e.currentTarget as HTMLElement).focus()
    dragging = true
    lastY = e.clientY
  }

  // window listeners: the pointer leaves the thin bar during a drag (no pointer capture in jsdom)
  function moveDrag(e: PointerEvent) {
    if (!dragging) return
    const delta = e.clientY - lastY
    lastY = e.clientY
    if (delta !== 0) onresize?.(delta)
  }

  function endDrag() {
    dragging = false
  }

  function onkeydown(e: KeyboardEvent) {
    const delta = e.key === 'ArrowUp' ? -KEY_STEP : e.key === 'ArrowDown' ? KEY_STEP : 0
    if (delta === 0) return
    e.preventDefault()
    onresize?.(delta)
  }
</script>

<!-- blur: a pointerup may never arrive when the window loses focus mid-drag -->
<svelte:window
  onpointermove={moveDrag}
  onpointerup={endDrag}
  onpointercancel={endDrag}
  onblur={endDrag}
/>

<!-- a focusable separator is the WAI-ARIA window splitter widget; Svelte classes the role as static -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter"
  role="separator"
  aria-orientation={orientation}
  tabindex="0"
  data-state={dragging ? 'dragging' : 'idle'}
  onpointerdown={startDrag}
  {onkeydown}
></div>

<style>
  .splitter {
    flex-shrink: 0;
    height: var(--space-1);
    background: var(--color-border);
    cursor: row-resize;
    touch-action: none;
  }

  .splitter:hover,
  .splitter[data-state='dragging'] {
    background: var(--color-accent);
  }

  .splitter:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: -1px;
  }
</style>

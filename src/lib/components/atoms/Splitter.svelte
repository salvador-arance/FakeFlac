<script lang="ts">
  import { RESIZE_STEP } from '../../tokens/layout'
  type Props = {
    orientation: 'horizontal' | 'vertical'
    label?: string
    description?: string
    value?: number
    min?: number
    max?: number
    valueText?: string
    variant?: 'panel' | 'column'
    onresize?: (delta: number) => void
    onresizeend?: () => void
    onreset?: () => void
  }
  let { orientation, label, description, value, min, max, valueText, variant = 'panel', onresize, onresizeend, onreset }: Props = $props()
  const descriptionId = $props.id()
  let node: HTMLDivElement | undefined = $state()
  let dragging = $state(false)
  let pointerId: number | undefined
  let last = 0
  let changed = false
  let observer: MutationObserver | undefined
  const blocked = () => !!node?.closest('[inert]')
  function stop(commit = true) {
    if (!dragging) return
    dragging = false
    observer?.disconnect()
    observer = undefined
    if (pointerId !== undefined && node?.hasPointerCapture?.(pointerId)) node.releasePointerCapture(pointerId)
    pointerId = undefined
    if (commit && changed && orientation === 'vertical') onresizeend?.()
    changed = false
  }
  function startDrag(e: PointerEvent) {
    if (e.button !== 0 || e.isPrimary === false || dragging || blocked()) return
    e.preventDefault()
    e.stopPropagation()
    node?.focus()
    pointerId = e.pointerId
    node?.setPointerCapture?.(e.pointerId)
    dragging = true
    changed = false
    last = orientation === 'vertical' ? e.clientX : e.clientY
    observer = new MutationObserver(() => { if (blocked()) stop() })
    for (let ancestor = node?.parentElement; ancestor; ancestor = ancestor.parentElement) {
      observer.observe(ancestor, { attributes: true, attributeFilter: ['inert'] })
    }
  }
  function moveDrag(e: PointerEvent) {
    if (!dragging || e.pointerId !== pointerId) return
    if (blocked()) { stop(); return }
    const position = orientation === 'vertical' ? e.clientX : e.clientY
    const delta = position - last
    last = position
    if (delta !== 0) { changed = true; onresize?.(delta) }
  }
  function endDrag(e: PointerEvent) {
    if (e.pointerId === pointerId) stop()
  }
  function onkeydown(e: KeyboardEvent) {
    if (blocked()) return
    const vertical = orientation === 'vertical'
    if (vertical && e.key === 'Enter') {
      e.preventDefault(); e.stopPropagation(); stop(false); onreset?.(); return
    }
    const delta = e.key === (vertical ? 'ArrowLeft' : 'ArrowUp') ? -RESIZE_STEP : e.key === (vertical ? 'ArrowRight' : 'ArrowDown') ? RESIZE_STEP : 0
    if (!delta) return
    e.preventDefault()
    e.stopPropagation()
    onresize?.(delta)
    if (vertical) onresizeend?.()
  }
  function reset(e: MouseEvent) {
    e.preventDefault(); e.stopPropagation()
    if (orientation === 'vertical' && !blocked()) { stop(false); onreset?.() }
  }
  $effect(() => () => { stop(false) })
</script>
<svelte:window onpointermove={moveDrag} onpointerup={endDrag} onpointercancel={endDrag} onblur={() => stop()} />
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div bind:this={node} class="splitter" class:vertical={orientation === 'vertical'} class:column={orientation === 'vertical' && variant === 'column'} role="separator"
  aria-orientation={orientation} aria-label={label} aria-describedby={description ? descriptionId : undefined}
  aria-valuenow={value === undefined ? undefined : Math.round(value)} aria-valuemin={min} aria-valuemax={max} aria-valuetext={valueText}
  tabindex="0" data-state={dragging ? 'dragging' : 'idle'} onpointerdown={startDrag} {onkeydown} ondblclick={reset}
  onclick={(event) => event.stopPropagation()}></div>
{#if description}<span id={descriptionId} hidden>{description}</span>{/if}
<style>
  .splitter {
    flex-shrink: 0;
    height: var(--space-1);
    background: var(--color-border);
    cursor: row-resize;
    touch-action: none;
  }

  .splitter.vertical {
    width: var(--sidebar-separator-width);
    height: auto;
    align-self: stretch;
    cursor: col-resize;
  }

  .splitter.column {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: var(--column-resize-width);
    background: transparent;
    border-right: var(--resize-boundary-width) solid var(--color-border);
    box-sizing: border-box;
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

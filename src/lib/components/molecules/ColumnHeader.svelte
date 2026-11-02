<script lang="ts">
  import Splitter from '../atoms/Splitter.svelte'
  import Icon from '../atoms/Icon.svelte'

  type Props = {
    resize?: { width: number; min: number; max: number; label: string; description: string; valueText: string }
    onresize?: (delta: number) => void
    onresizeend?: () => void
    onresetwidth?: () => void
    label: string
    sort?: 'asc' | 'desc' | null
    onsort?: () => void
    /** Tooltip of the header (only the Real bitrate column sets it). */
    tooltip?: string
  }

  let { label, sort = null, onsort, tooltip, resize, onresize, onresizeend, onresetwidth }: Props = $props()

  const tooltipId = $props.id()

  let ariaSort: 'none' | 'ascending' | 'descending' = $derived(
    sort === 'asc' ? 'ascending' : sort === 'desc' ? 'descending' : 'none',
  )
</script>

<!-- The tooltip: `title` on hover; on keyboard focus a CSS bubble drawn from `data-tooltip` (a
     pseudo-element). The button's accessible description is a hidden sibling, so the header's text
     content is unchanged; being `hidden`, it takes no cell of the parent grid. -->
<div class="column-header" class:resizable={!!resize} role="columnheader" aria-sort={ariaSort} title={resize ? (tooltip ? `${label}: ${tooltip}` : label) : tooltip}>
  <button
    type="button"
    class="header-button"
    onclick={onsort}
    aria-label={label}
    aria-describedby={tooltip ? tooltipId : undefined}
    data-tooltip={tooltip}
  >
    <span class="label">{label}</span>
    {#if sort === 'asc'}
      <Icon name="sort-asc" size={16} />
    {:else if sort === 'desc'}
      <Icon name="sort-desc" size={16} />
    {/if}
  </button>
  {#if resize}
    <Splitter orientation="vertical" variant="column" label={resize.label} description={resize.description} value={resize.width} min={resize.min} max={resize.max} valueText={resize.valueText} {onresize} {onresizeend} onreset={onresetwidth} />
  {/if}
</div>
{#if tooltip}
  <span id={tooltipId} hidden>{tooltip}</span>
{/if}

<style>
  .column-header {
    position: relative;
    display: inline-flex;
    align-items: center;
    user-select: none;
    height: 100%;
    min-width: 0;
    width: 100%;
  }

  .resizable { padding-right: var(--column-resize-width); box-sizing: border-box; }
  .label { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .header-button :global(svg) { flex-shrink: 0; }
  .header-button {
    display: inline-flex;
    min-width: 0;
    max-width: 100%;
    box-sizing: border-box;
    text-align: left;
    align-items: center;
    gap: var(--space-1);
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    font-family: inherit;
    font-size: var(--font-size);
    font-weight: 600;
    cursor: pointer;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--space-1);
    transition: color 0.1s ease, background-color 0.1s ease;
  }

  .header-button:hover {
    color: var(--color-text);
    background: var(--color-surface-raised);
  }

  .header-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: -1px;
    color: var(--color-text);
  }

  /* Right-aligned: the only header with a tooltip (Real bitrate) is the last column. */
  .header-button[data-tooltip]:focus-visible::after {
    /* Empty alt text: the tooltip is already the button's description (aria-describedby). */
    content: attr(data-tooltip) / "";
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 20;
    width: max-content;
    max-width: 280px;
    margin-top: var(--space-1);
    padding: var(--space-1) var(--space-2);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--space-1);
    color: var(--color-text);
    font-weight: 400;
    white-space: normal;
    text-align: left;
    pointer-events: none;
  }
</style>

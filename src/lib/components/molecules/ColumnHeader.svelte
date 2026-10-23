<script lang="ts">
  import Icon from '../atoms/Icon.svelte'

  type Props = {
    label: string
    sort?: 'asc' | 'desc' | null
    onsort?: () => void
    /** Tooltip of the header (only the Real bitrate column sets it). */
    tooltip?: string
  }

  let { label, sort = null, onsort, tooltip }: Props = $props()

  const tooltipId = $props.id()

  let ariaSort: 'none' | 'ascending' | 'descending' = $derived(
    sort === 'asc' ? 'ascending' : sort === 'desc' ? 'descending' : 'none',
  )
</script>

<!-- The tooltip: `title` on hover; on keyboard focus a CSS bubble drawn from `data-tooltip` (a
     pseudo-element). The button's accessible description is a hidden sibling, so the header's text
     content is unchanged; being `hidden`, it takes no cell of the parent grid. -->
<div class="column-header" role="columnheader" aria-sort={ariaSort} title={tooltip}>
  <button
    type="button"
    class="header-button"
    onclick={onsort}
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
  }

  .header-button {
    display: inline-flex;
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
    content: attr(data-tooltip);
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

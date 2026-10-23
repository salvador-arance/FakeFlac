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

  let ariaSort: 'none' | 'ascending' | 'descending' = $derived(
    sort === 'asc' ? 'ascending' : sort === 'desc' ? 'descending' : 'none',
  )
</script>

<div class="column-header" role="columnheader" aria-sort={ariaSort} title={tooltip}>
  <button type="button" class="header-button" onclick={onsort}>
    <span class="label">{label}</span>
    {#if sort === 'asc'}
      <Icon name="sort-asc" size={16} />
    {:else if sort === 'desc'}
      <Icon name="sort-desc" size={16} />
    {/if}
  </button>
</div>

<style>
  .column-header {
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
</style>

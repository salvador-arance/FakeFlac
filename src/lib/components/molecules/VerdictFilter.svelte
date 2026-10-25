<script lang="ts">
  // VerdictFilter (specs/ui/sort-filter.md section 10): four verdict toggle buttons in a group.
  import IconButton from '../atoms/IconButton.svelte'
  import { t } from '../../i18n'
  import type { Verdict } from '../../model'

  type Props = {
    selected: Verdict[]
    disabled?: boolean
    ontoggle?: (verdict: Verdict) => void
  }

  let { selected, disabled = false, ontoggle }: Props = $props()

  // The buttons' order is the canonical order of `RowFilter.verdicts`.
  const VERDICTS: Verdict[] = ['fake', 'suspicious', 'real', 'error']
</script>

<div class="verdict-filter" role="group" aria-label={t('filter.label')}>
  {#each VERDICTS as verdict (verdict)}
    <IconButton
      icon={`verdict-${verdict}` as const}
      label={t(`filter.${verdict}`)}
      size="toolbar"
      pressed={selected.includes(verdict)}
      tone={verdict}
      {disabled}
      onclick={() => ontoggle?.(verdict)}
    />
  {/each}
</div>

<style>
  .verdict-filter {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
</style>

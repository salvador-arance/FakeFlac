<script lang="ts">
  import Icon from '../atoms/Icon.svelte'
  import type { IconName } from '../atoms/icons'
  import { t } from '../../i18n'
  import type { SuspicionReason, Verdict } from '../../model'

  type Props = {
    verdict: Verdict
    reasons?: SuspicionReason[]
  }

  let { verdict, reasons }: Props = $props()

  const iconName: Record<Verdict, IconName> = {
    real: 'verdict-real',
    suspicious: 'verdict-suspicious',
    fake: 'verdict-fake',
    error: 'verdict-error',
  }

  let text = $derived(t(`verdict.${verdict}`))

  let tooltip = $derived.by(() => {
    if (!reasons || reasons.length === 0) return undefined
    return reasons.map((r) => t(`reason.${r}`)).join('\n')
  })
</script>

<div class="verdict-cell {verdict}" title={tooltip}>
  <Icon name={iconName[verdict]} size={16} />
  <span class="label">{text}</span>
</div>

<style>
  .verdict-cell {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--font-size);
    white-space: nowrap;
  }

  /* The icon keeps the saturated verdict colour; the label uses the readable tint (RNF-08 U3). */
  .verdict-cell.real {
    color: var(--verdict-real);
  }

  .verdict-cell.real .label {
    color: var(--verdict-real-text);
  }

  .verdict-cell.suspicious {
    color: var(--verdict-suspicious);
  }

  .verdict-cell.suspicious .label {
    color: var(--verdict-suspicious-text);
  }

  .verdict-cell.fake {
    color: var(--verdict-fake);
  }

  .verdict-cell.fake .label {
    color: var(--verdict-fake-text);
  }

  .verdict-cell.error {
    color: var(--verdict-error);
  }

  .verdict-cell.error .label {
    color: var(--verdict-error-text);
  }

  .label {
    user-select: none;
  }
</style>

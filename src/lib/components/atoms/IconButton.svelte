<script lang="ts">
  import Icon from './Icon.svelte'
  import type { IconName } from './icons'
  import type { Verdict } from '../../model'

  type Props = {
    icon: IconName
    label: string
    disabled?: boolean
    size?: 'toolbar' | 'small'
    /** RF-08 (sort-filter.md section 10): given = a toggle button with `aria-pressed`. Stub: not used yet. */
    pressed?: boolean
    /** RF-08: the icon takes the verdict colour token. Stub: not used yet. */
    tone?: Verdict
    onclick?: () => void
  }

  let { icon, label, disabled = false, size = 'toolbar', onclick }: Props = $props()
</script>

<button
  type="button"
  class="icon-button {size}"
  {disabled}
  title={label}
  aria-label={label}
  {onclick}
>
  <Icon name={icon} size={size === 'toolbar' ? 24 : 16} />
</button>

<style>
  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--space-1);
    color: var(--color-text);
    cursor: pointer;
    padding: 0;
    transition: background-color 0.1s ease, border-color 0.1s ease;
  }

  .icon-button.toolbar {
    width: 40px;
    height: 40px;
  }

  .icon-button.small {
    width: 24px;
    height: 24px;
  }

  .icon-button:hover:not(:disabled) {
    background: var(--color-surface-raised);
    border-color: var(--color-border);
  }

  .icon-button:active:not(:disabled) {
    background: var(--color-surface);
  }

  .icon-button:disabled {
    color: var(--color-text-muted);
    cursor: not-allowed;
    opacity: 0.5;
  }

  .icon-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 1px;
  }
</style>

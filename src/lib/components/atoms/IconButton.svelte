<script lang="ts">
  import Icon from './Icon.svelte'
  import type { IconName } from './icons'
  import type { Verdict } from '../../model'

  type Props = {
    icon: IconName
    label: string
    disabled?: boolean
    size?: 'toolbar' | 'small'
    /** Given (true or false) = a toggle button with `aria-pressed`; absent = a plain button. */
    pressed?: boolean
    /** The icon takes the verdict colour token (`--verdict-*`), except when disabled. */
    tone?: Verdict
    onclick?: () => void
  }

  let { icon, label, disabled = false, size = 'toolbar', pressed, tone, onclick }: Props = $props()
</script>

<button
  type="button"
  class="icon-button {size}"
  class:pressed
  class:toned={tone !== undefined}
  data-tone={tone}
  {disabled}
  title={label}
  aria-label={label}
  aria-pressed={pressed}
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

  .icon-button[data-tone='real'] {
    color: var(--verdict-real);
  }

  .icon-button[data-tone='suspicious'] {
    color: var(--verdict-suspicious);
  }

  .icon-button[data-tone='fake'] {
    color: var(--verdict-fake);
  }

  .icon-button[data-tone='error'] {
    color: var(--verdict-error);
  }

  .icon-button:hover:not(:disabled) {
    background: var(--color-surface-raised);
    border-color: var(--color-border);
  }

  .icon-button:active:not(:disabled) {
    background: var(--color-surface);
  }

  /* A pressed toggle keeps its look on hover; it differs from a hovered one by the border. */
  .icon-button.pressed,
  .icon-button.pressed:hover:not(:disabled) {
    background: var(--color-surface-raised);
    border-color: var(--color-accent);
  }

  .icon-button:disabled {
    cursor: not-allowed;
    opacity: var(--disabled-opacity);
  }

  .icon-button:disabled,
  .icon-button.toned:disabled {
    color: var(--color-text-muted);
  }

  .icon-button:focus-visible {
    outline: 2px solid var(--color-accent);
    outline-offset: 1px;
  }
</style>

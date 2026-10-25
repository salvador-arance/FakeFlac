<script lang="ts">
  // SearchField (specs/ui/sort-filter.md section 10): the name search of the toolbar. The native
  // input lives here (no `Input` atom yet, Q13).
  import Icon from '../atoms/Icon.svelte'
  import IconButton from '../atoms/IconButton.svelte'
  import { t } from '../../i18n'

  type Props = {
    value: string
    disabled?: boolean
    /** Ctrl+F on the window focuses the field (and always prevents the WebView2 find bar). */
    shortcut?: boolean
    /** Every change of this counter focuses the field, unless disabled. */
    focusRequest?: number
    oninput?: (value: string) => void
  }

  let { value, disabled = false, shortcut = false, focusRequest = 0, oninput }: Props = $props()

  let input: HTMLInputElement | undefined = $state()

  function onWindowKeydown(event: KeyboardEvent) {
    if (!shortcut) return
    if (!event.ctrlKey || event.altKey || event.shiftKey || event.metaKey) return
    if (event.key !== 'f' && event.key !== 'F' && event.code !== 'KeyF') return
    event.preventDefault()
    if (disabled) return
    input?.focus()
    input?.select()
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && value !== '') {
      event.preventDefault()
      oninput?.('')
    }
  }

  function clear() {
    oninput?.('')
    input?.focus()
  }

  // The initial value is not a request (the pattern of `ResultsTable`'s `revealRequest`).
  let seenRequest: number | undefined
  $effect(() => {
    const request = focusRequest
    const changed = seenRequest !== undefined && request !== seenRequest
    seenRequest = request
    if (changed && !disabled) input?.focus()
  })
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="search-field" class:disabled>
  <Icon name="search" size={16} />
  <input
    bind:this={input}
    type="text"
    role="searchbox"
    aria-label={t('search.label')}
    aria-keyshortcuts="Control+F"
    placeholder={t('search.placeholder')}
    spellcheck="false"
    autocomplete="off"
    {value}
    {disabled}
    oninput={(event) => oninput?.(event.currentTarget.value)}
    onkeydown={onKeydown}
  />
  {#if value !== '' && !disabled}
    <IconButton icon="close" label={t('search.clear')} size="small" onclick={clear} />
  {/if}
</div>

<style>
  .search-field {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    width: var(--search-width);
    box-sizing: border-box;
    height: var(--control-height);
    padding: 0 var(--space-1) 0 var(--space-2);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--space-1);
    color: var(--color-text-muted);
  }

  .search-field:focus-within {
    border-color: var(--color-accent);
  }

  .search-field.disabled {
    opacity: var(--disabled-opacity);
  }

  input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: var(--color-text);
    font-family: var(--font-ui);
    font-size: var(--font-size);
  }

  input::placeholder {
    color: var(--color-text-muted);
  }
</style>

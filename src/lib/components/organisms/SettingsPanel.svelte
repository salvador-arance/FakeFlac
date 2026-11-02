<script lang="ts">
  // SettingsPanel (specs/ui/language-settings.md sections 2 and 4): the language modal. The draft
  // belongs to the panel and starts at `language` on each opening (each mount); it owns the modal
  // keyboard and focus behaviour. No API calls: the page saves and decides when the panel closes.
  import { onMount, untrack } from 'svelte'
  import LanguageSelect from '../molecules/LanguageSelect.svelte'
  import { t } from '../../i18n'
  import type { Language, SettingsError, SettingsLoadStatus } from '../../model'

  type Props = {
    language: Language
    loadStatus: SettingsLoadStatus
    saving: boolean
    error: SettingsError | null
    onsave?: (language: Language) => void
    onclose?: () => void
  }

  let { language, loadStatus, saving, error, onsave, onclose }: Props = $props()

  const uid = $props.id()
  const headingId = `${uid}-heading`

  // Changing the select updates only this draft; a new opening starts from `language` again.
  let draft = $state(untrack(() => language))

  let container: HTMLDivElement | undefined = $state()
  let saveButton: HTMLButtonElement | undefined = $state()
  let dismissButton: HTMLButtonElement | undefined = $state()

  // A newer settings file (at startup or found by a save) can be closed but never replaced.
  let canSave = $derived(loadStatus !== 'newerVersion' && error !== 'readOnly')

  let warning = $derived(
    loadStatus === 'invalid' || loadStatus === 'unavailable' || loadStatus === 'newerVersion'
      ? t(`settings.${loadStatus}`)
      : undefined,
  )

  const ERROR_KEY: Record<SettingsError, string> = {
    writeFailed: 'error.SETTINGS_WRITE_FAILED',
    readOnly: 'error.SETTINGS_READ_ONLY',
    internal: 'settings.saveFailed',
  }

  let errorText = $derived(error ? t(ERROR_KEY[error]) : undefined)

  const selectElement = () => container?.querySelector('select') ?? undefined

  function save() {
    if (saving || !canSave) return
    onsave?.(draft)
  }

  function dismiss() {
    if (saving) return
    onclose?.()
  }

  /** The enabled controls of the panel, in tab order. */
  function focusables(): HTMLElement[] {
    if (!container) return []
    return Array.from(
      container.querySelectorAll<HTMLElement>('select:not(:disabled), button:not(:disabled)'),
    )
  }

  function trapTab(event: KeyboardEvent) {
    event.preventDefault()
    const items = focusables()
    // While saving every control is disabled: the focus stays on the dialog container.
    if (items.length === 0) {
      container?.focus()
      return
    }
    const position = items.indexOf(document.activeElement as HTMLElement)
    const step = event.shiftKey ? -1 : 1
    const next =
      position < 0
        ? event.shiftKey
          ? items.length - 1
          : 0
        : (position + step + items.length) % items.length
    items[next].focus()
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Tab') {
      trapTab(event)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      dismiss()
    }
  }

  // A press on the backdrop neither dismisses the panel nor takes the focus out of it.
  function keepFocus(event: MouseEvent) {
    if (event.target === event.currentTarget) event.preventDefault()
  }

  onMount(() => {
    if (canSave) selectElement()?.focus()
    else dismissButton?.focus()
  })

  // Saving moves the focus to the container; a failure brings it back to Save, or to Close when
  // the failure disables Save (SETTINGS_READ_ONLY).
  let wasSaving = false
  $effect(() => {
    const busy = saving
    const failure = error
    if (busy && !wasSaving) {
      container?.focus()
    } else if (!busy && wasSaving && failure) {
      if (failure === 'readOnly') dismissButton?.focus()
      else saveButton?.focus()
    }
    wasSaving = busy
  })
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="backdrop" role="presentation" onmousedown={keepFocus}>
  <div
    bind:this={container}
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby={headingId}
    aria-busy={saving}
    tabindex="-1"
  >
    <h2 id={headingId} class="heading">{t('settings.title')}</h2>
    <LanguageSelect
      value={draft}
      disabled={saving || !canSave}
      onchange={(next) => (draft = next)}
    />
    {#if warning}
      <p class="message warning">{warning}</p>
    {/if}
    {#if errorText}
      <p class="message error" role="alert">{errorText}</p>
    {/if}
    <div class="actions">
      <button
        bind:this={saveButton}
        type="button"
        class="action primary"
        disabled={saving || !canSave}
        onclick={save}
      >
        {saving ? t('settings.saving') : t('settings.save')}
      </button>
      <button
        bind:this={dismissButton}
        type="button"
        class="action"
        disabled={saving}
        onclick={dismiss}
      >
        {canSave ? t('settings.cancel') : t('settings.close')}
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
    box-sizing: border-box;
    background: var(--color-backdrop);
  }

  .dialog {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: var(--dialog-width);
    max-width: 100%;
    max-height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
    padding: var(--space-4);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--space-1);
    color: var(--color-text);
    font-size: var(--font-size);
    user-select: text;
  }

  .dialog:focus {
    outline: none;
  }

  .heading {
    margin: 0;
    font-size: var(--font-size);
    font-weight: 600;
  }

  .message {
    margin: 0;
    padding: var(--space-2);
    border-left: 2px solid var(--color-accent);
    background: var(--color-surface-raised);
    overflow-wrap: anywhere;
  }

  .message.error {
    border-left-color: var(--color-danger);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .action {
    min-height: var(--control-height);
    padding: 0 var(--space-3);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--space-1);
    color: var(--color-text);
    font-family: var(--font-ui);
    font-size: var(--font-size);
    cursor: pointer;
  }

  .action.primary {
    border-color: var(--color-accent);
  }

  .action:focus-visible {
    outline: 1px solid var(--color-accent);
  }

  .action:disabled {
    opacity: var(--disabled-opacity);
    cursor: default;
  }
</style>

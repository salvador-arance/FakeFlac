<script lang="ts">
  // LanguageSelect (specs/ui/language-settings.md section 4): the labelled native select of the
  // Settings panel. The options are autonyms with their own `lang`; no persistence or store access.
  import { t } from '../../i18n'
  import type { Language } from '../../model'

  type Props = {
    value: Language
    disabled?: boolean
    onchange?: (language: Language) => void
  }

  let { value, disabled = false, onchange }: Props = $props()

  const id = $props.id()

  const OPTIONS: { language: Language; key: string }[] = [
    { language: 'en', key: 'settings.english' },
    { language: 'es', key: 'settings.spanish' },
  ]

  function handleChange(event: Event & { currentTarget: HTMLSelectElement }) {
    const language = event.currentTarget.value
    if (language === 'en' || language === 'es') onchange?.(language)
  }
</script>

<div class="language-select">
  <label for={id}>{t('settings.language')}</label>
  <select {id} {disabled} onchange={handleChange}>
    {#each OPTIONS as option (option.language)}
      <option value={option.language} lang={option.language} selected={option.language === value}>
        {t(option.key)}
      </option>
    {/each}
  </select>
</div>

<style>
  .language-select {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--color-text);
    font-size: var(--font-size);
  }

  select {
    height: var(--control-height);
    padding: 0 var(--space-2);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--space-1);
    color: var(--color-text);
    font-family: var(--font-ui);
    font-size: var(--font-size);
  }

  select:focus-visible {
    outline: 1px solid var(--color-accent);
    border-color: var(--color-accent);
  }

  select:disabled {
    opacity: var(--disabled-opacity);
  }
</style>

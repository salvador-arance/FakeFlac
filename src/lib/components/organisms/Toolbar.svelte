<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte'
  import SearchField from '../molecules/SearchField.svelte'
  import VerdictFilter from '../molecules/VerdictFilter.svelte'
  import { t } from '../../i18n'
  import type { Verdict } from '../../model'

  type Props = {
    canOpen: boolean
    canCancel: boolean
    canExport?: boolean
    /** RF-11 (result-cache.md section 2.1). */
    canClearCache?: boolean
    canFilter: boolean
    /** RF-13 (language-settings.md section 2): enabled in every initialized main-screen phase. */
    canSettings: boolean
    verdictFilter: Verdict[]
    query: string
    searchFocusRequest?: number
    onopen?: () => void
    oncancel?: () => void
    onexport?: () => void
    onclearcache?: () => void
    onsettings?: () => void
    ontoggleverdict?: (verdict: Verdict) => void
    onquery?: (text: string) => void
  }

  let {
    canOpen,
    canCancel,
    canExport = false,
    canClearCache = false,
    canFilter,
    canSettings,
    verdictFilter,
    query,
    searchFocusRequest = 0,
    onopen,
    oncancel,
    onexport,
    onclearcache,
    onsettings,
    ontoggleverdict,
    onquery,
  }: Props = $props()
</script>

<div class="toolbar" role="toolbar" aria-label={t('toolbar.label')}>
  <IconButton
    icon="folder-open"
    label={t('toolbar.open')}
    disabled={!canOpen}
    size="toolbar"
    onclick={onopen}
  />
  <IconButton
    icon="cancel"
    label={t('toolbar.cancel')}
    disabled={!canCancel}
    size="toolbar"
    onclick={oncancel}
  />
  <div class="separator" role="separator" aria-orientation="vertical"></div>
  <IconButton
    icon="export"
    label={t('toolbar.export')}
    disabled={!canExport}
    size="toolbar"
    onclick={onexport}
  />
  <IconButton
    icon="clear-cache"
    label={t('toolbar.clearCache')}
    disabled={!canClearCache}
    size="toolbar"
    onclick={onclearcache}
  />
  <div class="separator" role="separator" aria-orientation="vertical"></div>
  <VerdictFilter selected={verdictFilter} disabled={!canFilter} ontoggle={ontoggleverdict} />
  <IconButton
    icon="settings"
    label={t('toolbar.settings')}
    disabled={!canSettings}
    size="toolbar"
    onclick={onsettings}
  />
  <div class="search">
    <SearchField
      value={query}
      disabled={!canFilter}
      shortcut
      focusRequest={searchFocusRequest}
      oninput={onquery}
    />
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 48px;
    padding: 0 var(--space-2);
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    box-sizing: border-box;
  }

  .search {
    margin-left: auto;
  }

  .separator {
    width: 1px;
    height: 24px;
    background: var(--color-border);
  }
</style>

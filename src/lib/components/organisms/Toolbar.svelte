<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte'
  import { t } from '../../i18n'

  type Props = {
    canOpen: boolean
    canCancel: boolean
    canExport?: boolean
    onopen?: () => void
    oncancel?: () => void
    onexport?: () => void
  }

  let { canOpen, canCancel, canExport = false, onopen, oncancel, onexport }: Props = $props()
</script>

<div class="toolbar" role="toolbar" aria-label="Main toolbar">
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

  .separator {
    width: 1px;
    height: 24px;
    background: var(--color-border);
  }
</style>

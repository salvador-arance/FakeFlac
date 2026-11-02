<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte'
  import { t } from '../../i18n'

  type Props = {
    onminimize?: () => void
    ontoggleMaximize?: () => void
    onclose?: () => void
  }

  let { onminimize, ontoggleMaximize, onclose }: Props = $props()
</script>

<div class="window-controls">
  <IconButton icon="minimize" label={t('window.minimize')} size="small" onclick={onminimize} />
  <IconButton icon="maximize" label={t('window.maximize')} size="small" onclick={ontoggleMaximize} />
  <IconButton icon="close" label={t('window.close')} size="small" onclick={onclose} />
</div>

<style>
  .window-controls {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
</style>

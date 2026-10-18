<script lang="ts">
  import IconButton from '../atoms/IconButton.svelte'

  type Props = {
    onminimize?: () => void
    ontoggleMaximize?: () => void
    onclose?: () => void
  }

  let { onminimize, ontoggleMaximize, onclose }: Props = $props()
</script>

<div class="window-controls">
  <IconButton icon="minimize" label="Minimize" size="small" onclick={onminimize} />
  <IconButton icon="maximize" label="Maximize" size="small" onclick={ontoggleMaximize} />
  <IconButton icon="close" label="Close" size="small" onclick={onclose} />
</div>

<style>
  .window-controls {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
</style>

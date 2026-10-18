<script lang="ts">
  type Props = { onminimize?: () => void; ontoggleMaximize?: () => void; onclose?: () => void }
  let { onminimize, ontoggleMaximize, onclose }: Props = $props()
</script>

<span data-stub="WindowControls" data-a={!!onminimize} data-b={!!ontoggleMaximize} data-c={!!onclose}></span>

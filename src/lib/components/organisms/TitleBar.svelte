<script lang="ts">
  type Props = {
    title: string
    onminimize?: () => void
    ontoggleMaximize?: () => void
    onclose?: () => void
  }
  let { title, onminimize, ontoggleMaximize, onclose }: Props = $props()
</script>

<span data-stub="TitleBar" data-title={title} data-a={!!onminimize} data-b={!!ontoggleMaximize} data-c={!!onclose}></span>

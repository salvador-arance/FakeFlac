<script lang="ts">
  type Props = { canOpen: boolean; canCancel: boolean; onopen?: () => void; oncancel?: () => void }
  let { canOpen, canCancel, onopen, oncancel }: Props = $props()
</script>

<span data-stub="Toolbar" data-open={canOpen} data-cancel={canCancel} data-a={!!onopen} data-b={!!oncancel}></span>

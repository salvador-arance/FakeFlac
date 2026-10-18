<script lang="ts">
  type Props = { hz: number | null; nyquistHz: number }
  let { hz, nyquistHz }: Props = $props()
</script>

<span data-stub="CutoffValue" data-hz={hz} data-nyquist={nyquistHz}></span>

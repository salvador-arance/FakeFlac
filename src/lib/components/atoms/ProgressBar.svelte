<script lang="ts">
  /** `value` in 0..1; absent means indeterminate. */
  type Props = { value?: number }
  let { value }: Props = $props()
</script>

<span data-stub="ProgressBar" data-value={value}></span>

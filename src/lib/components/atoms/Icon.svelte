<script lang="ts">
  import type { IconName } from './icons'
  type Props = { name: IconName; size?: 16 | 24 | 32; label?: string }
  let { name, size, label }: Props = $props()
</script>

<span data-stub="Icon" data-name={name} data-size={size} data-label={label}></span>

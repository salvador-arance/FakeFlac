<script lang="ts">
  import type { IconName } from './icons'
  type Props = {
    icon: IconName
    label: string
    disabled?: boolean
    size?: 'toolbar' | 'small'
    onclick?: () => void
  }
  let { icon, label, disabled, size, onclick }: Props = $props()
</script>

<span data-stub="IconButton" data-icon={icon} data-label={label} data-disabled={disabled} data-size={size} data-click={!!onclick}></span>

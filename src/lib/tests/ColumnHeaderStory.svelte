<script lang="ts">
  import type { ComponentProps } from 'svelte'
  import ColumnHeader from '../components/molecules/ColumnHeader.svelte'

  let props: ComponentProps<typeof ColumnHeader> = $props()
</script>

<!-- @RF-06: resizable-widths.feature, Rendered table layout remains usable after resizing. -->
<div class="header-frame" data-story-target style:width="{props.resize?.width}px">
  <ColumnHeader {...props} />
</div>

<style>
  .header-frame {
    min-height: var(--row-height);
    background: var(--color-surface);
  }
</style>

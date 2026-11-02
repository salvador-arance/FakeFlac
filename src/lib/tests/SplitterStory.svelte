<script lang="ts">
  import type { ComponentProps } from 'svelte'
  import Splitter from '../components/atoms/Splitter.svelte'
  import { MAIN_MIN, SIDEBAR_DEFAULT } from '../tokens/layout'

  let props: ComponentProps<typeof Splitter> = $props()
</script>

<!-- @RF-06 @RF-10: resizable-widths.feature, Bounds and keyboard operation. -->
{#if props.variant === 'column'}
  <div class="column-frame" data-story-target style:width="{props.value}px">
    <Splitter {...props} />
  </div>
{:else}
  <div class="panel-frame">
    <div class="panel" data-story-target aria-hidden="true" style:width="{props.value ?? SIDEBAR_DEFAULT}px"></div>
    <Splitter {...props} />
    <div class="main" aria-hidden="true" style:min-width="{MAIN_MIN}px"></div>
  </div>
{/if}

<style>
  .panel-frame {
    display: flex;
    height: 200px;
  }

  .panel {
    flex-shrink: 0;
    background: var(--color-surface);
  }

  .main {
    flex: 1;
    background: var(--color-bg);
  }

  .column-frame {
    position: relative;
    height: var(--control-height);
    background: var(--color-surface-raised);
  }
</style>

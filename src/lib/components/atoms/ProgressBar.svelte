<script lang="ts">
  type Props = {
    value?: number
  }

  let { value }: Props = $props()

  let percentage = $derived(
    value !== undefined ? Math.round(value * 100) : undefined,
  )
  let widthPercent = $derived(
    value !== undefined ? `${Math.min(100, Math.max(0, value * 100))}%` : '100%',
  )
</script>

{#if value !== undefined}
  <div
    class="progress-bar determinate"
    role="progressbar"
    aria-valuenow={percentage}
    aria-valuemin={0}
    aria-valuemax={100}
  >
    <div class="fill" style:width={widthPercent}></div>
  </div>
{:else}
  <div class="progress-bar indeterminate" role="progressbar">
    <div class="fill-indeterminate"></div>
  </div>
{/if}

<style>
  .progress-bar {
    position: relative;
    width: 100%;
    height: 6px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 3px;
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.15s ease-out;
  }

  .fill-indeterminate {
    height: 100%;
    width: 40%;
    background: var(--color-accent);
    position: absolute;
    animation: indeterminate-anim 1.5s infinite ease-in-out;
  }

  @keyframes indeterminate-anim {
    0% {
      left: -40%;
    }
    50% {
      left: 100%;
    }
    100% {
      left: -40%;
    }
  }
</style>

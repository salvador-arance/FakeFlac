<script lang="ts">
  import { t } from '../../i18n'

  type Props = {
    hz: number | null
    nyquistHz: number
  }

  let { hz, nyquistHz }: Props = $props()

  let formatted = $derived.by(() => {
    if (hz === null || hz === undefined) return ''
    if (hz === nyquistHz) return t('cutoff.fullBand')
    return `${(hz / 1000).toFixed(1)} kHz`
  })
</script>

{#if formatted}
  <span class="cutoff-value" class:full-band={hz === nyquistHz}>
    {formatted}
  </span>
{/if}

<style>
  .cutoff-value {
    font-size: var(--font-size);
    white-space: nowrap;
  }

  .cutoff-value.full-band {
    color: var(--color-text-muted);
  }
</style>

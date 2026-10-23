<script lang="ts">
  import { t } from '../../i18n'
  import type { BitrateMode } from '../../model'

  type Props = {
    kbps: number | null
    mode?: BitrateMode | null
    approx?: boolean
  }

  // specs/ui/lossy-formats.md, "Molecule BitrateValue". States: value · vbr · abr · approx · none.
  let { kbps, mode = null, approx = false }: Props = $props()

  let text = $derived.by(() => {
    if (kbps === null || kbps === undefined) return t('bitrate.none')
    if (approx) return t('bitrate.approx', { kbps })
    const value = t('bitrate.kbps', { kbps })
    if (mode === 'vbr') return `${value} ${t('bitrate.vbr')}`
    if (mode === 'abr') return `${value} ${t('bitrate.abr')}`
    return value
  })
</script>

<!-- Plain text, no colour of its own: the verdict carries the colour (RNF-08). -->
{#if kbps === null || kbps === undefined}
  <span class="bitrate-value" role="img" aria-label={t('bitrate.noneLabel')}>{text}</span>
{:else}
  <span class="bitrate-value">{text}</span>
{/if}

<style>
  .bitrate-value {
    font-size: var(--font-size);
    white-space: nowrap;
  }
</style>

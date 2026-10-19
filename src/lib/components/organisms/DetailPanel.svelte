<script lang="ts">
  import { t } from '../../i18n'
  import type { Spectrogram, TrackAnalysis } from '../../model'
  import SpectrogramView from './SpectrogramView.svelte'

  type Props = {
    track: TrackAnalysis | null
    spectrogram: Spectrogram | null
    loading: boolean
  }

  let { track, spectrogram, loading }: Props = $props()

  let cutoffHz = $derived(track?.cutoff?.frequencyHz ?? null)

  /** "Cutoff 16.0 kHz · Fake" then " · <reason>" for each reason (owner decision, G1). */
  let summary = $derived.by(() => {
    if (!track || track.verdict === 'error') return ''
    const parts: string[] = []
    if (cutoffHz !== null) parts.push(t('detail.cutoff', { khz: (cutoffHz / 1000).toFixed(1) }))
    parts.push(t(`verdict.${track.verdict}`))
    for (const reason of track.reasons) parts.push(t(`reason.${reason}`))
    return parts.join(' · ')
  })
</script>

<section class="detail-panel">
  {#if track}
    {#if track.verdict === 'error' || track.error}
      <p class="error-text">
        {track.error ? t(`error.${track.error.kind}`) : t('verdict.error')}
      </p>
    {:else}
      <p class="summary">{summary}</p>
      <div class="spectrogram">
        <SpectrogramView {spectrogram} {cutoffHz} {loading} />
      </div>
    {/if}
  {/if}
</section>

<style>
  .detail-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: var(--space-2);
    gap: var(--space-1);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--font-size);
  }

  .summary,
  .error-text {
    margin: 0;
    flex-shrink: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .error-text {
    color: var(--color-text-muted);
  }

  .spectrogram {
    flex: 1;
    min-height: 0;
  }
</style>

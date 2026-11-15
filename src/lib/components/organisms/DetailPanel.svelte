<script lang="ts">
  import { formatEvidence } from '../../format-evidence'
  import { t } from '../../i18n'
  import type { AlbumSummary, Spectrogram, TrackAnalysis } from '../../model'
  import SpectrogramView from './SpectrogramView.svelte'

  type Props = {
    track: TrackAnalysis | null
    album?: AlbumSummary | null
    spectrogram: Spectrogram | null
    loading: boolean
    /** The spectrogram request failed for this track (e.g. the file was moved after the analysis). */
    unavailable?: boolean
  }

  let { track, album, spectrogram, loading, unavailable = false }: Props = $props()

  let cutoffHz = $derived(track?.cutoff?.frequencyHz ?? null)
  let evidence = $derived(track?.verdict === 'fake' ? track.evidence : null)
  let albumNote = $derived.by(() => {
    if (!album || !evidence?.length) return null
    const { real, suspicious, fake } = album
    const tracks = real + suspicious + fake
    if (tracks < 2 || fake === 0 || fake >= tracks) return null
    return fake === 1 ? t('evidence.albumOnly', { tracks }) : t('evidence.albumSome', { fake, tracks })
  })

  /**
   * "Cutoff 16.0 kHz · Fake" then " · <reason>" for each reason (owner decision, G1).
   * A lossy track adds, after the cutoff, " · {declared} kbps" (with " VBR" / " ABR") and
   * " · ≈{estimate} kbps source" when the estimate is lower (specs/ui/lossy-formats.md, Detail line).
   */
  let summary = $derived.by(() => {
    if (!track || track.verdict === 'error') return ''
    const parts: string[] = []
    if (cutoffHz !== null) parts.push(t('detail.cutoff', { khz: (cutoffHz / 1000).toFixed(1) }))
    const declared = track.info.declaredBitrateKbps
    if (declared !== null) {
      const mode = track.info.bitrateMode
      const suffix =
        mode === 'vbr' ? ` ${t('bitrate.vbr')}` : mode === 'abr' ? ` ${t('bitrate.abr')}` : ''
      parts.push(t('detail.declared', { kbps: declared }) + suffix)
      const estimate = track.estimatedSourceBitrateKbps
      if (estimate !== null && estimate < declared) {
        parts.push(t('detail.source', { kbps: estimate }))
      }
    }
    parts.push(t(`verdict.${track.verdict}`))
    for (const reason of track.reasons) parts.push(t(`reason.${reason}`))
    return parts.join(' · ')
  })
</script>

<section class={['detail-panel', { 'with-evidence': !!evidence?.length }]}>
  {#if track}
    {#if track.verdict === 'error' || track.error}
      <p class="error-text">
        {track.error ? t(`error.${track.error.kind}`) : t('verdict.error')}
      </p>
    {:else}
      <p class="summary">{summary}</p>
      {#if evidence?.length}
        {#each evidence as item (item.kind)}
          <p class="evidence">{formatEvidence(item)}</p>
        {/each}
        {#if evidence.length === 1 && evidence[0].kind === 'gapStep'}
          <p class="evidence note">{t('evidence.gapOnly')}</p>
        {/if}
        {#if albumNote}
          <p class="evidence note">{albumNote}</p>
        {/if}
        <p class="evidence note">{t('evidence.audioNote')}</p>
      {/if}
      {#if unavailable}
        <p class="error-text">{t('spectrogram.unavailable')}</p>
      {:else}
        <div class="spectrogram">
          <SpectrogramView {spectrogram} {cutoffHz} {loading} />
        </div>
      {/if}
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
    overflow-y: auto;
    padding: var(--space-2);
    gap: var(--space-1);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: var(--font-size);
  }

  .with-evidence {
    padding-block: var(--space-1);
    gap: calc(var(--space-1) / 2);
    line-height: calc(var(--font-size) + var(--space-1) / 2);
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

  .evidence {
    margin: 0;
    flex-shrink: 0;
    overflow-wrap: anywhere;
  }

  .note {
    color: var(--color-text-muted);
  }

  .spectrogram {
    flex: 1;
    min-height: 0;
  }
</style>

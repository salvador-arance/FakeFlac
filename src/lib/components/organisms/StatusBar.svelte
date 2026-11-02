<script lang="ts">
  import ProgressBar from '../atoms/ProgressBar.svelte'
  import { t } from '../../i18n'
  import type { AnalysisPhase, CacheFigures, StatusSummary } from '../../model'

  type Props = {
    phase: AnalysisPhase
    summary: StatusSummary
    message?: string
    notice?: string
    /** RF-08 (sort-filter.md section 9): the number of shown rows. */
    shown?: number
    /** RF-11 (result-cache.md section 3): the cache figures of the latest ended run. */
    cache?: CacheFigures
  }

  let { phase, summary, message, notice, shown, cache }: Props = $props()

  /** translations.md section 5: the singular key exactly at 1, the general key otherwise. */
  function count(key: string, n: number): string {
    return n === 1 ? t(`${key}One`) : t(key, { n })
  }

  let statusText = $derived.by(() => {
    switch (phase) {
      case 'ready':
        return t('status.ready')
      case 'scanning':
        return t('status.scanning')
      case 'analysing':
        return [
          `${summary.done}/${summary.total}`,
          count('status.suspicious', summary.suspicious),
          count('status.fake', summary.fake),
        ].join(' · ')
      case 'finished':
        return [
          count('status.tracks', summary.total),
          count('status.real', summary.real),
          count('status.suspicious', summary.suspicious),
          count('status.fake', summary.fake),
          count('status.errors', summary.errors),
        ].join(' · ')
      case 'cancelled':
        return t('status.cancelled', { done: summary.done, total: summary.total })
      case 'error':
        return message ?? ''
      default:
        return ''
    }
  })

  // RF-11 (result-cache.md section 3): only once a run has ended finished or cancelled
  let cacheText = $derived.by(() => {
    if (!cache || (phase !== 'finished' && phase !== 'cancelled')) return ''
    let text = cache.fromCache > 0 ? ` · ${t('status.fromCache', { n: cache.fromCache })}` : ''
    if (cache.status === 'degraded') text += ` · ${t('status.cacheDegraded')}`
    else if (cache.status === 'unavailable') text += ` · ${t('status.cacheUnavailable')}`
    return text
  })

  // RF-08 (sort-filter.md section 9): only while the table has run rows to count
  let shownCount = $derived(
    shown !== undefined && (phase === 'analysing' || phase === 'finished' || phase === 'cancelled')
      ? ` · ${count('status.shown', shown)}`
      : '',
  )

  let shownText = $derived(
    notice && phase !== 'scanning' && phase !== 'analysing'
      ? `${statusText}${cacheText}${shownCount} · ${notice}`
      : `${statusText}${cacheText}${shownCount}`,
  )

  let progressValue = $derived.by(() => {
    if (phase !== 'analysing') return undefined
    return summary.total > 0 ? summary.done / summary.total : 0
  })
</script>

<footer class="status-bar" role="status">
  {#if phase === 'scanning' || phase === 'analysing'}
    <div class="progress-container">
      <ProgressBar value={progressValue} />
    </div>
  {/if}
  <span class="status-text">{shownText}</span>
</footer>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 24px;
    padding: 0 var(--space-2);
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
    font-size: var(--font-size);
    color: var(--color-text-muted);
    box-sizing: border-box;
    user-select: none;
  }

  .progress-container {
    width: 120px;
    flex-shrink: 0;
  }

  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

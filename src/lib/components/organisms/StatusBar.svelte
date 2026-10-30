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

  let { phase, summary, message, notice, shown }: Props = $props()

  let statusText = $derived.by(() => {
    switch (phase) {
      case 'ready':
        return t('status.ready')
      case 'scanning':
        return t('status.scanning')
      case 'analysing':
        return `${summary.done}/${summary.total} · ${summary.suspicious} suspicious · ${summary.fake} fake`
      case 'finished':
        return `${summary.total} tracks · ${summary.real} real · ${summary.suspicious} suspicious · ${summary.fake} fake · ${summary.errors} errors`
      case 'cancelled':
        return `Cancelled · ${summary.done} of ${summary.total}`
      case 'error':
        return message ?? ''
      default:
        return ''
    }
  })

  // RF-08 (sort-filter.md section 9): only while the table has run rows to count
  let shownCount = $derived(
    shown !== undefined && (phase === 'analysing' || phase === 'finished' || phase === 'cancelled')
      ? ` · ${t('status.shown', { n: shown })}`
      : '',
  )

  let shownText = $derived(
    notice && phase !== 'scanning' && phase !== 'analysing'
      ? `${statusText}${shownCount} · ${notice}`
      : `${statusText}${shownCount}`,
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

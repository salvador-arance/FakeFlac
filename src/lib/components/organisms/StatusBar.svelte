<script lang="ts">
  import type { StatusPhase, StatusSummary } from '../../model'
  type Props = { phase: StatusPhase; summary: StatusSummary; message?: string }
  let { phase, summary, message }: Props = $props()
</script>

<span data-stub="StatusBar" data-phase={phase} data-total={summary.total} data-message={message}></span>

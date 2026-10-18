<script lang="ts">
  import type { SuspicionReason, Verdict } from '../../model'
  type Props = { verdict: Verdict; reasons?: SuspicionReason[] }
  let { verdict, reasons }: Props = $props()
</script>

<span data-stub="VerdictCell" data-verdict={verdict} data-reasons={reasons?.length}></span>

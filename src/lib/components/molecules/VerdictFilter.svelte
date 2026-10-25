<script lang="ts">
  // VerdictFilter (specs/ui/sort-filter.md section 10): four verdict toggle buttons in a group.
  // RF-08 compile-only stub (stage `test`): props declared, nothing rendered yet.
  import type { Verdict } from '../../model'

  type Props = {
    selected: Verdict[]
    disabled?: boolean
    ontoggle?: (verdict: Verdict) => void
  }

  let _props: Props = $props()
</script>

<div class="verdict-filter"></div>

<script lang="ts">
  import type { BitrateMode } from '../../model'

  type Props = {
    kbps: number | null
    mode?: BitrateMode | null
    approx?: boolean
  }

  // Compile-only stub (test stage, RF-15). The implementer replaces it.
  let { kbps, mode = null, approx = false }: Props = $props()
  void kbps
  void mode
  void approx
</script>

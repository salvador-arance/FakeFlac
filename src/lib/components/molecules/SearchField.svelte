<script lang="ts">
  // SearchField (specs/ui/sort-filter.md section 10): the name search of the toolbar.
  // RF-08 compile-only stub (stage `test`): props declared, nothing rendered yet.
  type Props = {
    value: string
    disabled?: boolean
    shortcut?: boolean
    focusRequest?: number
    oninput?: (value: string) => void
  }

  let _props: Props = $props()
</script>

<div class="search-field"></div>

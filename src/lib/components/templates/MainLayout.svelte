<script lang="ts">
  import type { Snippet } from 'svelte'
  type Props = {
    titleBar: Snippet
    toolbar: Snippet
    table: Snippet
    /** Optional: the detail area arrives with RF-07. */
    detail?: Snippet
    statusBar: Snippet
  }
  let { titleBar, toolbar, table, detail, statusBar }: Props = $props()
</script>

<span data-stub="MainLayout" data-a={!!titleBar} data-b={!!toolbar} data-c={!!table} data-d={!!detail} data-e={!!statusBar}></span>

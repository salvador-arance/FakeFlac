<script lang="ts">
  type Props = { label: string; sort?: 'asc' | 'desc' | null; onsort?: () => void }
  let { label, sort, onsort }: Props = $props()
</script>

<span data-stub="ColumnHeader" data-label={label} data-sort={sort} data-click={!!onsort}></span>

<script lang="ts">
  type Props = { active: boolean; empty: boolean }
  let { active, empty }: Props = $props()
</script>

<span data-stub="FolderDropZone" data-active={active} data-empty={empty}></span>

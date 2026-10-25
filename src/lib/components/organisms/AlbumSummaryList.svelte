<script lang="ts">
  import type { AlbumUpdate } from '../../model'

  type Props = {
    albums: AlbumUpdate[]
    currentIndex: number | null
    onselect?: (albumIndex: number) => void
  }

  // RF-10 stub: renders nothing yet.
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  let _props: Props = $props()
</script>

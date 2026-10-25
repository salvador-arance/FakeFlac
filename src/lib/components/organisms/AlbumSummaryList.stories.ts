import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { makeAlbum } from '../../testing/fixtures'
import AlbumSummaryList from './AlbumSummaryList.svelte'

const meta = {
  component: AlbumSummaryList,
  title: 'Organisms/AlbumSummaryList',
  args: { albums: [], currentIndex: null },
} satisfies Meta<typeof AlbumSummaryList>

export default meta
type Story = StoryObj<typeof meta>

export const NoAlbums: Story = {}

export const OneAlbum: Story = {
  args: { albums: [makeAlbum({ index: 0, name: 'Album A', real: 12 })] },
}

export const ManyAlbums: Story = {
  args: {
    albums: Array.from({ length: 40 }, (_, i) =>
      makeAlbum({
        index: i,
        name: `Artist\Album ${i + 1}`,
        real: 10,
        fake: i % 7 === 0 ? 1 : 0,
        suspicious: i % 5 === 0 ? 2 : 0,
      }),
    ),
  },
}

export const PartialAlbum: Story = {
  args: { albums: [makeAlbum({ index: 0, name: 'Album A', real: 3, fake: 1, files: 12 })] },
}

export const ErrorOnlyAlbum: Story = {
  args: { albums: [makeAlbum({ index: 0, name: 'Lib\Broken', real: 0, errors: 2 })] },
}

export const CurrentAlbum: Story = {
  args: {
    albums: [
      makeAlbum({ index: 0, name: 'Artist', real: 1 }),
      makeAlbum({ index: 1, name: 'Artist\Album 1', real: 11, fake: 1 }),
      makeAlbum({ index: 2, name: 'Artist\Album 2', real: 7, suspicious: 2 }),
    ],
    currentIndex: 1,
  },
}

export const LongName: Story = {
  args: {
    albums: [
      makeAlbum({
        index: 0,
        name: 'Artist\A very long album name that does not fit in the panel (Remastered 2024 Deluxe Edition)',
        real: 12,
      }),
    ],
  },
}

export const UnicodeName: Story = {
  args: {
    albums: [
      makeAlbum({ index: 0, name: 'Álbum ″Ñandú″', real: 3 }),
      makeAlbum({ index: 1, name: 'Álbum ″Ñandú″\Disco 2 «Ça va» 🎵', real: 1, suspicious: 1 }),
    ],
  },
}

// focus-visible: tab into the list in the browser to see the ring.
export const FocusVisible: Story = {
  args: { albums: [makeAlbum({ index: 0, name: 'Album A', real: 3 })], currentIndex: 0 },
}

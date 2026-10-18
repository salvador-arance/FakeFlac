import type { Meta, StoryObj } from '@storybook/svelte-vite'
import FolderDropZone from './FolderDropZone.svelte'

const meta = {
  component: FolderDropZone,
  title: 'Organisms/FolderDropZone',
} satisfies Meta<typeof FolderDropZone>

export default meta
type Story = StoryObj<typeof meta>

export const Hint: Story = {
  args: {
    active: false,
    empty: true,
  },
}

export const DragOver: Story = {
  args: {
    active: true,
    empty: true,
  },
}

import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Icon from './Icon.svelte'

const meta = {
  component: Icon,
  title: 'Atoms/Icon',
} satisfies Meta<typeof Icon>

export default meta
type Story = StoryObj<typeof meta>

export const FolderOpen: Story = {
  args: {
    name: 'folder-open',
    size: 24,
  },
}

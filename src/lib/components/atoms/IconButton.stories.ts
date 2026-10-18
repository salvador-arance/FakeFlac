import type { Meta, StoryObj } from '@storybook/svelte-vite'
import IconButton from './IconButton.svelte'

const meta = {
  component: IconButton,
  title: 'Atoms/IconButton',
} satisfies Meta<typeof IconButton>

export default meta
type Story = StoryObj<typeof meta>

export const Toolbar: Story = {
  args: {
    icon: 'folder-open',
    label: 'Open folder',
    size: 'toolbar',
  },
}

export const Small: Story = {
  args: {
    icon: 'close',
    label: 'Close',
    size: 'small',
  },
}

export const Disabled: Story = {
  args: {
    icon: 'cancel',
    label: 'Cancel',
    disabled: true,
  },
}

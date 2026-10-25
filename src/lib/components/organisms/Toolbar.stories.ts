import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Toolbar from './Toolbar.svelte'

const meta = {
  component: Toolbar,
  title: 'Organisms/Toolbar',
} satisfies Meta<typeof Toolbar>

export default meta
type Story = StoryObj<typeof meta>

export const Idle: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: false,
  },
}

export const IdleExportEnabled: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: true,
  },
}

export const Running: Story = {
  args: {
    canOpen: false,
    canCancel: true,
    canExport: false,
  },
}

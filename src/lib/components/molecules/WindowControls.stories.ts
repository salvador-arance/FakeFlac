import type { Meta, StoryObj } from '@storybook/svelte-vite'
import WindowControls from './WindowControls.svelte'

const meta = {
  component: WindowControls,
  title: 'Molecules/WindowControls',
} satisfies Meta<typeof WindowControls>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {}

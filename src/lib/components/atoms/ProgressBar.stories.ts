import type { Meta, StoryObj } from '@storybook/svelte-vite'
import ProgressBar from './ProgressBar.svelte'

const meta = {
  component: ProgressBar,
  title: 'Atoms/ProgressBar',
} satisfies Meta<typeof ProgressBar>

export default meta
type Story = StoryObj<typeof meta>

export const Determinate: Story = {
  args: {
    value: 0.65,
  },
}

export const Indeterminate: Story = {
  args: {
    value: undefined,
  },
}

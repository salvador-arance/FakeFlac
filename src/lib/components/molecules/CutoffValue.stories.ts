import type { Meta, StoryObj } from '@storybook/svelte-vite'
import CutoffValue from './CutoffValue.svelte'

const meta = {
  component: CutoffValue,
  title: 'Molecules/CutoffValue',
} satisfies Meta<typeof CutoffValue>

export default meta
type Story = StoryObj<typeof meta>

export const Value: Story = {
  args: {
    hz: 16000,
    nyquistHz: 22050,
  },
}

export const FullBand: Story = {
  args: {
    hz: 22050,
    nyquistHz: 22050,
  },
}

export const None: Story = {
  args: {
    hz: null,
    nyquistHz: 22050,
  },
}

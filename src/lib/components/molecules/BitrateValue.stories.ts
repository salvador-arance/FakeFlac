import type { Meta, StoryObj } from '@storybook/svelte-vite'
import BitrateValue from './BitrateValue.svelte'

const meta = {
  component: BitrateValue,
  title: 'Molecules/BitrateValue',
} satisfies Meta<typeof BitrateValue>

export default meta
type Story = StoryObj<typeof meta>

export const Value: Story = { args: { kbps: 320, mode: 'cbr' } }
export const Vbr: Story = { args: { kbps: 245, mode: 'vbr' } }
export const Abr: Story = { args: { kbps: 192, mode: 'abr' } }
export const Approx: Story = { args: { kbps: 128, approx: true } }
export const None: Story = { args: { kbps: null } }

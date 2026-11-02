import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Splitter from './Splitter.svelte'

const meta = {
  component: Splitter,
  title: 'Atoms/Splitter',
} satisfies Meta<typeof Splitter>

export default meta
type Story = StoryObj<typeof meta>

export const Horizontal: Story = {
  args: {
    orientation: 'horizontal',
  },
}

// @RF-06 @RF-10: resizable-widths.md, vertical panel/column and both bounds.
const verticalArgs = (extra: Record<string, unknown> = {}) => ({
  orientation: 'vertical', label: 'Resize Albums panel',
  description: 'Drag or use Left/Right to resize. Double-click or press Enter to reset.',
  value: 240, min: 160, max: 480, valueText: '240 pixels', variant: 'panel', ...extra,
}) as unknown as Story['args']

export const VerticalPanel: Story = { args: verticalArgs() }
export const VerticalColumn: Story = { args: verticalArgs({ variant: 'column', label: 'Resize File column', value: 230, min: 100, max: 800, valueText: '230 pixels' }) }
export const VerticalMinimum: Story = { args: verticalArgs({ value: 160, valueText: '160 pixels' }) }
export const VerticalMaximum: Story = { args: verticalArgs({ value: 480, valueText: '480 pixels' }) }

import type { Meta, StoryObj } from '@storybook/svelte-vite'
import type { ComponentProps } from 'svelte'
import Splitter from './Splitter.svelte'
import SplitterStory from '../../tests/SplitterStory.svelte'
import { COLUMN_MAX, COLUMN_MIN, SIDEBAR_DEFAULT, SIDEBAR_MAX, SIDEBAR_MIN } from '../../tokens/layout'

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

// @RF-06 @RF-10: resizable-widths.feature, Bounds and keyboard operation.
const verticalArgs = (extra: Partial<ComponentProps<typeof Splitter>> = {}): ComponentProps<typeof Splitter> => ({
  orientation: 'vertical', label: 'Resize Albums panel',
  description: 'Drag or use Left/Right to resize. Double-click or press Enter to reset.',
  value: SIDEBAR_DEFAULT, min: SIDEBAR_MIN, max: SIDEBAR_MAX, valueText: '240 pixels', variant: 'panel', ...extra,
})
const renderVertical: Story['render'] = (args) => ({ Component: SplitterStory, props: args })

export const VerticalPanel: Story = { render: renderVertical, args: verticalArgs() }
export const VerticalColumn: Story = { render: renderVertical, args: verticalArgs({ variant: 'column', label: 'Resize File column', value: 230, min: COLUMN_MIN.file, max: COLUMN_MAX, valueText: '230 pixels' }) }
export const VerticalMinimum: Story = { render: renderVertical, args: verticalArgs({ value: SIDEBAR_MIN, valueText: '160 pixels' }) }
export const VerticalMaximum: Story = { render: renderVertical, args: verticalArgs({ value: SIDEBAR_MAX, valueText: '480 pixels' }) }

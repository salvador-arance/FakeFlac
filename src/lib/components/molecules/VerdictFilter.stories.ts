import type { Meta, StoryObj } from '@storybook/svelte-vite'
import VerdictFilter from './VerdictFilter.svelte'

// RF-08 (sort-filter.md section 10): the verdict filter of the toolbar.
const meta = {
  component: VerdictFilter,
  title: 'Molecules/VerdictFilter',
} satisfies Meta<typeof VerdictFilter>

export default meta
type Story = StoryObj<typeof meta>

export const NonePressed: Story = {
  args: { selected: [] },
}

export const OnePressed: Story = {
  args: { selected: ['fake'] },
}

export const SeveralPressed: Story = {
  args: { selected: ['fake', 'suspicious'] },
}

export const AllPressed: Story = {
  args: { selected: ['fake', 'suspicious', 'real', 'error'] },
}

export const Disabled: Story = {
  args: { selected: [], disabled: true },
}

/** A filter kept into the Empty state (section 8). */
export const DisabledOnePressed: Story = {
  args: { selected: ['fake'], disabled: true },
}

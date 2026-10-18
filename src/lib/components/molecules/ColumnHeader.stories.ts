import type { Meta, StoryObj } from '@storybook/svelte-vite'
import ColumnHeader from './ColumnHeader.svelte'

const meta = {
  component: ColumnHeader,
  title: 'Molecules/ColumnHeader',
} satisfies Meta<typeof ColumnHeader>

export default meta
type Story = StoryObj<typeof meta>

export const Unsorted: Story = {
  args: {
    label: 'Duration',
  },
}

export const Ascending: Story = {
  args: {
    label: 'Duration',
    sort: 'asc',
  },
}

export const Descending: Story = {
  args: {
    label: 'Duration',
    sort: 'desc',
  },
}

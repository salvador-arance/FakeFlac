import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { makeManyRows } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

const meta = {
  component: ResultsTable,
  title: 'Organisms/ResultsTable',
} satisfies Meta<typeof ResultsTable>

export default meta
type Story = StoryObj<typeof meta>

export const Empty: Story = {
  args: {
    rows: [],
    selectedIndex: null,
    sort: { column: 'file', direction: 'asc' },
  },
}

export const WithRows: Story = {
  args: {
    rows: makeManyRows(10),
    selectedIndex: 0,
    sort: { column: 'file', direction: 'asc' },
  },
}

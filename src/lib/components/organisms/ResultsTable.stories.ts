import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { makeManyRows, makeTrack } from '../../testing/fixtures'
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

export const LossyRows: Story = {
  args: {
    rows: [
      {
        index: 0,
        analysis: makeTrack({
          path: 'C:/Music/Mixed/a.mp3',
          format: 'mp3',
          codec: 'mp3',
          verdict: 'fake',
          cutoffHz: 17000,
          declaredBitrateKbps: 320,
          bitrateMode: 'cbr',
          estimatedSourceBitrateKbps: 128,
        }),
      },
      {
        index: 1,
        analysis: makeTrack({
          path: 'C:/Music/Mixed/b.m4a',
          format: 'm4a',
          codec: 'alac',
          cutoffHz: 22050,
        }),
      },
      {
        index: 2,
        analysis: makeTrack({
          path: 'C:/Music/Mixed/v.mp3',
          format: 'mp3',
          codec: 'mp3',
          cutoffHz: 19500,
          declaredBitrateKbps: 240,
          bitrateMode: 'vbr',
          estimatedSourceBitrateKbps: 240,
        }),
      },
    ],
    selectedIndex: null,
    sort: { column: 'file', direction: 'asc' },
  },
}

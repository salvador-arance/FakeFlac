import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { makeManyRows, makeRow, makeTrack } from '../../testing/fixtures'
import type { ResultRow, Verdict } from '../../model'
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
        albumIndex: 0,
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
        albumIndex: 0,
        analysis: makeTrack({
          path: 'C:/Music/Mixed/b.m4a',
          format: 'm4a',
          codec: 'alac',
          cutoffHz: 22050,
        }),
      },
      {
        index: 2,
        albumIndex: 0,
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

/** RF-10 (Q16): `revealRequest` changes with the selection unchanged; the row is scrolled back into view. */
export const RevealRequest: Story = {
  args: {
    rows: makeManyRows(60),
    selectedIndex: 0,
    sort: { column: 'file', direction: 'asc' },
    revealRequest: 1,
  },
}

// RF-08 (sort-filter.md sections 3, 5, 8 and 10): sort rules, no matches, hidden selection.

type StoryRowOptions = Partial<{ cutoffHz: number | null; bitDepth: number | null; folder: string }>

/** A row with the path `C:\Music\Lib\<name>`; its folder is the path's folder unless given. */
function storyRow(index: number, name: string, verdict: Verdict, o: StoryRowOptions = {}): ResultRow {
  const path = `C:\\Music\\Lib\\${name}`
  const row = makeRow(index, {
    path,
    verdict,
    cutoffHz: o.cutoffHz ?? null,
    bitDepth: o.bitDepth === undefined ? 16 : o.bitDepth,
  })
  row.analysis.info.folder = o.folder ?? path.slice(0, path.lastIndexOf('\\'))
  return row
}

const everyVerdict: ResultRow[] = [
  storyRow(0, 'A\\a.flac', 'real', { cutoffHz: 22050 }),
  storyRow(1, 'A\\b.flac', 'error', { bitDepth: null }),
  storyRow(2, 'A\\c.flac', 'fake', { cutoffHz: 16000 }),
  storyRow(3, 'A\\d.flac', 'suspicious', { cutoffHz: 19500 }),
]

/** A filter hides every row: the header stays, the body shows the message and Clear filters. */
export const NoMatches: Story = {
  args: {
    rows: [],
    selectedIndex: null,
    sort: { column: 'file', direction: 'asc' },
    noMatches: true,
  },
}

/** The selected row (index 0) is hidden by a filter: nothing is highlighted; ↓ selects the first row. */
export const SelectedRowNotInRows: Story = {
  args: {
    rows: everyVerdict.slice(1),
    selectedIndex: 0,
    sort: { column: 'file', direction: 'asc' },
  },
}

export const StateSortedAscending: Story = {
  args: {
    rows: everyVerdict,
    selectedIndex: null,
    sort: { column: 'state', direction: 'asc' },
  },
}

export const StateSortedDescending: Story = {
  args: {
    rows: everyVerdict,
    selectedIndex: null,
    sort: { column: 'state', direction: 'desc' },
  },
}

/** Cutoff descending: the error row without a cutoff stays last (also ascending). */
export const EmptyCellsLast: Story = {
  args: {
    rows: everyVerdict,
    selectedIndex: null,
    sort: { column: 'cutoff', direction: 'desc' },
  },
}

/** Folder ascending with an empty `TrackInfo.folder`: it sorts as the folder its cell shows. */
export const FolderSortWithEmptyFolder: Story = {
  args: {
    rows: [
      storyRow(0, 'B\\01.flac', 'real', { folder: '' }),
      storyRow(1, 'A\\01.flac', 'real'),
      storyRow(2, 'C\\01.flac', 'real'),
    ],
    selectedIndex: null,
    sort: { column: 'folder', direction: 'asc' },
  },
}

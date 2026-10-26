// ResultsTable for a track flagged by rule L6a (specs/features/detection-streaming-source.feature,
// @RF-16 @RF-06; specs/ui/lossy-formats.md, "Streaming sample rate (RF-16)"). The analysis is
// stubbed (no audio). New text: reason.streamingSampleRate.
import { render, screen, within } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import type { ResultRow, SortState } from '../../model'
import { makeTrack } from '../../testing/fixtures'
import ResultsTable from './ResultsTable.svelte'

const STREAMING_TEXT =
  'Full band at 48 kHz: possibly a streaming rip; genuine 48 kHz files look the same.'

const byPath: SortState = { column: 'file', direction: 'asc' }

function dataRows(): HTMLElement[] {
  return screen
    .queryAllByRole('row')
    .filter((row) => within(row).queryAllByRole('gridcell').length > 0)
}
const cells = (row: HTMLElement) => within(row).getAllByRole('gridcell')

const flagged: ResultRow = {
  index: 0,
  albumIndex: 0,
  analysis: makeTrack({
    path: 'C:/A/a.mp3',
    format: 'mp3',
    codec: 'mp3',
    sampleRate: 48000,
    bitDepth: null,
    declaredBitrateKbps: 320,
    bitrateMode: 'cbr',
    estimatedSourceBitrateKbps: null,
    cutoffHz: 20100,
    verdict: 'suspicious',
    reasons: ['streamingSampleRate'],
  }),
}

function setup() {
  render(ResultsTable, {
    props: {
      rows: [flagged],
      selectedIndex: null,
      sort: byPath,
      onselect: vi.fn(),
      onsort: vi.fn(),
    },
  })
}

describe('ResultsTable: a track flagged by its streaming sample rate', () => {
  // @RF-16 @RF-06 Scenario: Table cells and tooltip of a flagged track
  it('shows its cells and "—" in its real bitrate cell', () => {
    setup()
    const c = cells(dataRows()[0])
    expect(c[1].textContent?.trim()).toBe('a.mp3')
    expect(c[0].textContent).toContain('Suspicious')
    expect(c[4].textContent?.trim()).toBe('48000')
    expect(c[7].textContent?.trim()).toBe('20.1 kHz')
    expect(c[8].textContent?.trim()).toBe('320 kbps')
    expect(c[9].textContent?.trim()).toBe('—')
  })

  // @RF-16 @RF-06 "hovering its state cell shows a tooltip that contains ..."
  it('shows the reason text in the tooltip of its state cell', () => {
    setup()
    const state = cells(dataRows()[0])[0]
    const titled = state.matches('[title]') ? state : state.querySelector('[title]')
    expect(titled?.getAttribute('title') ?? '').toContain(STREAMING_TEXT)
  })
})

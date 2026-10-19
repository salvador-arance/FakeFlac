import type { Meta, StoryObj } from '@storybook/svelte-vite'
import DetailPanel from './DetailPanel.svelte'
import { makeLowPassSpectrogram, makeTrack } from '../../testing/fixtures'

const meta = {
  component: DetailPanel,
  title: 'Organisms/DetailPanel',
} satisfies Meta<typeof DetailPanel>

export default meta
type Story = StoryObj<typeof meta>

export const NothingSelected: Story = {
  args: { track: null, spectrogram: null, loading: false },
}

export const Loading: Story = {
  args: {
    track: makeTrack({ path: 'C:/Music/Album A/lp16k.flac', verdict: 'fake', cutoffHz: 16000 }),
    spectrogram: null,
    loading: true,
  },
}

export const Fake: Story = {
  args: {
    track: makeTrack({ path: 'C:/Music/Album A/lp16k.flac', verdict: 'fake', cutoffHz: 16000 }),
    spectrogram: makeLowPassSpectrogram(16000),
    loading: false,
  },
}

export const SuspiciousWithReason: Story = {
  args: {
    track: makeTrack({ verdict: 'suspicious', cutoffHz: 17000, reasons: ['highBandGaps'] }),
    spectrogram: makeLowPassSpectrogram(17000),
    loading: false,
  },
}

export const ErrorTrack: Story = {
  args: {
    track: makeTrack({
      path: 'C:/Music/Album A/corrupt.flac',
      verdict: 'error',
      error: { kind: 'corruptStream', detail: 'bad frame header' },
    }),
    spectrogram: null,
    loading: false,
  },
}

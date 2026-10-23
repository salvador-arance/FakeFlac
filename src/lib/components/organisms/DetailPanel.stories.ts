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

export const LossyFake: Story = {
  args: {
    track: makeTrack({
      path: 'C:/Music/Album A/a.mp3',
      format: 'mp3',
      codec: 'mp3',
      verdict: 'fake',
      cutoffHz: 17000,
      declaredBitrateKbps: 320,
      bitrateMode: 'cbr',
      estimatedSourceBitrateKbps: 128,
    }),
    spectrogram: makeLowPassSpectrogram(17000),
    loading: false,
  },
}

export const LossyVbr: Story = {
  args: {
    track: makeTrack({
      path: 'C:/Music/Album A/v.mp3',
      format: 'mp3',
      codec: 'mp3',
      cutoffHz: 19500,
      declaredBitrateKbps: 240,
      bitrateMode: 'vbr',
      estimatedSourceBitrateKbps: 240,
    }),
    spectrogram: makeLowPassSpectrogram(19500),
    loading: false,
  },
}

export const UnsupportedFormat: Story = {
  args: {
    track: makeTrack({
      path: 'C:/Music/Album A/he-aac.m4a',
      format: 'm4a',
      codec: 'aac',
      verdict: 'error',
      error: { kind: 'unsupportedFormat', detail: 'HE-AAC is not supported' },
    }),
    spectrogram: null,
    loading: false,
  },
}

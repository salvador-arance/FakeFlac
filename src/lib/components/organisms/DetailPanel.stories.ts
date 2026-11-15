import type { Meta, StoryObj } from '@storybook/svelte-vite'
import DetailPanel from './DetailPanel.svelte'
import { makeAlbum, makeLowPassSpectrogram, makeTrack } from '../../testing/fixtures'
import { setLocale } from '../../i18n'

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

// @RF-17 Evidence, album context, loading/unavailable state, and translated text.
export const FakeGapOnly: Story = {
  args: {
    track: makeTrack({
      verdict: 'fake',
      cutoffHz: 22_000,
      evidence: [{ kind: 'gapStep', value: 9.6, threshold: 6, margin: 3.6 }],
    }),
    spectrogram: makeLowPassSpectrogram(22_000),
    loading: false,
  },
}

export const FakeStepOnly: Story = {
  args: {
    track: makeTrack({
      verdict: 'fake',
      cutoffHz: 16_000,
      evidence: [{ kind: 'cutoffStep', value: 13.2, threshold: 10, margin: 3.2 }],
    }),
    spectrogram: makeLowPassSpectrogram(16_000),
    loading: false,
  },
}

export const FakeBoth: Story = {
  args: {
    track: makeTrack({
      verdict: 'fake',
      cutoffHz: 16_000,
      evidence: [
        { kind: 'gapStep', value: 12, threshold: 6, margin: 6 },
        { kind: 'cutoffStep', value: 24.7, threshold: 10, margin: 14.7 },
      ],
    }),
    spectrogram: makeLowPassSpectrogram(16_000),
    loading: false,
  },
}

export const FakeLossyEvidence: Story = {
  args: {
    ...LossyFake.args,
    track: makeTrack({
      path: 'C:/Music/Album A/a.mp3',
      format: 'mp3',
      codec: 'mp3',
      verdict: 'fake',
      cutoffHz: 17_000,
      declaredBitrateKbps: 320,
      bitrateMode: 'cbr',
      estimatedSourceBitrateKbps: 128,
      evidence: [
        { kind: 'shortfall', value: 3500, threshold: 2500, margin: 1000 },
        { kind: 'cutoffStep', value: 40, threshold: 10, margin: 30 },
      ],
    }),
  },
}

export const OnlyFakeInAlbum: Story = {
  args: { ...FakeGapOnly.args, album: makeAlbum({ real: 13, fake: 1 }).summary },
}
export const SomeFakeInAlbum: Story = {
  args: { ...FakeBoth.args, album: makeAlbum({ real: 9, suspicious: 2, fake: 3 }).summary },
}
export const AllFakeInAlbum: Story = {
  args: { ...FakeBoth.args, album: makeAlbum({ real: 0, fake: 12 }).summary },
}
export const OnlyTrackInAlbum: Story = {
  args: { ...FakeStepOnly.args, album: makeAlbum({ real: 0, fake: 1 }).summary },
}
export const EvidenceUnavailableSpectrogram: Story = {
  args: { ...OnlyFakeInAlbum.args, spectrogram: null, unavailable: true },
}
export const EvidenceLoadingSpectrogram: Story = {
  args: { ...OnlyFakeInAlbum.args, spectrogram: null, loading: true },
}
export const EvidenceInSpanish: Story = {
  args: OnlyFakeInAlbum.args,
  beforeEach: () => {
    setLocale('es')
    return () => setLocale('en')
  },
}

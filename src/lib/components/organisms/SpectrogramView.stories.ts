import type { Meta, StoryObj } from '@storybook/svelte-vite'
import SpectrogramView from './SpectrogramView.svelte'
import { makeLowPassSpectrogram } from '../../testing/fixtures'

const meta = {
  component: SpectrogramView,
  title: 'Organisms/SpectrogramView',
} satisfies Meta<typeof SpectrogramView>

export default meta
type Story = StoryObj<typeof meta>

export const LowPass16k: Story = {
  args: { spectrogram: makeLowPassSpectrogram(16000), cutoffHz: 16000, loading: false },
}

export const FullBand: Story = {
  args: { spectrogram: makeLowPassSpectrogram(22050), cutoffHz: null, loading: false },
}

export const Loading: Story = {
  args: { spectrogram: null, cutoffHz: null, loading: true },
}

export const None: Story = {
  args: { spectrogram: null, cutoffHz: null, loading: false },
}

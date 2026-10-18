import type { Meta, StoryObj } from '@storybook/svelte-vite'
import VerdictCell from './VerdictCell.svelte'

const meta = {
  component: VerdictCell,
  title: 'Molecules/VerdictCell',
} satisfies Meta<typeof VerdictCell>

export default meta
type Story = StoryObj<typeof meta>

export const Real: Story = {
  args: {
    verdict: 'real',
  },
}

export const Suspicious: Story = {
  args: {
    verdict: 'suspicious',
    reasons: ['gradualRolloff', 'highBandGaps'],
  },
}

export const Fake: Story = {
  args: {
    verdict: 'fake',
    reasons: ['lowHighFrequencyContent'],
  },
}

export const ErrorState: Story = {
  args: {
    verdict: 'error',
  },
}

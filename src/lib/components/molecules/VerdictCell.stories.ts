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

// @RF-17 Fake rows with the three documented evidence lists.
export const FakeGapEvidence: Story = {
  args: {
    verdict: 'fake',
    evidence: [{ kind: 'gapStep', value: 9.6, threshold: 6, margin: 3.6 }],
  },
}

export const FakeBothEvidence: Story = {
  args: {
    verdict: 'fake',
    evidence: [
      { kind: 'gapStep', value: 12, threshold: 6, margin: 6 },
      { kind: 'cutoffStep', value: 24.7, threshold: 10, margin: 14.7 },
    ],
  },
}

export const FakeLossyEvidence: Story = {
  args: {
    verdict: 'fake',
    evidence: [
      { kind: 'shortfall', value: 3500, threshold: 2500, margin: 1000 },
      { kind: 'cutoffStep', value: 40, threshold: 10, margin: 30 },
    ],
  },
}

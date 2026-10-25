import type { Meta, StoryObj } from '@storybook/svelte-vite'
import StatusBar from './StatusBar.svelte'

const meta = {
  component: StatusBar,
  title: 'Organisms/StatusBar',
} satisfies Meta<typeof StatusBar>

export default meta
type Story = StoryObj<typeof meta>

export const Ready: Story = {
  args: {
    phase: 'ready',
    summary: { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 },
  },
}

export const Scanning: Story = {
  args: {
    phase: 'scanning',
    summary: { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 },
  },
}

export const Analysing: Story = {
  args: {
    phase: 'analysing',
    summary: { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 },
  },
}

export const Finished: Story = {
  args: {
    phase: 'finished',
    summary: { total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 },
  },
}

export const Cancelled: Story = {
  args: {
    phase: 'cancelled',
    summary: { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 },
  },
}

export const ErrorState: Story = {
  args: {
    phase: 'error',
    summary: { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 },
    message: 'No supported audio files in this folder',
  },
}

export const FinishedWithNotice: Story = {
  args: {
    phase: 'finished',
    summary: { total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 },
    notice: 'Saved 31 rows to FakeFlac - Album A.csv',
  },
}

export const CancelledWithNotice: Story = {
  args: {
    phase: 'cancelled',
    summary: { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 },
    notice: 'Saved 18 rows to FakeFlac - Album A.csv',
  },
}

export const ErrorWithNotice: Story = {
  args: {
    phase: 'error',
    summary: { total: 0, done: 0, real: 0, suspicious: 0, fake: 0, errors: 0 },
    message: 'The analysis stopped because of an unexpected error',
    notice: 'Cannot save the CSV file',
  },
}

// RF-08 (sort-filter.md sections 9 and 10): the shown count.

export const FinishedWithShown: Story = {
  args: {
    phase: 'finished',
    summary: { total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 },
    shown: 3,
  },
}

export const AnalysingWithShown: Story = {
  args: {
    phase: 'analysing',
    summary: { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 },
    shown: 1,
  },
}

export const CancelledWithShown: Story = {
  args: {
    phase: 'cancelled',
    summary: { total: 10, done: 3, real: 2, suspicious: 0, fake: 1, errors: 0 },
    shown: 1,
  },
}

/** The error message alone: the shown count is not appended in the error phase. */
export const ErrorWithShown: Story = {
  args: {
    phase: 'error',
    summary: { total: 2, done: 2, real: 1, suspicious: 0, fake: 0, errors: 1 },
    message: 'The analysis stopped because of an unexpected error',
    shown: 1,
  },
}

export const WithShownAndNotice: Story = {
  args: {
    phase: 'finished',
    summary: { total: 31, done: 31, real: 30, suspicious: 0, fake: 1, errors: 0 },
    shown: 1,
    notice: 'Saved 31 rows to results.csv',
  },
}

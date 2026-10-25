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

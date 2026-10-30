// RF-11 (specs/ui/result-cache.md sections 3 and 4, `StatusBar` row): the cache figures. A file of
// its own so that `StatusBar.stories.ts` stays as it was (result-cache.md 12.2).
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import StatusBar from './StatusBar.svelte'

const meta = {
  component: StatusBar,
  title: 'Organisms/StatusBar (result cache)',
} satisfies Meta<typeof StatusBar>

export default meta
type Story = StoryObj<typeof meta>

const FINISHED = { total: 31, done: 31, real: 28, suspicious: 2, fake: 1, errors: 0 }
const RUNNING = { total: 31, done: 18, real: 15, suspicious: 2, fake: 1, errors: 0 }

export const FinishedFromCache: Story = {
  args: { phase: 'finished', summary: FINISHED, cache: { fromCache: 31, status: 'ok' } },
}

export const FinishedUnavailable: Story = {
  args: { phase: 'finished', summary: FINISHED, cache: { fromCache: 0, status: 'unavailable' } },
}

export const FinishedDegradedFromCache: Story = {
  args: { phase: 'finished', summary: FINISHED, cache: { fromCache: 10, status: 'degraded' } },
}

export const CancelledFromCache: Story = {
  args: { phase: 'cancelled', summary: RUNNING, cache: { fromCache: 12, status: 'ok' } },
}

/** `ok` with 0 from cache: nothing is appended. */
export const FinishedOkNoneFromCache: Story = {
  args: { phase: 'finished', summary: FINISHED, cache: { fromCache: 0, status: 'ok' } },
}

/** Order: cache figures, the warning, the shown count, the notice. */
export const FinishedWithCacheShownAndNotice: Story = {
  args: {
    phase: 'finished',
    summary: FINISHED,
    cache: { fromCache: 10, status: 'degraded' },
    shown: 3,
    notice: 'Saved 31 rows to FakeFlac - Album A.csv',
  },
}

/** While analysing the cache figures are not shown. */
export const AnalysingWithCache: Story = {
  args: { phase: 'analysing', summary: RUNNING, cache: { fromCache: 31, status: 'degraded' } },
}

/** In the error phase only the error text is shown. */
export const ErrorWithCache: Story = {
  args: {
    phase: 'error',
    summary: RUNNING,
    message: 'The analysis stopped because of an unexpected error',
    cache: { fromCache: 31, status: 'ok' },
  },
}

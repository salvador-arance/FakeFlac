// RF-13 (specs/ui/language-settings.md sections 2 and 4, `Toolbar` row): the Settings gear after
// the verdict filters. A file of its own so that `Toolbar.stories.ts` stays as it was.
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Toolbar from './Toolbar.svelte'

const meta = {
  component: Toolbar,
  title: 'Organisms/Toolbar (settings)',
} satisfies Meta<typeof Toolbar>

export default meta
type Story = StoryObj<typeof meta>

export const IdleSettingsEnabled: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: false,
    canClearCache: true,
    canFilter: false,
    canSettings: true,
    verdictFilter: [],
    query: '',
  },
}

/** While an analysis runs, Settings stays available. */
export const RunningSettingsEnabled: Story = {
  args: {
    canOpen: false,
    canCancel: true,
    canExport: false,
    canClearCache: false,
    canFilter: true,
    canSettings: true,
    verdictFilter: [],
    query: '',
  },
}

/** Before the settings read completes, every action is disabled. */
export const StartupDisabled: Story = {
  args: {
    canOpen: false,
    canCancel: false,
    canExport: false,
    canClearCache: false,
    canFilter: false,
    canSettings: false,
    verdictFilter: [],
    query: '',
  },
}

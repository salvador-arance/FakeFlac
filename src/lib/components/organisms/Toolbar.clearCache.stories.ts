// RF-11 (specs/ui/result-cache.md sections 2 and 4, `Toolbar` row): "Clear results cache". A file
// of its own so that `Toolbar.stories.ts` stays as it was (result-cache.md 12.2).
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Toolbar from './Toolbar.svelte'

const meta = {
  component: Toolbar,
  title: 'Organisms/Toolbar (result cache)',
} satisfies Meta<typeof Toolbar>

export default meta
type Story = StoryObj<typeof meta>

export const IdleClearEnabled: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: true,
    canClearCache: true,
    canFilter: true,
    canSettings: false,
    verdictFilter: [],
    query: '',
  },
}

/** While `clearCache()` is pending: Clear and Open folder disabled. */
export const Clearing: Story = {
  args: {
    canOpen: false,
    canCancel: false,
    canExport: true,
    canClearCache: false,
    canFilter: true,
    canSettings: false,
    verdictFilter: [],
    query: '',
  },
}

export const RunningClearDisabled: Story = {
  args: {
    canOpen: false,
    canCancel: true,
    canExport: false,
    canClearCache: false,
    canFilter: true,
    canSettings: false,
    verdictFilter: [],
    query: '',
  },
}

/** Every button enabled: the tab order of result-cache.md section 6. */
export const EverythingEnabled: Story = {
  args: {
    canOpen: true,
    canCancel: true,
    canExport: true,
    canClearCache: true,
    canFilter: true,
    canSettings: false,
    verdictFilter: [],
    query: '',
  },
}

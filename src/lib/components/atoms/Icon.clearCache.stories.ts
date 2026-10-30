// RF-11 (specs/ui/result-cache.md section 4, `Icon` row): the `clear-cache` icon. A file of its own
// so that `Icon.stories.ts` stays as it was (result-cache.md 12.1 lists the only edits).
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Icon from './Icon.svelte'

const meta = {
  component: Icon,
  title: 'Atoms/Icon (clear-cache)',
} satisfies Meta<typeof Icon>

export default meta
type Story = StoryObj<typeof meta>

export const ClearCache: Story = {
  args: {
    name: 'clear-cache',
    size: 24,
  },
}

export const ClearCacheLabelled: Story = {
  args: {
    name: 'clear-cache',
    size: 24,
    label: 'Clear results cache',
  },
}

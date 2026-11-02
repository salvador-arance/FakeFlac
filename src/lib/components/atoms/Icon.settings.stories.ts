// RF-13 (specs/ui/language-settings.md section 4, `Icon` row): the `settings` gear. A file of its
// own so that `Icon.stories.ts` stays as it was.
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Icon from './Icon.svelte'

const meta = {
  component: Icon,
  title: 'Atoms/Icon (settings)',
} satisfies Meta<typeof Icon>

export default meta
type Story = StoryObj<typeof meta>

export const Settings: Story = {
  args: {
    name: 'settings',
    size: 24,
  },
}

export const SettingsLabelled: Story = {
  args: {
    name: 'settings',
    size: 24,
    label: 'Settings',
  },
}

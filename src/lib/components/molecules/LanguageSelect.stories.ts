// RF-13 (specs/ui/language-settings.md section 4, `LanguageSelect` row): English or Spanish
// selected, enabled or disabled.
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import LanguageSelect from './LanguageSelect.svelte'

const meta = {
  component: LanguageSelect,
  title: 'Molecules/LanguageSelect',
} satisfies Meta<typeof LanguageSelect>

export default meta
type Story = StoryObj<typeof meta>

export const English: Story = {
  args: { value: 'en' },
}

export const Spanish: Story = {
  args: { value: 'es' },
}

/** While a save is in progress. */
export const Disabled: Story = {
  args: { value: 'es', disabled: true },
}

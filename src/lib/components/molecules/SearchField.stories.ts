import type { Meta, StoryObj } from '@storybook/svelte-vite'
import SearchField from './SearchField.svelte'

// RF-08 (sort-filter.md section 10): the name search of the toolbar.
const meta = {
  component: SearchField,
  title: 'Molecules/SearchField',
} satisfies Meta<typeof SearchField>

export default meta
type Story = StoryObj<typeof meta>

export const Empty: Story = {
  args: { value: '' },
}

/** With text: the clear button is shown. */
export const WithText: Story = {
  args: { value: 'intro' },
}

export const Focused: Story = {
  args: { value: 'intro', shortcut: true },
  play: async ({ canvasElement }) => {
    canvasElement.querySelector('input')?.focus()
  },
}

export const Disabled: Story = {
  args: { value: '', disabled: true, shortcut: true },
}

/** A query kept into the Empty state: shown, disabled, without the clear button. */
export const DisabledWithText: Story = {
  args: { value: '01', disabled: true, shortcut: true },
}

/** RNF-09: the text exactly as typed (leading and trailing spaces, accents, primes, emoji). */
export const UnicodeText: Story = {
  args: { value: '  \u00c1lbum \u2033\u00d1and\u00fa\u2033 \u00ab\u00c7a va\u00bb \u{1F3B5} ' },
}

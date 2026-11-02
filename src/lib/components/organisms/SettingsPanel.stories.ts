// RF-13 (specs/ui/language-settings.md sections 2 and 4, `SettingsPanel` row): normal, warning,
// saving, save error and newer/read-only states of the language modal.
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import SettingsPanel from './SettingsPanel.svelte'

const meta = {
  component: SettingsPanel,
  title: 'Organisms/SettingsPanel',
  args: { onsave: () => {}, onclose: () => {} },
} satisfies Meta<typeof SettingsPanel>

export default meta
type Story = StoryObj<typeof meta>

export const Normal: Story = {
  args: { language: 'en', loadStatus: 'loaded', saving: false, error: null },
}

export const SpanishActive: Story = {
  args: { language: 'es', loadStatus: 'loaded', saving: false, error: null },
}

/** First launch without a settings file: silent. */
export const DefaultSilent: Story = {
  args: { language: 'es', loadStatus: 'default', saving: false, error: null },
}

export const InvalidWarning: Story = {
  args: { language: 'en', loadStatus: 'invalid', saving: false, error: null },
}

export const UnavailableWarning: Story = {
  args: { language: 'en', loadStatus: 'unavailable', saving: false, error: null },
}

/** Select and Save disabled; the dismissal button is Close. */
export const NewerVersion: Story = {
  args: { language: 'en', loadStatus: 'newerVersion', saving: false, error: null },
}

export const Saving: Story = {
  args: { language: 'en', loadStatus: 'loaded', saving: true, error: null },
}

export const WriteFailed: Story = {
  args: { language: 'en', loadStatus: 'loaded', saving: false, error: 'writeFailed' },
}

/** A newer file was found by the pre-save check: Save disabled, Close usable. */
export const ReadOnly: Story = {
  args: { language: 'en', loadStatus: 'loaded', saving: false, error: 'readOnly' },
}

export const UnexpectedFailure: Story = {
  args: { language: 'en', loadStatus: 'loaded', saving: false, error: 'internal' },
}

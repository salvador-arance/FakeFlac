// Storybook for the atomic components (S6, ADR-0014, ADR-0037): one story per state.
import type { StorybookConfig } from '@storybook/svelte-vite'

const config: StorybookConfig = {
  stories: ['../src/**/*.stories.@(ts|svelte)'],
  framework: { name: '@storybook/svelte-vite', options: {} },
  core: { disableTelemetry: true },
}

export default config

// @RF-10: layout states from specs/ui/resizable-widths.md, before implementation.
import type { Meta, StoryObj } from '@storybook/svelte-vite'
import { createRawSnippet } from 'svelte'
import MainLayout from './MainLayout.svelte'

const snippet = (text: string) => createRawSnippet(() => ({ render: () => `<div>${text}</div>` }))
const meta = { component: MainLayout, title: 'Templates/MainLayout widths' } satisfies Meta<typeof MainLayout>
export default meta
type Story = StoryObj<typeof meta>
const args = (width: number) => ({ titleBar: snippet('FakeFlac'), toolbar: snippet('Toolbar'), sidebar: snippet('Albums (1)'), table: snippet('Results table'), detail: snippet('Spectrogram'), statusBar: snippet('Ready'), sidebarWidth: width }) as unknown as Story['args']
export const Default: Story = { args: args(240) }
export const Minimum: Story = { args: args(160) }
export const Maximum: Story = { args: args(480) }
export const HiddenSidebar: Story = { args: { ...args(320), sidebar: undefined } }
export const SettingsInert: Story = { args: { ...args(320), settings: snippet('Settings modal') } }

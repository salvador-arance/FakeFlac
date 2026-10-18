import type { Meta, StoryObj } from '@storybook/svelte-vite'
import TitleBar from './TitleBar.svelte'

const meta = {
  component: TitleBar,
  title: 'Organisms/TitleBar',
} satisfies Meta<typeof TitleBar>

export default meta
type Story = StoryObj<typeof meta>

export const Default: Story = {
  args: {
    title: 'FakeFlac',
  },
}

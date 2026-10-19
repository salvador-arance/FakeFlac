import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Splitter from './Splitter.svelte'

const meta = {
  component: Splitter,
  title: 'Atoms/Splitter',
} satisfies Meta<typeof Splitter>

export default meta
type Story = StoryObj<typeof meta>

export const Horizontal: Story = {
  args: {
    orientation: 'horizontal',
  },
}

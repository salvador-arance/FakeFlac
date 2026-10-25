import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Icon from './Icon.svelte'

const meta = {
  component: Icon,
  title: 'Atoms/Icon',
} satisfies Meta<typeof Icon>

export default meta
type Story = StoryObj<typeof meta>

export const FolderOpen: Story = {
  args: {
    name: 'folder-open',
    size: 24,
  },
}

export const Export: Story = {
  args: {
    name: 'export',
    size: 24,
  },
}

export const ExportLabelled: Story = {
  args: {
    name: 'export',
    size: 24,
    label: 'Export CSV',
  },
}

/** RF-08 (sort-filter.md section 10): the search magnifier, decorative. */
export const Search: Story = {
  args: {
    name: 'search',
    size: 16,
  },
}

/** RF-08: the search magnifier, labelled. */
export const SearchLabelled: Story = {
  args: {
    name: 'search',
    size: 24,
    label: 'Search by name',
  },
}

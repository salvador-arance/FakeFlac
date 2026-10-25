import type { Meta, StoryObj } from '@storybook/svelte-vite'
import Toolbar from './Toolbar.svelte'

const meta = {
  component: Toolbar,
  title: 'Organisms/Toolbar',
} satisfies Meta<typeof Toolbar>

export default meta
type Story = StoryObj<typeof meta>

export const Idle: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: false,
    canFilter: false,
    verdictFilter: [],
    query: '',
  },
}

export const IdleExportEnabled: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: true,
    canFilter: true,
    verdictFilter: [],
    query: '',
  },
}

export const Running: Story = {
  args: {
    canOpen: false,
    canCancel: true,
    canExport: false,
    canFilter: true,
    verdictFilter: [],
    query: '',
  },
}

// RF-08 (sort-filter.md section 10): the verdict filter and the search field.

export const IdleWithFilters: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: true,
    canFilter: true,
    verdictFilter: [],
    query: '',
  },
}

export const FiltersPressedAndQuery: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: true,
    canFilter: true,
    verdictFilter: ['fake', 'suspicious'],
    query: 'intro',
  },
}

/** Empty state (app start): the filters are disabled. */
export const EmptyFiltersDisabled: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: false,
    canFilter: false,
    verdictFilter: [],
    query: '',
  },
}

/** Empty state with a kept filter: disabled, still pressed and with its text. */
export const EmptyWithKeptFilter: Story = {
  args: {
    canOpen: true,
    canCancel: false,
    canExport: false,
    canFilter: false,
    verdictFilter: ['fake'],
    query: '01',
  },
}

export const RunningWithFilters: Story = {
  args: {
    canOpen: false,
    canCancel: true,
    canExport: false,
    canFilter: true,
    verdictFilter: ['fake'],
    query: 'album',
  },
}

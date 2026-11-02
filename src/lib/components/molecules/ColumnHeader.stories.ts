import type { Meta, StoryObj } from '@storybook/svelte-vite'
import ColumnHeader from './ColumnHeader.svelte'

const meta = {
  component: ColumnHeader,
  title: 'Molecules/ColumnHeader',
} satisfies Meta<typeof ColumnHeader>

export default meta
type Story = StoryObj<typeof meta>

export const Unsorted: Story = {
  args: {
    label: 'Duration',
  },
}

export const Ascending: Story = {
  args: {
    label: 'Duration',
    sort: 'asc',
  },
}

export const Descending: Story = {
  args: {
    label: 'Duration',
    sort: 'desc',
  },
}

export const WithTooltip: Story = {
  args: {
    label: 'Real bitrate',
    tooltip: 'The real bitrate is an estimate from the cutoff.',
  },
}

// @RF-06 resize variants: resize controls remain separate from the sortable button.
const resize = { width: 100, min: 100, max: 800, label: 'Resize File column', description: 'Drag or use Left/Right to resize. Double-click or press Enter to reset.', valueText: '100 pixels' }
export const Resizable: Story = { args: { label: 'File', sort: 'asc', resize } as unknown as Story['args'] }
export const ResizableSpanish: Story = { args: { label: 'Frecuencia de muestreo', sort: 'desc', resize: { ...resize, width: 80, min: 80, label: 'Cambiar ancho de la columna Frecuencia de muestreo', valueText: '80 píxeles' } } as unknown as Story['args'] }
export const ResizableWithHelp: Story = { args: { label: 'Real bitrate', tooltip: 'The real bitrate is an estimate from the cutoff.', resize: { ...resize, label: 'Resize Real bitrate column' } } as unknown as Story['args'] }
export const ResizableMaximum: Story = { args: { label: 'Folder', resize: { ...resize, width: 800, label: 'Resize Folder column', valueText: '800 pixels' } } as unknown as Story['args'] }

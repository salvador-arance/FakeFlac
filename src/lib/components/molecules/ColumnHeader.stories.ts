import type { Meta, StoryObj } from '@storybook/svelte-vite'
import ColumnHeader from './ColumnHeader.svelte'
import ColumnHeaderStory from '../../tests/ColumnHeaderStory.svelte'
import { COLUMN_MAX, COLUMN_MIN } from '../../tokens/layout'

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

// @RF-06: resizable-widths.feature, Rendered table layout remains usable after resizing.
const resize = { width: COLUMN_MIN.file, min: COLUMN_MIN.file, max: COLUMN_MAX, label: 'Resize File column', description: 'Drag or use Left/Right to resize. Double-click or press Enter to reset.', valueText: '100 pixels' }
const renderResizable: Story['render'] = (args) => ({ Component: ColumnHeaderStory, props: args })
export const Resizable: Story = { render: renderResizable, args: { label: 'File', sort: 'asc', resize } }
export const ResizableSpanish: Story = { render: renderResizable, args: { label: 'Frecuencia de muestreo', sort: 'desc', resize: { ...resize, width: COLUMN_MIN.sampleRate, min: COLUMN_MIN.sampleRate, label: 'Cambiar ancho de la columna Frecuencia de muestreo', description: 'Arrastra o usa Izquierda/Derecha para cambiar el ancho. Haz doble clic o pulsa Intro para restablecerlo.', valueText: '80 píxeles' } } }
export const ResizableWithHelp: Story = { render: renderResizable, args: { label: 'Real bitrate', tooltip: 'The real bitrate is an estimate from the cutoff.', resize: { ...resize, label: 'Resize Real bitrate column' } } }
export const ResizableMaximum: Story = { render: renderResizable, args: { label: 'Folder', resize: { ...resize, width: COLUMN_MAX, label: 'Resize Folder column', valueText: '800 pixels' } } }

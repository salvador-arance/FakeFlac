import type { Meta, StoryObj } from '@storybook/svelte-vite'
import IconButton from './IconButton.svelte'

const meta = {
  component: IconButton,
  title: 'Atoms/IconButton',
} satisfies Meta<typeof IconButton>

export default meta
type Story = StoryObj<typeof meta>

export const Toolbar: Story = {
  args: {
    icon: 'folder-open',
    label: 'Open folder',
    size: 'toolbar',
  },
}

export const Small: Story = {
  args: {
    icon: 'close',
    label: 'Close',
    size: 'small',
  },
}

export const Disabled: Story = {
  args: {
    icon: 'cancel',
    label: 'Cancel',
    disabled: true,
  },
}

// RF-08 (sort-filter.md section 10): toggle (`pressed`) and verdict tone (`tone`).

export const ToggleNotPressed: Story = {
  args: {
    icon: 'verdict-fake',
    label: 'Show fake tracks',
    pressed: false,
  },
}

export const TogglePressed: Story = {
  args: {
    icon: 'verdict-fake',
    label: 'Show fake tracks',
    pressed: true,
  },
}

/** Hover the button: the pressed look (accent border) stays, unlike a released button's hover. */
export const TogglePressedHover: Story = {
  args: {
    icon: 'verdict-fake',
    label: 'Show fake tracks',
    pressed: true,
    tone: 'fake',
  },
}

/** Focused from the keyboard: the focus ring over the pressed look. */
export const TogglePressedFocusVisible: Story = {
  args: {
    icon: 'verdict-fake',
    label: 'Show fake tracks',
    pressed: true,
    tone: 'fake',
  },
  play: async ({ canvasElement }) => {
    canvasElement.querySelector('button')?.focus()
  },
}

export const ToggleDisabled: Story = {
  args: {
    icon: 'verdict-fake',
    label: 'Show fake tracks',
    pressed: false,
    disabled: true,
  },
}

/** A kept filter in the Empty state: the pressed look under the disabled opacity. */
export const ToggleDisabledPressed: Story = {
  args: {
    icon: 'verdict-fake',
    label: 'Show fake tracks',
    pressed: true,
    disabled: true,
    tone: 'fake',
  },
}

export const ToneFake: Story = {
  args: { icon: 'verdict-fake', label: 'Show fake tracks', pressed: false, tone: 'fake' },
}

export const ToneFakePressed: Story = {
  args: { icon: 'verdict-fake', label: 'Show fake tracks', pressed: true, tone: 'fake' },
}

export const ToneSuspicious: Story = {
  args: { icon: 'verdict-suspicious', label: 'Show suspicious tracks', pressed: false, tone: 'suspicious' },
}

export const ToneSuspiciousPressed: Story = {
  args: { icon: 'verdict-suspicious', label: 'Show suspicious tracks', pressed: true, tone: 'suspicious' },
}

export const ToneReal: Story = {
  args: { icon: 'verdict-real', label: 'Show genuine tracks', pressed: false, tone: 'real' },
}

export const ToneRealPressed: Story = {
  args: { icon: 'verdict-real', label: 'Show genuine tracks', pressed: true, tone: 'real' },
}

export const ToneError: Story = {
  args: { icon: 'verdict-error', label: 'Show tracks with errors', pressed: false, tone: 'error' },
}

export const ToneErrorPressed: Story = {
  args: { icon: 'verdict-error', label: 'Show tracks with errors', pressed: true, tone: 'error' },
}

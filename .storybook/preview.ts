// Stories render on the app's dark tokens, like the real window.
import '../src/lib/tokens/tokens.css'
import type { Preview } from '@storybook/svelte-vite'

const preview: Preview = {
  parameters: { layout: 'padded' },
}

export default preview

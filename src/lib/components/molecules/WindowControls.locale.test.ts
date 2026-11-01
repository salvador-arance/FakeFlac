// WindowControls names (@RF-13; specs/ui/translations.md section 1: `window.minimize`,
// `window.maximize`, `window.close`). The maximize button keeps its toggle action and name; only
// the name is translated. Acceptance L1 audits window buttons as app-owned literals.
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import WindowControls from './WindowControls.svelte'

afterEach(() => setLocale('en'))

describe('WindowControls: translated names (@RF-13)', () => {
  it('Spanish: Minimizar, Maximizar, Cerrar, with the same actions', async () => {
    setLocale('es')
    const onminimize = vi.fn()
    const ontoggleMaximize = vi.fn()
    const onclose = vi.fn()
    render(WindowControls, { props: { onminimize, ontoggleMaximize, onclose } })
    for (const name of ['Minimizar', 'Maximizar', 'Cerrar']) {
      expect(screen.getByRole('button', { name }).getAttribute('title')).toBe(name)
    }
    await userEvent.click(screen.getByRole('button', { name: 'Minimizar' }))
    await userEvent.click(screen.getByRole('button', { name: 'Maximizar' }))
    await userEvent.click(screen.getByRole('button', { name: 'Cerrar' }))
    expect(onminimize).toHaveBeenCalledTimes(1)
    expect(ontoggleMaximize).toHaveBeenCalledTimes(1)
    expect(onclose).toHaveBeenCalledTimes(1)
  })

  it('re-renders the names while mounted', () => {
    render(WindowControls)
    expect(screen.getByRole('button', { name: 'Minimize' })).toBeTruthy()
    setLocale('es')
    flushSync()
    expect(screen.getByRole('button', { name: 'Minimizar' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Maximizar' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Cerrar' })).toBeTruthy()
    setLocale('en')
    flushSync()
    expect(screen.getByRole('button', { name: 'Close' })).toBeTruthy()
  })
})

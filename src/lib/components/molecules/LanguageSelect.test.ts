// LanguageSelect (@RF-13 @RNF-08; specs/ui/language-settings.md section 4, `LanguageSelect` row):
// props `value: Language`, `disabled?: boolean`, event `onchange(language)`. A native select
// labelled `settings.language` with the autonym options English (`en`, lang="en") and Español
// (`es`, lang="es"). No persistence or store access.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The native select is found by its role `combobox` and its accessible name (the label).
// - `onchange` fires on the select's change with the chosen language; the component does not
//   need to keep a value of its own beyond the prop.
import { render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setLocale } from '../../i18n'
import LanguageSelect from './LanguageSelect.svelte'

afterEach(() => setLocale('en'))

const select = (name = 'Language') => screen.getByRole('combobox', { name }) as HTMLSelectElement

describe('LanguageSelect (@RF-13)', () => {
  it('is a native select labelled "Language" with the two autonym options', () => {
    render(LanguageSelect, { props: { value: 'en' } })
    const s = select()
    expect(s.tagName).toBe('SELECT')
    const options = within(s).getAllByRole('option') as HTMLOptionElement[]
    expect(options.map((o) => o.textContent?.trim())).toEqual(['English', 'Español'])
    expect(options.map((o) => o.value)).toEqual(['en', 'es'])
    expect(options.map((o) => o.getAttribute('lang'))).toEqual(['en', 'es'])
  })

  it.each(['en', 'es'] as const)('shows %s selected', (value) => {
    render(LanguageSelect, { props: { value } })
    expect(select().value).toBe(value)
  })

  it('emits the chosen language', async () => {
    const onchange = vi.fn()
    render(LanguageSelect, { props: { value: 'en', onchange } })
    await userEvent.selectOptions(select(), 'es')
    expect(onchange).toHaveBeenCalledTimes(1)
    expect(onchange).toHaveBeenCalledWith('es')
  })

  it('is enabled by default and focusable', () => {
    render(LanguageSelect, { props: { value: 'en' } })
    expect(select().disabled).toBe(false)
    select().focus()
    expect(document.activeElement).toBe(select())
  })

  it('disabled: no change can be made', async () => {
    const onchange = vi.fn()
    render(LanguageSelect, { props: { value: 'en', disabled: true, onchange } })
    expect(select().disabled).toBe(true)
    await userEvent.selectOptions(select(), 'es').catch(() => {})
    expect(onchange).not.toHaveBeenCalled()
  })

  it('translates its label but keeps the autonyms (translations.md section 1)', () => {
    setLocale('es')
    render(LanguageSelect, { props: { value: 'es' } })
    const options = within(select('Idioma')).getAllByRole('option')
    expect(options.map((o) => o.textContent?.trim())).toEqual(['English', 'Español'])
  })

  it('re-renders its label when the locale changes while mounted', () => {
    render(LanguageSelect, { props: { value: 'en' } })
    expect(select('Language')).toBeTruthy()
    setLocale('es')
    flushSync()
    expect(select('Idioma')).toBeTruthy()
  })
})

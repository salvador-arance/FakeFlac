// SearchField (specs/ui/sort-filter.md section 10, `SearchField` row; sections 4.2, 4.3 and 13;
// @RF-08 @RNF-09): the name search. A native text input with role searchbox, the decorative search
// icon, a clear button when it has text and is enabled, Escape, the Ctrl+F shortcut on the window
// and the `focusRequest` counter.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The field is controlled by `value`: tests re-render with the emitted value, as the page does.
// - "Its text is selected" = `selectionStart` 0 and `selectionEnd` the length of the text; "text not
//   selected" (focusRequest) = an empty selection.
// - The width token is checked in tokens.css (`--search-width: 240px`); the CSS that uses it is
//   not testable in jsdom.
import { fireEvent, render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import tokens from '../../tokens/tokens.css?raw'
import SearchField from './SearchField.svelte'

const box = () => screen.getByRole('searchbox', { name: 'Search by name' }) as HTMLInputElement
const clearButton = () => screen.queryByRole('button', { name: 'Clear search' }) as HTMLButtonElement | null

/** A field whose `value` follows its `input` events, as the page does. */
function controlled(props: Record<string, unknown> = {}) {
  const oninput = vi.fn()
  const view = render(SearchField, {
    props: {
      value: '',
      ...props,
      oninput: (value: string) => {
        oninput(value)
        void view.rerender({ value })
      },
    },
  })
  return { oninput, ...view }
}

/**
 * Ctrl+F (or another combination) on `target`; resolves to whether the default was prevented
 * (the fireEvent of @testing-library/svelte is async: it resolves to dispatchEvent's result).
 */
async function ctrlF(target: Element | Document | Window = document.body, init: KeyboardEventInit = {}) {
  return !(await fireEvent.keyDown(target, { key: 'f', code: 'KeyF', ctrlKey: true, ...init }))
}

describe('SearchField: markup (@RF-08)', () => {
  it('empty: a searchbox named "Search by name" with its placeholder, and no clear button', () => {
    const { container } = render(SearchField, { props: { value: '' } })
    expect(box().tagName).toBe('INPUT')
    expect(box().type).toBe('text')
    expect(box().value).toBe('')
    expect(box().placeholder).toBe('Search (Ctrl+F)')
    expect(box().getAttribute('spellcheck')).toBe('false')
    expect(box().getAttribute('autocomplete')).toBe('off')
    expect(box().getAttribute('aria-keyshortcuts')).toBe('Control+F')
    expect(clearButton()).toBeNull()
    const icons = Array.from(container.querySelectorAll('svg')).filter((s) => !s.closest('button'))
    expect(icons).toHaveLength(1)
    expect(icons[0].getAttribute('aria-hidden')).toBe('true')
  })

  it('with text: shows the text and a small "Clear search" button', () => {
    render(SearchField, { props: { value: 'intro' } })
    expect(box().value).toBe('intro')
    const clear = clearButton()
    expect(clear).not.toBeNull()
    expect(clear?.getAttribute('title')).toBe('Clear search')
    expect(clear?.querySelector('svg')?.getAttribute('width')).toBe('16')
  })

  it('disabled: the input is disabled', () => {
    render(SearchField, { props: { value: '', disabled: true } })
    expect(box().disabled).toBe(true)
    expect(clearButton()).toBeNull()
  })

  it('disabled with text: shows the text, without the clear button', () => {
    render(SearchField, { props: { value: '01', disabled: true } })
    expect(box().disabled).toBe(true)
    expect(box().value).toBe('01')
    expect(clearButton()).toBeNull()
  })

  // @RNF-09 the field keeps the text exactly (not trimmed or normalised)
  it('Unicode text: shows the value code point for code point', () => {
    const text = '  Álbum ″Ñandú″ \u{1F3B5} Ñ '
    render(SearchField, { props: { value: text } })
    expect(box().value).toBe(text)
  })

  it('follows the `value` prop', async () => {
    const view = render(SearchField, { props: { value: 'a' } })
    await view.rerender({ value: 'abc' })
    expect(box().value).toBe('abc')
    await view.rerender({ value: '' })
    expect(box().value).toBe('')
    expect(clearButton()).toBeNull()
  })

  it('the width token exists: --search-width: 240px', () => {
    expect(tokens).toMatch(/--search-width:\s*240px\s*;/)
  })
})

describe('SearchField: input, Escape and clear (@RF-08)', () => {
  it('emits input on every change of the text', async () => {
    const { oninput } = controlled()
    await userEvent.type(box(), 'ab')
    expect(oninput.mock.calls.map((c) => c[0])).toEqual(['a', 'ab'])
    expect(box().value).toBe('ab')
  })

  // @RNF-09 a paste emits the text as given
  it('emits a pasted text as given', async () => {
    const { oninput } = controlled()
    await userEvent.click(box())
    await userEvent.paste('  Álbum ″Ñandú″ ')
    expect(oninput).toHaveBeenLastCalledWith('  Álbum ″Ñandú″ ')
    expect(box().value).toBe('  Álbum ″Ñandú″ ')
  })

  it('Escape clears the text and keeps the focus', async () => {
    const { oninput } = controlled({ value: 'intro' })
    box().focus()
    await userEvent.keyboard('{Escape}')
    expect(oninput).toHaveBeenLastCalledWith('')
    expect(box().value).toBe('')
    expect(document.activeElement).toBe(box())
  })

  it('Escape with an empty field does nothing', async () => {
    const { oninput } = controlled()
    box().focus()
    await userEvent.keyboard('{Escape}')
    expect(oninput).not.toHaveBeenCalled()
    expect(document.activeElement).toBe(box())
  })

  it('the clear button empties the field, focuses it and goes away', async () => {
    const { oninput } = controlled({ value: 'intro' })
    await userEvent.click(clearButton() as HTMLButtonElement)
    expect(oninput).toHaveBeenLastCalledWith('')
    expect(box().value).toBe('')
    expect(document.activeElement).toBe(box())
    expect(clearButton()).toBeNull()
  })

  it('the clear button appears once there is text', async () => {
    controlled()
    expect(clearButton()).toBeNull()
    await userEvent.type(box(), 'x')
    expect(clearButton()).not.toBeNull()
  })
})

describe('SearchField: Ctrl+F (@RF-08)', () => {
  it('with `shortcut`, Ctrl+F anywhere focuses the field, selects its text and prevents the default', async () => {
    render(SearchField, { props: { value: 'intro', shortcut: true } })
    expect(document.activeElement).not.toBe(box())
    expect(await ctrlF(document.body)).toBe(true)
    expect(document.activeElement).toBe(box())
    expect(box().selectionStart).toBe(0)
    expect(box().selectionEnd).toBe(5)
  })

  it.each([
    ['key "F"', { key: 'F' }],
    ['code KeyF with another key value', { key: 'ф', code: 'KeyF' }],
  ])('also reacts to %s', async (_, init) => {
    render(SearchField, { props: { value: 'x', shortcut: true } })
    expect(await ctrlF(document.body, init)).toBe(true)
    expect(document.activeElement).toBe(box())
  })

  it.each([
    ['Ctrl+Shift+F', { shiftKey: true }],
    ['Ctrl+Alt+F', { altKey: true }],
    ['Ctrl+Meta+F', { metaKey: true }],
    ['F without Ctrl', { ctrlKey: false }],
    ['Ctrl+G', { key: 'g', code: 'KeyG' }],
  ])('ignores %s', async (_, init) => {
    render(SearchField, { props: { value: 'x', shortcut: true } })
    expect(await ctrlF(document.body, init)).toBe(false)
    expect(document.activeElement).not.toBe(box())
  })

  it('disabled: Ctrl+F is still prevented, but the focus does not move', async () => {
    render(SearchField, { props: { value: '01', shortcut: true, disabled: true } })
    const other = document.createElement('button')
    document.body.appendChild(other)
    other.focus()
    expect(await ctrlF(other)).toBe(true)
    expect(document.activeElement).toBe(other)
    other.remove()
  })

  it('without `shortcut`, Ctrl+F is not handled', async () => {
    render(SearchField, { props: { value: 'x' } })
    expect(await ctrlF(document.body)).toBe(false)
    expect(document.activeElement).not.toBe(box())
  })

  it('stops listening once unmounted', async () => {
    const view = render(SearchField, { props: { value: 'x', shortcut: true } })
    view.unmount()
    expect(await ctrlF(document.body)).toBe(false)
  })
})

describe('SearchField: focusRequest (@RF-08)', () => {
  it('the initial value does not focus the field', () => {
    render(SearchField, { props: { value: 'abc', focusRequest: 3 } })
    expect(document.activeElement).not.toBe(box())
  })

  it('each change focuses the field without selecting its text', async () => {
    const view = render(SearchField, { props: { value: 'abc', focusRequest: 0 } })
    await view.rerender({ focusRequest: 1 })
    expect(document.activeElement).toBe(box())
    expect(box().selectionStart).toBe(box().selectionEnd)
    box().blur()
    await view.rerender({ focusRequest: 2 })
    expect(document.activeElement).toBe(box())
  })

  it('does not focus a disabled field', async () => {
    const view = render(SearchField, { props: { value: '', focusRequest: 0, disabled: true } })
    await view.rerender({ focusRequest: 1 })
    expect(document.activeElement).not.toBe(box())
  })
})

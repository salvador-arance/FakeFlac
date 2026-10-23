// ColumnHeader (specs/ui/components.md, Molecules): label, sort state -> aria-sort, `sort` event.
// Reading chosen: the `sort` event carries no payload (the column is known by the parent).
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import ColumnHeader from './ColumnHeader.svelte'

describe('ColumnHeader', () => {
  it('is a columnheader named by its label', () => {
    render(ColumnHeader, { props: { label: 'Duration' } })
    expect(screen.getByRole('columnheader', { name: /Duration/ })).not.toBeNull()
  })

  // @RF-06 states: unsorted / asc / desc -> aria-sort
  it.each([
    [undefined, 'none'],
    [null, 'none'],
    ['asc', 'ascending'],
    ['desc', 'descending'],
  ] as const)('sort %s -> aria-sort %s', (sort, ariaSort) => {
    render(ColumnHeader, { props: { label: 'Duration', sort } })
    expect(screen.getByRole('columnheader').getAttribute('aria-sort')).toBe(ariaSort)
  })

  // @RF-06 "clicking a column header sorts by it"
  it('emits sort when clicked', async () => {
    const onsort = vi.fn()
    render(ColumnHeader, { props: { label: 'Duration', onsort } })
    await userEvent.click(screen.getByRole('button', { name: /Duration/ }))
    expect(onsort).toHaveBeenCalledTimes(1)
  })

  it('emits sort from the keyboard', async () => {
    const onsort = vi.fn()
    render(ColumnHeader, { props: { label: 'Duration', onsort } })
    const user = userEvent.setup()
    await user.tab()
    await user.keyboard('{Enter}')
    expect(onsort).toHaveBeenCalledTimes(1)
  })

  // @RF-06 @RF-15 The Real bitrate column explains the estimate: optional `tooltip` prop.
  // Reading: the tooltip is the `title` attribute of the columnheader element.
  it('shows the tooltip as the title of the header', () => {
    const tooltip = 'The real bitrate is an estimate from the cutoff.'
    render(ColumnHeader, { props: { label: 'Real bitrate', tooltip } })
    expect(screen.getByRole('columnheader').getAttribute('title')).toBe(tooltip)
  })

  // @RF-06 @RF-15 only the Real bitrate column sets a tooltip
  it('has no title without a tooltip', () => {
    render(ColumnHeader, { props: { label: 'Duration' } })
    expect(screen.getByRole('columnheader').hasAttribute('title')).toBe(false)
  })
})

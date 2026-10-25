// AlbumSummaryList (specs/ui/album-summary.md "What an item shows", "Interaction", "Accessibility";
// scenarios of features/album-summary-app.feature, interface layer; @RF-10).
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - The list is `role="listbox"` named "Album summaries"; items are `role="option"`. The header
//   ("Albums (3)" / "Albums") is text of the component, outside the options.
// - The name has the folder as `title`; the counts text has itself as `title`.
// - The listbox is the one tab stop and receives the keys; the options are not tab stops.
// - CSS (ellipsis, selected background, focus ring) is not testable in jsdom: manual check M2.
import { fireEvent, render, screen, within } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { AlbumUpdate } from '../../model'
import { makeAlbum } from '../../testing/fixtures'
import type { AlbumOptions } from '../../testing/fixtures'
import AlbumSummaryList from './AlbumSummaryList.svelte'

function setup(albums: AlbumUpdate[], currentIndex: number | null = null) {
  const onselect = vi.fn()
  const view = render(AlbumSummaryList, { props: { albums, currentIndex, onselect } })
  return { onselect, ...view }
}

const options = () => screen.queryAllByRole('option')
const listbox = () => screen.getByRole('listbox', { name: 'Album summaries' })
const album = (index: number, name: string, o: AlbumOptions = {}) =>
  makeAlbum({ index, name, ...o })

const THREE = [
  album(0, 'Artist', { real: 1 }),
  album(1, 'Artist\\Album 1', { real: 11, fake: 1 }),
  album(2, 'Artist\\Album 2', { real: 7, suspicious: 2 }),
]

describe('AlbumSummaryList: content', () => {
  // @RF-10 Scenario: Album summaries are shown after the analysis
  it('lists one item per album, in the order given, with verdict and counts', () => {
    setup(THREE)
    expect(screen.getByText('Albums (3)')).not.toBeNull()
    const items = options()
    expect(items).toHaveLength(3)
    const expected = [
      ['Artist', 'Genuine', '1 track · 1 real'],
      ['Artist\\Album 1', 'Fake', '12 tracks · 11 real · 1 fake'],
      ['Artist\\Album 2', 'Suspicious', '9 tracks · 7 real · 2 suspicious'],
    ]
    expected.forEach(([name, verdict, counts], i) => {
      const item = within(items[i])
      expect(item.getByText(name)).not.toBeNull()
      expect(item.getByText(verdict)).not.toBeNull()
      expect(item.getByText(counts)).not.toBeNull()
    })
  })

  // @RF-10 @RNF-08 "each verdict is shown with its icon, its text and its colour"
  it('shows every verdict with an icon and its text', () => {
    setup([
      album(0, 'A', { real: 1 }),
      album(1, 'B', { suspicious: 1, real: 0 }),
      album(2, 'C', { fake: 1, real: 0 }),
      album(3, 'D', { errors: 1, real: 0 }),
    ])
    const texts = ['Genuine', 'Suspicious', 'Fake', 'Error']
    options().forEach((item, i) => {
      expect(within(item).getByText(texts[i])).not.toBeNull()
      expect(item.querySelector('svg')).not.toBeNull()
    })
    const icons = options().map((o) => o.querySelector('svg')?.outerHTML ?? '')
    expect(new Set(icons).size).toBe(4)
  })

  // @RF-10 the header without albums
  it('reads "Albums" and has no items without albums', () => {
    setup([])
    expect(screen.getByText('Albums')).not.toBeNull()
    expect(screen.queryByText(/Albums \(/)).toBeNull()
    expect(options()).toHaveLength(0)
  })

  // @RF-10 Scenario: Summaries update during the analysis (partial albums)
  it('shows "{done} of {files} tracks" for a partial album', () => {
    setup([album(0, 'Album A', { real: 3, files: 12 })])
    expect(within(options()[0]).getByText('3 of 12 tracks · 3 real')).not.toBeNull()
  })

  it('shows the partial text with a fake track too', () => {
    setup([album(0, 'Album A', { real: 3, fake: 1, files: 12 })])
    const item = within(options()[0])
    expect(item.getByText('Fake')).not.toBeNull()
    expect(item.getByText('4 of 12 tracks · 3 real · 1 fake')).not.toBeNull()
  })

  it('shows the non-zero counts in the order real, suspicious, fake, errors', () => {
    setup([album(0, 'X', { real: 2, suspicious: 1, fake: 1, errors: 3 })])
    expect(within(options()[0]).getByText('7 tracks · 2 real · 1 suspicious · 1 fake · 3 errors')).not.toBeNull()
  })

  it('uses the singular for one track and one error', () => {
    setup([
      album(0, 'One', { real: 1 }),
      album(1, 'Bad', { real: 1, errors: 1 }),
    ])
    expect(within(options()[0]).getByText('1 track · 1 real')).not.toBeNull()
    expect(within(options()[1]).getByText('2 tracks · 1 real · 1 error')).not.toBeNull()
  })

  it('writes the numbers with plain digits', () => {
    setup([album(0, 'Big', { real: 1200 })])
    expect(within(options()[0]).getByText('1200 tracks · 1200 real')).not.toBeNull()
  })

  // @RF-10 @RNF-04 Scenario: A folder whose files all failed
  it('shows an error-only album with the error icon and its counts', () => {
    setup([album(0, 'Lib\\Broken', { real: 0, errors: 2 })])
    const item = options()[0]
    expect(within(item).getByText('Error')).not.toBeNull()
    expect(item.querySelector('svg')).not.toBeNull()
    expect(within(item).getByText('2 tracks · 2 errors')).not.toBeNull()
  })

  // @RF-10 Scenario: A long name is cut with an ellipsis and shown whole in the tooltip
  it('puts the name on one element whose tooltip is the full folder path', () => {
    const name = 'Artist\\A very long album name that does not fit in the panel (Remastered 2024 Deluxe Edition)'
    const folder = `C:\\Music\\${name}`
    setup([album(0, name, { folder })])
    const el = within(options()[0]).getByText(name)
    expect(el.getAttribute('title')).toBe(folder)
    expect(el.textContent).toBe(name)
  })

  // @RF-10 the counts text has itself as tooltip
  it('gives the counts text itself as tooltip', () => {
    setup([album(0, 'Artist\\Album 1', { real: 11, fake: 1 })])
    const counts = within(options()[0]).getByText('12 tracks · 11 real · 1 fake')
    expect(counts.getAttribute('title')).toBe('12 tracks · 11 real · 1 fake')
  })

  // @RF-10 @RNF-09 Scenario: Unicode folder names in the panel
  it('shows Unicode names exactly and the folder in the tooltip', () => {
    const root = 'Álbum ″Ñandú″'
    const disc = 'Álbum ″Ñandú″\\Disco 2 «Ça va» 🎵'
    setup([
      album(0, root, { folder: `C:\\Music\\${root}` }),
      album(1, disc, { folder: `C:\\Music\\${disc}` }),
    ])
    const items = options()
    expect(within(items[0]).getByText(root).textContent).toBe(root)
    const second = within(items[1]).getByText(disc)
    expect(second.textContent).toBe(disc)
    expect(second.getAttribute('title')).toBe(`C:\\Music\\${disc}`)
  })

  // @RF-10 @RNF-09 Scenario: Two albums with the same name stay two items
  it('keeps two albums with the same name as two items', () => {
    const name = 'Lib\\Disc \uFFFD'
    const folder = 'C:\\Music\\Lib\\Disc \uFFFD'
    setup([
      album(0, name, { folder, real: 1 }),
      album(1, name, { folder, fake: 1, real: 0 }),
    ])
    const items = options()
    expect(items).toHaveLength(2)
    expect(within(items[0]).getByText('Genuine')).not.toBeNull()
    expect(within(items[1]).getByText('Fake')).not.toBeNull()
  })
})

describe('AlbumSummaryList: accessibility', () => {
  // @RF-10 Scenario: The panel's accessible names
  it('is a listbox named "Album summaries" with named options', () => {
    setup([album(0, 'Artist\\Album 1', { real: 11, fake: 1 })])
    expect(listbox()).not.toBeNull()
    expect(
      screen.getByRole('option', {
        name: 'Artist\\Album 1, Fake, 12 tracks · 11 real · 1 fake',
      }),
    ).not.toBeNull()
  })

  it('names a partial album with its partial counts', () => {
    setup([album(0, 'Album A', { real: 3, files: 12 })])
    expect(
      screen.getByRole('option', { name: 'Album A, Genuine, 3 of 12 tracks · 3 real' }),
    ).not.toBeNull()
  })

  // @RF-10 Scenario: Selecting a track marks its album as current
  it('marks only the current album with aria-selected', () => {
    setup(THREE, 1)
    expect(options().map((o) => o.getAttribute('aria-selected'))).toEqual([
      'false',
      'true',
      'false',
    ])
  })

  it('marks no album when there is no current album', () => {
    setup(THREE, null)
    expect(options().map((o) => o.getAttribute('aria-selected'))).toEqual([
      'false',
      'false',
      'false',
    ])
  })

  it('compares the current album by index, not by name', () => {
    const name = 'Lib\\Disc \uFFFD'
    setup([album(0, name), album(1, name)], 1)
    expect(options().map((o) => o.getAttribute('aria-selected'))).toEqual(['false', 'true'])
  })

  // @RF-10 the list is one tab stop
  it('is one tab stop: the listbox takes the focus, the options do not', async () => {
    setup(THREE)
    const user = userEvent.setup()
    await user.tab()
    expect(document.activeElement).toBe(listbox())
    await user.tab()
    expect(document.activeElement).not.toBe(listbox())
    expect(options().every((o) => !o.contains(document.activeElement))).toBe(true)
  })
})

describe('AlbumSummaryList: interaction', () => {
  // @RF-10 Scenario: Selecting an album selects its first track
  it('emits select(albumIndex) when an item is clicked', async () => {
    const { onselect } = setup(THREE)
    await userEvent.click(options()[2])
    expect(onselect).toHaveBeenCalledWith(2)
  })

  it('emits select again when the current album is clicked', async () => {
    const { onselect } = setup(THREE, 1)
    await userEvent.click(options()[1])
    expect(onselect).toHaveBeenCalledWith(1)
  })

  it('emits the album index, not the position, when indexes have gaps', async () => {
    const { onselect } = setup([album(3, 'X'), album(8, 'Y')])
    await userEvent.click(options()[1])
    expect(onselect).toHaveBeenCalledWith(8)
  })

  // @RF-10 Scenario: Keyboard in the Albums panel
  it('ArrowDown with no current album selects the first, then the next', async () => {
    const { onselect } = setup(THREE, null)
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenLastCalledWith(0)
  })

  it('ArrowDown selects the next album of the current one', async () => {
    const { onselect } = setup(THREE, 0)
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenLastCalledWith(1)
  })

  it('ArrowUp selects the previous album', async () => {
    const { onselect } = setup(THREE, 2)
    await fireEvent.keyDown(listbox(), { key: 'ArrowUp' })
    expect(onselect).toHaveBeenLastCalledWith(1)
  })

  it('ArrowUp with no current album selects the last', async () => {
    const { onselect } = setup(THREE, null)
    await fireEvent.keyDown(listbox(), { key: 'ArrowUp' })
    expect(onselect).toHaveBeenLastCalledWith(2)
  })

  it('Home selects the first album and End the last', async () => {
    const { onselect } = setup(THREE, 1)
    await fireEvent.keyDown(listbox(), { key: 'End' })
    expect(onselect).toHaveBeenLastCalledWith(2)
    await fireEvent.keyDown(listbox(), { key: 'Home' })
    expect(onselect).toHaveBeenLastCalledWith(0)
  })

  it('Home and End with no current album select the first and the last', async () => {
    const { onselect } = setup(THREE, null)
    await fireEvent.keyDown(listbox(), { key: 'Home' })
    expect(onselect).toHaveBeenLastCalledWith(0)
    await fireEvent.keyDown(listbox(), { key: 'End' })
    expect(onselect).toHaveBeenLastCalledWith(2)
  })

  it('navigates by position when album indexes have gaps', async () => {
    const { onselect } = setup([album(3, 'X'), album(8, 'Y'), album(9, 'Z')], 3)
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenLastCalledWith(8)
  })

  it('does nothing without albums', async () => {
    const { onselect } = setup([])
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    await fireEvent.keyDown(listbox(), { key: 'End' })
    expect(onselect).not.toHaveBeenCalled()
  })

  // @RF-10 Scenario: Keyboard in the Albums panel ("the focus is still in the panel")
  it('keeps the focus in the list after a key press', async () => {
    setup(THREE, 0)
    const user = userEvent.setup()
    await user.tab()
    await user.keyboard('{ArrowDown}')
    expect(document.activeElement).toBe(listbox())
  })
})

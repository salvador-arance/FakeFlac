// AlbumSummaryList: the keys at the edges of the list (@RF-10; specs/ui/album-summary.md
// "Interaction", Keyboard: "At the first or last album, ↑/↓ select the current album again, with
// the effect of a click on it (Q16)"; Q16 (4) "Edge keys keep emitting select"). With the page's
// reveal request, that `select` brings the album's first row back into view (scenario "Selecting
// the current album again brings its first track back into view").
//
// Reading: "select the current album again" is one `select(albumIndex)` event with the current
// album's index per key press, exactly as a click on the current item emits.
import { fireEvent, render, screen } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import type { AlbumUpdate } from '../../model'
import { makeAlbum } from '../../testing/fixtures'
import AlbumSummaryList from './AlbumSummaryList.svelte'

function setup(albums: AlbumUpdate[], currentIndex: number | null) {
  const onselect = vi.fn()
  render(AlbumSummaryList, { props: { albums, currentIndex, onselect } })
  return { onselect }
}

const listbox = () => screen.getByRole('listbox', { name: 'Album summaries' })

const THREE = [
  makeAlbum({ index: 0, name: 'Lib\\A', real: 1 }),
  makeAlbum({ index: 1, name: 'Lib\\B', real: 1 }),
  makeAlbum({ index: 2, name: 'Lib\\C', real: 1 }),
]
// album indexes with gaps: the edges are by position, the event carries the index
const GAPS = [makeAlbum({ index: 3, name: 'X', real: 1 }), makeAlbum({ index: 8, name: 'Y', real: 1 })]

describe('AlbumSummaryList: keys at the edges select the current album again', () => {
  // @RF-10 album-summary.md Keyboard (Q16)
  it('ArrowUp on the first album emits select of the first album', async () => {
    const { onselect } = setup(THREE, 0)
    await fireEvent.keyDown(listbox(), { key: 'ArrowUp' })
    expect(onselect).toHaveBeenCalledTimes(1)
    expect(onselect).toHaveBeenCalledWith(0)
  })

  it('ArrowDown on the last album emits select of the last album', async () => {
    const { onselect } = setup(THREE, 2)
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    expect(onselect).toHaveBeenCalledTimes(1)
    expect(onselect).toHaveBeenCalledWith(2)
  })

  it('Home on the first album emits select of that album', async () => {
    const { onselect } = setup(THREE, 0)
    await fireEvent.keyDown(listbox(), { key: 'Home' })
    expect(onselect).toHaveBeenCalledTimes(1)
    expect(onselect).toHaveBeenCalledWith(0)
  })

  it('End on the last album emits select of that album', async () => {
    const { onselect } = setup(THREE, 2)
    await fireEvent.keyDown(listbox(), { key: 'End' })
    expect(onselect).toHaveBeenCalledTimes(1)
    expect(onselect).toHaveBeenCalledWith(2)
  })

  it('emits once per key press, every time', async () => {
    const { onselect } = setup(THREE, 0)
    for (let i = 0; i < 3; i++) await fireEvent.keyDown(listbox(), { key: 'ArrowUp' })
    expect(onselect.mock.calls).toEqual([[0], [0], [0]])
  })

  it('a single album is both the first and the last', async () => {
    const { onselect } = setup([THREE[1]], 1)
    await fireEvent.keyDown(listbox(), { key: 'ArrowUp' })
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    expect(onselect.mock.calls).toEqual([[1], [1]])
  })

  it('ArrowUp on the first album emits its index when indexes have gaps', async () => {
    const { onselect } = setup(GAPS, 3)
    await fireEvent.keyDown(listbox(), { key: 'ArrowUp' })
    expect(onselect.mock.calls).toEqual([[3]])
  })

  it('ArrowDown on the last album emits its index when indexes have gaps', async () => {
    const { onselect } = setup(GAPS, 8)
    await fireEvent.keyDown(listbox(), { key: 'ArrowDown' })
    expect(onselect.mock.calls).toEqual([[8]])
  })
})

// AlbumSummaryList in both languages (@RF-13 @RF-10 @RNF-10; specs/ui/translations.md section 4):
// singular keys exactly at count 1, only nonzero verdict counts in their existing order, ` · `,
// `albums.partial` for a partial album. Album names are user data. Scenario "The two catalogues
// cover all user-facing states explicitly" (singular forms) and the album part of "Existing
// mounted states translate after a successful Save".
import { render, screen, within } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import type { AlbumOptions } from '../../testing/fixtures'
import { makeAlbum } from '../../testing/fixtures'
import AlbumSummaryList from './AlbumSummaryList.svelte'

afterEach(() => setLocale('en'))

const album = (index: number, name: string, o: AlbumOptions = {}) => makeAlbum({ index, name, ...o })
const options = () => screen.queryAllByRole('option')
/** Each option shows its counts text (as AlbumSummaryList.test.ts reads it). */
const expectCounts = (texts: string[]) => {
  expect(options()).toHaveLength(texts.length)
  texts.forEach((text, i) => expect(within(options()[i]).getByText(text)).toBeTruthy())
}

const ALBUMS = [
  album(0, 'Artist', { real: 1 }),
  album(1, 'Artist\\Álbum 1', { real: 11, fake: 1 }),
  album(2, 'Artist\\Album 2', { real: 2, suspicious: 1, fake: 1, errors: 3 }),
  album(3, 'Artist\\Album 3', { real: 1, errors: 1 }),
  album(4, 'Artist\\Album 4', { real: 3, fake: 1, files: 12 }),
  album(5, 'Artist\\Album 5', { real: 0, suspicious: 1 }),
  album(6, 'Artist\\Album 6', { real: 2, suspicious: 2, fake: 2, errors: 2 }),
]

const ENGLISH = [
  '1 track · 1 real',
  '12 tracks · 11 real · 1 fake',
  '7 tracks · 2 real · 1 suspicious · 1 fake · 3 errors',
  '2 tracks · 1 real · 1 error',
  '4 of 12 tracks · 3 real · 1 fake',
  '1 track · 1 suspicious',
  '8 tracks · 2 real · 2 suspicious · 2 fake · 2 errors',
]

const SPANISH = [
  '1 pista · 1 real',
  '12 pistas · 11 reales · 1 falsa',
  '7 pistas · 2 reales · 1 sospechosa · 1 falsa · 3 errores',
  '2 pistas · 1 real · 1 error',
  '4 de 12 pistas · 3 reales · 1 falsa',
  '1 pista · 1 sospechosa',
  '8 pistas · 2 reales · 2 sospechosas · 2 falsas · 2 errores',
]

describe('AlbumSummaryList: counts (@RF-13)', () => {
  it('English, singular exactly at 1', () => {
    render(AlbumSummaryList, { props: { albums: ALBUMS, currentIndex: null } })
    expectCounts(ENGLISH)
  })

  it('Spanish, singular exactly at 1', () => {
    setLocale('es')
    render(AlbumSummaryList, { props: { albums: ALBUMS, currentIndex: null } })
    expectCounts(SPANISH)
  })
})

describe('AlbumSummaryList: Spanish texts (@RF-13)', () => {
  it('header, list name, verdict and option names; names unchanged', () => {
    setLocale('es')
    render(AlbumSummaryList, { props: { albums: ALBUMS.slice(0, 2), currentIndex: 1 } })
    expect(screen.getByText('Álbumes (2)')).toBeTruthy()
    const list = screen.getByRole('listbox', { name: 'Resúmenes de álbumes' })
    const second = within(list).getAllByRole('option')[1]
    expect(second.getAttribute('aria-label')).toBe('Artist\\Álbum 1, Falso, 12 pistas · 11 reales · 1 falsa')
    expect(within(second).getByText('Artist\\Álbum 1')).toBeTruthy()
    expect(within(second).getByText('Falso')).toBeTruthy()
    expect(second.getAttribute('aria-selected')).toBe('true')
  })

  it('an empty list says "Álbumes"', () => {
    setLocale('es')
    render(AlbumSummaryList, { props: { albums: [], currentIndex: null } })
    expect(screen.getByText('Álbumes')).toBeTruthy()
  })

  it('re-renders while mounted, keeping the order and the current album', () => {
    render(AlbumSummaryList, { props: { albums: ALBUMS, currentIndex: 3 } })
    const before = options().map((o) => o.getAttribute('data-album-index'))
    expect(screen.getByText('Albums (7)')).toBeTruthy()
    setLocale('es')
    flushSync()
    expect(screen.getByText('Álbumes (7)')).toBeTruthy()
    expect(options().map((o) => o.getAttribute('data-album-index'))).toEqual(before)
    expectCounts(SPANISH)
    expect(options()[3].getAttribute('aria-selected')).toBe('true')
    const title = within(options()[1]).getByText(SPANISH[1]).getAttribute('title')
    expect(title).toBe(SPANISH[1])
  })
})

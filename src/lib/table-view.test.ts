// View checks V1-V11 of specs/ui/sort-filter.md section 11 (unit tests of src/lib/table-view.ts),
// plus the rules of sections 3 and 4 they rest on (@RF-08; @RNF-04 empty cells, @RNF-09 Unicode).
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - Rows are built with `makeRow`; `TrackInfo.folder` is set explicitly (the path's folder part,
//   split at the last `\` or `/`) unless a check gives it, since `makeTrack` only splits at `/`.
// - "01 and 1 compare equal" (3.2) + ties (3.3): two such files are ordered by index in both
//   directions of File.
// - V10's "reference computation" is an independent implementation of sections 3 and 4 written in
//   this file. A sort is checked by comparing every adjacent pair of the result with the reference
//   (strictly increasing under a total order = the reference order), which needs n comparisons.
import { describe, expect, it } from 'vitest'
import type { ResultRow, RowFilter, SortColumn, SortState, Verdict } from './model'
import { makeRow } from './testing/fixtures'
import type { TrackOptions } from './testing/fixtures'
import { compareRows, foldForSearch, isFilterActive, rowMatches, searchTerms } from './table-view'

const folderPart = (path: string) => {
  const cut = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return cut >= 0 ? path.slice(0, cut) : ''
}
const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path

/** A row whose `TrackInfo.folder` is the path's folder part unless `folder` is given. */
function row(index: number, path: string, o: TrackOptions = {}, folder?: string, albumIndex = 0): ResultRow {
  const r = makeRow(index, { ...o, path }, albumIndex)
  r.analysis.info.folder = folder ?? folderPart(path)
  return r
}

const sorted = (rows: ResultRow[], sort: SortState, locale: 'en' | 'es' = 'en') =>
  [...rows].sort((a, b) => compareRows(a, b, sort, locale))
const asc = (column: SortColumn): SortState => ({ column, direction: 'asc' })
const desc = (column: SortColumn): SortState => ({ column, direction: 'desc' })
const files = (rows: ResultRow[]) => rows.map((r) => fileName(r.analysis.info.path))
const paths = (rows: ResultRow[]) => rows.map((r) => r.analysis.info.path)
const indexes = (rows: ResultRow[]) => rows.map((r) => r.index)
const verdicts = (rows: ResultRow[]) => rows.map((r) => r.analysis.verdict)

describe('V1: State order (@RF-08)', () => {
  const rows = [
    row(0, 'C:\\L\\A\\a.flac', { verdict: 'real' }),
    row(1, 'C:\\L\\A\\b.flac', { verdict: 'error' }),
    row(2, 'C:\\L\\A\\c.flac', { verdict: 'fake' }),
    row(3, 'C:\\L\\A\\d.flac', { verdict: 'suspicious' }),
  ]

  it('ascending puts what needs attention first: fake, suspicious, error, real', () => {
    expect(verdicts(sorted(rows, asc('state')))).toEqual(['fake', 'suspicious', 'error', 'real'])
  })

  it('descending is the exact reverse: real, error, suspicious, fake', () => {
    expect(verdicts(sorted(rows, desc('state')))).toEqual(['real', 'error', 'suspicious', 'fake'])
  })
})

describe('V2: natural, case-insensitive names (@RF-08)', () => {
  it('File ascending gives Track 1, track 2, Track 10', () => {
    const rows = [
      row(0, 'C:\\L\\Album\\Track 10.flac'),
      row(1, 'C:\\L\\Album\\track 2.flac'),
      row(2, 'C:\\L\\Album\\Track 1.flac'),
    ]
    expect(files(sorted(rows, asc('file')))).toEqual(['Track 1.flac', 'track 2.flac', 'Track 10.flac'])
  })

  it('compares "01" and "1" equal, so the index breaks the tie in both directions', () => {
    const rows = [row(1, 'C:\\L\\A\\01.flac'), row(0, 'C:\\L\\A\\1.flac')]
    expect(indexes(sorted(rows, asc('file')))).toEqual([0, 1])
    expect(indexes(sorted(rows, desc('file')))).toEqual([0, 1])
  })

  it('compares "a.flac" and "A.flac" equal, so the index breaks the tie', () => {
    const rows = [row(4, 'C:\\L\\A\\a.flac'), row(2, 'C:\\L\\A\\A.flac')]
    expect(indexes(sorted(rows, asc('file')))).toEqual([2, 4])
    expect(indexes(sorted(rows, desc('file')))).toEqual([2, 4])
  })
})

describe('V3: path components (@RF-08)', () => {
  it("puts a folder's rows before a sibling folder whose name extends it", () => {
    const rows = [
      row(0, 'C:\\L\\Album\\CD2\\01.flac'),
      row(1, 'C:\\L\\Album (deluxe)\\01.flac'),
      row(2, 'C:\\L\\Album\\02.flac'),
    ]
    expect(paths(sorted(rows, asc('file')))).toEqual([
      'C:\\L\\Album\\02.flac',
      'C:\\L\\Album\\CD2\\01.flac',
      'C:\\L\\Album (deluxe)\\01.flac',
    ])
  })

  it("interleaves a subfolder with its parent's files by its name (intended, Q3)", () => {
    const rows = [
      row(0, 'C:\\L\\Album\\Z.flac'),
      row(1, 'C:\\L\\Album\\Bonus\\x.flac'),
      row(2, 'C:\\L\\Album\\01.flac'),
    ]
    const ascending = ['C:\\L\\Album\\01.flac', 'C:\\L\\Album\\Bonus\\x.flac', 'C:\\L\\Album\\Z.flac']
    expect(paths(sorted(rows, asc('file')))).toEqual(ascending)
    expect(paths(sorted(rows, desc('file')))).toEqual([...ascending].reverse())
  })

  it('splits at "/" as well as "\\"', () => {
    const rows = [row(0, 'C:/L/Album (deluxe)/01.flac'), row(1, 'C:/L/Album/CD2/01.flac')]
    expect(paths(sorted(rows, asc('file')))).toEqual(['C:/L/Album/CD2/01.flac', 'C:/L/Album (deluxe)/01.flac'])
  })
})

describe('V4: ties (@RF-08)', () => {
  const rows = [
    row(0, 'C:\\L\\A\\b.flac', { verdict: 'fake' }),
    row(1, 'C:\\L\\A\\a.flac', { verdict: 'fake' }),
    row(2, 'C:\\L\\A\\c.flac', { verdict: 'real' }),
  ]

  it('State ascending: a.flac, b.flac, then the real row', () => {
    expect(files(sorted(rows, asc('state')))).toEqual(['a.flac', 'b.flac', 'c.flac'])
  })

  it('State descending: the real row, then a.flac, b.flac', () => {
    expect(files(sorted(rows, desc('state')))).toEqual(['c.flac', 'a.flac', 'b.flac'])
  })

  it('does not depend on the input order', () => {
    expect(files(sorted([...rows].reverse(), asc('state')))).toEqual(['a.flac', 'b.flac', 'c.flac'])
    expect(files(sorted([rows[1], rows[2], rows[0]], desc('state')))).toEqual(['c.flac', 'a.flac', 'b.flac'])
  })

  // @RNF-09 two files with the same path (U+FFFD names) keep their scan order
  it('orders two rows with the same path by index in both directions of File', () => {
    const same = 'C:\\L\\Disc \uFFFD\\01.flac'
    const twins = [row(5, same), row(3, same, { verdict: 'fake' })]
    expect(indexes(sorted(twins, asc('file')))).toEqual([3, 5])
    expect(indexes(sorted(twins, desc('file')))).toEqual([3, 5])
  })

  it('never returns 0 for two rows with different indexes', () => {
    const same = 'C:\\L\\A\\x.flac'
    const a = row(3, same)
    const b = row(5, same)
    const columns: SortColumn[] = ['state', 'file', 'folder', 'format', 'sampleRate', 'bitDepth', 'duration', 'cutoff', 'bitrate', 'realBitrate']
    for (const column of columns) {
      for (const direction of ['asc', 'desc'] as const) {
        const sort = { column, direction }
        expect(Math.sign(compareRows(a, b, sort, 'en')), `${column} ${direction}`).toBe(-1)
        expect(Math.sign(compareRows(b, a, sort, 'en')), `${column} ${direction}`).toBe(1)
      }
    }
  })
})

describe('V5: empty cells last (@RF-08 @RNF-04)', () => {
  it('Cutoff: null last in both directions', () => {
    const rows = [
      row(0, 'C:\\L\\A\\low.flac', { cutoffHz: 16000, verdict: 'fake' }),
      row(1, 'C:\\L\\A\\corrupt.flac', { cutoffHz: null, verdict: 'error' }),
      row(2, 'C:\\L\\A\\full.flac', { cutoffHz: 20000 }),
    ]
    expect(files(sorted(rows, asc('cutoff')))).toEqual(['low.flac', 'full.flac', 'corrupt.flac'])
    expect(files(sorted(rows, desc('cutoff')))).toEqual(['full.flac', 'low.flac', 'corrupt.flac'])
  })

  it('Bits: null last in both directions', () => {
    const rows = [
      row(0, 'C:\\L\\A\\b16.flac', { bitDepth: 16 }),
      row(1, 'C:\\L\\A\\none.mp3', { bitDepth: null, format: 'mp3', codec: 'mp3' }),
      row(2, 'C:\\L\\A\\b24.flac', { bitDepth: 24 }),
    ]
    expect(files(sorted(rows, asc('bitDepth')))).toEqual(['b16.flac', 'b24.flac', 'none.mp3'])
    expect(files(sorted(rows, desc('bitDepth')))).toEqual(['b24.flac', 'b16.flac', 'none.mp3'])
  })

  it('Bitrate and Real bitrate: null last in both directions (unchanged)', () => {
    const rows = [
      row(0, 'C:\\L\\A\\a.mp3', { declaredBitrateKbps: 128, estimatedSourceBitrateKbps: 128 }),
      row(1, 'C:\\L\\A\\b.flac', { declaredBitrateKbps: null, estimatedSourceBitrateKbps: null }),
      row(2, 'C:\\L\\A\\c.mp3', { declaredBitrateKbps: 320, estimatedSourceBitrateKbps: 192 }),
    ]
    for (const column of ['bitrate', 'realBitrate'] as const) {
      expect(files(sorted(rows, asc(column)))).toEqual(['a.mp3', 'c.mp3', 'b.flac'])
      expect(files(sorted(rows, desc(column)))).toEqual(['c.mp3', 'a.mp3', 'b.flac'])
    }
  })

  it('Samplerate and Duration: numeric', () => {
    const rows = [
      row(0, 'C:\\L\\A\\a.flac', { sampleRate: 96000, durationMs: 60000 }),
      row(1, 'C:\\L\\A\\b.flac', { sampleRate: 44100, durationMs: 300000 }),
      row(2, 'C:\\L\\A\\c.flac', { sampleRate: 48000, durationMs: 9000 }),
    ]
    expect(files(sorted(rows, asc('sampleRate')))).toEqual(['b.flac', 'c.flac', 'a.flac'])
    expect(files(sorted(rows, desc('sampleRate')))).toEqual(['a.flac', 'c.flac', 'b.flac'])
    expect(files(sorted(rows, asc('duration')))).toEqual(['c.flac', 'a.flac', 'b.flac'])
    expect(files(sorted(rows, desc('duration')))).toEqual(['b.flac', 'a.flac', 'c.flac'])
  })
})

describe('V6: accents in order (@RF-08 @RNF-09)', () => {
  it('sorts by letter, accents only breaking ties (locale en)', () => {
    const names = [
      'Canci\u00f3n.flac',
      'Cancion.flac',
      '\u00c1lbum.flac',
      'apple.flac',
      '\u00c7a va.flac',
      'zebra.flac',
    ]
    const rows = names.map((n, i) => row(i, `C:\\L\\A\\${n}`))
    expect(files(sorted(rows, asc('file')))).toEqual([
      '\u00c1lbum.flac',
      'apple.flac',
      '\u00c7a va.flac',
      'Cancion.flac',
      'Canci\u00f3n.flac',
      'zebra.flac',
    ])
  })

  it('uses the collation of the UI locale: with es, "\u00f1" sorts after "n"', () => {
    const rows = [row(0, 'C:\\L\\A\\\u00f1a.flac'), row(1, 'C:\\L\\A\\nz.flac')]
    expect(files(sorted(rows, asc('file'), 'en'))).toEqual(['\u00f1a.flac', 'nz.flac'])
    expect(files(sorted(rows, asc('file'), 'es'))).toEqual(['nz.flac', '\u00f1a.flac'])
  })

  it('Format sorts by the cell text', () => {
    const rows = [
      row(0, 'C:\\L\\A\\a.mp3', { format: 'mp3', codec: 'mp3' }),
      row(1, 'C:\\L\\A\\b.m4a', { format: 'm4a', codec: 'alac' }),
      row(2, 'C:\\L\\A\\c.flac', { format: 'flac' }),
      row(3, 'C:\\L\\A\\d.m4a', { format: 'm4a', codec: 'aac' }),
    ]
    // FLAC, M4A, M4A (ALAC), MP3
    expect(files(sorted(rows, asc('format')))).toEqual(['c.flac', 'd.m4a', 'b.m4a', 'a.mp3'])
    expect(files(sorted(rows, desc('format')))).toEqual(['a.mp3', 'b.m4a', 'd.m4a', 'c.flac'])
  })
})

describe('V7: folding (@RF-08 @RNF-09)', () => {
  it.each([
    ['Canci\u00f3n', 'cancion'],
    ['CANCION', 'cancion'],
    ['\u00d1and\u00fa', 'nandu'],
    ['N\u0303andu\u0301', 'nandu'],
    ['\u00c7a', 'ca'],
    ['\u2033', '\u2033'],
    ['\u{1F3B5}', '\u{1F3B5}'],
    ['\u{1F3B5}\uFE0F', '\u{1F3B5}'],
    ['\u304C', '\u304B'],
  ])('foldForSearch(%j) = %j', (input, folded) => {
    expect(foldForSearch(input)).toBe(folded)
  })

  it('keeps punctuation: the ASCII quote does not fold to the double prime', () => {
    expect(foldForSearch('"')).toBe('"')
    expect(foldForSearch('\u2018x\u2019')).toBe('\u2018x\u2019')
  })
})

describe('V8: terms (@RF-08)', () => {
  it('splits at runs of white space and folds', () => {
    expect(searchTerms('  Beatles   help ')).toEqual(['beatles', 'help'])
  })

  it('reads "/" as "\\"', () => {
    const terms = searchTerms('Album 1/01')
    expect(terms).toEqual(['album', '1\\01'])
    expect([...terms[1]]).toEqual(['1', '\\', '0', '1'])
  })

  it('gives no term for a blank query', () => {
    expect(searchTerms('   ')).toEqual([])
    expect(searchTerms('')).toEqual([])
    expect(searchTerms('\t\n ')).toEqual([])
  })

  it('keeps the order of the query', () => {
    expect(searchTerms('Z a M')).toEqual(['z', 'a', 'm'])
  })
})

describe('V9: match (@RF-08)', () => {
  const r = row(0, 'C:\\Music\\Artist\\Album 1\\01 \u2013 Intro.flac', {}, undefined, 1)
  const album = 'Artist\\Album 1'
  const by = (query: string, verdictsFilter: Verdict[] = []): RowFilter => ({ verdicts: verdictsFilter, query })

  it.each([
    ['intro', true],
    ['album 1 intro', true],
    ['intro album', true],
    ['album 2', false],
    ['Music', false],
    ['artist\\album', true],
    ['artist/album', true],
    ['', true],
    ['   ', true],
  ])('query %j: %s', (query, expected) => {
    expect(rowMatches(r, by(query), album)).toBe(expected)
  })

  it('a verdict filter without the row verdict hides it, whatever the query', () => {
    for (const query of ['', 'intro', 'zzz']) {
      expect(rowMatches(r, by(query, ['fake']), album)).toBe(false)
    }
  })

  it('a verdict filter with the row verdict lets the query decide', () => {
    expect(rowMatches(r, by('intro', ['fake', 'real']), album)).toBe(true)
    expect(rowMatches(r, by('zzz', ['real']), album)).toBe(false)
  })

  it('reads the verdicts as a set (order and duplicates do not matter)', () => {
    expect(rowMatches(r, by('', ['real', 'real']), album)).toBe(true)
    expect(rowMatches(r, by('', ['error', 'real', 'fake']), album)).toBe(true)
  })

  // @RNF-04 the error rows are a verdict like the others
  it('the Error filter shows error rows and hides them when only other verdicts are pressed', () => {
    const broken = row(1, 'C:\\Music\\Artist\\Album 1\\corrupt.flac', { verdict: 'error' })
    expect(rowMatches(broken, by('', ['error']), album)).toBe(true)
    expect(rowMatches(broken, by('', ['fake', 'suspicious', 'real']), album)).toBe(false)
  })

  it('searches the album name and the file name only, not the rest of the path or the folder', () => {
    const other = row(2, 'C:\\Music\\Artist\\Album 1\\CD1\\02.flac', {}, 'C:\\Music\\Artist\\Album 1\\CD1')
    expect(rowMatches(other, by('cd1'), 'Artist\\Album 1\\CD1')).toBe(true)
    expect(rowMatches(other, by('music'), 'Artist\\Album 1\\CD1')).toBe(false)
  })

  it('uses the last component of the path as the file name ("/" too)', () => {
    const slash = row(3, 'C:/Music/Artist/Album 1/03 Outro.flac')
    expect(rowMatches(slash, by('album 1\\03'), album)).toBe(true)
    expect(rowMatches(slash, by('music'), album)).toBe(false)
  })

  it('searches the full path when the album name is unknown (guard)', () => {
    expect(rowMatches(r, by('music'), undefined)).toBe(true)
    expect(rowMatches(r, by('intro'), undefined)).toBe(true)
  })

  // @RNF-09 Unicode: case and accents folded on both sides, symbols not
  it('folds the searched text as well as the query', () => {
    const u = row(4, 'C:\\Music\\\u00c1lbum \u2033\u00d1and\u00fa\u2033\\01 \u2013 Canci\u00f3n \u2018x\u2019.flac')
    const name = '\u00c1lbum \u2033\u00d1and\u00fa\u2033'
    expect(rowMatches(u, by('CANCI\u00d3N'), name)).toBe(true)
    expect(rowMatches(u, by('nandu'), name)).toBe(true)
    expect(rowMatches(u, by('\u2033\u00f1and\u00fa\u2033'), name)).toBe(true)
    expect(rowMatches(u, by('"nandu"'), name)).toBe(false)
    expect(rowMatches(u, by("'x'"), name)).toBe(false)
    expect(rowMatches(u, by('\u2018x\u2019'), name)).toBe(true)
  })

  it('matches a decomposed name like the precomposed one', () => {
    const d = row(5, 'C:\\Music\\N\u0303and\u00fa\\01.flac')
    expect(rowMatches(d, by('\u00f1and\u00fa'), 'N\u0303and\u00fa')).toBe(true)
  })
})

describe('isFilterActive (@RF-08)', () => {
  it.each([
    [{ verdicts: [], query: '' }, false],
    [{ verdicts: [], query: '   ' }, false],
    [{ verdicts: ['fake'], query: '' }, true],
    [{ verdicts: [], query: 'x' }, true],
    [{ verdicts: ['fake', 'suspicious', 'real', 'error'], query: '' }, true],
  ] as [RowFilter, boolean][])('%j: %s', (filter, expected) => {
    expect(isFilterActive(filter)).toBe(expected)
  })
})

describe('V11: Folder fallback (@RF-08)', () => {
  const rows = [
    row(0, 'C:\\L\\B\\01.flac', {}, ''),
    row(1, 'C:\\L\\A\\01.flac', {}, 'C:\\L\\A'),
    row(2, 'C:\\L\\C\\01.flac', {}, 'C:\\L\\C'),
  ]

  it('sorts an empty folder as the folder part of its path', () => {
    const ascending = ['C:\\L\\A\\01.flac', 'C:\\L\\B\\01.flac', 'C:\\L\\C\\01.flac']
    expect(paths(sorted(rows, asc('folder')))).toEqual(ascending)
    expect(paths(sorted(rows, desc('folder')))).toEqual([...ascending].reverse())
  })

  it('does not depend on the separator of the fallback text', () => {
    const mixed = [row(0, 'C:/L/B/01.flac', {}, ''), row(1, 'C:\\L\\A\\01.flac', {}, 'C:\\L\\A'), row(2, 'C:\\L\\C\\01.flac', {}, 'C:\\L\\C')]
    expect(paths(sorted(mixed, asc('folder')))).toEqual(['C:\\L\\A\\01.flac', 'C:/L/B/01.flac', 'C:\\L\\C\\01.flac'])
  })

  it('compares folders component by component and naturally', () => {
    const rows2 = [
      row(0, 'C:\\L\\Album (deluxe)\\01.flac'),
      row(1, 'C:\\L\\Album 10\\01.flac'),
      row(2, 'C:\\L\\Album\\CD2\\01.flac'),
      row(3, 'C:\\L\\Album 2\\01.flac'),
    ]
    // "Album" is a prefix component; then the collator: "(" sorts before digits, 2 before 10
    expect(paths(sorted(rows2, asc('folder')))).toEqual([
      'C:\\L\\Album\\CD2\\01.flac',
      'C:\\L\\Album (deluxe)\\01.flac',
      'C:\\L\\Album 2\\01.flac',
      'C:\\L\\Album 10\\01.flac',
    ])
  })
})

// ---------------------------------------------------------------------------------------------
// V10: 10 000 rows (feature scenario "Ten thousand rows") and a reference computation.

const VERDICT_OF: Verdict[] = ['real', 'suspicious', 'fake', 'error']
const pad = (n: number, width: number) => String(n).padStart(width, '0')
const albumNameOf = (i: number) => `Lib\\Album ${pad(Math.floor(i / 100), 2)}`
const nameOf = (i: number) => `${albumNameOf(i)}\\${pad(i, 4)} \u2013 Track.flac`

/**
 * Rows i = 0..9999 by the rule of the scenario (name, index, verdict by i mod 4, duration). The
 * other cells, which the scenario leaves free, vary so every column has distinct values and ties.
 */
function tenThousand(): ResultRow[] {
  return Array.from({ length: 10000 }, (_, i) => {
    const verdict = VERDICT_OF[i % 4]
    const mp3 = i % 7 === 0
    const r = row(
      i,
      `C:\\Music\\${nameOf(i)}`,
      {
        verdict,
        durationMs: (i + 1) * 1000,
        cutoffHz: verdict === 'error' ? null : 15000 + ((i * 7919) % 7000),
        bitDepth: verdict === 'error' || mp3 ? null : i % 3 === 0 ? 24 : 16,
        sampleRate: i % 5 === 0 ? 48000 : 44100,
        format: mp3 ? 'mp3' : i % 11 === 0 ? 'm4a' : 'flac',
        codec: mp3 ? 'mp3' : i % 11 === 0 ? (i % 2 === 0 ? 'alac' : 'aac') : 'flac',
        declaredBitrateKbps: mp3 ? 32 + ((i * 13) % 289) : null,
        estimatedSourceBitrateKbps: mp3 && i % 3 !== 0 ? 64 + ((i * 17) % 257) : null,
      },
      undefined,
      Math.floor(i / 100),
    )
    return r
  })
}

// ---- reference implementation of sections 3 and 4 (independent of table-view.ts) ----
const refCollator = new Intl.Collator('en', { numeric: true, sensitivity: 'accent' })
function refName(a: string, b: string): number {
  const pa = a.split(/[\\/]/)
  const pb = b.split(/[\\/]/)
  for (let i = 0; i < Math.min(pa.length, pb.length); i++) {
    const c = refCollator.compare(pa[i], pb[i])
    if (c !== 0) return c
  }
  return pa.length - pb.length
}
const RANK: Record<Verdict, number> = { fake: 0, suspicious: 1, error: 2, real: 3 }
function refFormat(r: ResultRow): string {
  const info = r.analysis.info
  if (info.format === 'm4a' && info.codec === 'alac') return 'M4A (ALAC)'
  return info.format.toUpperCase()
}
function refNumber(r: ResultRow, column: SortColumn): number | null {
  const a = r.analysis
  switch (column) {
    case 'sampleRate':
      return a.info.sampleRate
    case 'bitDepth':
      return a.info.bitDepth
    case 'duration':
      return a.info.durationMs
    case 'cutoff':
      return a.cutoff?.frequencyHz ?? null
    case 'bitrate':
      return a.info.declaredBitrateKbps
    case 'realBitrate':
      return a.estimatedSourceBitrateKbps
    default:
      throw new Error(column)
  }
}
function refCompare(a: ResultRow, b: ResultRow, sort: SortState): number {
  const dir = sort.direction === 'asc' ? 1 : -1
  let key = 0
  switch (sort.column) {
    case 'state':
      key = dir * (RANK[a.analysis.verdict] - RANK[b.analysis.verdict])
      break
    case 'file':
      key = dir * refName(a.analysis.info.path, b.analysis.info.path)
      break
    case 'folder':
      key =
        dir *
        refName(
          a.analysis.info.folder || folderPart(a.analysis.info.path),
          b.analysis.info.folder || folderPart(b.analysis.info.path),
        )
      break
    case 'format':
      key = dir * refName(refFormat(a), refFormat(b))
      break
    default: {
      const x = refNumber(a, sort.column)
      const y = refNumber(b, sort.column)
      if (x === null && y !== null) key = 1
      else if (x !== null && y === null) key = -1
      else if (x !== null && y !== null) key = dir * (x - y)
    }
  }
  if (key !== 0) return key
  const byFile = refName(a.analysis.info.path, b.analysis.info.path)
  if (byFile !== 0) return byFile
  return a.index - b.index
}
const refFold = (s: string) => s.toLowerCase().normalize('NFD').replace(/\p{Mn}/gu, '').normalize('NFC')
function refMatches(r: ResultRow, filter: RowFilter, albumName: string): boolean {
  if (filter.verdicts.length > 0 && !filter.verdicts.includes(r.analysis.verdict)) return false
  const text = refFold(`${albumName}\\${fileName(r.analysis.info.path)}`)
  const terms = refFold(filter.query.replaceAll('/', '\\'))
    .split(/\s+/)
    .filter((t) => t.length > 0)
  return terms.every((t) => text.includes(t))
}

describe('V10: ten thousand rows (@RF-08)', () => {
  const rows = tenThousand()
  const albumOf = (r: ResultRow) => albumNameOf(r.index)

  it('State ascending, suspicious, query "00": exactly the 52 rows of the scenario, in its order', () => {
    const filter: RowFilter = { verdicts: ['suspicious'], query: '00' }
    const shown = rows.filter((r) => rowMatches(r, filter, albumOf(r)))
    expect(shown).toHaveLength(52)
    const expected = [
      ...Array.from({ length: 25 }, (_, k) => nameOf(1 + 4 * k)),
      ...[1, 2, 3, 4, 5, 6, 7, 8, 9].flatMap((d) => [1, 5, 9].map((y) => nameOf(d * 1000 + y))),
    ]
    expect(expected).toHaveLength(52)
    const order = sorted(shown, asc('state')).map((r) => r.analysis.info.path.slice('C:\\Music\\'.length))
    expect(order).toEqual(expected)
  })

  it('sorts like the reference for every column and both directions', { timeout: 60000 }, () => {
    const columns: SortColumn[] = ['state', 'file', 'folder', 'format', 'sampleRate', 'bitDepth', 'duration', 'cutoff', 'bitrate', 'realBitrate']
    const shuffled = [...rows].sort((a, b) => ((a.index * 7919) % 10007) - ((b.index * 7919) % 10007))
    for (const column of columns) {
      for (const direction of ['asc', 'desc'] as const) {
        const sort = { column, direction }
        const result = sorted(shuffled, sort)
        expect(result).toHaveLength(rows.length)
        for (let i = 0; i + 1 < result.length; i++) {
          if (refCompare(result[i], result[i + 1], sort) >= 0) {
            expect.fail(
              `${column} ${direction}: position ${i} (index ${result[i].index}) is not before index ${result[i + 1].index}`,
            )
          }
        }
      }
    }
  })

  it('filters like the reference', { timeout: 60000 }, () => {
    const filters: RowFilter[] = [
      { verdicts: [], query: '' },
      { verdicts: ['suspicious'], query: '00' },
      { verdicts: ['fake', 'error'], query: 'album 4' },
      { verdicts: [], query: ' 12  TRACK ' },
      { verdicts: ['real'], query: 'album 1/1' },
      { verdicts: [], query: '\u2013 9' },
      { verdicts: ['error'], query: 'zzz' },
    ]
    for (const filter of filters) {
      const got = rows.filter((r) => rowMatches(r, filter, albumOf(r))).map((r) => r.index)
      const want = rows.filter((r) => refMatches(r, filter, albumOf(r))).map((r) => r.index)
      expect(got, JSON.stringify(filter)).toEqual(want)
    }
  })
})

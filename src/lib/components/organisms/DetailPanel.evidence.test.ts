import { render, screen } from '@testing-library/svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import type { AlbumSummary } from '../../model'
import { makeAlbum, makeTrack } from '../../testing/fixtures'
import DetailPanel from './DetailPanel.svelte'

const gapOnly = 'The evidence is the gap pattern at 16 kHz, not the cutoff.'
const audioNote = 'These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio.'
const gap = 'Gaps above 16 kHz, as in an MP3: gap step 9.6 dB, 3.6 dB over the Fake threshold (6.0 dB)'
const step = 'Sharp cutoff where lossy encoders cut: step 13.2 dB, 3.2 dB over the Fake threshold (10.0 dB)'
const only = (tracks: number) =>
  `The only Fake of the ${tracks} analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram.`
const some = (fake: number, tracks: number) =>
  `${fake} of the ${tracks} analysed tracks in this album are Fake. Transcodes usually affect whole albums: check their spectrograms.`

const gapTrack = () => makeTrack({
  path: 'C:/Music/Álbum/01 – Canción ‘x’.flac',
  verdict: 'fake',
  cutoffHz: 22_000,
  evidence: [{ kind: 'gapStep', value: 9.6, threshold: 6, margin: 3.6 }],
})
const stepTrack = () => makeTrack({
  verdict: 'fake',
  cutoffHz: 16_000,
  evidence: [{ kind: 'cutoffStep', value: 13.2, threshold: 10, margin: 3.2 }],
})

function panel(track: ReturnType<typeof makeTrack> | null, album?: AlbumSummary, unavailable = false) {
  const { container } = render(DetailPanel, {
    props: { track, album, spectrogram: null, loading: !unavailable, unavailable },
  })
  return (container.textContent ?? '').replace(/\s+/g, ' ')
}

function expectInOrder(text: string, lines: string[]) {
  let end = 0
  for (const line of lines) {
    const start = text.indexOf(line, end)
    expect(start, `missing or out of order: ${line}`).toBeGreaterThanOrEqual(end)
    end = start + line.length
  }
}

afterEach(() => setLocale('en'))

describe('DetailPanel evidence', () => {
  // @RF-17 @RF-07 Detail panel of a gap-only fake whose cutoff is near full band.
  it('shows gap evidence, context, and the audio note below an unchanged verdict while loading', () => {
    const text = panel(gapTrack(), makeAlbum({ real: 13, fake: 1 }).summary)
    expectInOrder(text, ['Cutoff 22.0 kHz · Fake', gap, gapOnly, only(14), audioNote])
    expect(screen.getByTestId('spectrogram-loading')).not.toBeNull()
  })

  // @RF-17 @RF-07 @RF-13 The same detail panel in Spanish.
  it('uses the Spanish evidence and album wording with decimal points', () => {
    setLocale('es')
    const text = panel(gapTrack(), makeAlbum({ real: 13, fake: 1 }).summary)
    expectInOrder(text, [
      'Corte 22.0 kHz · Falso',
      'Huecos por encima de 16 kHz, como en un MP3: escalón de huecos de 9.6 dB, 3.6 dB por encima del umbral de Falso (6.0 dB)',
      'La prueba es el patrón de huecos a partir de 16 kHz, no el corte.',
      'Es la única pista falsa de las 14 analizadas en este álbum. Las transcodificaciones suelen afectar a álbumes enteros: revisa su espectrograma.',
      'Estas huellas están en el propio audio. No distinguen un archivo convertido de una edición auténtica cuyo máster se hizo con audio con pérdida.',
    ])
  })

  // @RF-17 @RF-07 Detail panel of a fake with both kinds in an album where every track is fake.
  it('lists both items in order and omits the gap-only and album notes', () => {
    const track = makeTrack({
      verdict: 'fake',
      cutoffHz: 16_000,
      evidence: [
        { kind: 'gapStep', value: 12, threshold: 6, margin: 6 },
        { kind: 'cutoffStep', value: 24.7, threshold: 10, margin: 14.7 },
      ],
    })
    const text = panel(track, makeAlbum({ real: 0, fake: 12 }).summary)
    expectInOrder(text, [
      'Cutoff 16.0 kHz · Fake',
      'Gaps above 16 kHz, as in an MP3: gap step 12.0 dB, 6.0 dB over the Fake threshold (6.0 dB)',
      'Sharp cutoff where lossy encoders cut: step 24.7 dB, 14.7 dB over the Fake threshold (10.0 dB)',
      audioNote,
    ])
    expect(text).not.toContain(gapOnly)
    expect(text).not.toContain('analysed tracks')
  })

  // @RF-17 @RF-07 @RF-15 Detail panel of a lossy fake.
  it('shows shortfall in kHz before step in dB and keeps the estimated source', () => {
    const track = makeTrack({
      format: 'mp3',
      codec: 'mp3',
      verdict: 'fake',
      cutoffHz: 17_000,
      declaredBitrateKbps: 320,
      bitrateMode: 'cbr',
      estimatedSourceBitrateKbps: 128,
      evidence: [
        { kind: 'shortfall', value: 3500, threshold: 2500, margin: 1000 },
        { kind: 'cutoffStep', value: 40, threshold: 10, margin: 30 },
      ],
    })
    const text = panel(track, makeAlbum({ real: 9, fake: 3 }).summary)
    expectInOrder(text, [
      'Cutoff 17.0 kHz · 320 kbps · ≈128 kbps source · Fake',
      'Content ends 3.5 kHz below what its bitrate keeps: 1.0 kHz over the Fake threshold (2.5 kHz)',
      'Sharp cutoff where lossy encoders cut: step 40.0 dB, 30.0 dB over the Fake threshold (10.0 dB)',
      some(3, 12),
      audioNote,
    ])
  })

  // @RF-17 @RF-07 @RF-10 The album note of a fake track.
  it.each([
    [13, 0, 1, 0, only(14)],
    [0, 1, 1, 0, only(2)],
    [1, 0, 1, 3, only(2)],
    [9, 2, 3, 0, some(3, 14)],
    [1, 0, 11, 0, some(11, 12)],
    [0, 0, 12, 0, null],
    [0, 0, 12, 2, null],
    [0, 0, 1, 0, null],
    [0, 0, 1, 4, null],
  ] as const)('counts analysed tracks for real=%s suspicious=%s fake=%s errors=%s', (real, suspicious, fake, errors, note) => {
    const text = panel(stepTrack(), makeAlbum({ real, suspicious, fake, errors }).summary)
    expectInOrder(text, [step, ...(note ? [note] : []), audioNote])
    if (!note) expect(text).not.toContain('analysed tracks')
    expect(text).not.toContain(gapOnly)
  })

  // @RF-17 @RF-07 Margins are written with one decimal.
  it('rounds a small positive margin to zero', () => {
    const track = makeTrack({
      verdict: 'fake',
      cutoffHz: 16_000,
      evidence: [{ kind: 'cutoffStep', value: 10.04, threshold: 10, margin: 0.04 }],
    })
    const text = panel(track)
    expect(text).toContain('Sharp cutoff where lossy encoders cut: step 10.0 dB, 0.0 dB over the Fake threshold (10.0 dB)')
    expect(text).not.toContain('analysed tracks')
  })

  // @RF-17 @RF-07 Tracks that are not fake show no evidence and no album note.
  it('keeps the suspicious verdict and reason', () => {
    const track = makeTrack({ verdict: 'suspicious', cutoffHz: 21_500, reasons: ['highBandGaps'] })
    const text = panel(track, makeAlbum({ real: 13, suspicious: 1 }).summary)
    expect(text).toContain('Cutoff 21.5 kHz · Suspicious · Frequent gaps above 16 kHz, as in an MP3')
    expect(text).not.toMatch(/threshold|gap pattern|analysed tracks|These traces/)
  })

  // @RF-17 @RF-07 The evidence stays when the spectrogram cannot be shown.
  it('keeps evidence and context above the unavailable message', () => {
    const text = panel(stepTrack(), makeAlbum({ real: 13, fake: 1 }).summary, true)
    expectInOrder(text, [
      'Cutoff 16.0 kHz · Fake',
      step,
      only(14),
      audioNote,
      'Cannot show the spectrogram of this file',
    ])
  })
})

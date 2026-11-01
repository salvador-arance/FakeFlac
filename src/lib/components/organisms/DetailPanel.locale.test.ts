// DetailPanel and SpectrogramView in Spanish (@RF-13; specs/ui/translations.md sections 2 and 3,
// acceptance L4: "detail line, spectrogram label/loading/unavailable"). Numbers keep the decimal
// point, units and approximation sign; friendly track errors translate.
import { render, screen } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import { makeTrack } from '../../testing/fixtures'
import { installCanvasStub, makeSpectrogram } from '../../testing/spectrogram'
import type { CanvasStub } from '../../testing/spectrogram'
import DetailPanel from './DetailPanel.svelte'

let stub: CanvasStub
beforeEach(() => {
  stub = installCanvasStub()
})
afterEach(() => {
  stub.restore()
  setLocale('en')
})

const text = (container: HTMLElement) => (container.textContent ?? '').replace(/\s+/g, ' ').trim()

const lossy = () =>
  makeTrack({
    path: 'C:/Music/A/x.mp3',
    format: 'mp3',
    codec: 'mp3',
    bitDepth: null,
    verdict: 'suspicious',
    cutoffHz: 16000,
    declaredBitrateKbps: 320,
    bitrateMode: 'vbr',
    estimatedSourceBitrateKbps: 128,
    reasons: ['cutoffBelowDeclared'],
  })

describe('DetailPanel: Spanish (@RF-13)', () => {
  it('the detail line', () => {
    setLocale('es')
    const { container } = render(DetailPanel, { props: { track: lossy(), spectrogram: null, loading: true } })
    expect(text(container)).toContain(
      'Corte 16.0 kHz · 320 kbps VBR · Fuente de ≈128 kbps · Sospechoso · Corte inferior al esperado para su tasa de bits',
    )
  })

  it('the spectrogram label and loading text', () => {
    setLocale('es')
    const view = render(DetailPanel, {
      props: { track: makeTrack({ cutoffHz: 16000, verdict: 'fake' }), spectrogram: null, loading: true },
    })
    expect(text(view.container)).toContain('Cargando espectrograma…')
    view.unmount()
    render(DetailPanel, {
      props: {
        track: makeTrack({ cutoffHz: 16000, verdict: 'fake' }),
        spectrogram: makeSpectrogram(),
        loading: false,
      },
    })
    expect(screen.getByRole('img', { name: 'Espectrograma' })).toBeTruthy()
  })

  it('the unavailable spectrogram', () => {
    setLocale('es')
    const { container } = render(DetailPanel, {
      props: { track: makeTrack({ cutoffHz: 16000 }), spectrogram: null, loading: false, unavailable: true },
    })
    expect(text(container)).toContain('No se puede mostrar el espectrograma de este archivo')
  })

  it.each([
    [{ kind: 'corruptStream', detail: 'd' } as const, 'Flujo de audio dañado'],
    [{ kind: 'unreadable', detail: 'd' } as const, 'No se puede leer el archivo'],
    [{ kind: 'unsupportedFormat', detail: 'd' } as const, 'Formato de audio no compatible'],
    [{ kind: 'tooShort' } as const, 'Demasiado corto para analizar'],
  ])('the friendly error %o', (error, friendly) => {
    setLocale('es')
    const { container } = render(DetailPanel, {
      props: { track: makeTrack({ verdict: 'error', cutoffHz: null, error }), spectrogram: null, loading: false },
    })
    expect(text(container)).toBe(friendly)
  })

  it('re-renders the mounted detail in the new language', () => {
    const { container } = render(DetailPanel, { props: { track: lossy(), spectrogram: makeSpectrogram(), loading: false } })
    expect(text(container)).toContain('Cutoff 16.0 kHz · 320 kbps VBR · ≈128 kbps source · Suspicious')
    expect(screen.getByRole('img', { name: 'Spectrogram' })).toBeTruthy()
    setLocale('es')
    flushSync()
    expect(text(container)).toContain('Corte 16.0 kHz · 320 kbps VBR · Fuente de ≈128 kbps · Sospechoso')
    expect(screen.getByRole('img', { name: 'Espectrograma' })).toBeTruthy()
  })
})

// StatusBar in both languages (@RF-13 @RNF-10; specs/ui/translations.md section 5, acceptance L1
// and L4 of specs/ui/language-settings.md). Scenario "The two catalogues cover all user-facing
// states explicitly": "singular forms are selected exactly when their count is 1"; and the status
// part of "Existing mounted states translate after a successful Save".
//
// Composition (translations.md section 5): during analysis `done/total`, suspicious and fake with
// ` · `, both zeros kept; finished: tracks, real, suspicious, fake and errors, zeros included;
// cancelled uses `status.cancelled`; then cache fields, the shown count and the notice, as before.
import { render, screen } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import type { AnalysisPhase, StatusSummary } from '../../model'
import StatusBar from './StatusBar.svelte'

afterEach(() => setLocale('en'))

const summary = (o: Partial<StatusSummary> = {}): StatusSummary => ({
  total: 0,
  done: 0,
  real: 0,
  suspicious: 0,
  fake: 0,
  errors: 0,
  ...o,
})
const statusText = () => (screen.getByRole('status').textContent ?? '').replace(/\s+/g, ' ').trim()

function show(phase: AnalysisPhase, s: Partial<StatusSummary>, extra: Record<string, unknown> = {}) {
  return render(StatusBar, { props: { phase, summary: summary(s), ...extra } })
}

describe('StatusBar: singular forms in English (@RF-13)', () => {
  it.each([
    [{ total: 2, done: 2, real: 1, errors: 1 }, '2 tracks · 1 real · 0 suspicious · 0 fake · 1 error'],
    [{ total: 1, done: 1, real: 1 }, '1 track · 1 real · 0 suspicious · 0 fake · 0 errors'],
    [{ total: 1, done: 1, errors: 1 }, '1 track · 0 real · 0 suspicious · 0 fake · 1 error'],
    [{ total: 3, done: 3, real: 1, suspicious: 1, fake: 1 }, '3 tracks · 1 real · 1 suspicious · 1 fake · 0 errors'],
    [{ total: 0, done: 0 }, '0 tracks · 0 real · 0 suspicious · 0 fake · 0 errors'],
    [{ total: 31, done: 31, real: 28, suspicious: 2, fake: 1 }, '31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors'],
  ])('finished %o', (s, text) => {
    show('finished', s)
    expect(statusText()).toBe(text)
  })

  it('1 shown uses the singular key (same English words)', () => {
    show('finished', { total: 3, done: 3, real: 3 }, { shown: 1 })
    expect(statusText()).toBe('3 tracks · 3 real · 0 suspicious · 0 fake · 0 errors · 1 shown')
  })
})

describe('StatusBar: Spanish (@RF-13)', () => {
  it.each<[AnalysisPhase, Partial<StatusSummary>, string]>([
    ['ready', {}, 'Listo'],
    ['scanning', {}, 'Buscando archivos…'],
    ['analysing', { total: 3, done: 1, suspicious: 1 }, '1/3 · 1 sospechosa · 0 falsas'],
    ['analysing', { total: 3, done: 2, suspicious: 2, fake: 1 }, '2/3 · 2 sospechosas · 1 falsa'],
    ['analysing', { total: 3, done: 0 }, '0/3 · 0 sospechosas · 0 falsas'],
    ['finished', { total: 2, done: 2, real: 1, errors: 1 }, '2 pistas · 1 real · 0 sospechosas · 0 falsas · 1 error'],
    ['finished', { total: 1, done: 1, real: 1 }, '1 pista · 1 real · 0 sospechosas · 0 falsas · 0 errores'],
    [
      'finished',
      { total: 31, done: 31, real: 28, suspicious: 2, fake: 1 },
      '31 pistas · 28 reales · 2 sospechosas · 1 falsa · 0 errores',
    ],
    [
      'finished',
      { total: 7, done: 7, real: 2, suspicious: 1, fake: 1, errors: 3 },
      '7 pistas · 2 reales · 1 sospechosa · 1 falsa · 3 errores',
    ],
    ['cancelled', { total: 3, done: 1, real: 1 }, 'Cancelado · 1 de 3'],
  ])('%s %o', (phase, s, text) => {
    setLocale('es')
    show(phase, s)
    expect(statusText()).toBe(text)
  })

  it('shown count, singular and plural', () => {
    setLocale('es')
    const view = show('finished', { total: 3, done: 3, real: 3 }, { shown: 1 })
    expect(statusText()).toBe('3 pistas · 3 reales · 0 sospechosas · 0 falsas · 0 errores · 1 mostrada')
    view.unmount()
    show('cancelled', { total: 10, done: 3, real: 3 }, { shown: 2 })
    expect(statusText()).toBe('Cancelado · 3 de 10 · 2 mostradas')
  })

  it('cache fields, then the notice', () => {
    setLocale('es')
    show(
      'finished',
      { total: 2, done: 2, real: 2 },
      { cache: { fromCache: 2, status: 'unavailable' }, notice: 'aviso' },
    )
    expect(statusText()).toBe(
      '2 pistas · 2 reales · 0 sospechosas · 0 falsas · 0 errores · 2 de la caché · Caché de resultados no disponible · aviso',
    )
  })

  it('a degraded cache', () => {
    setLocale('es')
    show('cancelled', { total: 3, done: 1, real: 1 }, { cache: { fromCache: 0, status: 'degraded' } })
    expect(statusText()).toBe(
      'Cancelado · 1 de 3 · Error en la caché de resultados: algunos resultados no se han guardado',
    )
  })
})

describe('StatusBar: live locale change (@RF-13)', () => {
  it('re-renders the mounted status in the new language, English -> Spanish -> English', () => {
    show('finished', { total: 2, done: 2, real: 1, errors: 1 }, { shown: 1 })
    expect(statusText()).toBe('2 tracks · 1 real · 0 suspicious · 0 fake · 1 error · 1 shown')
    setLocale('es')
    flushSync()
    expect(statusText()).toBe('2 pistas · 1 real · 0 sospechosas · 0 falsas · 1 error · 1 mostrada')
    setLocale('en')
    flushSync()
    expect(statusText()).toBe('2 tracks · 1 real · 0 suspicious · 0 fake · 1 error · 1 shown')
  })

  it.each<[AnalysisPhase, Partial<StatusSummary>, string, string]>([
    ['ready', {}, 'Ready', 'Listo'],
    ['scanning', {}, 'Scanning…', 'Buscando archivos…'],
    ['analysing', { total: 3, done: 1, suspicious: 1 }, '1/3 · 1 suspicious · 0 fake', '1/3 · 1 sospechosa · 0 falsas'],
    ['cancelled', { total: 3, done: 1, real: 1 }, 'Cancelled · 1 of 3', 'Cancelado · 1 de 3'],
  ])('%s', (phase, s, en, es) => {
    show(phase, s)
    expect(statusText()).toBe(en)
    setLocale('es')
    flushSync()
    expect(statusText()).toBe(es)
  })
})

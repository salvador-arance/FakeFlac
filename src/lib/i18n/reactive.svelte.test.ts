// One reactive locale (@RF-13; specs/ui/language-settings.md section 1: "One reactive locale drives
// all mounted app-owned visible strings ... `getLocale()` reflects the active language and `t()`
// resolves the catalogue for it"; "No restart, remount of the page ... is needed").
//
// This file is a `.svelte.test.ts` module so it can use runes: a `$derived` that calls `t()` or
// `getLocale()` must update when the locale changes, without being created again.
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { getLocale, setLocale, t } from './index'

afterEach(() => setLocale('en'))

describe('reactive locale (@RF-13)', () => {
  it('a derived text follows setLocale', () => {
    const seen: string[] = []
    const cleanup = $effect.root(() => {
      const text = $derived(t('status.ready' as never))
      $effect(() => {
        seen.push(text)
      })
    })
    try {
      flushSync()
      expect(seen).toEqual(['Ready'])
      setLocale('es')
      flushSync()
      expect(seen).toEqual(['Ready', 'Listo'])
      setLocale('en')
      flushSync()
      expect(seen).toEqual(['Ready', 'Listo', 'Ready'])
    } finally {
      cleanup()
    }
  })

  it('a derived getLocale() follows setLocale', () => {
    const seen: string[] = []
    const cleanup = $effect.root(() => {
      const locale = $derived(getLocale())
      $effect(() => {
        seen.push(locale)
      })
    })
    try {
      flushSync()
      setLocale('es')
      flushSync()
      expect(seen).toEqual(['en', 'es'])
    } finally {
      cleanup()
    }
  })

  it('an interpolated text re-renders with its data in the new language', () => {
    const seen: string[] = []
    const cleanup = $effect.root(() => {
      $effect(() => {
        seen.push(t('status.exported' as never, { rows: 3, file: 'Álbum 日本語.csv' }))
      })
    })
    try {
      flushSync()
      setLocale('es')
      flushSync()
      expect(seen).toEqual([
        'Saved 3 rows to Álbum 日本語.csv',
        'Se han guardado 3 filas en Álbum 日本語.csv',
      ])
    } finally {
      cleanup()
    }
  })
})

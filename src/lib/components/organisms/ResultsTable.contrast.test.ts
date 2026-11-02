// ResultsTable verdict label contrast (@RNF-08 @RF-06; specs/ui/verdict-usability.md U3 and
// "Known U3 gap", applying the components.md text contrast criterion of at least 4.5:1).
// Scenario Outline: Verdict text remains readable in every row state (normal, hovered, selected).
//
// How the rendered colours are obtained: jsdom does no layout or painting, but it runs the CSS
// cascade. The test loads the real `tokens.css` and the scoped CSS that the Svelte compiler emits
// for every production component (same compiler and file names as the Vite plugin, so the scope
// classes match the rendered DOM), renders ResultsTable with synthetic rows, and reads the computed
// colour of the element that carries each verdict label and the backgrounds behind it, up to the
// first opaque one. No colour value is copied into this file. Harness adaptations, none of which
// changes a colour:
// - jsdom drops a `background` shorthand whose value is a `var()`; a shorthand whose value is a
//   single colour or `var()` is rewritten to `background-color` (images and gradients are not).
// - jsdom never matches `:hover`; the pointer is simulated by rewriting `:hover` to an attribute
//   set on the label and all its ancestors, the elements `:hover` matches under a pointer.
// - `var()` references are resolved against the element's inherited custom properties.
// The comparison uses the unrounded WCAG 2.x contrast ratio (relative luminance); ratios are
// rounded only in the failure message. The step "its whole label remains readable at a window size
// of 900 by 500" needs a rendered layout and stays in the manual U3 check.
import { render, screen, within } from '@testing-library/svelte'
import { compile } from 'svelte/compiler'
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest'
import { setLocale } from '../../i18n'
import type { Locale } from '../../i18n/messages'
import type { ResultRow, SortState, Verdict } from '../../model'
import { makeRow } from '../../testing/fixtures'
import tokens from '../../tokens/tokens.css?raw'
import ResultsTable from './ResultsTable.svelte'

const MIN_CONTRAST = 4.5
const HOVER_ATTR = 'data-contrast-hover'

// ---------------------------------------------------------------------------------------------
// The stylesheets the running app applies: tokens.css plus every production component's CSS.

const sources = import.meta.glob(['/src/**/*.svelte', '!/src/lib/tests/**'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

const LEVELS = ['/atoms/', '/molecules/', '/organisms/', '/templates/', '/pages/']
const level = (path: string) => {
  const i = LEVELS.findIndex((l) => path.includes(l))
  return i === -1 ? LEVELS.length : i
}

function adaptForJsdom(css: string): string {
  return css
    .replace(/:hover\b/g, `[${HOVER_ATTR}]`)
    .replace(/(^|[{;\s])background\s*:\s*([^;{}]+?)\s*(?=;|\})/g, (whole, pre: string, value: string) => {
      if (/url\(|gradient/i.test(value)) return whole
      if (/^none$/i.test(value)) return `${pre}background-color: transparent`
      if (/^(var\(.*\)|#[0-9a-f]{3,8}|rgba?\([^)]*\)|hsla?\([^)]*\)|[a-z]+)$/i.test(value)) {
        return `${pre}background-color: ${value}`
      }
      return whole
    })
}

let tokenSheet: HTMLStyleElement
let componentSheet: HTMLStyleElement

beforeAll(() => {
  const componentCss = Object.keys(sources)
    .sort((a, b) => level(a) - level(b) || a.localeCompare(b))
    .map((path) => compile(sources[path], { filename: path.slice(1), css: 'external' }).css?.code ?? '')
    .join('\n')
  tokenSheet = document.createElement('style')
  tokenSheet.textContent = adaptForJsdom(tokens)
  componentSheet = document.createElement('style')
  componentSheet.textContent = adaptForJsdom(componentCss)
  document.head.append(tokenSheet, componentSheet)
})

afterAll(() => {
  tokenSheet.remove()
  componentSheet.remove()
})

afterEach(() => setLocale('en'))

// ---------------------------------------------------------------------------------------------
// Colour resolution and WCAG 2.x contrast.

type Rgba = { r: number; g: number; b: number; a: number }

function resolveVars(value: string, el: Element, depth = 0): string {
  const start = value.indexOf('var(')
  if (start === -1) return value.trim()
  if (depth > 20) throw new Error(`var() nesting too deep in "${value}"`)
  let open = 0
  let end = start + 3
  let comma = -1
  for (; end < value.length; end++) {
    const ch = value[end]
    if (ch === '(') open++
    else if (ch === ')' && --open === 0) break
    else if (ch === ',' && open === 1 && comma === -1) comma = end
  }
  const inner = value.slice(start + 4, end)
  const name = (comma === -1 ? inner : value.slice(start + 4, comma)).trim()
  const fallback = comma === -1 ? undefined : value.slice(comma + 1, end).trim()
  let resolved = getComputedStyle(el).getPropertyValue(name).trim()
  if (!resolved) {
    if (fallback === undefined) throw new Error(`custom property ${name} is not defined for the element`)
    resolved = fallback
  }
  return resolveVars(value.slice(0, start) + resolved + value.slice(end + 1), el, depth + 1)
}

function parseColour(value: string): Rgba {
  const v = value.trim().toLowerCase()
  if (v === 'transparent') return { r: 0, g: 0, b: 0, a: 0 }
  if (v === 'white') return { r: 255, g: 255, b: 255, a: 1 }
  if (v === 'black') return { r: 0, g: 0, b: 0, a: 1 }
  const hex = v.match(/^#([0-9a-f]{3,8})$/)
  if (hex && [3, 4, 6, 8].includes(hex[1].length)) {
    const h = hex[1].length <= 4 ? [...hex[1]].map((c) => c + c).join('') : hex[1]
    const n = (i: number) => parseInt(h.slice(i, i + 2), 16)
    return { r: n(0), g: n(2), b: n(4), a: h.length === 8 ? n(6) / 255 : 1 }
  }
  const fn = v.match(/^rgba?\(([^)]*)\)$/)
  if (fn) {
    const parts = fn[1].split(/[\s,/]+/).filter(Boolean)
    const channel = (p: string) => (p.endsWith('%') ? (parseFloat(p) * 255) / 100 : parseFloat(p))
    const alpha = parts[3] === undefined ? 1 : parts[3].endsWith('%') ? parseFloat(parts[3]) / 100 : parseFloat(parts[3])
    return { r: channel(parts[0]), g: channel(parts[1]), b: channel(parts[2]), a: alpha }
  }
  throw new Error(`unsupported colour value "${value}"`)
}

const over = (top: Rgba, below: Rgba): Rgba => ({
  r: top.r * top.a + below.r * (1 - top.a),
  g: top.g * top.a + below.g * (1 - top.a),
  b: top.b * top.a + below.b * (1 - top.a),
  a: 1,
})

function luminance({ r, g, b }: Rgba): number {
  const lin = (c: number) => {
    const s = c / 255
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

function contrast(a: Rgba, b: Rgba): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

const hex = ({ r, g, b }: Rgba) =>
  '#' + [r, g, b].map((c) => Math.round(c).toString(16).padStart(2, '0')).join('')

/** The background painted behind `el`: its own and its ancestors' backgrounds, composited. */
function backgroundBehind(el: Element): Rgba {
  const layers: Rgba[] = []
  for (let node: Element | null = el; node; node = node.parentElement) {
    const raw = getComputedStyle(node).backgroundColor
    if (!raw) continue
    const colour = parseColour(resolveVars(raw, node))
    if (colour.a === 0) continue
    layers.push(colour)
    if (colour.a >= 1) break
  }
  const base = layers.pop()
  if (!base || base.a < 1) throw new Error('no opaque background behind the verdict label')
  return layers.reduceRight((below, top) => over(top, below), base)
}

/** The colour the label text is painted with, composited over its background. */
function textColour(el: Element, background: Rgba): Rgba {
  const colour = parseColour(resolveVars(getComputedStyle(el).color, el))
  return colour.a >= 1 ? colour : over(colour, background)
}

/** Component rules (not tokens) that apply to `el`: proves the scoped styles reached the DOM. */
function componentRulesMatching(el: Element): string[] {
  const rules = [...(componentSheet.sheet?.cssRules ?? [])].filter(
    (rule): rule is CSSStyleRule => rule instanceof CSSStyleRule,
  )
  return rules.filter((rule) => {
    try {
      return el.matches(rule.selectorText)
    } catch {
      return false
    }
  }).map((rule) => rule.selectorText)
}

// ---------------------------------------------------------------------------------------------
// Synthetic table: one row per verdict.

const VERDICTS: Verdict[] = ['real', 'suspicious', 'fake', 'error']
const LABELS: Record<Locale, Record<Verdict, string>> = {
  en: { real: 'Genuine', suspicious: 'Suspicious', fake: 'Fake', error: 'Error' },
  es: { real: 'Real', suspicious: 'Sospechoso', fake: 'Falso', error: 'Error' },
}
const ROWS: ResultRow[] = [
  makeRow(0, { path: 'C:/Music/A/a.flac', verdict: 'real', cutoffHz: 22050 }),
  makeRow(1, { path: 'C:/Music/A/b.flac', verdict: 'suspicious', cutoffHz: 19000, reasons: ['gradualRolloff'] }),
  makeRow(2, { path: 'C:/Music/A/c.flac', verdict: 'fake', cutoffHz: 16000 }),
  makeRow(3, {
    path: 'C:/Music/A/d.flac',
    verdict: 'error',
    cutoffHz: null,
    bitDepth: null,
    error: { kind: 'corruptStream', detail: 'invalid frame header at byte 4096' },
  }),
]
const byFile: SortState = { column: 'file', direction: 'asc' }

type RowState = 'normal' | 'hovered' | 'selected'

/** Renders the table with the verdict's row in `state` and returns the element showing its label. */
function labelInRow(locale: Locale, verdict: Verdict, state: RowState): { label: HTMLElement; row: HTMLElement } {
  setLocale(locale)
  const index = VERDICTS.indexOf(verdict)
  render(ResultsTable, {
    props: { rows: ROWS, selectedIndex: state === 'selected' ? index : null, sort: byFile },
  })
  const row = screen
    .getAllByRole('row')
    .find((r) => within(r).queryAllByRole('gridcell')[1]?.textContent?.trim() === `${'abcd'[index]}.flac`)
  if (!row) throw new Error(`row for ${verdict} not rendered`)
  expect(row.getAttribute('aria-selected')).toBe(state === 'selected' ? 'true' : 'false')
  const label = within(row).getByText(LABELS[locale][verdict])
  if (state === 'hovered') {
    for (let node: Element | null = label; node; node = node.parentElement) node.setAttribute(HOVER_ATTR, '')
  }
  return { label, row }
}

// ---------------------------------------------------------------------------------------------

describe('Scenario Outline: Verdict text remains readable in every row state (@RNF-08 @RF-06)', () => {
  // Guard against a false pass: if the component styles did not reach the DOM, every label would
  // fall back to the body text colour on the body background and pass trivially.
  it('harness: the rendered row and verdict cell receive their own component styles', () => {
    for (const verdict of VERDICTS) {
      const { label, row } = labelInRow('en', verdict, 'normal')
      expect(componentRulesMatching(row).length, `no component rule applies to the ${verdict} row`).toBeGreaterThan(0)
      const cell = label.closest('[role="gridcell"]')
      const cellChain: Element[] = []
      for (let node: Element | null = label; node && node !== cell; node = node.parentElement) cellChain.push(node)
      expect(
        cellChain.some((node) => componentRulesMatching(node).length > 0),
        `no component rule applies to the ${verdict} label or its verdict cell`,
      ).toBe(true)
    }
  })

  const cases = (['en', 'es'] as Locale[]).flatMap((locale) =>
    (['normal', 'hovered', 'selected'] as RowState[]).flatMap((state) =>
      VERDICTS.map((verdict) => [state, verdict, locale] as const),
    ),
  )

  // @RNF-08 @RF-06 Scenario Outline "Verdict text remains readable in every row state":
  // "Then its text contrast against the rendered background is at least 4.5 to 1"
  it.each(cases)('%s row: the %s label (%s) has text contrast of at least 4.5:1', (state, verdict, locale) => {
    const { label } = labelInRow(locale, verdict, state)
    const background = backgroundBehind(label)
    const foreground = textColour(label, background)
    const ratio = contrast(foreground, background)
    expect(
      ratio,
      `${verdict} label "${LABELS[locale][verdict]}" ${hex(foreground)} on ${state} row ${hex(background)}: ` +
        `contrast ${ratio.toFixed(2)}:1 is below ${MIN_CONTRAST}:1`,
    ).toBeGreaterThanOrEqual(MIN_CONTRAST)
  })
})

// Spectrogram palette token (docs/architecture/frontend.md, Tokens). The canvas needs the colours
// as numbers, so this token lives in TypeScript rather than in tokens.css.
//
// Colour stops from quiet (byte 0 = bottom of `dbRange`) to loud (byte 255 = top of `dbRange`):
// near-black, deep purple, red, orange, pale yellow. Monotonic in brightness, so every byte maps to
// a distinct colour and the picture stays readable without hue.

/** `[position 0..1, '#rrggbb']`, sorted by position; the first is 0 and the last is 1. */
export const SPECTROGRAM_PALETTE: readonly (readonly [number, string])[] = [
  [0, '#000004'],
  [0.25, '#3b0f70'],
  [0.5, '#b5367a'],
  [0.75, '#fb8861'],
  [1, '#fcfdbf'],
]

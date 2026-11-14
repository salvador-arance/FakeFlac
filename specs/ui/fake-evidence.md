---
id: SPEC-UI-FAKE-EVIDENCE
requirements: [RF-17, RF-06, RF-07, RF-10, RF-13, RNF-08, RNF-09]
status: proposed
---

# Evidence of a `Fake` verdict in the main screen (RF-17)

> Where and how the interface shows the evidence of a `Fake` track (`TrackAnalysis.evidence`, contract version 9, `specs/contracts/commands.md` "Fake evidence (version 9)") and the album note. The rule behind both, and the G1 questions Q1–Q15, are in `specs/detection/fake-evidence.md`. Layout, states and interaction stay those of `main-screen.md`, `lossy-formats.md` and `album-summary.md`; components follow `components.md` (atomic design, ADR-0014: a story per state and a component test before use, S6). Presentational components get data through props; only `AnalysisPage` touches the store and the API client. Scenarios: `specs/features/fake-evidence.feature` (tag `@RF-17`).

## 1. Where it appears

| Place | What | Since |
|---|---|---|
| Detail panel (`DetailPanel`), below the verdict line | One line per evidence item, the gap-only sentence, the album note (sections 2 and 3) | RF-17 |
| State cell tooltip (`VerdictCell` in `ResultsTable`) | The evidence item sentences, as reasons are shown there today (section 4) | RF-17 |
| Albums panel, table columns, status bar, CSV | Nothing (fake-evidence.md Q8, Q11) | — |

The verdict line of the detail panel ("Cutoff 16.0 kHz · Fake", and its lossy form of `lossy-formats.md`) does **not** change: the evidence comes on the lines below it. A `Fake` row keeps its colour, icon and label (RNF-08).

## 2. Detail panel: evidence lines

When the selected track's verdict is `fake` and its `evidence` is not empty, the panel shows, right below the verdict line and above the spectrogram (or its loading or unavailable state), in this order:

1. **One line per item**, in the order of `evidence` (`gapStep` before `cutoffStep`; `shortfall` before `cutoffStep`), with the text `evidence.<kind>` of section 6.
2. **The gap-only sentence** `evidence.gapOnly`, when `evidence` is exactly one `gapStep` item. It says that the evidence is the gap pattern at 16 kHz, not the cutoff: a gap-only `Fake` often reports a cutoff near or at full band (fake-evidence.md section 1).
3. **The album note** of section 3, when one applies.
4. **The audio note** `evidence.audioNote`, always, as the last line. It says what the evidence can and cannot tell: the traces are in the audio, and a genuine release mastered from lossy audio shows them too (fake-evidence.md section 1, "Authentic releases", and Q16).

Otherwise (not `fake`, or `fake` with an empty `evidence`, which the backend never sends) the panel shows no evidence line, no gap-only sentence, no album note and no audio note, exactly as before RF-17.

**Numbers in the texts.**
- `{value}`, `{threshold}` and `{margin}` of `gapStep` and `cutoffStep`: the number in dB with one decimal.
- `{value}`, `{threshold}` and `{margin}` of `shortfall`: the number in Hz divided by 1000, in kHz with one decimal, as `detail.cutoff` writes kHz.
- Every number with plain digits, the decimal point `.` in both languages and no grouping (`translations.md` section 3); a negative value (only possible with an unusual params file) keeps its `-`. A margin below 0.05 is written `0.0`. Values exactly halfway between two decimals are not specified; tests avoid them.
- `{fake}` and `{tracks}` of the album note: plain integers.

**Layout.** Each line wraps at the panel's width (no ellipsis), so the whole text is readable; the spectrogram takes the remaining height. Item lines use `--color-text`; the gap-only sentence, the album note and the audio note use `--color-text-muted`. Both tokens keep the ≥ 4.5:1 contrast of `components.md` ("Accessibility").

**Examples** (English; Spanish in section 6):

- Gap only, cutoff near full band, the only `Fake` of its album:
  ```
  Cutoff 22.0 kHz · Fake
  Gaps above 16 kHz, as in an MP3: gap step 9.6 dB, 3.6 dB over the Fake threshold (6.0 dB)
  The evidence is the gap pattern at 16 kHz, not the cutoff.
  The only Fake of the 14 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram.
  These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio.
  ```
- Both kinds, every track of the album `Fake` (no note):
  ```
  Cutoff 16.0 kHz · Fake
  Gaps above 16 kHz, as in an MP3: gap step 12.0 dB, 6.0 dB over the Fake threshold (6.0 dB)
  Sharp cutoff where lossy encoders cut: step 24.7 dB, 14.7 dB over the Fake threshold (10.0 dB)
  These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio.
  ```
- Lossy (L3):
  ```
  Cutoff 17.0 kHz · 320 kbps · ≈128 kbps source · Fake
  Content ends 3.5 kHz below what its bitrate keeps: 1.0 kHz over the Fake threshold (2.5 kHz)
  Sharp cutoff where lossy encoders cut: step 40.0 dB, 30.0 dB over the Fake threshold (10.0 dB)
  These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio.
  ```

## 3. Album note

Shown only for a selected `fake` track, below its evidence lines, from the summary of its album: the `summary` of the latest `AlbumUpdate` whose `index` equals the row's `albumIndex` (`ui/album-summary.md`; the store already keeps it). With `f = summary.fake` and `n = summary.real + summary.suspicious + summary.fake` (errors left out):

| Case | Note |
|---|---|
| `n ≤ 1` | none |
| `f = n` | none |
| `f = 1` and `n ≥ 2` | `evidence.albumOnly` with `{tracks}` = `n` |
| `2 ≤ f < n` | `evidence.albumSome` with `{fake}` = `f`, `{tracks}` = `n` |

- **Live.** During a run the note follows the album's latest update, like the Albums panel (it may appear, change or disappear as the album's tracks arrive; same flush as the table, `main-screen.md` "Screen states", row Analysing). After a cancel or an `INTERNAL` failure it uses the counts kept.
- **Filters and sort** do not change it: the counts are those of the whole album in the run (`sort-filter.md` section 6).
- **It changes nothing else**: not the track's verdict or row, not the album's item in the Albums panel, not the status bar.
- The panel never computes a verdict: comparing `f` with `n` only selects one of two texts (fake-evidence.md section 4, Q5).

## 4. State cell tooltip

`VerdictCell` shows a tooltip on hover and focus with the reason texts of a track (`components.md`). For a `fake` track with evidence, the tooltip shows the item sentences of section 2 (one per item, in order), joined as reasons are joined in that tooltip today. It does not show the gap-only sentence, the album note or the audio note. The State cell of `Real`, `Suspicious` and `Error` rows, and every `VerdictCell` of the Albums panel (which gets no evidence), are unchanged.

## 5. Component changes

| Component | Level | Change | States / variants to add (story + test) |
|---|---|---|---|
| `VerdictCell` | molecule | New optional prop `evidence?: FakeEvidence[]` (default: none). When `verdict` is `fake` and `evidence` is not empty, the tooltip shows the item sentences (section 4); otherwise unchanged | fake with a `gapStep` item · fake with `gapStep` and `cutoffStep` · fake with `shortfall` and `cutoffStep` |
| `DetailPanel` | organism | New optional prop `album?: AlbumSummary \| null` (default: none). Evidence lines, gap-only sentence, album note and audio note (sections 2 and 3) between the verdict line and the spectrogram, in the states spectrogram, loading and unavailable. Without `album` there is no note | fake gap only, cutoff near full band · fake step only · fake both · fake lossy · album note "only" · album note "some" · whole-album fake (no note) · single-track album (no note) · fake with the spectrogram unavailable · Spanish |
| `ResultsTable` | organism | Passes `analysis.evidence` to the State cell's `VerdictCell` | a fake row with evidence (story) |
| `AlbumSummaryList` | organism | No change (`VerdictCell` without evidence) | — |
| `AnalysisPage` | page | Passes `DetailPanel` the `summary` of the latest `AlbumUpdate` of the selected row's `albumIndex` (`null` when no row is selected) | — |

- `model.ts` re-exports `FakeEvidence` and `EvidenceKind` from the generated bindings (components may not import `src/lib/api`). No type is hand-written (S3).
- One pure formatter in `src/lib/` (not a component) turns an item into its sentence for the current language, so the tooltip and the panel always show the same text. Its name and place are left to the implementation.
- The store does not change: rows already carry the whole `TrackAnalysis`, and the albums are already kept per index (`ui/album-summary.md`).
- No new token, icon, prop of `MainLayout`, IPC command or event.

## 6. Exact new translations

These keys supplement `translations.md` (the same convention: braces are literal interpolation parameters; numbers formatted as in section 2).

| Key | English | Spanish |
|---|---|---|
| `evidence.gapStep` | Gaps above 16 kHz, as in an MP3: gap step {value} dB, {margin} dB over the Fake threshold ({threshold} dB) | Huecos por encima de 16 kHz, como en un MP3: escalón de huecos de {value} dB, {margin} dB por encima del umbral de Falso ({threshold} dB) |
| `evidence.cutoffStep` | Sharp cutoff where lossy encoders cut: step {value} dB, {margin} dB over the Fake threshold ({threshold} dB) | Corte abrupto donde cortan los codificadores con pérdida: escalón de {value} dB, {margin} dB por encima del umbral de Falso ({threshold} dB) |
| `evidence.shortfall` | Content ends {value} kHz below what its bitrate keeps: {margin} kHz over the Fake threshold ({threshold} kHz) | El contenido termina {value} kHz por debajo de lo que conserva su tasa de bits: {margin} kHz por encima del umbral de Falso ({threshold} kHz) |
| `evidence.gapOnly` | The evidence is the gap pattern at 16 kHz, not the cutoff. | La prueba es el patrón de huecos a partir de 16 kHz, no el corte. |
| `evidence.albumOnly` | The only Fake of the {tracks} analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram. | Es la única pista falsa de las {tracks} analizadas en este álbum. Las transcodificaciones suelen afectar a álbumes enteros: revisa su espectrograma. |
| `evidence.albumSome` | {fake} of the {tracks} analysed tracks in this album are Fake. Transcodes usually affect whole albums: check their spectrograms. | {fake} de las {tracks} pistas analizadas en este álbum son falsas. Las transcodificaciones suelen afectar a álbumes enteros: revisa sus espectrogramas. |
| `evidence.audioNote` | These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio. | Estas huellas están en el propio audio. No distinguen un archivo convertido de una edición auténtica cuyo máster se hizo con audio con pérdida. |

`evidence.albumSome` is used only when `{fake}` is at least 2, so it needs no singular. "Fake" / "Falso" is the verdict label of `verdict.fake`. "16 kHz" is literal (fake-evidence.md Q12).

## 7. Acceptance and manual checks

- **Automated:** the interface scenarios of `fake-evidence.feature` (detail panel lines in English and Spanish, gap-only sentence, album note table, tooltip, unchanged rows and Albums panel, empty state), the component tests and stories of section 5, and the catalogue test with the keys of section 6.
- **MU1 (layout, release build, 900 × 500, English and Spanish):** with the default detail panel height, a `fake` track with two items and an album note shows every line in full, wrapped, and the spectrogram stays visible below them; the lines do not overlap the spectrogram or the status bar. At the minimum panel height (160 px) the text stays readable (the spectrogram may be small).
- **M1 of fake-evidence.md** (owner walkthrough on the owner's library) covers comprehension.

## 8. Accessibility

The evidence lines, the gap-only sentence and the album note are plain text in the panel, read after the verdict line in this order. The tooltip keeps the `VerdictCell` behaviour (hover and focus). The verdict stays icon + text + colour (RNF-08); no information is in colour alone.

---
id: SPEC-UI-LOSSY-H2
requirements: [RF-06, RF-14, RF-15, RNF-08]
status: proposed
---

# Lossy formats in the main screen (H2)

> What the main screen shows for MP3 and M4A files (contract version 3, `specs/contracts/commands.md`, "Lossy formats"): the declared and the real (estimated source) bitrate in two new table columns and in the detail line, the codec where the container is ambiguous, and the texts of the new reason and errors. Layout, states and interaction stay those of `main-screen.md`; components follow `components.md` (atomic design, ADR-0014: a story per state and a component test before use, S6). Rules behind the values: `specs/detection/lossy-source.md`.

## Table (`ResultsTable`, `main-screen.md`)

Columns, in order: State · File · Folder · Format · Samplerate · Bits · Duration · Cutoff · **Bitrate** · **Real bitrate** (the two new columns follow the reference layout, `docs/architecture/frontend.md`).

| Column | Cell | Lossless rows | Sort |
|---|---|---|---|
| Format | `format` in upper case (`MP3`, `M4A`, `FLAC`, `WAV`); an M4A with codec `alac` shows `M4A (ALAC)` (`format.m4aAlac`) | as H1 | by the text |
| Bitrate | `BitrateValue` with `declaredBitrateKbps` and `bitrateMode` | "—" (`BitrateValue` state none, accessible label "No bitrate") | numeric; "—" cells last |
| Real bitrate | `BitrateValue` with `estimatedSourceBitrateKbps`, `approx` = the estimate is lower than the declared bitrate. Its column header carries the tooltip `help.realBitrate` | "—" (state none) | numeric; "—" cells last |

`ColumnHeader` (`components.md`) gains an optional prop `tooltip?: string`, shown as the header's tooltip; only the Real bitrate column sets it.

## Molecule `BitrateValue`

| Props | Events | States / variants | Notes |
|---|---|---|---|
| `kbps: number \| null`, `mode?: BitrateMode \| null`, `approx?: boolean` | — | value ("320 kbps") · vbr ("245 kbps VBR") · abr ("192 kbps ABR") · approx ("≈128 kbps") · none ("—", with an accessible label "No bitrate") | Plain text, no colour of its own: the verdict carries the colour (RNF-08). Uses `bitrate.kbps`, `bitrate.vbr`, `bitrate.abr`, `bitrate.approx`, `bitrate.none`. Level: molecule (a formatted value, like `CutoffValue`) |

## Detail line (`DetailPanel`, `components.md`)

H1: "Cutoff 16.0 kHz · Fake", then " · <reason>" when the track has reasons. For a lossy track, two parts follow the cutoff:

- " · {declared} kbps" (`detail.declared`), with " VBR" or " ABR" appended for those modes;
- " · ≈{estimate} kbps source" (`detail.source`), only when the estimate is lower than the declared bitrate.

Examples: "Cutoff 17.0 kHz · 320 kbps · ≈128 kbps source · Fake"; "Cutoff 18.5 kHz · 320 kbps · ≈192 kbps source · Suspicious · Cutoff lower than expected for its bitrate"; "Cutoff 20.4 kHz · 320 kbps · Genuine". Lossless tracks keep the H1 line.

## Texts (keys in `src/lib/i18n/`; English is the default, Spanish falls back to English until RF-13)

New: `column.bitrate` Bitrate · `column.realBitrate` Real bitrate · `bitrate.kbps` {kbps} kbps · `bitrate.vbr` VBR · `bitrate.abr` ABR · `bitrate.approx` ≈{kbps} kbps · `bitrate.none` — (accessible label: No bitrate) · `format.m4aAlac` M4A (ALAC) · `detail.declared` {kbps} kbps · `detail.source` ≈{kbps} kbps source · `reason.cutoffBelowDeclared` Cutoff lower than expected for its bitrate · `error.unsupportedFormat` Unsupported audio format (shown for HE-AAC and MP3 layers I/II, and for any other `unsupportedFormat` error) · `help.realBitrate` The real bitrate is an estimate from the cutoff, in terms of the file's own codec. Files from sources with little treble, from encoders without a low-pass, or re-encoded from a close bitrate may not be detected.

Changed: `error.NO_AUDIO_FOUND` "No WAV or FLAC files in this folder" → **"No supported audio files in this folder"** (`main-screen.md`).

Where they appear: `reason.cutoffBelowDeclared` in the `VerdictCell` tooltip and in the detail line, like every reason; `help.realBitrate` as the tooltip of the Real bitrate column header (it explains the limits of `lossy-source.md` section 8 in plain words); `error.unsupportedFormat` in the detail panel and the `VerdictCell` tooltip of an error row.

## Scenarios

`specs/features/decode-lossy.feature`: "A lossy row has the same fields as a lossless row", "A variable bitrate is marked in the table", "BitrateValue states", "Table cells of a stubbed lossy row", "The Real bitrate column explains the estimate" (`@RF-06`), "Detail line of a lossy track" and "An unsupported lossy file shows its error text" (`@RF-07`).

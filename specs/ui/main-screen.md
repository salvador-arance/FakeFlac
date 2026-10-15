---
id: SPEC-UI-MAIN-SCREEN
requirements: [RF-01, RF-05, RF-06, RF-07, RNF-08, RNF-10]
status: proposed
---

# Main screen (H1)

> One dark window modelled on the reference tool (`docs/reference/`, `docs/architecture/frontend.md`): custom title bar, icon toolbar, results table, detail panel with the spectrogram, status bar. H1 has no folder tree, no search, no verdict filters and no export: those arrive in H2 and the layout keeps their place.

## Layout (`MainLayout` template, rendered by `AnalysisPage`)

```
┌ TitleBar ─────────────────────────────────────────── ─ □ ✕ ┐  32 px, draggable
├ Toolbar ───────────────────────────────────────────────────┤  48 px
│ [Open folder] [Cancel]                                      │
├ ResultsTable ──────────────────────────────────────────────┤  flexible (min 120 px)
│ State · File · Folder · Format · Samplerate · Bits · Duration · Cutoff │
├ Splitter (drag to resize) ─────────────────────────────────┤
├ DetailPanel / SpectrogramView ─────────────────────────────┤  default 260 px (min 160)
├ StatusBar ─────────────────────────────────────────────────┤  24 px
│ [progress] 18/31 · 2 suspicious · 1 fake                    │
└────────────────────────────────────────────────────────────┘
```

Minimum window: 900 × 500 (already in `tauri.conf.json`).

## Screen states

| State | Table | Detail panel | Toolbar | Status bar |
|---|---|---|---|---|
| Empty (start) | Hidden; a centred hint "Drop a folder here or click Open folder" (`FolderDropZone`) | Hidden | Open enabled, Cancel disabled | "Ready" |
| Dragging over | Overlay highlights the whole window | — | — | — |
| Scanning | Empty table | Hidden | Open disabled, Cancel enabled | Indeterminate progress, "Scanning…" |
| Analysing | Rows appear as `trackDone` arrives (batched ≤ 1 update / 100 ms) | Spectrogram of the selected row, if any | Open disabled, Cancel enabled | Determinate progress `done/total` + counters |
| Finished | All rows | Selected row | Open enabled, Cancel disabled | "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors" |
| Cancelled | Rows done so far | Selected row | Open enabled | "Cancelled · 18 of 31" |
| Error (`AppError`) | Previous content kept | — | Open enabled | Error message in the status bar (e.g. "No WAV or FLAC files in this folder") |

## Interaction

- Rows are sorted by file path by default; clicking a column header sorts by it (asc → desc).
- Single selection. ↑/↓ move the selection; Home/End jump. The first row is not auto-selected.
- Selecting a row requests its spectrogram (`get_spectrogram`); a loading spinner shows in the panel; responses for a row that is no longer selected are discarded.
- Double-clicking a row does nothing in H1 (player is out of v1).
- The window title bar supports drag, double-click to maximise, and the three buttons.

## Texts (English and Spanish keys in `src/lib/i18n/`; English is the H1 default)

`app.title` FakeFlac · `toolbar.open` Open folder · `toolbar.cancel` Cancel · `empty.hint` Drop a folder here or click Open folder · `status.ready` Ready · `status.scanning` Scanning… · `verdict.real` Genuine · `verdict.suspicious` Suspicious · `verdict.fake` Fake · `verdict.error` Error · `reason.lowHighFrequencyContent` Little high-frequency content: cannot confirm · `reason.gradualRolloff` High frequencies fade out gradually · `reason.ambiguousCutoff` Sharp cutoff at a high frequency · `error.tooShort` Too short to analyse · `error.corruptStream` Corrupt stream · `error.unreadable` Cannot read the file · `error.NO_AUDIO_FOUND` No WAV or FLAC files in this folder.

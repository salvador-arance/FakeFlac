# Frontend design (Svelte 5, atomic design) and UI style

> Svelte 5 + TypeScript + Vite. Atomic design levels with enforced import rules. The UI is a classic dark desktop application modelled on the layout and handling of the reference tool (`docs/reference/`), with its own assets. No component calls `invoke()`: everything goes through a generated typed client.

## Levels (ADR-0014)

| Level | What | May contain | Knows app state | Examples |
|---|---|---|---|---|
| Tokens | Design values | — | No | verdict colours, spacing, typography, spectrogram palette |
| Atoms | Indivisible element | tokens | No | `Button`, `Icon`, `Text`, `Badge`, `ProgressBar`, `Input`, `Tooltip`, `Splitter`, `TreeNode`, `Spinner` |
| Molecules | Small group with one function | atoms | No | `VerdictCell` (icon+text+colour), `SearchField`, `ToolbarButton`, `ToolbarGroup`, `ColumnHeader` |
| Organisms | Section with its own meaning | molecules, atoms | Via props/events | `Toolbar`, `FolderTree`, `ResultsTable` (with column chooser), `DetailPanel`/`SpectrogramView`, `StatusBar` |
| Templates | Layout without real data | organisms | No | `MainLayout`, `EmptyLayout` |
| Pages | Template + real data + store connection | templates | **Yes (only place)** | `AnalysisPage`, `SettingsPage` |

Rules (linted by `scripts/lint-arch.mjs`): see `dependencies.md`. Every atom/molecule has a spec in `specs/ui/` and a Storybook story before it is built (S6).

```
src/lib/{tokens, components/{atoms,molecules,organisms,templates}, pages, stores, api, i18n}
```

## Verdict tokens (RNF-08)

| Verdict | Colour token | Icon | Text (es / en) |
|---|---|---|---|
| Real | `--verdict-real` (green) | ✔ | Real / Genuine |
| Suspicious | `--verdict-suspicious` (amber) | ! | Sospechoso / Suspicious |
| Fake | `--verdict-fake` (red) | ✖ | Falso / Fake |
| Error | `--verdict-error` (grey) | ⚠ | Error / Error |

The verdict never depends on colour alone.

## UI style (RNF-10, ADR-0027)

Reference screenshots: `docs/reference/ftf-main-window-1.png`, `ftf-main-window-2.png`. Observed and decided:

```
┌─ FakeFlac ─────────────────────────────────────────────────── ─ □ ✕ ┐
│ [📂][〰 Analyze][✕ Remove][💾]  [👎][👆][👍][✋]  …  [⚙][?][⏏]  [Search (Ctrl+F)…] │
├──────────┬──────────────────────────────────────────────────────────┤
│ My PC    │ State Format File  Folder  Hz  Bits  Duration  Cutoff  Bitrate  Real bitrate │
│  ▸ D:    │ ✖ Fake  MP3  a.mp3 Album A 44100 —  03:01    16.0 k  320      ≈128          │
│   ▸ …    │ ✔ Real  FLAC b.flac…                                                        │
├──────────┴──────────────────────────────────────────────────────────┤
│ Spectrogram of the selected track, with cutoff line + explanation   │  ← takes the reference's player area
├─────────────────────────────────────────────────────────────────────┤
│ ████████░░░░ 18/31 · 2 suspicious · 1 fake                          │
└─────────────────────────────────────────────────────────────────────┘
```

| Observed feature | Decision |
|---|---|
| Theme | **Dark** (near-black grey background, high-contrast rows, orange accent). Dark by default; light optional (H2) |
| Top bar | Large icon buttons in groups: open folder · analyse · remove from list · save/export · verdict filters (thumb down / middle / up) · settings · help · exit. Tooltips. **Search box** at the right, shortcut `Ctrl+F` |
| Left panel | **Folder tree** ("My PC" → drives → folders); selecting a folder lists its files. Collapsible |
| Central table | Very dense, many resizable and sortable columns, horizontal scroll. First column *State* with the verdict icon. Default columns: State, Format, File, Folder, Samplerate, Bits, Duration, Size, Cutoff, Bitrate, **Real bitrate**; optional: Artist, Album, Title, Genre, Date, Extension (tags via `lofty`) |
| Bottom area | The reference has a **player** here. The player is out of v1, so the **spectrogram** and its explanation take that space; a player is a possible extension |
| Input | Drag and drop over the whole window; the "drop here" hint only in the empty state |
| Window | Own dark title bar with minimise/maximise/close (Tauri without native decorations); size and position remembered |

Not in v1 (the reference has them): Silence seconds, Clipping %, Max clipping, Comment, Real duration, store links. They can be added as new RF without changing the table structure.

**Not copied:** the reference tool's logo, name, icons, illustrations and texts. All graphic assets are our own (D12; the repository is public).

## Table performance

Row virtualisation for folders with thousands of tracks; sorting and filtering on the client over the store. Rows are inserted in batches (at most once every 100 ms) so the UI stays responsive during analysis.

---
id: SPEC-UI-COMPONENTS-H1
requirements: [RF-01, RF-05, RF-06, RF-07, RNF-08, RNF-10]
status: proposed
---

# Components for H1 (atomic design, ADR-0014)

> Every component listed here gets a Storybook story per state and a component test before it is used in a page (S6). Presentational components receive data through props and emit events; only `AnalysisPage` touches stores and the API client. Styling only through tokens (`src/lib/tokens/tokens.css`).

## Atoms

| Component | Props | Events | States / variants | Notes |
|---|---|---|---|---|
| `Icon` | `name: IconName`, `size?: 16 \| 24 \| 32`, `label?: string` | — | — | Own SVG set (no copied icons). `label` → `aria-label`; without it the icon is decorative (`aria-hidden`) |
| `IconButton` | `icon: IconName`, `label: string`, `disabled?: boolean`, `size?: 'toolbar' \| 'small'` | `click` | normal · hover · pressed · disabled · focus-visible | Tooltip with `label`; toolbar size 40×40 |
| `ProgressBar` | `value?: number` (0–1; absent = indeterminate) | — | determinate · indeterminate | `role="progressbar"`, `aria-valuenow` |
| `Spinner` | `size?: 16 \| 24` | — | — | |
| `Splitter` | `orientation: 'horizontal'` | `resize(delta: number)` | idle · dragging · focus | Keyboard: ↑/↓ move 10 px |

## Molecules

| Component | Props | Events | States | Notes |
|---|---|---|---|---|
| `VerdictCell` | `verdict: Verdict`, `reasons?: SuspicionReason[]` | — | real · suspicious · fake · error | Icon + text + colour token; reasons in the tooltip. Never colour alone (RNF-08) |
| `ColumnHeader` | `label: string`, `sort?: 'asc' \| 'desc' \| null` | `sort` | unsorted · asc · desc | `aria-sort` |
| `CutoffValue` | `hz: number \| null`, `nyquistHz: number` | — | value · full band · none | "16.0 kHz"; when `hz == nyquistHz` shows "Full band" |
| `WindowControls` | — | `minimize`, `toggleMaximize`, `close` | — | Part of the custom title bar |

## Organisms

| Component | Props | Events | States | Notes |
|---|---|---|---|---|
| `TitleBar` | `title: string` | `minimize`, `toggleMaximize`, `close` | — | `data-tauri-drag-region` |
| `Toolbar` | `canOpen: boolean`, `canCancel: boolean` | `open`, `cancel` | idle · running | Groups and separators like the reference; H2 adds remove, export, filters and search |
| `ResultsTable` | `rows: TrackAnalysis[]`, `selectedPath: string \| null`, `sort: SortState` | `select(path)`, `sort(column)` | empty · rows · many rows (virtualised) | Columns of `main-screen.md`; virtualised rows (row height token); keyboard navigation |
| `SpectrogramView` | `spectrogram: Spectrogram \| null`, `cutoffHz: number \| null`, `loading: boolean` | — | loading · image · none | `<canvas>`; colour map from tokens; frequency axis in kHz; cutoff line |
| `DetailPanel` | `track: TrackAnalysis \| null`, `spectrogram`, `loading` | — | nothing selected · loading · spectrogram · error | Text line: "Cutoff 16.0 kHz · Fake · <reason>"; for errors shows the error text |
| `StatusBar` | `phase: 'ready' \| 'scanning' \| 'analysing' \| 'finished' \| 'cancelled' \| 'error'`, `summary`, `message?` | — | one per phase | Uses `ProgressBar` |
| `FolderDropZone` | `active: boolean` (drag over), `empty: boolean` | — | hint · drag-over | Overlay; the drop itself is handled by the page |

## Templates and pages

- `MainLayout`: slots `titleBar`, `toolbar`, `table`, `detail`, `statusBar`; owns the splitter position (remembered in `localStorage`, fallback to default).
- `AnalysisPage`: connects the `analysis` store (rows, phase, selection, summary) and the API client (`pickFolder`, `startAnalysis`, `cancelAnalysis`, `getSpectrogram`, drag-drop). The only place with side effects.

## Accessibility

Focus visible on every interactive element; table is a `grid` with row selection announced; contrast of text tokens ≥ 4.5:1 on the dark background.

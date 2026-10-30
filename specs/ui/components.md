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
| `ColumnHeader` | `label: string`, `sort?: 'asc' \| 'desc' \| null`, `tooltip?: string` (H2, `lossy-formats.md`) | `sort` | unsorted · asc · desc | `aria-sort`; `tooltip` is shown on hover and focus |
| `CutoffValue` | `hz: number \| null`, `nyquistHz: number` | — | value · full band · none | "16.0 kHz"; when `hz == nyquistHz` shows "Full band" |
| `BitrateValue` (H2) | `kbps: number \| null`, `mode?: BitrateMode \| null`, `approx?: boolean` | — | value · vbr · abr · approx · none | Declared and real bitrate cells; spec in `lossy-formats.md` |
| `WindowControls` | — | `minimize`, `toggleMaximize`, `close` | — | Part of the custom title bar |

## Organisms

| Component | Props | Events | States | Notes |
|---|---|---|---|---|
| `TitleBar` | `title: string` | `minimize`, `toggleMaximize`, `close` | — | `data-tauri-drag-region` |
| `Toolbar` | `canOpen: boolean`, `canCancel: boolean` | `open`, `cancel` | idle · running | Groups and separators like the reference; H2 adds remove, export, filters and search. Export (RF-09): prop `canExport`, event `export`, and `StatusBar`'s `notice` and `Icon`'s `export` in `export-csv.md`. Filters and search (RF-08): the props, events and states they add are in `sort-filter.md` (section 10). Result cache (RF-11): prop `canClearCache`, event `clearCache` and `Icon`'s `clear-cache` in `result-cache.md` |
| `ResultsTable` | `rows: ResultRow[]` (`{ index: number; analysis: TrackAnalysis }`, `index` from the progress events; H2 RF-10 adds `albumIndex: number`, unused by the table), `selectedIndex: number \| null`, `sort: SortState`; H2 RF-10: `revealRequest?: number` (default 0) | `select(index)`, `sort(column)` | empty · rows · many rows (virtualised) · H2 RF-10: selected row scrolled into view · reveal request with the selection unchanged | Columns of `main-screen.md`; virtualised rows (row height token); keyboard navigation. **Decided at G1 (RF-10, `album-summary.md` Q13):** when `selectedIndex` changes to a row outside the visible range, the table scrolls the least needed so it is visible, whatever changed it: a prop change (selecting an album) or ↑/↓/Home/End. The approved keyboard rule of `main-screen.md` does not say whether the table scrolls and today it does not, so this also changes what those keys do visibly. **Decided at G1 (RF-10, `album-summary.md` Q16):** `revealRequest` is a counter the page increments on every album selection; every time its value changes, the table scrolls the least needed so the row of `selectedIndex` is visible, even when `selectedIndex` did not change; the initial mount does not count as a change of `revealRequest`. Only a change of `selectedIndex` or of `revealRequest` scrolls the table: new rows arriving and a re-sort never do. Sort and filter (RF-08): the sort rules, the props, events and states it adds and the keyboard rule with a selection that is not shown are in `sort-filter.md` |
| `SpectrogramView` | `spectrogram: Spectrogram \| null`, `cutoffHz: number \| null`, `loading: boolean` | — | loading · image · none | `<canvas>`; colour map from tokens; frequency axis in kHz; cutoff line |
| `DetailPanel` | `track: TrackAnalysis \| null`, `spectrogram`, `loading` | — | nothing selected · loading · spectrogram · unavailable (request failed: verdict line + `spectrogram.unavailable`) · error | Text line: "Cutoff 16.0 kHz · Fake"; " · <reason>" (`reason.*` text) is appended only when the track has reasons (a `Fake` verdict has none); for errors shows the error text. Lossy tracks (H2) add the declared and the source bitrate after the cutoff (`lossy-formats.md`) |
| `StatusBar` | `phase: 'ready' \| 'scanning' \| 'analysing' \| 'finished' \| 'cancelled' \| 'error'`, `summary`, `message?` | — | one per phase | Uses `ProgressBar`. Shown count (RF-08): the prop and states it adds are in `sort-filter.md`. Cache figures and warnings (RF-11): prop `cache` in `result-cache.md` |
| `FolderDropZone` | `active: boolean` (drag over), `empty: boolean` | — | hint · drag-over | Overlay; the drop itself is handled by the page |

## Templates and pages

- `MainLayout`: slots `titleBar`, `toolbar`, `table`, `detail`, `statusBar`; owns the splitter position (remembered in `localStorage`, fallback to default). H2 (RF-10): optional slot `sidebar` for the Albums panel (`AlbumSummaryList` organism), width from the new layout token `--sidebar-width` (240 px), and `ResultsTable` scrolls the selected row into view (proposed for G1, see its row above); both in `album-summary.md`.
- `AnalysisPage`: connects the `analysis` store (rows, phase, selection, summary) and the API client (`pickFolder`, `startAnalysis`, `cancelAnalysis`, `getSpectrogram`, drag-drop). The only place with side effects.

## Accessibility

Focus visible on every interactive element; table is a `grid` with row selection announced; contrast of text tokens ≥ 4.5:1 on the dark background.

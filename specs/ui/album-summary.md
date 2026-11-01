---
id: SPEC-UI-ALBUM-SUMMARY
requirements: [RF-10, RNF-04, RNF-08, RNF-09]
status: approved
---

# Album summaries in the main screen (H2)

> Where the summary verdict of each folder/album appears, what it shows, when it changes and how it works with the table and the spectrogram. The rule that produces the summary is `specs/detection/album-summary.md` (H0, unchanged); the data arrives with every `trackDone` event (`specs/contracts/commands.md`, "Album summary (version 5)"); scenarios: `specs/features/album-summary-app.feature` (tag `@RF-10`). Layout, the other states and interaction stay those of `main-screen.md`; export stays that of `export-csv.md`. Components follow `components.md` (atomic design, ADR-0014: a story per state and a component test before use, S6). Presentational components get data through props and emit events; only `AnalysisPage` touches the store and the API client.

## Where it appears: the Albums panel

An **Albums panel** on the left of the content area, in the place the reference tool gives its folder tree (`docs/reference/ftf-main-window-1.png`, `docs/architecture/frontend.md`, "Left panel"). It lists one item per album of the latest run with its summary verdict and counts. The results table is **not grouped**: it keeps one row per track, its columns, its sort and (with RF-08) its filters.

```
┌ TitleBar ──────────────────────────────────────────────────────── ─ □ ✕ ┐
├ Toolbar ────────────────────────────────────────────────────────────────┤
│ [Open folder] [Cancel] │ [Export CSV]                                   │
├ AlbumSummaryList ──┬ ResultsTable ──────────────────────────────────────┤
│ Albums (3)         │ State · File · Folder · Format · …                  │
│ Artist             │ ✔ Genuine  intro.flac   C:\Music\Artist  FLAC …     │
│ ✔ Genuine  1 track │ ✖ Fake     01.flac      C:\Music\Artist\Album 1 …   │
│▌Artist\Album 1     │ …                                                   │
│▌✖ Fake  12 tracks ·├ Splitter ──────────────────────────────────────────┤
│▌ 11 real · 1 fake  │ DetailPanel / SpectrogramView                       │
│ Artist\Album 2     │                                                     │
│ ! Suspicious  9 …  │                                                     │
├────────────────────┴────────────────────────────────────────────────────┤
│ StatusBar                                                               │
└─────────────────────────────────────────────────────────────────────────┘
```

- The panel spans the content area's full height (beside the table, the splitter and the detail panel), as the reference's left panel does. Fixed width **240 px** in H2 (no splitter, not collapsible; G1 decision Q3). The width is a **new layout token** `--sidebar-width: 240px` in `src/lib/tokens/tokens.css` (sizes come from tokens, `src/CLAUDE.md`; today the only size token is `--row-height`); the folder tree of a later milestone reuses it. It scrolls vertically on its own. A 1 px `--color-border` line separates it from the table.
- The window's minimum size (900 × 500) does not change; the table keeps its horizontal scroll.
- Why not group the table (header rows per album) or add an "album verdict" column: see Q2. A side list gives the at-a-glance answer for a whole library ("which albums have fakes?") without changing the table, so the table's sort, keyboard navigation, virtualisation and the coming RF-08 sort/filter work on tracks exactly as today.

## What an item shows

Each item has two lines (item height = 2 × the row height token, 44 px with the current 22 px):

1. **Name**: `AlbumUpdate.name` (the folder relative to the parent of the analysed root, commands.md name rule; for example `Artist\Album 1`), in `--color-text`, one line, end ellipsis. Its tooltip (`title`) is the full folder path (`summary.folder`).
2. **Verdict and counts**: a `VerdictCell` with `summary.verdict` and no reasons (icon + text + verdict colour token, as in the table's State column; never colour alone, RNF-08), then the counts text in `--color-text-muted`, one line, end ellipsis, tooltip the same text.

Counts text, parts joined by " · ":

- First part: `albums.tracks` "{n} tracks" (`albums.tracksOne` "1 track" when `n` is 1) when the album is complete (`summary.total == files`); `albums.partial` "{done} of {files} tracks" when it is not (`summary.total < files`: during the analysis, or after a cancel or an `INTERNAL` failure). `files` is never 1 in a partial album, so the partial text needs no singular.
- Then the **non-zero** counts, in the status bar's order: `albums.real` "{n} real", `albums.suspicious` "{n} suspicious", `albums.fake` "{n} fake", `albums.errors` "{n} errors" (`albums.errorsOne` "1 error").

Examples: "12 tracks · 11 real · 1 fake" · "1 track · 1 real" · "3 of 12 tracks · 2 real · 1 suspicious" · "2 tracks · 2 errors" (verdict Error, an album whose files all failed, RNF-04).

Numbers are written with plain digits (no thousands separator), like the status bar.

## Order of the items

By `AlbumUpdate.index` (path order of the album folders, a folder before its subfolders; commands.md). An album appears when its first track's `trackDone` is applied, **at its place in that order**, which may be above albums already shown (tracks finish out of order). The list is not sortable in H2 (Q6).

The header line shows `albums.titleCount` "Albums ({count})" with the number of items, or `albums.title` "Albums" when there are none.

## Screen states (`main-screen.md`)

The panel always shows exactly the albums of the rows the store holds, and each album's counts are the counts by verdict of those rows (acceptance A1).

| Screen state | Albums panel |
|---|---|
| Empty (start) | Not shown (the drop hint takes the window, as for the table) |
| Dragging over | As before the drag |
| Scanning | Shown, header "Albums", no items: the previous run's albums are gone, with its rows |
| Analysing | Items appear and update with the table's batches (the same flush; the batching rule "≤ 1 update / 100 ms" is the existing one of `main-screen.md`, "Screen states", row Analysing, implemented by the store's batch timer); partial albums show "{done} of {files} tracks" |
| Finished | Every album of the run; all complete |
| Cancelled | The albums with at least one track done; those not finished stay partial ("3 of 12 tracks"). With no track done: header "Albums", no items |
| Error (`AppError`) | As the table: a start that fails before Scanning (`NOT_A_FOLDER`, `FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`, `ANALYSIS_RUNNING`) keeps the previous albums and the current album (store `restore()`, check S2); a run that ends with `INTERNAL` keeps the albums of the tracks it reported, partial where not finished, including those still in the pending batch (store `fail()`, check S3) |
| Exporting (`export-csv.md`) | Unchanged |

## Interaction

- **Current album.** The album of the selected row (`ResultRow.albumIndex` of `selectedIndex`) is the current album: its item has the selected-row background (`--color-row-selected`) and `aria-selected="true"`. No row selected → no current album. It follows every change of the selection: a click or the arrow keys in the table, or a click in the panel.
- **Selecting an album** (click on its item, or the keyboard below) selects **the album's row with the lowest `index`** (its first file in scan order), whatever the table's sort, and always brings that row into the table's visible range (the table scrolls the least needed; it does not move when the row is already visible). This holds also when the album is already current and its first row is already selected: the selection does not change, but the row comes back into view if the table was scrolled away from it (Q16). When the selection changes, the detail panel shows the row's spectrogram through the usual flow (`main-screen.md`: request, spinner, stale responses discarded); when it does not change, the spectrogram is not requested again and the detail panel stays as it is. Focus stays in the panel.
- **Keyboard** in the panel (it is one tab stop, between the toolbar and the table): ↑/↓ select the previous/next album, Home/End the first/last, each with the effect of a click. With no current album, ↓ and Home select the first album, ↑ and End the last. At the first or last album, ↑/↓ select the current album again, with the effect of a click on it (Q16).
- **Double-click** does nothing (as in the table).
- **Table sort** never changes the panel: not its order, not its counts, not the current album.
- **RF-08 (sort and filter, built after this spec in #76).** Nothing here blocked it: the table is not grouped, so RF-08 sorts and filters track rows as it would without this panel. The panel's counts always cover every track of the album in the run, whatever the table's filter. RF-08 may add "show only the current album in the table" (the reference's folder-tree behaviour) or sorting the panel by verdict; if a filter hides an album's first row, RF-08 decides which row selecting that album selects. Decided at G1 in `sort-filter.md` section 6: the panel ignores the filters, and selecting an album selects its lowest-index shown row, or its lowest-index row when none is shown.
- **CSV export** (RF-09) is unchanged: one record per track, no album record (Q8).

## Component changes

| Component | Level | Change | States / variants to add (story + test) |
|---|---|---|---|
| `AlbumSummaryList` | organism (new) | Props `albums: AlbumUpdate[]` (sorted by `index`), `currentIndex: number \| null` (album index); event `select(albumIndex: number)` (`onselect`). Renders the header and the items with `VerdictCell` (molecule) and text; keyboard of "Interaction". Not virtualised in H2 (manual check M1) | no albums · one album · many albums (scroll) · partial album · error-only album · current album · long name (ellipsis) · Unicode name · focus-visible |
| `MainLayout` | template | New optional snippet `sidebar`, rendered at the left of the content area, full content height, width `var(--sidebar-width)` (new layout token, 240 px), with the right border. Without it the layout is as today | with sidebar · without sidebar |
| `ResultsTable` | organism | **Decided at G1 (Q13)** (also in `components.md`): when `selectedIndex` changes to a row outside the visible range, whatever changed it (a prop change from the page, or ↑/↓/Home/End in the table), it scrolls the least needed so the row is visible. `main-screen.md`'s approved keyboard rule ("↑/↓ move the selection; Home/End jump") says nothing about scrolling, and today the table does not scroll (End can select a row off screen), so this changes what those keys do visibly. **Decided at G1 (Q16):** new optional prop `revealRequest: number` (default 0), a counter the page increments on every album selection; every time its value changes, the table scrolls the least needed so the row of `selectedIndex` is visible, even when `selectedIndex` did not change (nothing happens when the row is already visible, `selectedIndex` is `null` or the row is not in `rows`); the initial mount does not count as a change of `revealRequest`. Only a change of `selectedIndex` or of `revealRequest` scrolls the table: new rows arriving and a re-sort never do (Q13) | selected row scrolled into view (from a prop change and from each key) · reveal request with the selection unchanged and the selected row scrolled out of view |
| `VerdictCell` | molecule | No change (used with `reasons` absent) | — |
| `StatusBar`, `DetailPanel`, `Toolbar` | organisms | No change | — |
| `AnalysisPage` | page | Passes `sidebar` with `AlbumSummaryList` in every state but Empty; computes the current album from the selection; on `select(albumIndex)` selects that album's lowest-index row and increments the counter it passes to `ResultsTable` as `revealRequest`, every time (also when the selection does not change); requests the spectrogram only when the selection changed (Q16) | — |

Store (`src/lib/stores/analysis.svelte.ts`) and model (`src/lib/model.ts`):

- `ResultRow` becomes `{ index: number; analysis: TrackAnalysis; albumIndex: number }` (`albumIndex` = `album.index` of its `trackDone`).
- `model.ts` re-exports `AlbumSummary` and `AlbumUpdate` from the generated bindings (components may not import `src/lib/api`). No type is hand-written (S3).
- The store keeps `albums: AlbumUpdate[]` sorted by `index`. On `trackDone` it stores the event's `album` for its index (replacing the previous update of that album) **in the same pending batch as the row**, so rows and albums are flushed together. `begin()` saves the albums (flushed and pending) in its snapshot and clears them with the rows; `restore()` brings them back with the snapshot; `fail()` flushes the pending batch and keeps them. The store never computes a verdict or a count of an album: it only keeps the latest update.
- If an update arrives whose `summary.total` is not greater than the stored one for that index (not expected: the contract orders them), the store keeps the stored one (check S4).
- New `selectAlbum(albumIndex)`: selects the row with the lowest `index` among the rows of that album (rows pending in the batch included).

### Store checks (store unit tests, `src/lib/stores/`)

Named checks of the store rules above. S2 and S3 are the store side of the Error row of "Screen states"; the page side is the scenarios "A start that fails before Scanning keeps the albums" and "A run that fails unexpectedly keeps the albums it reported" of the feature.

- **S1 (`begin`).** After `trackDone` events for two albums and `begin()`, `albums` is empty, also when some updates were still pending.
- **S2 (`restore`).** Finished run with two albums, a row of the second selected; `begin()` then `restore()`: `albums` equals what it was before `begin()` (same items, same updates), and the current album (from `selectedIndex`) is the second again. With updates still pending at `begin()`, they come back too.
- **S3 (`fail`).** Two `trackDone` events of one album (totals 1 and 2), the second still pending; `fail(message)`: `albums` holds that album with `summary.total` 2, and the rows hold both tracks.
- **S4 (stale update).** For one album index, an update with `summary.total` 3 then one with 2 (after or within the same batch): the stored update keeps total 3.
- **S5 (`selectAlbum`).** Rows of album 1 with indexes 7, 4 (pending) and 9: `selectAlbum(1)` selects 4.

## Texts (keys in `src/lib/i18n/`; English is the default, Spanish falls back to English until RF-13)

New: `albums.title` Albums · `albums.titleCount` Albums ({count}) · `albums.label` Album summaries (accessible name of the list) · `albums.tracks` {n} tracks · `albums.tracksOne` 1 track · `albums.partial` {done} of {files} tracks · `albums.real` {n} real · `albums.suspicious` {n} suspicious · `albums.fake` {n} fake · `albums.errors` {n} errors · `albums.errorsOne` 1 error. The verdict texts are the existing `verdict.*`. Album names are never translated.

## Accessibility

The list is a `listbox` with the accessible name `albums.label` and one tab stop (like the table's `grid`); items are `option`s with `aria-selected` on the current album. An item's accessible name is "{name}, {verdict text}, {counts text}" (for example "Artist\Album 1, Fake, 12 tracks · 11 real · 1 fake"). Visible focus ring on the list and on the active item. The verdict is icon + text + colour (RNF-08). Text contrast ≥ 4.5:1 on the panel background (`components.md`).

## Acceptance (measurable)

- **A1 (consistency).** After every flush of the store, with any sequence of stubbed `trackDone` events: the panel has one item per distinct `albumIndex` of the rows, and each item's `total`, `real`, `suspicious`, `fake` and `errors` equal the counts by verdict of the rows with that `albumIndex` (page/store tests).
- **A2 (rule).** Every `album.summary` sent by the backend equals `summarize_album(folder, tracks of that album sent so far)`, and for each album the n-th `trackDone` carries `summary.total == n` (core use-case tests with fakes, 1 and several workers).
- **A3 (complete).** After `finished`, every item shows "{files} tracks" (no partial text) and the sum of the items' totals equals `AnalysisSummary.done`.
- **A4 (batches).** During the analysis the panel updates at most once per 100 ms, in the same flush as the table rows (the existing batching rule of `main-screen.md`, "Screen states", row Analysing: "batched ≤ 1 update / 100 ms"; this spec adds no new timing).
- **A5 (selection).** Selecting an album (click or keyboard) selects its lowest-index row and that row is within the table's visible range afterwards, also when the album was already current and its row was scrolled out of view; in that case the selection does not change and no new spectrogram request is made. A different row of the current album being selected counts as a selection change.

## Manual checks (before the feature PR, release build)

- M1. A test folder outside the audio corpus with 1 000 subfolders of one short generated fixture each (`specs/detection/fixtures.md`, for example `short`): the panel lists 1 000 albums and scrolls without visible stutter; the window stays responsive during the run.
- M2. Visual check against the reference screenshots: dark panel, dense items, verdict icons aligned in one column.

## G1 decisions

Decided at G1 (2026-10-25): every default below is accepted, and this spec and the contract are written with them. Q14 and Q15 were raised during the review. Q16 was raised in the feature review (code review of the RF-10 PR) and decided on 2026-10-25: its questions were answered by a delegate on the owner's behalf, and the owner approved the answers.

| # | Question | Decision |
|---|---|---|
| Q1 | **Where the summary is computed.** In the backend (`fakeflac-core`, sent with every `trackDone`, commands.md version 5) or in the frontend from the rows already received (no contract change, but the rule would be duplicated in TypeScript outside the core)? | Backend |
| Q2 | **Where it appears.** A left Albums panel (as written), header rows grouping the table by album (conflicts with column sort, virtualisation, keyboard navigation and RF-08 filters), an "Album" verdict column on every row (sortable, but no counts at a glance), or a line in the detail panel for the selected track's album? | Left Albums panel |
| Q3 | Panel width: fixed 240 px, not resizable, not collapsible in H2? A vertical splitter or a collapse button can come with the folder tree | Fixed 240 px |
| Q4 | **Display name**: the folder relative to the parent of the analysed root (`Artist\Album 1`), full path for a drive or UNC share root; full path in the tooltip. Alternatives: only the folder's own name (`Album 1`, ambiguous for `CD1`/`CD2`), or the full path | Relative to the root's parent |
| Q5 | **Partial albums** (during the run, after a cancel) show "{done} of {files} tracks", which needs `files` in the contract. Alternatively show only the tracks done, with no hint that the verdict may change | Show "{done} of {files}" |
| Q6 | Order of the panel: path order of the folders (`index`). Sort it by verdict severity ("fake albums first")? Proposed to leave it to RF-08 | Path order; RF-08 may add sorting |
| Q7 | **Selecting an album** selects its first track (scan order) and scrolls the table to it. Alternatively filter the table to the album (the reference's folder-tree behaviour), which is filtering and belongs with RF-08 | Select the first track |
| Q8 | **CSV export of album summaries.** No: the CSV stays one record per track (`csv_format_version` 1); a second file or a second section would break RFC 4180 tooling, and the `folder` column already allows grouping | Not exported |
| Q9 | Counts text: only non-zero counts, in the status bar's order (real · suspicious · fake · errors). Alternatively always all four, or problems first (fake · suspicious · errors) | Non-zero, status bar order |
| Q10 | A run with a single album still shows the panel (one item), so the layout does not jump between runs | Shown |
| Q11 | **Album = direct parent folder** (album-summary.md, approved in H0): `Album\CD1` and `Album\CD2` are two albums, and the folder `Album` (no files of its own) is none. Merging discs or using the `album` tag (lofty, not read yet) would be a new rule and a new requirement | Direct parent folder |
| Q12 | The status bar does not show the number of albums or of bad albums (for example "· 2 albums with fakes") | Unchanged |
| Q13 | **Table scroll-into-view** (`components.md`, `ResultsTable`, proposed): selecting an album needs the table to scroll to its row (A5), and the same rule makes ↑/↓/Home/End keep the selected row visible, which the approved RF-06 keyboard rule did not state and the table does not do today. Alternatively scroll only when the selection comes from the panel | Scroll for every selection change |
| Q14 | Edits to approved specs: one cross-reference sentence each in `main-screen.md` and `components.md`, and the scroll-into-view note on `ResultsTable` in `components.md` | Approved |
| Q15 | A core scenario relied on a panic in the progress sink propagating out of `analyze_folder`, which no spec states. Specify panic behaviour here? | No: that scenario is dropped. Panic semantics are not RF-10's subject; the ordering guarantee stays covered by the event-order scenarios and A2, and the UI `INTERNAL` scenario (store check S3) uses a stubbed rejection |
| Q16 | **Selecting the current album again** when its first row is already selected: the selection does not change, so a scroll that follows only `selectedIndex` (Q13) does nothing and the row stays out of view if the user scrolled the table away, against "Interaction" and A5. Alternatives: leave it as is (A5 only for a selection change); scroll the table only for selections from the panel (drops Q13 for the table keys); an exported `ResultsTable` function the page calls (`bind:this`, outside the props/events contract); a reveal request prop (`revealRequest: number`, a counter the page increments on every album selection; also in `components.md`) | Reveal request prop. Every album selection (click or keyboard) brings its first row into view; the spectrogram is requested only when the selection changes; new rows and re-sorts still never scroll the table (Q13). (1) `revealRequest?: number` (default 0): a prop, not an exported function via `bind:this` (organisms are presentational: props in, events out); a counter, not a boolean (a boolean flipped twice in one flush reads as no change). (2) `main-screen.md` unchanged (Q14 limits edits to approved specs); so re-selecting the same album does not retry a failed spectrogram, as in the table. (3) No new scenario for "new rows and re-sorts never scroll": covered by `ResultsTable` component tests derived from `components.md` (reveal request; a rows or sort rerender leaves `scrollTop` unchanged), like S1–S5. (4) Edge keys keep emitting `select` |

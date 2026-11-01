---
id: SPEC-UI-SORT-FILTER
requirements: [RF-08, RNF-04, RNF-08, RNF-09]
status: approved
---

# Sort and filter the results table (H2)

> How the user sorts the results table and narrows it down by verdict and by name (RF-08). Scenarios: `specs/features/sort-filter.feature` (tag `@RF-08`). Layout, the other states and interaction stay those of `main-screen.md`; the Albums panel stays that of `album-summary.md`; export stays that of `export-csv.md`. Components follow `components.md` (atomic design, ADR-0014: a story per state and a component test before use, S6). Presentational components get data through props and emit events; only `AnalysisPage` touches the store and the API client.

## 0. Contract: unchanged

Sorting and filtering are **frontend-only**, over the rows the store already holds (`docs/architecture/frontend.md`, "Table performance": "sorting and filtering on the client over the store"; planning document section 3.10: RF-08 has no backend module). **`specs/contracts/commands.md` does not change: contract version 5 stays, no command, event, type or `AppError` code is added or changed, and `just gen` produces the same bindings.** The CSV export is not affected (section 7). The new types of this spec (`RowFilter`) are UI model types in `src/lib/model.ts`, like `SortState`; they are not contract types (S3 covers contract types only).

## 1. What RF-08 adds and what already exists

Already there (RF-06, approved): every column header sorts the table (click: ascending, click again: descending), default order File ascending, the sort is kept in the `analysis` store. Not specified until now: the order of the State column (the code compares the verdict identifiers alphabetically: error, fake, real, suspicious), how names are compared, how ties and empty cells are ordered.

RF-08 adds:

- **Sorting rules** for every column (section 3): verdict order by attention, natural and case-insensitive names, deterministic ties, empty cells last.
- **Verdict filter**: four toggle buttons in the toolbar (section 4.1).
- **Name search**: a search field in the toolbar, shortcut `Ctrl+F` (section 4.2).
- What a filter does to the selection, the keyboard, the Albums panel, the status bar and the screen states (sections 5–9).

## 2. Where the controls are

In the main `Toolbar`, as in the reference tool (`docs/architecture/frontend.md`, "Top bar": verdict filter buttons after save/export, search box at the right with `Ctrl+F`; `components.md`, `Toolbar`: "H2 adds remove, export, filters and search"). No separate `TableToolbar` above the table (G1 question Q1; `export-csv.md` U4 stays answered "main `Toolbar`").

```
┌ Toolbar ─────────────────────────────────────────────────────────────────────────────┐
│ [Open folder] [Cancel] │ [Export CSV] │ [✖] [!] [✔] [⚠]          [🔍 Search (Ctrl+F)  ✕] │
└──────────────────────────────────────────────────────────────────────────────────────┘
                                          Fake Susp. Genuine Error       (search field, right-aligned)
```

- A separator after Export CSV, then the **verdict filter** group in this order: Fake, Suspicious, Genuine, Error (most severe first, as the reference's "thumb down / middle / up"; our own icons: the existing `verdict-*` icons, D12).
- The **search field** is pushed to the right end of the toolbar. Width: new layout token `--search-width: 240px` in `src/lib/tokens/tokens.css` (sizes come from tokens, `src/CLAUDE.md`). At the minimum window width (900 px) everything fits on one line (7 toolbar buttons, 2 separators and the field take about 600 px). RF-11 adds an eighth button, "Clear results cache", after Export CSV, which brings the line to about 650 px (`result-cache.md` section 2).

## 3. Sorting

### 3.1 Rules

Clicking a header still sorts by that column ascending, and clicking the sorted column again toggles the direction (RF-06, unchanged). There is one sort column at a time (no multi-column sort, Q8).

| Column | Ascending order | Empty cells |
|---|---|---|
| State | **Fake, Suspicious, Error, Genuine** (what needs attention first). Descending is the exact reverse: Genuine, Error, Suspicious, Fake (Q2) | — (every row has a verdict) |
| File | The **full path** (`TrackInfo.path`), compared with the name comparison of 3.2, component by component. So everything below a folder (its files and the rows of its subfolders) stays together and comes before a sibling folder whose name extends it (`Album\…` before `Album (deluxe)\…`). Inside one folder, its files and its subfolders are ordered **together, by their own names**: a subfolder's rows sit where the subfolder's name sorts among the file names, so `Album\01.flac`, `Album\Bonus\x.flac`, `Album\Z.flac` is the intended order (the subfolder `Bonus` sorts between `01.flac` and `Z.flac`). Files are not put before subfolders. This is the scan order's rule (`commands.md`: component by component, case-insensitive), plus the natural order of numbers. Unchanged meaning: `main-screen.md` "sorted by file path" (Q3) | — |
| Folder | The **Folder cell's text**: `TrackInfo.folder` (the absolute folder), or, when it is the empty string, the folder part of `TrackInfo.path` (everything before its last `\` or `/`), the fallback the cell already shows (`ResultsTable`, unchanged). Name comparison of 3.2; since it splits at both `\` and `/`, the order does not depend on which separator the fallback text uses | — |
| Format | The cell text (`MP3`, `M4A (ALAC)`, …), name comparison of 3.2 | — |
| Samplerate, Bits, Duration, Cutoff | Numeric | `null` values (no bit depth, no cutoff: error rows and some lossy rows) go **last in both directions** |
| Bitrate, Real bitrate | Numeric (unchanged, `lossy-formats.md`) | "—" last in both directions (unchanged) |

Today the code sorts a `null` cutoff or bit depth as 0 (first when ascending); this spec makes every numeric column follow the rule `lossy-formats.md` already set for the bitrate columns (Q4).

### 3.2 Name comparison

Used by File, Folder and Format, and by the tie-break.

1. Split each string into **path components** at `\` and `/` and compare component by component; when one string runs out of components first (its components are a prefix of the other's), it comes first. So `Album\CD2\01.flac` comes before `Album (deluxe)\01.flac` (component `Album` before `Album (deluxe)`), as in the scan order and the album order (`commands.md`, "Events" and "Album summary"); and `Album\Bonus\x.flac` comes between `Album\01.flac` and `Album\Z.flac` (component `Bonus` between `01.flac` and `Z.flac`, 3.1).
2. Compare two components with one `Intl.Collator` created for the **UI locale** (`getLocale()` of `src/lib/i18n`, English by default) with the options `{ numeric: true, sensitivity: 'accent' }`:
   - **natural order of numbers**: `Track 2` before `Track 10`; `01` and `1` compare equal;
   - **case-insensitive**: `a.flac` and `A.flac` compare equal;
   - accents only break ties between names that are otherwise equal (`Cancion` before `Canción`), and never reorder different letters (`Álbum` sorts with the a's);
   - with the Spanish UI (RF-13) the Spanish collation applies (`ñ` after `n`).
3. Strings are compared as the code points they arrived with; nothing is normalised or rewritten for display (RNF-09).

### 3.3 Ties and stability

Rows whose sort key compares equal are ordered by the **File order ascending** (3.2 over the full path) and then by **`index` ascending**, in both directions. So the order of the table depends only on the rows and the sort, never on the order the `trackDone` events arrived in, and two rows with the same path (names that were not valid Unicode, both shown with U+FFFD; `commands.md`, "Events") keep their scan order.

### 3.4 During a run, and across runs

- During a run the sort applies to every batch: new rows appear at their sorted place. A new batch or a re-sort never scrolls the table (`components.md`, `ResultsTable`, G1 Q13 of RF-10, unchanged), and the selection is kept (it is a row `index`).
- The header can be clicked during a run.
- The sort is kept across runs for the whole session (the store's `begin()` does not reset it, as today) and is not saved when the app closes (Q7).

## 4. Filtering

A row is shown when it passes **both** the verdict filter and the name search (AND). A filter never removes a row from the store: hiding is only a view.

### 4.1 Verdict filter

- Four toggle buttons, one per verdict (`real`, `suspicious`, `fake`, `error`). Each is pressed or not.
- **None pressed** (the default): no verdict filter, every verdict is shown.
- **One or more pressed**: only the rows whose verdict is one of the pressed ones are shown (union). Pressing all four shows every row, like none (Q5).
- Clicking a button (or Space/Enter on it) toggles it. The other buttons do not change.
- Error rows (RNF-04) are a verdict like the others: the Error button shows them, and they are hidden when other buttons are pressed and Error is not.

### 4.2 Name search

- **What is searched**: for each row, the text `<album name>\<file name>`, where the album name is `AlbumUpdate.name` of the row's `albumIndex` (the folder relative to the parent of the analysed root, `commands.md` "Name rule", for example `Artist\Album 1`) and the file name is the last component of `TrackInfo.path` (the File cell; split at `\` or `/`). Example: `Artist\Album 1\01 – Intro.flac`. So the search finds a track by its own name or by any folder of its album below the analysed root's parent, but a word of the root's location (`Music` in `C:\Music\Artist`) does not match every row (Q6). For a drive or share root the album name is the full folder (`D:\Album A`), so its drive matches too.
- **Terms**: the query is split at runs of white space (JavaScript `\s`); empty terms are dropped. No term (empty or blank query): no name filter. A row passes when **every** term is a substring of its searched text, in any order (`beatles help` matches `The Beatles\Help!\01.flac`).
- **Separators**: a `/` in the query counts as `\`.
- **Folding** (applied to both the query terms and the searched text, so the comparison is code-point exact after it):
  1. `toLowerCase()` (locale-independent);
  2. Unicode normalisation NFD, then remove every code point of general category `Mn` (nonspacing marks: accents, the tilde of `ñ`, variation selectors);
  3. normalisation NFC.

  So the search is **case-insensitive and accent-insensitive**: `cancion` matches `Canción`, `ÇA VA` matches `Ça va`, `nandu` and `ñandú` match `Ñandú`, and a name stored decomposed (`N` + U+0303) matches like the precomposed `Ñ`. Punctuation and symbols are not folded: `″` (U+2033) matches only `″`, and the ASCII `"` does not match it; `🎵` matches `🎵` (Q6).

  **Accepted consequence for other scripts (RNF-09).** Removing every `Mn` mark also removes marks that are not accents in the Latin sense: the Japanese voicing marks (`が` U+304C decomposes to `か` U+304B + U+3099 and folds to `か`, so `か` and `が` match each other; likewise `は`/`ば`/`ぱ`), the Thai vowel and tone marks above and below the line (for example U+0E34, U+0E48), and the Indic nonspacing signs (for example the Devanagari virama U+094D and anusvara U+0902). The effect is **extra matches only**: since both sides are folded the same way, a query typed with the same characters as the name still finds it; it can also find names that differ from it only in those marks. Names are never changed on screen (3.2 rule 3, 4.2 "The field keeps…"). The alternative (remove only the Latin, Greek and Cyrillic diacritics, U+0300–U+036F) is listed in Q6.
- **When it applies**: on every change of the field's text (each keystroke, paste, clear), with no debounce; timing in acceptance P1.
- The field keeps exactly the text typed (it is not trimmed or folded on screen).

### 4.3 Clearing

- **Escape** in the search field clears its text (if any); the focus stays in the field. Escape with an empty field does nothing.
- The field's **clear button** (✕, shown only when the field has text and is enabled) clears the text and puts the focus back in the field.
- The **Clear filters** button of the no-match state (section 8) clears the text and releases every verdict button. The sort does not change. The button disappears with the no-match state (the rows come back), so **the focus moves to the search field** (empty after the clear; Q16). The selection does not change.

### 4.4 During a run, and across runs

- During a run the filters apply to every batch: a new row that passes them appears at its sorted place; one that does not stays hidden until the filters change.
- The filters can be changed during a run (the controls are enabled whenever the table is shown, section 8).
- The filters are kept across runs for the whole session, like the sort, and are not saved when the app closes (Q7). Because the controls stay pressed/filled (also when disabled, section 8) and, once rows arrive, the status bar shows the shown count (section 9), a filter carried into a new run is always visible.
- A failed start (`restore()`, then `reset()` for `NOT_A_FOLDER`) and an `INTERNAL` failure (`fail()`) do not change the filters.

## 5. Selection, keyboard and scroll

- **The selection is kept when its row is hidden** (Q9). A filter change never changes `selectedIndex`: the hidden row is no longer highlighted in the table (it is not there), the detail panel keeps showing that track, and its album stays the current album in the Albums panel. When a filter change shows the row again, it is highlighted again.
- **Keyboard in the table** works over the **shown rows in their display order** (the existing rule, now over shown rows): ↑/↓ move to the previous/next shown row, Home/End to the first/last. When the selection is `null` **or hidden**, ↓ and Home select the first shown row, End the last, and ↑ does nothing (today, with a hidden selection, ↓ does nothing; this makes it behave as with no selection). With no shown row the keys do nothing.
- **Clicking** a shown row selects it, as today.
- **Scroll.** A change of the filters, like a re-sort, never scrolls the table by itself (Q13 of RF-10); when fewer rows are shown than the scroll position needs, the scroll position shrinks to fit, as any scroll container does. The scroll-into-view rule of `components.md` (`ResultsTable`: on a change of `selectedIndex` or `revealRequest`) is unchanged; when the selected row is hidden it has nothing to scroll to and does nothing.
- The search field does not move the selection or the focus of the table while typing.

## 6. Albums panel (RF-10)

- **The panel does not change with the filters**: same items, same order, same counts (they cover every track of the album in the run, `album-summary.md`), same current album.
- **Selecting an album** (click or keyboard in the panel) selects the album's row with the **lowest `index` among the shown rows** (not the one shown first: under a sort other than File the lowest-index row can be anywhere in the display order); when none of its rows is shown, it selects the album's row with the lowest `index` anyway (a hidden selection, section 5): the detail panel shows that track and the album becomes current, the table highlights nothing, and the filters do not change (Q10). The reveal request is still counted (`album-summary.md`, Q16); it scrolls only when the selected row is shown. The spectrogram is requested only when the selection changes, as today.
- RF-08 does **not** add "show only the current album in the table" (the reference's folder-tree behaviour) nor sorting the panel by verdict; both were left open by `album-summary.md` (Q6, Q7 of RF-10) and stay out of H2 (Q11).

## 7. CSV export (RF-09)

**Unchanged.** Export CSV writes every track of the latest run, in scan order, whatever the table's sort and filters (`csv-export.md` Q6 default, `commands.md` "CSV export (version 4)"). The button's enabled rule does not look at the filters: it is enabled with a filter that hides every row. Exporting only the shown rows would need a contract change (the frontend would send row indexes, contract version 6) and is not part of RF-08 (Q12).

## 8. Screen states (`main-screen.md`)

| Screen state | Verdict buttons and search field | Table |
|---|---|---|
| Empty (start) | **Disabled, showing the kept filter** (Q7). At app start nothing is pressed and the field shows its placeholder. The Empty state can also come back with a filter set: after a run that left no row (for example cancelled before its first track) a start rejected with `NOT_A_FOLDER` calls `restore()` then `reset()` (phase `ready`, no row: `isEmpty` of `AnalysisPage`). Then the pressed buttons stay pressed (`aria-pressed="true"`) and disabled, and the field shows the kept text, disabled, without its clear button. The next run's Scanning state enables them with that filter. `Ctrl+F` does not focus the field, but its default is still prevented, so the WebView2 find bar does not open (section 10, `SearchField`) | Hidden (drop hint) |
| Dragging over | As before the drag | — |
| Scanning | Enabled, with the filters of the previous run | Empty, no message (no rows yet) |
| Analysing | Enabled | Shown rows of each batch, sorted |
| Finished, Cancelled | Enabled | Shown rows |
| Error (`AppError`) | Enabled when the table is shown (it is: the previous rows, or the rows reported before `INTERNAL`) | Shown rows of the rows kept |
| Exporting | Unchanged (export does not touch the filters) | Unchanged |
| **No match** (any state above with at least one row in the store and none shown) | Enabled | Its header stays; the body shows `table.noMatches` "No tracks match the filters" and a **Clear filters** button (`table.clearFilters`), centred in the body. After Clear filters the focus is in the search field (4.3) |

"Enabled" in the table is `canFilter` = the screen is not in the Empty state (the same condition under which the page shows the table).

## 9. Status bar

The status bar appends " · {n} shown" (`status.shown`) after the state text, before the export notice if any, when **all** of these hold:

- a filter is active (at least one verdict button pressed, or at least one search term);
- the store holds at least one (flushed) row;
- the phase is `analysing`, `finished` or `cancelled`.

`n` is the number of shown rows after the latest flush. It is **not** appended in `ready` (the start, or rows kept after a `NOT_A_FOLDER` start; Q17), in `scanning` (no rows yet), in `error` (the bar shows the error message alone, as today: `StatusBar` renders `message` for that phase; Q17), nor while the store has no row (for example a run cancelled before its first track: "Cancelled · 0 of 10", no " · 0 shown").

Examples: "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 3 shown"; "18/31 · 2 suspicious · 1 fake · 1 shown"; "Cancelled · 3 of 10 · 1 shown"; with every verdict button pressed and no term: "… · 31 shown"; after `INTERNAL` with a filter: "The analysis stopped because of an unexpected error" only. The counters of the state text are always those of the whole run, never of the shown rows.

## 10. Component changes

| Component | Level | Change | States / variants to add (story + test) |
|---|---|---|---|
| `Icon` | atom | New `IconName` `search` (own SVG, 16×16 view box: a magnifier) | `search` decorative and labelled |
| `IconButton` | atom | New optional prop `pressed?: boolean`: when given (true or false) the button is a toggle with `aria-pressed`; when absent, a plain button as today. **Pressed look**: background `--color-surface-raised` and border `--color-accent` (kept on hover; the hover look of a released button keeps the border `--color-border`, so pressed and hovered differ by the border). Not `--color-row-selected`: the error and fake icons fail 3:1 on it (M3). When disabled and pressed, the pressed look stays under the existing disabled opacity, so a kept filter stays visible (section 8). New optional prop `tone?: Verdict`: the icon takes the verdict colour token (`--verdict-*`) in every state but disabled | toggle not pressed · toggle pressed · pressed + hover · pressed + focus-visible · toggle disabled · toggle disabled and pressed · tone each verdict (released and pressed) |
| `VerdictFilter` | molecule (new) | Props `selected: Verdict[]`, `disabled?: boolean`; event `toggle(verdict: Verdict)` (`ontoggle`). Renders a `role="group"` named `filter.label` with four `IconButton`s (size `toolbar`, `pressed`, `tone`), in the order fake, suspicious, real, error, with the `verdict-*` icons and the labels `filter.fake`, `filter.suspicious`, `filter.real`, `filter.error` (label = tooltip = accessible name). It keeps no state: `pressed` comes from `selected` (also when `disabled`) | none pressed · one pressed · several pressed · all pressed · disabled · disabled with one pressed |
| `SearchField` | molecule (new) | Props `value: string`, `disabled?: boolean`, `shortcut?: boolean` (default false), `focusRequest?: number` (default 0); event `input(value: string)` (`oninput`). Renders the `search` icon (decorative), a native `<input type="text">` with `role="searchbox"`, accessible name `search.label`, placeholder `search.placeholder`, `spellcheck="false"`, `autocomplete="off"`, and, when `value` is not empty **and the field is not disabled**, an `IconButton` size `small`, icon `close`, label `search.clear`. The input shows `value` also when disabled. Emits `input` on every text change, on Escape (with `''`, only when `value` is not empty) and on the clear button (with `''`, then focuses the input). **Shortcut**: with `shortcut`, it listens to `keydown` on the window; on `Ctrl+F` (Ctrl without Alt, Shift or Meta, and `event.key` `f` or `F` or `event.code` `KeyF`) it **always calls `preventDefault()`**, also when `disabled`, so the WebView2 find bar never opens (M2); when not disabled it then focuses the input and selects its text; when disabled the focus does not move. **`focusRequest`**: a counter; every time its value changes the input gets the focus (text not selected), unless disabled; the initial mount does not count as a change (the pattern of `ResultsTable`'s `revealRequest`). Width `var(--search-width)`. No `Input` atom is introduced: the native input lives in this molecule (Q13) | empty · with text (clear button) · focused · disabled · disabled with text (no clear button) · Unicode text |
| `Toolbar` | organism | New props `canFilter: boolean`, `verdictFilter: Verdict[]`, `query: string`, `searchFocusRequest?: number` (default 0, passed to `SearchField`'s `focusRequest`); new events `toggleVerdict(verdict)` (`ontoggleverdict`) and `query(text)` (`onquery`). Adds a separator and `VerdictFilter` after the export group, and `SearchField` (with `shortcut`) right-aligned; both disabled when `canFilter` is false, still showing `verdictFilter` and `query` | idle with filters · filters pressed and query · empty (filters disabled) · empty with a kept filter (disabled, pressed, text) · running with filters |
| `ResultsTable` | organism | `rows` is now **the rows to show** (the page passes the filtered rows); the table still sorts them by `sort`, now with the rules of section 3 (`compareRows`, section 10 "Model"). New optional prop `noMatches?: boolean` (default false) and event `clearFilters` (`onclearfilters`): when `noMatches` is true and `rows` is empty, the body shows the no-match message and the Clear filters button (section 8). Keyboard rule of section 5 for a `selectedIndex` that is not in `rows` | no matches · selected row not in `rows` (nothing highlighted; ↓ selects the first row) · State sorted asc/desc with every verdict · empty cells last · Folder sort with an empty `folder` (fallback) |
| `StatusBar` | organism | New optional prop `shown?: number`: when defined, " · {shown} shown" (`status.shown`) after the state text and before the notice, **only in the phases `analysing`, `finished` and `cancelled`** (never in `ready`, `scanning` or `error`, section 9) | finished with shown · analysing with shown · cancelled with shown · error with shown (not appended) · with shown and notice |
| `AnalysisPage` | page | Passes `store.visibleRows` to `ResultsTable`, `noMatches` = `store.rows.length > 0 && store.visibleRows.length === 0`, `canFilter` = not Empty, the filters and their events to `Toolbar`, `shown` = `store.visibleRows.length` when `isFilterActive(store.filter)` and `store.rows.length > 0` (else `undefined`) to `StatusBar`; `clearFilters` calls `store.clearFilter()` and then increments the `searchFocusRequest` it passes to `Toolbar` (4.3) | — |

`VerdictCell`, `ColumnHeader`, `AlbumSummaryList`, `DetailPanel`, `MainLayout` and `TitleBar` do not change.

### Model, store and the view rules

- `src/lib/model.ts` adds `export type RowFilter = { verdicts: Verdict[]; query: string }` (UI type, hand-written like `SortState`). `verdicts` empty = no verdict filter; when not empty it holds each pressed verdict once, **in the canonical order `fake`, `suspicious`, `real`, `error`** (the buttons' order), whatever order they were pressed in. `query` is the field's raw text.
- New pure module **`src/lib/table-view.ts`** (no store, no API, no DOM; importable by organisms, the store and the page), with exactly these exports, which the tests import:
  - `export function foldForSearch(text: string): string`: the folding of 4.2.
  - `export function searchTerms(query: string): string[]`: the folded terms of 4.2 (`/` → `\`, split at white space, empty terms dropped), in the order they appear in the query.
  - `export function rowMatches(row: ResultRow, filter: RowFilter, albumName: string | undefined): boolean`: section 4 (verdict and name). `albumName` is the `name` of the row's album (`row.albumIndex`); the searched text is `` `${albumName}\${fileName}` `` with `fileName` the last component of `row.analysis.info.path`. When `albumName` is `undefined` the searched text is the full `TrackInfo.path` (a guard only: the store always has the name, below). `filter.verdicts` is read as a set (order and duplicates do not matter here).
  - `export function compareRows(a: ResultRow, b: ResultRow, sort: SortState, locale: Locale): number`: section 3, ties included (never 0 for two rows with different `index`). `Locale` is the type of `getLocale()` in `src/lib/i18n`.
  - `export function isFilterActive(filter: RowFilter): boolean`: at least one verdict or one term.
- Store (`src/lib/stores/analysis.svelte.ts`): new `filter: RowFilter` (initially `{ verdicts: [], query: '' }`), `visibleRows: ResultRow[]` (the flushed rows that pass `filter`; store order, unsorted), and the methods `toggleVerdictFilter(verdict: Verdict)` (adds or removes it, keeping the canonical order), `setQuery(query: string)` (stores it as given), `clearFilter()` (sets `{ verdicts: [], query: '' }`). `begin()`, `restore()`, `reset()` and `fail()` do not change `filter` or `sort`.
  - **Album names for `visibleRows`**: from the published `albums` (by `AlbumUpdate.index`). `flushBatch()` publishes the rows and their albums together (`publishAlbums()` in the same flush), and `restore()`/`reset()` keep `albums` consistent with `rows`, so every flushed row's album is in `albums`. An album's `name` never changes within a run (it comes from its folder, `commands.md` "Name rule"), so `albums` and `latest` give the same name; `visibleRows` never reads `latest`.
  - `selectAlbum(albumIndex)` follows section 6: the lowest `index` among the album's rows that pass the filter (rows pending in the batch included, judged with the album names of `latest`, which holds the albums of pending rows), else the lowest `index` of the album.
- **Implementation note (non-binding, no timing assertion).** At 10 000 rows a sort makes about 133 000 comparisons (n log₂ n), and every filter change tests every row. The implementation should build the expensive parts once, not per comparison or per test: one `Intl.Collator` per locale (cached, not created in `compareRows`), the path components of each row split once per row (and the Folder/Format keys once per row), the folded searched text once per row (recomputed only when its album name or the row changes), and the folded terms once per query change. Caches keyed by the row object (for example a `WeakMap`) or by the query string are fine, as long as every result equals the definitions above. P1 is the only timing check.

## 11. Checks

### View checks (unit tests of `src/lib/table-view.ts`)

- **V1 (State order).** Rows with verdicts real, error, fake, suspicious: ascending gives fake, suspicious, error, real; descending gives real, error, suspicious, fake.
- **V2 (natural, case-insensitive).** In one folder `Track 10.flac`, `track 2.flac`, `Track 1.flac`: File ascending gives `Track 1`, `track 2`, `Track 10`.
- **V3 (components).** `C:\L\Album\CD2\01.flac`, `C:\L\Album (deluxe)\01.flac`, `C:\L\Album\02.flac`: File ascending gives `Album\02.flac`, `Album\CD2\01.flac`, `Album (deluxe)\01.flac`. **Interleave (intended):** `C:\L\Album\Z.flac`, `C:\L\Album\Bonus\x.flac`, `C:\L\Album\01.flac`: File ascending gives `Album\01.flac`, `Album\Bonus\x.flac`, `Album\Z.flac`; descending gives the exact reverse.
- **V4 (ties).** Two fake rows `b.flac` (index 0) and `a.flac` (index 1) and one real row: State ascending gives `a.flac`, `b.flac`, then the real row; State descending gives the real row, then `a.flac`, `b.flac`. Two rows with the same path and indexes 3 and 5 are in the order 3, 5 in both directions of File.
- **V5 (empty last).** Cutoff with values 16 000, `null`, 20 000: ascending 16 000, 20 000, `null`; descending 20 000, 16 000, `null`. The same for Bits with `null`.
- **V6 (accents in order).** `Canción.flac`, `Cancion.flac`, `Álbum.flac`, `apple.flac`, `Ça va.flac`, `zebra.flac` (locale `en`): `Álbum`, `apple`, `Ça va`, `Cancion`, `Canción`, `zebra` (the space of `Ça va` sorts before the letter `n`; `Cancion` before `Canción` because accents only break ties).
- **V7 (folding).** `foldForSearch('Canción')` = `foldForSearch('CANCION')` = `cancion`; `foldForSearch('Ñandú')` = `foldForSearch('N\u0303andu\u0301')` = `nandu`; `foldForSearch('Ça')` = `ca`; `foldForSearch('″')` = `″`; `foldForSearch('🎵')` = `🎵`; `foldForSearch('\u{1F3B5}\uFE0F')` = `🎵` (variation selector removed); `foldForSearch('が')` = `か` (U+304C → U+304B, the accepted consequence of 4.2).
- **V8 (terms).** `searchTerms('  Beatles   help ')` = `['beatles', 'help']`; `searchTerms('Album 1/01')` = `['album', '1\01']` (the second term is the four characters `1`, `\`, `0`, `1`); `searchTerms('   ')` = `[]`.
- **V9 (match).** Album name `Artist\Album 1`, file `01 – Intro.flac`: `intro` yes, `album 1 intro` yes, `intro album` yes, `album 2` no, `Music` no (root location), `artist\album` yes, `artist/album` yes; with `verdicts: ['fake']` and a real row: no, whatever the query.
- **V10 (size).** 10 000 stubbed rows built by the rule of the feature scenario "Ten thousand rows": with State ascending, `verdicts: ['suspicious']` and the query `00`, filtering gives exactly 52 rows and sorting gives the order stated in that scenario; the test also compares filtering and sorting for every column and both directions with a reference computation. No wall-clock assertion (timing is P1).
- **V11 (Folder fallback).** Rows with (`folder`, `path`): (`''`, `C:\L\B\01.flac`), (`C:\L\A`, `C:\L\A\01.flac`), (`C:\L\C`, `C:\L\C\01.flac`): Folder ascending gives the paths `C:\L\A\01.flac`, `C:\L\B\01.flac`, `C:\L\C\01.flac` (the empty `folder` sorts as `C:\L\B`); descending the reverse.

### Store checks (store unit tests)

- **F1 (filter is a view).** After 10 `trackDone` events and `toggleVerdictFilter('fake')`, `rows` still has 10 rows and `visibleRows` only the fake ones; `toggleVerdictFilter('fake')` again gives back all 10.
- **F2 (kept across runs).** With `filter` `{ verdicts: ['fake'], query: 'x' }` and sort State desc: `begin()`, `restore()`, `begin()` again, `fail()`, `reset()` leave both unchanged (`filter` deep-equals `{ verdicts: ['fake'], query: 'x' }`). Also with no row in the store (`begin()`, `restore()`, `reset()` from a run that left no row: phase `ready`, `rows` empty).
- **F3 (batches).** With an active filter, rows pending in the batch are not in `visibleRows` until the flush; after it, exactly the flushed rows that pass are, a name filter included (a query matching only the album name `Artist\Album 1` of a just-flushed row finds it).
- **F4 (`selectAlbum` with a filter).** Album 1 with rows 7 (fake), 4 (real, pending) and 9 (fake), filter `['fake']`: `selectAlbum(1)` selects 7. With filter `['suspicious']`: selects 4.
- **F5 (`clearFilter`).** Clears verdicts and query (`filter` deep-equals `{ verdicts: [], query: '' }`); `sort` and `selectedIndex` unchanged.
- **F6 (canonical order).** `toggleVerdictFilter('error')`, `('fake')`, `('real')` give `verdicts` = `['fake', 'real', 'error']`; then `toggleVerdictFilter('suspicious')` gives `['fake', 'suspicious', 'real', 'error']`; then `toggleVerdictFilter('real')` gives `['fake', 'suspicious', 'error']`.

## 12. Texts (keys in `src/lib/i18n/`; English is the default, Spanish falls back to English until RF-13)

New: `filter.label` Filter by verdict · `filter.fake` Show fake tracks · `filter.suspicious` Show suspicious tracks · `filter.real` Show genuine tracks · `filter.error` Show tracks with errors · `search.label` Search by name · `search.placeholder` Search (Ctrl+F) · `search.clear` Clear search · `table.noMatches` No tracks match the filters · `table.clearFilters` Clear filters · `status.shown` {n} shown.

## 13. Accessibility

- The verdict buttons are a `role="group"` named "Filter by verdict"; each is a toggle button (`aria-pressed`) whose accessible name and tooltip are its label ("Show fake tracks"). The verdict is shown by the icon's shape (the State column's icons) and the name, never by colour alone (RNF-08). The pressed state is shown by `aria-pressed` and the accent border, not by the icon colour.
- The search field is a `searchbox` named "Search by name"; the clear button is named "Clear search". The shortcut is in the placeholder ("Search (Ctrl+F)") and in `aria-keyshortcuts="Control+F"` on the input.
- Tab order: toolbar buttons (open, cancel, export, the four verdict buttons), search field (and its clear button when shown), Albums panel, table. Every control has the visible focus ring of `components.md`. RF-11 adds one stop, "Clear results cache", right after export (`result-cache.md` section 6).
- The no-match message is plain text in the table body; the Clear filters button is a normal button reachable with Tab. Activating it moves the focus to the search field (4.3), so the focus is never lost on a removed element.
- The status bar is a `role="status"` region, so the shown count is announced when it changes.
- `ColumnHeader` keeps `aria-sort`.

## 14. Acceptance (measurable)

- **A1 (order).** For every column and both directions, the table's row order equals the order of `compareRows` (section 3) over the shown rows: checks V1–V6 and V11, and the scenarios of the feature for State, File and Folder.
- **A2 (filter).** After every flush and every filter change, the table shows exactly the rows of the store that pass `rowMatches`, in `compareRows` order; the status bar's shown count follows section 9 (equal to their number where it is shown); the Albums panel's items and counts are those without a filter (page tests with stubbed events, 200 events in shuffled order and filter changes between batches).
- **A3 (selection).** A filter change never changes `selectedIndex`; with a hidden selection, ↓ selects the first shown row; selecting an album selects its lowest-index shown row, or its lowest-index row when none is shown (F4 and the scenarios).
- **A4 (export).** With any sort and filter, `exportCsv` is called with no argument and the export writes every track of the run in scan order (unchanged backend behaviour; page test of the call and the existing `export-csv.feature` scenario "Rows follow the scan order").
- **A5 (contract).** `just gen-check` passes with no change to `src/lib/api/bindings.ts`; no file under `src-tauri/` or `crates/` changes in the feature PR.
- **P1 (timing, manual, release build).** With 10 000 rows (M1's folder, finished run), each of these updates the table in **under 200 ms** from the input: clicking the State header, clicking the File header, pressing a verdict button, typing one character in the search field, and clearing the search. Measured with a 60 fps screen recording of the window: at most 12 frames between the frame where the input is first visible (the button's pressed look, the header's arrow, the character in the field) and the frame where the table shows the new rows. Fails if any of the five takes more.
- **P2 (during a run, manual, release build).** While M1's folder is analysed with the table sorted by State and the query `0` active, typing in the search field and scrolling the table show no visible stutter, and the batches keep arriving.

## 15. Manual checks (before the feature PR, release build)

- **M1.** A test folder outside the audio corpus with 10 000 generated fixtures of `specs/detection/fixtures.md` in 100 subfolders of 100 files: 9 000 `short` (error, fast to analyse), 500 `full_band` (Genuine) and 500 `lp16k` (Fake), names `NNNN – Track.flac`. Run P1 and P2 on it.
- **M2.** `Ctrl+F` anywhere in the window (table focused, Albums panel focused, toolbar focused) focuses the search field and selects its text, and the WebView2 find bar does not open. In the Empty state (filters disabled) `Ctrl+F` leaves the focus where it was, the field stays unfocused, and the find bar does not open either (the default is prevented in every state).
- **M3.** Visual check against the reference screenshots: the four verdict buttons and the search field fit on the toolbar line at 900 px width; pressed buttons are clearly distinct from released and from hovered ones; the verdict colours of the icons stay readable when pressed. **Non-text contrast (WCAG 1.4.11, 3:1)**, computed from `tokens.css` with the WCAG relative luminance and confirmed on screen:

  | Element | On | Ratio | Result |
  |---|---|---|---|
  | `--verdict-error` #8a8a8a icon | `--color-row-selected` #4a4a4a (the first draft's pressed background) | 2.6:1 | **fails** |
  | `--verdict-fake` #d9534f icon | #4a4a4a | 2.2:1 | **fails** |
  | `--verdict-error` #8a8a8a icon | `--color-surface-raised` #2e2e2e (pressed background of this spec) | 3.9:1 | passes |
  | `--verdict-fake` #d9534f icon | #2e2e2e | 3.4:1 | passes |
  | `--verdict-real` #5fb36b icon | #2e2e2e | 5.3:1 | passes |
  | `--verdict-suspicious` #e0b13a icon | #2e2e2e | 6.8:1 | passes |
  | `--color-accent` #e8892b border (pressed indicator) | the toolbar `--color-surface` #262626 | 5.8:1 | passes |

  Hence the pressed look of section 10 (`IconButton`): background `--color-surface-raised`, border `--color-accent`. Released icons are on the toolbar background #262626, where every verdict colour has at least 3.8:1. Disabled buttons are exempt (WCAG 1.4.11).

## 16. Edits to approved specs (approved at G1, Q14)

One cross-reference sentence each, no other text changed:

- `main-screen.md` (intro): H2 sort rules and filters are in this spec.
- `components.md` (`Toolbar`, `ResultsTable` and `StatusBar` rows, column "Notes"): the RF-08 props, events and states (and, for `ResultsTable`, the sort rules and the keyboard rule with a hidden selection) are in this spec. The Props, Events and States columns of those rows are not edited.
- `album-summary.md` ("Interaction", bullet "RF-08"): which row selecting an album selects under a filter is decided here (section 6).
- `commands.md`: a short "Sort and filter (RF-08)" note stating that the contract does not change (version 5 stays).

Behaviour refined by this spec over approved text: the State column order (unspecified until now; the code is alphabetical by identifier), `null` numeric cells last (as `lossy-formats.md` already does for bitrates), natural name comparison (the approved "sorted by file path" now compares paths naturally and case-insensitively), and ↓/Home with a selection that is not shown. The Folder fallback for an empty `TrackInfo.folder` is kept as the code does it. **Consequence on existing tests (declared):** none of the current tests is known to assert the old State order, the old place of `null` cells or a case-sensitive path order; if one does, updating it to this spec is a declared consequence done by the test author in `test(RF-08)`, without changing what else it asserts.

## 17. Open questions for G1

Decided at G1 (2026-10-25): every default below is accepted, and this spec is written with them. The questions were answered by a delegate on the owner's behalf; the owner approves the answers by merging the spec PR (#75).

| # | Question | Default |
|---|---|---|
| Q1 | **Where the controls go.** In the main `Toolbar` (as the reference and `frontend.md`), or in a new `TableToolbar` above the table (planning section 3.10, with `FilterChip`s)? | Main `Toolbar`, no `TableToolbar`; Export CSV stays where it is (`export-csv.md` U4) |
| Q2 | **State sort order.** Ascending = Fake, Suspicious, Error, Genuine (attention first, so the first click puts fakes on top), descending the reverse. Alternatives: severity ascending (Genuine, Suspicious, Fake) with errors always last in both directions; or errors first | Fake, Suspicious, Error, Genuine |
| Q3 | **What "name" sorts by.** The File column keeps sorting by the full path (component by component, natural), so albums stay together and a subfolder sorts among its parent's files by name (`Album\01.flac`, `Album\Bonus\x.flac`, `Album\Z.flac`, as the scan order). Alternatives: put a folder's files before its subfolders; or File sorts by the file name only (all "01 …" tracks together across folders), and Folder by folder then file | Full path, files and subfolders together by name |
| Q4 | **Empty numeric cells** (no cutoff, no bit depth) last in both directions, as the bitrate columns already do. Alternative: keep sorting them as 0 | Last in both directions |
| Q5 | **Verdict filter semantics.** Toggles narrow the table to the pressed verdicts (none pressed = all). Alternatives: all pressed by default and releasing hides (the "show/hide each verdict" reading of the reference); or single-select (radio) | Multi-select union, none pressed = all |
| Q6 | **Name search.** Substring per term, all terms required, case- and accent-insensitive, over `<album name>\<file name>`. Alternatives: file name only; full path (then the root's location matches every row); also fold typographic quotes and primes (`″`, `“`, `’`) to ASCII so `"` matches `″`; also search the Format text or tags. **Mark folding:** remove every `Mn` mark (as written; also merges `か`/`が` and Thai/Indic marks, 4.2), or remove only U+0300–U+036F (Latin, Greek and Cyrillic diacritics), leaving other scripts' marks significant | As written: every `Mn` removed; no punctuation folding |
| Q7 | **Persistence.** Sort and filters are kept across runs within a session and reset when the app starts; a filter kept into the Empty state is shown on the disabled controls. Alternatives: reset the filters (not the sort) on every new run; or save both in `localStorage` like the splitter position | Kept for the session, not saved |
| Q8 | **Multi-column sort** (Shift+click a second header) or a secondary key chosen by the user? | No: one column, fixed tie-break by path then index |
| Q9 | **A selected row that a filter hides.** Kept (detail panel unchanged, row reappears selected). Alternatives: clear the selection; or move it to the nearest shown row | Kept |
| Q10 | **Selecting an album whose rows are all hidden.** Selects its lowest-index row anyway (hidden selection, detail panel shows it). Alternatives: clear the filters so the row shows; do nothing; or show the item as unavailable | Hidden selection, filters unchanged |
| Q11 | **Album filter and panel sort.** Add "show only the current album in the table" or sorting the Albums panel by verdict (left to RF-08 by RF-10 Q6/Q7)? | Neither in H2 |
| Q12 | **Export the shown rows only** (contract version 6: the frontend sends the shown row indexes, or a second command)? | No: export stays all rows, scan order |
| Q13 | **No `Input` atom.** `frontend.md` lists an `Input` atom; this spec keeps the native input inside `SearchField` because it is the only text input in H2. Add the atom now? | Not now; the next text input (for example in Settings) introduces it |
| Q14 | Edits to approved specs: the cross-reference sentences of section 16 | Approved |
| Q15 | **Counts on the verdict buttons** (for example "Show fake tracks (3)" in the tooltip, or a badge)? The status bar already shows the counts | No counts |
| Q16 | **Focus after Clear filters** (the button disappears with the no-match state). The search field (ready for a new query; needs `focusRequest` on `SearchField` and `searchFocusRequest` on `Toolbar`). Alternative: the table body (ready for ↑/↓; no new prop, `ResultsTable` focuses itself) | Search field |
| Q17 | **Shown count in `ready` and `error`.** The status bar shows the bare state text ("Ready", or the error message) there, even when the table shows filtered rows (rows kept after a `NOT_A_FOLDER` start; rows kept after an error). Alternative: append " · {n} shown" whenever the table has rows and a filter is active, whatever the phase | Not appended in `ready` and `error` (section 9) |

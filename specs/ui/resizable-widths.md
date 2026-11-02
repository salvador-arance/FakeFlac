---
id: SPEC-UI-RESIZABLE-WIDTHS
requirements: [RF-06, RF-10, RF-13, RNF-08, RNF-09]
status: approved
---

# Resizable Albums panel and result columns

The owner requested mouse-drag resizing of the Albums panel and table columns
on 2026-11-02. This extension of RF-06/RF-10 was approved by the owner in chat and merged at G1 in #113 on 2026-11-02. Implemented and merged in #115 (2026-11-02).
Scenarios: `../features/resizable-widths.feature`. Local preference contract:
`../contracts/layout-widths.md`. No backend command/event/DTO or contract-version
change is proposed; `Settings` remains language-only.

## 1. Scope and defaults

A vertical separator at the Albums panel's right edge changes its width. Each
of the ten column headers has a resize handle at its right edge, including the
last column after horizontal scrolling. Resize handles are distinct from the
header's sort button. Double-clicking a handle, or Enter while it is focused,
resets that target to its default. No column hiding, reordering, content autofit,
panel collapse or global reset control is added.

The Albums panel retains its existing visibility rules and full content-area
height. Its default width is 240 CSS px; approved limits are 160–480 px. Reserve
at least 360 px for the main area and 4 px for the separator: effective maximum
is `max(160, min(480, contentWidth - 360 - 4))`. Before the container has a
positive measured width, use 480 as that maximum. Automatic clamping on window
resize changes only the displayed width; it does not overwrite the preference,
so enlarging the window restores a previously chosen width where space permits.
The panel's hidden state has no separator or tab stop and does not erase its size.

| Column ID | Default width / rule | Minimum | Maximum |
|---|---|---|---|
| `state` | 120 | 120 | 800 |
| `file` | Responsive, minimum 140, weight 1.5 | 100 | 800 |
| `folder` | Responsive, minimum 140, weight 2 | 100 | 800 |
| `format` | 70 | 70 | 800 |
| `sampleRate` | 80 | 80 | 800 |
| `bitDepth` | 50 | 50 | 800 |
| `duration` | 70 | 70 | 800 |
| `cutoff` | 90 | 90 | 800 |
| `bitrate` | 110 | 110 | 800 |
| `realBitrate` | 100 | 100 | 800 |

All dimensions above are CSS pixels. Bounds are approved design choices, not
claims of optimal usability. Sizes/weights are named layout tokens/data under
`src/lib/tokens/`; components do not duplicate literal dimensions.

Pre-G1 fit evidence (2026-11-02, Edge canvas, Segoe UI 12 px): `Sospechoso`
measures 63.32 px at normal weight, so 16 px icon + 4 px gap + 16 px padding
totals 99.32 px within State's 120 px minimum. Bold `Bits` measures 19.89 px;
adding its 16 px sort icon, 4 px gap and 16 px padding already needs 55.89 px
before the 6 px resize area, exceeding its 50 px track. This is why section 2
requires bounded labels and full-label access instead of assuming wrapping
alone always fits. Bold `Archivo`/`Carpeta` measure 41.31/42.36 px and fit their
100 px minima with those affordances. At 900 px, sidebar 480 + separator 4
leaves 416 px for the main area, above 360. These measurements support the
constraints; they do not establish M1 for the unimplemented feature.

Compute each column's default width from the table viewport independently of
user overrides. The eight fixed defaults sum to 690. With viewport width `W`,
let `F = max(280, W - 690)`: default File is `max(140, 3 * F / 7)`, and default
Folder is `F - File`; cap each resulting default at 800. Below that cap this
preserves today's `minmax(140px, 1.5fr)` and `minmax(140px, 2fr)` behavior.
An override replaces only its column's computed
default. Thus dragging a column does not squeeze its neighbors; changing the
viewport, including resizing the sidebar, recalculates only columns still in
default mode. Reset removes that column's override. Default and user-chosen
widths are limited to the table above, leaving trailing space on a very wide
window once both responsive columns reach their cap.

Header and every data row use the same track widths. If their sum exceeds the
viewport, use the existing horizontal scrollbar; if narrower, leave trailing
space rather than stretching overridden columns. All ten columns remain
reachable. Row height stays unchanged. Headers reserve their actual multiline
height and never overlap a data row: preserve the correction merged in PR #112
(`8b24925`) when implementing this proposal.

## 2. Interaction

- Dragging with the primary pointer button changes the target width live. Use
  horizontal pointer deltas in CSS px; moving right increases width. A column
  and sidebar both start each pointer or keyboard adjustment from the rendered
  width, including responsive columns and an automatically clamped sidebar.
  Once a drag
  actually changes a width, that column has an override. A click without motion
  does not create an override. Keep fractional motion during a drag; on finish,
  round and clamp the committed width to integer CSS px.
- Continue tracking the active pointer outside the thin handle. Other pointers
  and non-primary buttons do not resize. Pointer-up, pointer cancellation or
  window blur ends the gesture and commits the last visible width; later motion
  has no effect. Reaching a bound discards excess delta, so reversing direction
  moves immediately away from the bound. Unmounting cleans up listeners/capture.
- Handle interactions never sort, select a row/album, open a folder, or start an
  analysis. The header sort button keeps its existing click/keyboard behavior.
  Suppress the click/double-click generated by a resize gesture from reaching it.
- A focused vertical handle uses Left/Right to change width by 10 px per key
  event and Enter to reset. Prevent those keys from also navigating the table.
  Tab moves between the sort button and handle in each column's order. The
  sidebar handle comes after the Albums list and before the main table.
- Use `col-resize`, a visible boundary, and the existing accent/focus tokens.
  The sidebar separator occupies 4 px. A column handle has a 6 px hit area inside
  the header's right edge and takes no grid track; reserve that area from the
  sort button's content. Bound the sort button and label to that remaining
  width; sort icons do not shrink. Labels may wrap at spaces and use ellipsis
  when a word cannot fit, rather than drawing beneath the handle or another
  column. The complete localized label remains the accessible name and is
  available in hover text; where a help tooltip already exists, include both
  the label and that help. Its existing keyboard-focus help stays available.
- New handles are focusable `role="separator"`, `aria-orientation="vertical"`,
  with localized accessible name/description and current/minimum/maximum width
  values. `aria-valuetext` expresses the current width in pixels. The old
  horizontal detail splitter keeps its current behavior and tab order.
- Resizing works during analysis, on retained results after an error, and in
  the no-matches/empty table where headers exist. It does not change sort,
  filters, selection, album data, verdicts, displayed filenames or CSV output.
  The language-settings modal makes the handles inert with the rest of the
  background. Opening it during a drag ends that gesture at its last visible
  width; subsequent pointer motion cannot resize the inert background.
  Replacing results or changing language retains the widths.

## 3. Component contract

Presentational components continue to receive data and emit events. New
preferences are owned by `AnalysisPage`; a small frontend helper reads/writes
the local record described in `layout-widths.md`. Existing detail-height
ownership/storage in `MainLayout` is unchanged.

| Component / level | Approved addition | States/stories |
|---|---|---|
| `Splitter` / atom | Extend `orientation` to `horizontal \| vertical`; optional `label`, `description`, `value`, `min`, `max`, `valueText`, `onreset`, `onresizeend`, and `variant: 'panel' \| 'column'` (default panel). Existing `onresize(delta)` remains incremental. New vertical consumers supply all accessibility fields. `onresizeend()` identifies pointer completion; keyboard steps also commit; reset emits only `onreset`. Horizontal callers need no new props. | existing horizontal; vertical panel/column idle, hover, focus, dragging, lower/upper bound |
| `ColumnHeader` / molecule | Optional `resize: { width: number; min: number; max: number; label: string; description: string; valueText: string }`, `onresize(delta)`, `onresizeend()`, `onresetwidth()`. Omitted means existing non-resizable header. Render the handle beside, never inside, the sort button. | sortable+resizable, wrapped Spanish label, resize limits, tooltip+resize |
| `ResultsTable` / organism | Optional `columnWidths: Partial<Record<SortColumn, number>>` (default `{}`); events `oncolumnresize(column, width)`, `oncolumnresizeend()`, `oncolumnreset(column)`. Own only transient pointer/measurement state; report absolute clamped widths to the page. Keep pointer baseline independent of parent rerenders. | defaults, one/several overrides, empty/no matches, wide/scrolled, analysis updates, Unicode/error rows |
| `MainLayout` / template | Optional `sidebarWidth` (default 240); `onsidebarresize(width)`, `onsidebarresizeend()`, `onresetsidebar()`. Clamp displayed width to its measured container; emit user changes only, not automatic container clamps. Render vertical separator only with `sidebar`. | sidebar default/min/max, no sidebar, modal inert, small/resized window |
| `AnalysisPage` / page | Own sidebar preference and column overrides, load once before rendering initialized content, pass callbacks, persist on completion/reset/keyboard changes. Keep state across layout/table remounts. | startup/defaults, saved widths, unavailable storage, language switch, new run |

UI-only type `ColumnWidths = Partial<Record<SortColumn, number>>` belongs in
`model.ts`; it is not generated from Rust. Existing optional defaults keep
unrelated callers and stories valid. New states receive stories before use.

## 4. Exact new translations

These catalogue additions extend `translations.md`, approved at G1 in #113. The test-stage catalogue coverage must
include this table as an additional approved source; no implementation-stage
edit to `translations.md` is required.
`{column}` is the current localized `column.*` label; `{width}` is the rounded
integer displayed width. Existing column labels and tooltip text are unchanged.

| Key | English | Spanish |
|---|---|---|
| `layout.resizeAlbums` | Resize Albums panel | Cambiar ancho del panel de álbumes |
| `layout.resizeColumn` | Resize {column} column | Cambiar ancho de la columna {column} |
| `layout.resizeHelp` | Drag or use Left/Right to resize. Double-click or press Enter to reset. | Arrastra o usa Izquierda/Derecha para cambiar el ancho. Haz doble clic o pulsa Intro para restablecerlo. |
| `layout.widthPixels` | {width} pixels | {width} píxeles |

## 5. Acceptance and affected tests

- **W1:** Sidebar 240 + drag 80 = 320; dragging beyond either limit clamps;
  after the maximum, a -10 delta shrinks immediately. Hidden sidebar removes
  only its handle; modal prevents every resize input.
  In a measured test container that clamps a preferred 480 to a displayed 400,
  Left changes the displayed and preferred width to 390, without jumping from
  the stored 480 baseline.
- **W2:** At viewport 1040, default File/Folder are 150/200. Resize File to 230:
  Folder stays 200 and all other widths stay unchanged. At viewport 1390,
  File stays 230 and untouched Folder becomes 400. Reset File: 300/400.
  Apply corresponding boundary tests to all ten columns, including the last.
- **W3:** Resize gestures and Enter on handles emit no sort/selection event;
  ordinary header activation still sorts. Pointer-up/cancel/blur stops movement;
  secondary pointers are ignored. Check fractional deltas and keyboard bounds.
- **W4:** Relaunch after committed changes restores those overrides in either
  language. Test the storage recovery table and individual reset. No audio name,
  path, verdict or result goes into the local record.
- **W5:** A new run, error, Unicode row, filter, sort, locale change or 10,000-row
  update preserves widths and the existing selection/reveal behavior.
- **M1, rendered acceptance:** on Windows/WebView2 or a real browser, at 900×500
  and 1280×720, English and Spanish: drag sidebar and every column at default,
  minimum and maximum; headers/rows align, multiline headers do not overlap,
  the last handle is reachable via horizontal scroll, and focused handles,
  sort buttons and the Real bitrate tooltip remain usable. Confirm widths by
  bounding rectangles with 1 CSS px tolerance for rounding. Repeat with 0,
  3 and 1,000 rows and after vertical scrolling/End/Home. Record candidate and
  dimensions; jsdom alone cannot establish this geometry.

Test stage: add preference-helper, vertical Splitter, ColumnHeader resize,
ResultsTable width, MainLayout sidebar and page persistence/i18n tests. Existing
tests that may change **only in the test stage**: `MainLayout.sidebar.test.ts`
(the approved fixed-width assumption), `Splitter.test.ts` (new vertical API),
`ColumnHeader.test.ts`, `ResultsTable{,.locale,.scroll,.sortFilter}.test.ts`,
`MainLayout{,.locale}.test.ts`, `AnalysisPage{,.locale,.settings,.albums}.test.ts`
and `i18n/catalogue.test.ts` where they assert exact tab order, separator counts,
props or catalogue coverage. Preserve prior behavior assertions; new optional
props alone are not a reason to rewrite unrelated tests. Component story
fixtures and focused resize cases are added in the test stage. Implementation
must not alter protected tests. `just check` must pass before completion.

## 6. Approved amendments and G1 decisions

This approved extension supersedes **only** the fixed-width/no-splitter part of
`album-summary.md` section "Where it appears" and Q3, and extends the component
rows for `Splitter`, `ColumnHeader`, `ResultsTable`, `MainLayout` and
`AnalysisPage` in `components.md`. Panel collapse remains out of scope. The width-specific amendments were approved at G1 in #113.

| Decision | Approved answer (#113) |
|---|---|
| R-Q1: both the sidebar and all ten columns, with per-target double-click/Enter reset? | Yes. |
| R-Q2: remember local layout widths across launches using the existing WebView local-storage pattern, separate from RF-13 language settings? | Yes; convenience-only recovery in the local contract. |
| R-Q3: the limits/default-mode rules above, including unchanged neighboring widths while dragging and horizontal overflow? | Yes. The arithmetic/text measurements above support the proposed bounds; actual interaction and geometry require post-implementation M1. |

Implementation preserves the header correction merged in PR #112. This
extension does not close RNF-07/08 or approve
their separate PR #111. No dependency is proposed.

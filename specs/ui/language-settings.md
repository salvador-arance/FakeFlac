---
id: SPEC-UI-LANGUAGE-SETTINGS
requirements: [RF-13, RF-08, RNF-04, RNF-08, RNF-09, RNF-10]
status: proposed
---

# Spanish, English and the language Settings panel

Contract and G1 questions: `../contracts/settings.md`. Texts: `translations.md`. Scope was
selected by the owner on 2026-11-01: bilingual UI with a persistent preference. All behavior
below is proposed for G1; implementation follows approval and independent tests.

## 1. Startup and language state

On mount, `AnalysisPage` gets settings before enabling folder-open/drop, export, clear-cache
or Settings actions. While waiting, show the app title and a busy placeholder with no English
main-screen flash; a language-neutral `FakeFlac` accessible name and `aria-busy="true"` identify
it. Keep window controls usable with the provisional English labels. When the call settles,
render the ordinary screen using the returned language and set the document's `lang` to `en`
or `es`. An unexpected `getSettings` failure becomes English plus the unavailable warning;
the main screen must still be usable. Do not impose a fixed delay or hide a hung IPC behind
an arbitrary timeout.

One reactive locale drives all mounted app-owned visible strings, tooltips, accessibility
names, explanatory text and status messages. `getLocale()` reflects the active language and
`t()` resolves the catalogue for it. Keep messages as a key plus interpolation data, or an
equivalent representation that can re-render in another language; do not retain translated
strings as the only state for an error/export/cache notice. No restart, remount of the page,
new run or extra spectrogram request is needed to translate the current view.

Missing/invalid/unavailable/newer settings are the contract's cases. Missing is silent. For
the other three, show their localized warning as a status-bar notice until another existing
notice replaces it, and always show it inside Settings while its snapshot has that status.
The warning never replaces an analysis error. A successful Save clears the settings warning.

## 2. Entry point and interaction

Add a settings gear `IconButton` after the four verdict filter buttons and before Search,
with `toolbar.settings` as tooltip and accessible name. It is enabled in every initialized
main-screen phase, including scanning/analysing and empty/error states. No keyboard shortcut
is added. The remaining toolbar order follows `sort-filter.md` and `result-cache.md`.

Activation opens a modal `SettingsPanel` over the current screen. Its controls, in order:
heading `settings.title`; labelled native select `settings.language`; options **English**
(`en`) and **Español** (`es`), always shown as autonyms; warning/error region when present;
Save and Cancel buttons. The selected draft begins with the active language on each opening.
Changing the select updates only the draft. Save is enabled even for an unchanged language
so the user can explicitly persist English or repair an invalid file.

On Save, disable the select, Save and Cancel and mark the panel busy; use `settings.saving`
for the Save button. Move focus to the dialog container (`tabindex="-1"`) while its controls
are disabled; Tab and Shift+Tab keep focus there. Ignore repeated activation, Escape and backdrop dismissal while saving.
Only one save is in flight. On success, apply the returned language in one render cycle,
close the panel and return focus to the Settings toolbar button (whose name is now translated).
On failure, keep the panel, draft and old active language; re-enable controls, show
`error.SETTINGS_WRITE_FAILED` or `error.SETTINGS_READ_ONLY` (unexpected failure uses
`settings.saveFailed`), and move focus to Save for retryable errors or Close when Save becomes
disabled for `SETTINGS_READ_ONLY`. Retry calls the API once again with the draft.

Cancel or Escape outside the saving state closes without an API write or language change.
Backdrop clicks do not dismiss. With `newerVersion`, the select and Save are disabled, and
the dismissal button says Close (`settings.close`); the warning explains the newer version.
If a save discovers a newer file, show `error.SETTINGS_READ_ONLY` and disable further Saves
for that open panel; Close remains usable. A later opening retries only after restarting the
app when the initial snapshot was `newerVersion`, as specified by the contract.

Background content is inert while the modal is open: it accepts no clicks, key commands
(including Ctrl+F) or folder drops. Background analysis/events continue normally. Tab and
Shift+Tab stay inside the panel, initial focus is the select (Close for newerVersion), and
Escape returns focus to the trigger. The panel has an accessible dialog role, modal state
and heading association. Its error is an alert; its loading state is announced. Window
controls retain their usual operating-system actions.

## 3. Existing view state and sorting

Saving a language preserves the run, row/album DTOs, pending event batches, phase, counts,
selected track index, selected album, query, enabled verdict filters, sort column/direction,
splitter size, cached spectrogram and export/cache notice data. The main table re-evaluates
text comparisons with the new `Intl.Collator` locale as already specified in `sort-filter.md`.
Verdict sort remains the fixed enum order. Search folding and matches remain exactly those
of that spec; translating a verdict must not change the filter keys or matches.

Locale changes act like a re-sort: no automatic scroll/reveal and no selection change.
The next ↑/↓ uses the new shown-row order. Album order remains its backend index order.
New analysis events and late spectrogram/error replies render in the current language.
An existing export success notice translates its sentence while preserving the path bytes;
existing AppError/track errors translate their friendly message while preserving diagnostics
only in their previously specified technical surfaces.

## 4. Component contracts and layout

| Component | Atomic level | Props / events | States and responsibility |
|---|---|---|---|
| `Icon` | atom | Extend `IconName` with `settings`; existing props | Original gear SVG, decorative inside the labelled IconButton; use existing size/color tokens. |
| `LanguageSelect` | molecule | `value: Language`, `disabled?: boolean`, `onchange(language)` | English/Spanish selected; enabled/disabled/focus. Label `settings.language`, native select, autonym options with `lang="en"` / `lang="es"`. No persistence or store access. |
| `SettingsPanel` | organism | `language: Language`, `loadStatus: SettingsLoadStatus`, `saving: boolean`, `error: 'writeFailed' \| 'readOnly' \| 'internal' \| null`; `onsave(language)`, `onclose()` | Draft belongs to this panel and resets on opening; normal, warning, saving, save error, newer/read-only. Hosts LanguageSelect and ordinary buttons; owns modal keyboard/focus behavior; no API calls. |
| `Toolbar` | organism | Add `canSettings: boolean`, `onsettings()` | Existing states plus startup-disabled; gear after filters, before search. |
| `MainLayout` | template | Add optional `settings` snippet | Slot for modal; no settings/store/API knowledge. |
| `AnalysisPage` | page | no new external props | Loads settings, owns request state, calls typed client, applies the reactive locale and document language, suppresses background input while panel open. |

Presentational components can continue resolving catalogue messages; only the page touches
the API and settings state. Use Svelte 5 runes, atomic boundaries and existing theme/focus
tokens. Add a Storybook story per new molecule/organism state before use; translations are
also exercised through the existing components' stories/tests. The modal fits within the
900 × 500 minimum window, wraps warnings, and keeps both action buttons visible. The gear
uses the existing toolbar button dimensions. At that minimum width all toolbar controls
remain reachable and Search remains usable in both languages; no new fixed-size threshold
or timing budget is added. Long table labels may use the existing table horizontal scroll;
tooltips and accessible names retain their complete translation.

## 5. Acceptance

- **L1 catalogue:** every key in `translations.md` is explicit in both catalogues, with the
  same placeholder multiset; no supported UI key relies on English fallback in Spanish.
  Unknown keys retain the existing last-resort key behavior. Audit app-owned literals outside
  the catalogue, including status sentences, column labels, window buttons and the toolbar name.
- **L2 persistence:** backend tests cover every row of the loading table, supported languages,
  invalid IPC input, Unicode settings roots, and failures at write/flush/replace with a fake
  filesystem or narrow injected seam. Failed saves preserve byte-identical existing files;
  a second initialized settings owner reads a successfully saved value. Audio fixture bytes,
  cache file and unrelated files remain unchanged.
- **L3 startup:** page tests hold the read promise pending, then resolve Spanish or each
  fallback; no analysis starts early; the initialized screen and document language agree.
- **L4 live updates:** with the page already mounted, switch English→Spanish→English through
  Save in empty, running, finished, cancelled and error states. Check toolbar, headers, row
  verdict/reason/error tooltips, detail line, spectrogram label/loading/unavailable, album
  counts, filters/search, window controls and existing export/cache/error notices.
- **L5 invariants:** use a running stub with queued events, an existing selected track,
  filters/query and a loaded spectrogram. Save, then deliver further events. Assert preserved
  data/selection/request count, updated rendered text and new-locale name order. Include the
  paths `nube.flac`, `ñandú.flac`, `nora.flac`: File ascending orders them as
  `ñandú,nora,nube` in English and `nora,nube,ñandú` in Spanish. No automatic reveal occurs.
- **L6 modal:** component/page tests cover keyboard focus, disabled saving actions, one request
  per Save, Cancel/Escape, retry, successful focus restoration, newer version and background
  shortcut/drop suppression. Modal does not stop event processing.
- **L7 regression:** serialize the same completed backend results before/after a UI language
  change and compare CSV bytes; existing cache engine identity and analysis DTOs are equal.
  Use a mixed result set including a corrupt track and Unicode names; friendly errors translate.
- **M1 manual, feature PR:** run the desktop app from an isolated repository-local test config
  directory, choose Español, quit normally and relaunch; confirm Spanish first main screen.
  Repeat with English. Record the app version and result; do not alter the owner's real settings.
- **M2 manual, feature PR:** inspect both languages at 900 × 500, keyboard-only modal behavior,
  full translated tooltips/accessible names, and switching while a generated fixture run is
  active. Record clipping/overflow findings and fix them before review. Native dialogs follow
  Windows language as documented. No audio corpus or measurable delay is needed for these checks.

## 6. Proposed changes to earlier specifications and tests

This RF-13 proposal supersedes the temporary English-only/fallback rules in `main-screen.md`,
`components.md`, `album-summary.md`, `export-csv.md`, `sort-filter.md`, `result-cache.md` and
`lossy-formats.md` only for the translations and singular forms listed in `translations.md`.
The existing English words are retained except grammatical singular counts (`1 track`,
`1 error`; Spanish also has singular real/suspicious/fake counts). All specified composition
order, separators, counts, placeholders, data identity and technical number formatting remain.
Native folder/save/overwrite dialogs, OS diagnostics, installer UI and release notes keep their
own language. CSV identifiers and values stay fixed English per `../contracts/csv-export.md`.

After G1, the independent **test stage** must update obsolete English-fallback assertions in
`src/lib/i18n/{index,export,sortFilter,streaming,resultCache}.test.ts`, and audit catalogue
expectations in existing component/page tests (including StatusBar, ResultsTable, CutoffValue,
WindowControls, Toolbar, albums, export and cache). It must add locale isolation/reset to avoid
test order dependence. Bindings/contract tests must cover contract version 8 and its added
types/errors. Existing sort/filter and CSV tests keep their data semantics. The implementation
stage must never change these protected tests to make a translation pass.

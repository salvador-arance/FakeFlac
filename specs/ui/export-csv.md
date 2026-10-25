---
id: SPEC-UI-EXPORT-CSV
requirements: [RF-09]
status: proposed
---

# Export CSV in the main screen (H2)

> Where "Export CSV" lives, when it can be used and what the user sees afterwards. Command: `export_csv` (`specs/contracts/commands.md`, "CSV export (version 4)"); file format: `specs/contracts/csv-export.md`; scenarios: `specs/features/export-csv.feature` (tag `@RF-09`). Layout, other states and interaction stay those of `main-screen.md`; components follow `components.md` (atomic design, ADR-0014: a story per state and a component test before use, S6). Presentational components get data through props and emit events; only `AnalysisPage` calls the API client.

## Toolbar

```
[Open folder] [Cancel] │ [Export CSV]
```

The export button is a second group after a separator, as the reference tool groups "save/export" after the open and analyse buttons (`docs/architecture/frontend.md`, top bar). It is an `IconButton` (size `toolbar`) with the new icon `export` and the label `toolbar.export` (also its tooltip and accessible name). No keyboard shortcut in H2 (open question U1).

### When it is enabled

`canExport` = no run in progress **and** the table has at least one row **and** no export is in progress.

| Screen state (`main-screen.md`) | Export CSV |
|---|---|
| Empty (start) | Disabled |
| Scanning, Analysing | Disabled |
| Finished | Enabled (a finished run has ≥ 1 row) |
| Cancelled | Enabled when at least one track was done; disabled when the table is empty |
| Error (`AppError`) | Enabled when the table kept rows (a failed start after an earlier run, or a run that ended with `INTERNAL` after reporting tracks); disabled when it is empty |
| Exporting (the `exportCsv` call is pending: dialog open or file being written) | Disabled. Open folder and Cancel keep the state they had |

The backend enforces the same rules (`ANALYSIS_RUNNING`, `NOT_ANALYZED`), so a stale button cannot export a running or empty result set.

## After the export

The export never changes the screen state, the table, the selection or the detail panel. It only sets a **notice** in the status bar, shown after the state text as " · <notice>":

| Outcome of `exportCsv()` | Notice |
|---|---|
| Resolved with `ExportResult` | `status.exported` with `rows` and the file name (last component of `path`); `status.exportedOne` when `rows` is 1. Example: "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · Saved 31 rows to FakeFlac - Album A.csv" |
| Resolved with `null` (dialog cancelled) | No change: the previous notice, if any, stays |
| Rejected with `EXPORT_FAILED` or `INTERNAL` | `error.EXPORT_FAILED` |
| Rejected with `NOT_ANALYZED` | `error.NOT_ANALYZED` |
| Rejected with `ANALYSIS_RUNNING` | `error.ANALYSIS_RUNNING` (existing text) |

The notice is replaced by the next export's notice, and it is cleared by every attempt to start an analysis that changes the screen state:

- the start succeeds: the state becomes Scanning and the status bar shows "Scanning…" without the notice;
- the start fails before Scanning (`NOT_A_FOLDER` from Open folder, `FOLDER_UNREADABLE`, `NO_AUDIO_FOUND`, `ANALYSIS_RUNNING`): the state becomes Error and the status bar shows that error's text (`main-screen.md`) **without** the old notice, which the new error replaces. `NOT_A_FOLDER` has no text (`main-screen.md`), so the status bar shows "Ready" without the notice. The table keeps its rows, so Export CSV stays enabled when it has any;
- a drop whose paths are all files starts nothing and shows no error (`commands.md`, drag and drop): nothing changes and the notice stays. A drop clears the notice only when one of its `startAnalysis` calls ends other than with `NOT_A_FOLDER`, by the two rules above.

The contract's technical `message` is not shown.

## Component changes

| Component | Level | Change | States / variants to add (story + test) |
|---|---|---|---|
| `Icon` | atom | New `IconName` `export` (own SVG, 16×16 view box, like the others: a sheet with a downward arrow) | `export` decorative and labelled |
| `Toolbar` | organism | New prop `canExport: boolean` and event `export` (`onexport`); a separator between the open/cancel group and the export group | idle with export enabled · idle with export disabled · running (export disabled) |
| `StatusBar` | organism | New optional prop `notice?: string`, appended after the state text as " · <notice>" in every phase except `scanning` and `analysing` (where the page never sets it) | finished with notice · cancelled with notice · error with notice |
| `AnalysisPage` | page | Calls `exportCsv()` on `export`; keeps `exporting` (true while the call is pending) and the notice; computes `canExport` | — |

`IconButton` does not change. No new molecule.

## Texts (keys in `src/lib/i18n/`; English is the default, Spanish falls back to English until RF-13)

New: `toolbar.export` Export CSV · `status.exported` Saved {rows} rows to {file} · `status.exportedOne` Saved 1 row to {file} · `error.EXPORT_FAILED` Cannot save the CSV file · `error.NOT_ANALYZED` There are no results to export.

The save dialog's own texts (title, buttons, the overwrite question) are Windows' and follow the Windows language. The filter name "CSV (*.csv)" is not translated.

## Accessibility

The export button has a visible focus ring and its label as accessible name; when disabled it stays focusable through the toolbar like the other buttons (as `IconButton` does today). The status bar is a `role="status"` region, so the notice is announced.

## Manual checks (native dialog, not automatable)

The automated tests stub the dialog (the steps around it are `run::begin_export` and `run::export_csv`, csv-export.md section 5). Before the feature PR, check by hand in a release build:

- M1. The dialog opens modal to the window with the file name `FakeFlac - <folder>.csv` and the filter "CSV (*.csv)".
- M2. Choosing an existing CSV asks to replace it; answering No returns to the dialog and nothing is written.
- M3. Cancelling the dialog leaves the status bar unchanged.
- M4. Exporting to a CSV that is open in Excel shows "Cannot save the CSV file" and the open file is unchanged.
- M5. The written file opens in a text editor as UTF-8 with CRLF line ends and shows `″` correctly.
- M6. Extension (csv-export.md Q9). (a) Typing `results` with no extension writes `results.csv`, and when `results.csv` already exists the dialog asks to replace `results.csv` before returning. (b) Typing `results.txt`: record whether the dialog returns `results.txt` (then the status bar shows "Cannot save the CSV file" and no `results.txt` is created) or `results.txt.csv` (then that file is written). Either outcome passes; an outcome where a file not ending in `.csv` is written fails the check.
- M7. Timing (csv-export.md section 7, criterion 5). With 10 000 results kept (a run on a test folder outside the audio corpus holding 10 000 copies of a generated fixture of `specs/detection/fixtures.md`, for example `short`, which ends quickly as `tooShort`), the notice appears less than 1 second after choosing the file in the dialog, measured from the log or a stopwatch.

## Open questions for G1

| # | Question | Default |
|---|---|---|
| U1 | Keyboard shortcut for Export CSV (for example `Ctrl+S` or `Ctrl+E`)? | None in H2 |
| U2 | After a successful export, offer "Open file" or "Show in folder"? It needs the `shell`/opener capability, which the security model excludes (RNF-03) | No: notice only |
| U3 | Show the notice in the status bar (as written) or as a transient toast? There is no toast component yet | Status bar |
| U4 | The planning document (section 3.10) puts the export button in a `TableToolbar` above the table, which arrives with RF-08 (search and filters). This spec puts it in the main `Toolbar`, where the reference tool has "save/export" (section 3.14.2) and which exists today. Move it when RF-08 adds `TableToolbar`? | Main `Toolbar`; RF-08 may revisit |

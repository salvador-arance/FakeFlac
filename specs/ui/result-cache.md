---
id: SPEC-UI-RESULT-CACHE
requirements: [RF-11, RNF-08]
status: proposed
---

# Result cache in the main screen (H2)

> What the user sees of the result cache: how many tracks of a run came from the cache, a warning when the cache could not be used, and a toolbar button that clears it. Behaviour and contract: `specs/contracts/result-cache.md` and `specs/contracts/commands.md` ("Result cache (version 7)"). Scenarios: `specs/features/result-cache.feature` (tag `@RF-11`, section "Interface"). Layout, the other states and interaction stay those of `main-screen.md`, `export-csv.md` and `sort-filter.md`. Components follow `components.md` (atomic design, ADR-0014: a story per state and a component test before use, S6). Only `AnalysisPage` calls the API client.

## 1. What is not shown (G1 defaults, result-cache.md Q4)

- **No per-row mark.** The table, the detail panel and the CSV show no "from cache" mark: a reused result is the same result. The run's count is in the status bar.
- **No live count.** While a run is in progress the status bar shows nothing about the cache; the figures arrive with the run's summary.
- **No setting** to turn the cache off (result-cache.md D3: settings are RF-13).

## 2. Toolbar

```
[Open folder] [Cancel] │ [Export CSV] [Clear cache] │ [✖] [!] [✔] [⚠]          [🔍 Search (Ctrl+F)  ✕]
```

- **Clear results cache** is an `IconButton` (size `toolbar`) placed right after Export CSV, in the same group (result-cache.md Q5).
  - The group holds actions on the results rather than on the folder.
  - Its icon is the new `clear-cache`. Its label `toolbar.clearCache` is also its tooltip and accessible name.
  - No keyboard shortcut.
- **Width.** `sort-filter.md` section 2 counted 7 buttons, 2 separators and the field at about 600 px. One more 40 px button (plus the toolbar gap) brings it to about 650 px, within the 900 px minimum window (manual check M1). `sort-filter.md` section 2 gets a cross-reference sentence with the new count (`contracts/result-cache.md` section 13).

### 2.1 When it is enabled

`canClearCache` = no run in progress (phase not `scanning` and not `analysing`) **and** no clear in progress.

| Screen state (`main-screen.md`) | Clear results cache |
|---|---|
| Empty (start) | Enabled |
| Scanning, Analysing | Disabled |
| Finished, Cancelled, Error | Enabled |
| Exporting (`export-csv.md`) | Enabled (the export does not use the cache) |
| Clearing (the `clearCache()` call is pending) | Disabled |

**While clearing**, Open folder is disabled as well (`canOpen` is false), and a folder dropped on the window is ignored, as during a run (the page returns before `startAnalysis`). The backend holds the analysis while it clears (`ANALYSIS_RUNNING`), so a stale click could not start a run anyway. Clearing takes milliseconds, so this is rarely visible.

### 2.2 After clearing

There is **no confirmation** before clearing (result-cache.md Q6). Clearing never changes:
- the screen state, the table or the selection;
- the detail panel or the Albums panel;
- the filters;
- the results kept for the export.

It only sets the status bar **notice** (the mechanism of `export-csv.md`, "After the export"):

| Outcome of `clearCache()` | Notice |
|---|---|
| Resolved | `status.cacheCleared` |
| Rejected with `CACHE_FAILED` or `INTERNAL`, or a non-`ApiError` | `error.CACHE_FAILED` |
| Rejected with `ANALYSIS_RUNNING` | `error.ANALYSIS_RUNNING` (existing text) |

The notice follows the existing rules: it replaces the previous notice (for example an export's), and every attempt to start an analysis clears it (`export-csv.md`). The contract's technical `message` is not shown.

## 3. Status bar

The run's cache figures are appended to the **state text** in the phases `finished` and `cancelled` only. They come **before** the shown count of RF-08 and the notice:

```
<state text>[ · {n} from cache][ · <cache warning>][ · {shown} shown][ · <notice>]
```

| Part | When | Text |
|---|---|---|
| ` · {n} from cache` | `cache.fromCache > 0` | `status.fromCache` with `n` = `fromCache` |
| ` · <cache warning>` | `cache.status` is `degraded` | `status.cacheDegraded` |
| | `cache.status` is `unavailable` | `status.cacheUnavailable` |
| | `ok` or `off` | nothing |

Neither part appears in `ready`, `scanning`, `analysing` or `error`. A run that ends with `INTERNAL` has no summary, so its status bar shows only its error text, as today.

Examples:
- "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 31 from cache";
- "Cancelled · 18 of 31 · 12 from cache";
- "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · Results cache unavailable";
- "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 10 from cache · Results cache error: some results were not saved · 3 shown · Saved 31 rows to FakeFlac - Album A.csv";
- after clearing: "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 31 from cache · Results cache cleared". The count still describes the last run.

**Which run the figures come from.** They are those of the latest run that ended `finished` or `cancelled`:
- a new run removes them when it starts (`begin`);
- a start that fails before any event brings back the previous ones along with the previous rows (`restore`, as `export-csv.md` does for the notice).

## 4. Component changes

| Component | Level | Change | States / variants to add (story + test) |
|---|---|---|---|
| `Icon` | atom | New `IconName` `clear-cache` (own SVG, 16×16 view box like the others: a database cylinder with a small diagonal cross at its lower right; D12, no copied icon) | `clear-cache` decorative and labelled |
| `Toolbar` | organism | New **optional** prop `canClearCache?: boolean`, default `false` (like `canExport`), and event `clearCache` (`onclearcache`); the button right after Export CSV in the export group (section 2). Optional so that the existing `Toolbar` tests and stories type-check unchanged (`contracts/result-cache.md` section 12.2); `AnalysisPage` always passes it | idle with clear enabled · clear disabled while clearing (Open folder disabled too) · running (clear disabled) · tab order with clear enabled (section 6) |
| `StatusBar` | organism | New optional prop `cache?: CacheFigures`, rendered as in section 3. `src/lib/model.ts` re-exports `CacheStatus` from the generated bindings (components may not import `src/lib/api`) and adds the UI type `export type CacheFigures = { fromCache: number; status: CacheStatus }` | finished from cache · finished with `unavailable` · finished with `degraded` and from cache · cancelled from cache · finished with `ok` and 0 from cache (nothing appended) · finished with cache, shown and notice (order) · analysing and error with `cache` set (not appended) |
| `AnalysisPage` | page | Keeps `clearing` (true while `clearCache()` is pending); `canClearCache` (2.1); `canOpen` is also false while clearing, and `handleOpenFolder` / `handleDrop` return at once while clearing; calls `clearCache()` on `clearCache` and sets the notice (2.2); passes `store.cache` (below) to `StatusBar` | — |

`IconButton`, `ResultsTable`, `DetailPanel`, `AlbumSummaryList` and `MainLayout` do not change. No new molecule.

**Store** (`src/lib/stores/analysis.svelte.ts`):
- New `cache: CacheFigures | null`, initially `null`.
- The `finished` and `cancelled` events set it from `event.summary` (`fromCache`, `cache`).
- `begin()` sets it to `null` and keeps the previous value for `restore()`, which brings it back.
- `reset()` and `fail()` leave it as it is; the status bar does not show it in `ready` or `error`.

**API client** (`src/lib/api/`): `clearCache(): Promise<void>`. It calls the generated `clearCache` command and rejects with `ApiError` on a contract error, like `exportCsv`.

## 5. Texts (keys in `src/lib/i18n/`; English is the default, Spanish falls back to English until RF-13)

New:
- `toolbar.clearCache`: Clear results cache
- `status.fromCache`: {n} from cache
- `status.cacheDegraded`: Results cache error: some results were not saved
- `status.cacheUnavailable`: Results cache unavailable
- `status.cacheCleared`: Results cache cleared
- `error.CACHE_FAILED`: Cannot clear the results cache

`status.fromCache` takes no plural form, because "1 from cache" reads correctly.

## 6. Accessibility

- The button's accessible name and tooltip are "Clear results cache". It has the visible focus ring.
- **When disabled it is not a tab stop**, like the other toolbar buttons: `IconButton` sets the native `disabled` attribute (`src/lib/components/atoms/IconButton.svelte`). That is also why the existing tab-order test of `Toolbar.filters.test.ts` enables Cancel.
- Tab order with every toolbar button enabled: Open folder, Cancel, Export CSV, Clear results cache, the verdict buttons, the search field. This is `sort-filter.md` section 13 with one more stop, and that section gets a cross-reference sentence (`contracts/result-cache.md` section 13). The new `Toolbar.clearCache.test.ts` checks this order with `canClearCache` true.
- The status bar is a `role="status"` region, so the cache figures, the warning and the notice are announced. The warning is text, never colour alone (RNF-08).

## 7. Manual checks (before the feature PR, release build)

- **M1.** At 900 px window width the toolbar line holds every button, both separators and the search field; the `clear-cache` icon reads as distinct from `export`.
- **M2.** The speed-up and location checks M1–M3 of `contracts/result-cache.md` section 11 are done through this screen. The status bar shows "N from cache" with N = the track count on the warm run.
- **M3.** With a second FakeFlac window holding the cache (analysing a large folder), "Clear results cache" in the first window shows "Cannot clear the results cache" and the cache still works afterwards.

---
id: SPEC-LOOK-AND-FEEL
requirements: [RNF-10]
status: approved
---

# Look and feel (RNF-10)

> RNF-10 asks for a Windows desktop application styled like the reference tool (`docs/reference/`): single window, icon toolbar, folder tree, dense table, detail panel, with its own assets and no copied branding. Most of it is already specified, one piece at a time, under `specs/ui/`. This spec only maps each element of the requirement to the spec and the tests that already cover it, says what is not covered, adds a register of the shipped graphic assets and one manual check (M1). It changes no approved spec, no component, no prop or event, and no command, event or DTO (`contracts/commands.md` is unchanged). Decisions: ADR-0027 (classic desktop UI inspired by the reference tool), ADR-0014 (atomic design), `docs/architecture/frontend.md` "UI style (RNF-10, ADR-0027)", and risk D12 of `docs/planning/fakeflac-phases-1-3.es.md` (copy only the layout and the handling, never logos, icons, texts or brand colors). Scenarios: `../features/look-and-feel.feature` (tag `@RNF-10`).

## 1. Element map

Tests were checked by file listing and grep on 2026-11-19. "Guard" means an automated test that fails if the element regresses. CSS geometry (sizes, colors as rendered) cannot be checked in jsdom (ADR-0037), so the visual side of every row is left to M1.

| # | Element | Specified in | Automated guard today | Not covered | Proposed closure |
|---|---|---|---|---|---|
| L1 | **Single window** | `main-screen.md` "Layout" (one window, own title bar, minimum 900 × 500 "already in `tauri.conf.json`") and "Interaction" (drag, double-click to maximize, three buttons); `components.md` `TitleBar`, `WindowControls`; `language-settings.md` section 2 (Settings is a modal over the current screen, not a second window) and section 4 (`MainLayout` slot `settings`) | `src/lib/components/organisms/TitleBar.test.ts`, `src/lib/components/molecules/WindowControls{,.locale}.test.ts`, `src/lib/pages/AnalysisPage.test.ts` ("minimises, toggles maximise and closes through the client", "maximises on double click of the title"), `src/lib/components/templates/MainLayout.settings.test.ts`, `src/App.test.ts` | No test reads `app.windows` of `src-tauri/tauri.conf.json` (one window, `decorations: false`, minimum size); `scripts/release.test.mjs` reads that file only for the version, the product name and the bundle. Nothing would notice code that opens a second window | S4 |
| L2 | **Icon toolbar** | `main-screen.md` "Layout"; `components.md` `Icon` (own SVG set), `IconButton` (tooltip with `label`, toolbar size 40 × 40), `Toolbar` (groups and separators like the reference); `export-csv.md` "Toolbar"; `result-cache.md` section 2; `sort-filter.md` sections 2 and 10 (verdict filters, search field at the right); `language-settings.md` section 2 (Settings button) | `src/lib/components/atoms/{Icon,Icon.search,Icon.settings,Icon.clearCache,IconButton,IconButton.toggle}.test.ts`, `src/lib/components/molecules/{VerdictFilter,SearchField}.test.ts`, `src/lib/components/organisms/{Toolbar,Toolbar.filters,Toolbar.clearCache,Toolbar.settings}.test.ts` | No toolbar-level check that every command is an icon-only button whose tooltip is its label (`IconButton.test.ts` checks one button alone). Button and bar sizes are CSS. Reference buttons with no v1 counterpart (analyze, remove from list, help, exit, store links) are out of v1 (`frontend.md`) | S3; M1 E2 |
| L3 | **Folder tree** | No spec for a folder tree. Its place is taken by the Albums panel: `album-summary.md` "Where it appears: the Albums panel" ("in the place the reference tool gives its folder tree"), `resizable-widths.md` section 1 (resizable, not collapsible). `frontend.md` says the folder tree is "decided but not built yet" (its row "My PC", drives, folders), and `album-summary.md` (lines 43 and 152) says "the folder tree of a later milestone reuses" the panel: the tree was deferred, not forgotten | `src/lib/components/organisms/{AlbumSummaryList,AlbumSummaryList.edges,AlbumSummaryList.locale}.test.ts`, `src/lib/components/templates/{MainLayout.sidebar,MainLayout.widths}.test.ts`, `src/lib/pages/AnalysisPage.albums.test.ts` | The folder tree itself ("My PC", then drives, then folders; selecting a folder lists its files; collapsible) has no requirement, spec or code | Q1 (default: the Albums panel closes this element for v1; the tree is a deliberate difference recorded in M1) |
| L4 | **Dense table** | `main-screen.md` "Layout" (ten columns); `components.md` `ResultsTable` (virtualized rows, row height token), `ColumnHeader`, `VerdictCell`; `lossy-formats.md` (bitrate columns); `resizable-widths.md` (column widths, horizontal scroll); `sort-filter.md` section 3; `verdict-usability.md` U3 (layout at 900 × 500) | `src/lib/components/organisms/ResultsTable.test.ts` ("is a grid with the ten columns and no data rows when empty", "renders only a window of a long list"), `ResultsTable.{widths,scroll,sortFilter,locale,contrast,streaming,reveal.svelte}.test.ts` in the same folder, `src/lib/components/molecules/{ColumnHeader,ColumnHeader.resize,VerdictCell}.test.ts`, `src/lib/pages/AnalysisPage.tenThousand.test.ts` | Density itself: no approved spec fixes a row height or a font size as a requirement (`album-summary.md` names "the current 22 px" of `--row-height`) and no test reads those tokens | M1 E4; Q4 (default: no token guard) |
| L5 | **Detail panel** | `main-screen.md` "Layout" (below the table, splitter, default 260 px, min 160) and "Interaction"; `components.md` `DetailPanel`, `SpectrogramView`, `Splitter`; `lossy-formats.md`; `fake-evidence.md` (proposed, RF-17) | `src/lib/components/organisms/{DetailPanel,DetailPanel.lossy,DetailPanel.streaming,DetailPanel.locale,DetailPanel.evidence,SpectrogramView}.test.ts`, `src/lib/components/atoms/Splitter.test.ts`, `src/lib/components/templates/MainLayout.test.ts` (splitter), `src/lib/pages/AnalysisPage.spectrogram.test.ts` | Only the visual placement in the reference's player area | M1 E5 |
| L6 | **Windows desktop styling** | `main-screen.md` intro ("One dark window modelled on the reference tool"); `components.md` (styling only through tokens, text contrast ≥ 4.5:1 on the dark background); `verdict-usability.md` U3; tokens in `src/lib/tokens/tokens.css` (`color-scheme: dark`, `--font-ui: 'Segoe UI', system-ui, sans-serif`); `frontend.md` "UI style" (architecture, not a spec). The platform itself is RNF-07 (`contracts/windows-platform.md`) | `src/lib/components/organisms/ResultsTable.contrast.test.ts` (text contrast computed from the tokens), `src/lib/components/templates/MainLayout.sidebar.test.ts` (layout token) | Dark theme by default and the Windows system font are not checked automatically. Two `frontend.md` decisions are neither specified in `specs/ui/` nor built: "size and position remembered" (no code stores the window position) and "light optional (H2)" | M1 E6; Q5 (default: both out of RNF-10's scope) |
| L7 | **Own assets, no copied branding** | `components.md` `Icon` ("Own SVG set (no copied icons)"); `frontend.md` "Not copied"; D12; section 2 of this spec | None. The `Icon` tests check structure (view box, `aria-*`), not origin | Origin of the UI icons and of the app icon files is not recorded anywhere; no check would notice a copied file | Section 2: asset register, S5 and S6; Q2, Q3, Q6; M1 E7 |

The page as a whole: `MainLayout.test.ts` and `MainLayout.sidebar.test.ts` check the order of the regions with stub content, and no test checks them together on the real page. S1 does that after a run, with an error row for a corrupt file (RNF-04) and a Unicode name (RNF-09), and S2 does it in the empty state (`main-screen.md` "Screen states": table and detail panel hidden).

Scenarios of `../features/look-and-feel.feature` (new, for the test stage):

| Id | Scenario | Closes |
|---|---|---|
| S1 | A finished run shows the regions the requirement names | L1, L3, L4, L5 on the real page |
| S2 | The empty state shows the window frame without the content regions | Same, empty state |
| S3 | Every toolbar command is an icon button with its label as tooltip | L2 |
| S4 | The application declares one window without native decorations | L1 |
| S5 | Every shipped graphic file has a recorded origin | L7, check A1 |
| S6 | No shipped file copies a reference screenshot | L7, check A2 |
| — | The owner compares the release app with the reference (M1) | Visual side of L1 to L7 |

## 2. Own assets, no copied branding

### 2.1 What the repository ships (listed on 2026-11-19)

- **App icon.** Six files in `src-tauri/icons/`: `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.ico`. `tauri.conf.json` `bundle.icon` names four of them (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`); the six are treated as shipped. `icon.png` and `32x32.png` were viewed: they show the drawing of `docs/reference/app-icon-source.png` (orange bars on a dark square), which the owner confirmed on 2026-11-20 as their own drawing (Q3). The tool that made the six files from it is not recorded.
- **UI icons.** 15 inline SVG paths in `src/lib/components/atoms/Icon.svelte`, one per `IconName` of `src/lib/components/atoms/icons.ts`. No icon package is a dependency (`package.json`: the only runtime dependency is `@tauri-apps/api`).
- **Fonts.** None shipped. `--font-ui` uses the Windows system font (Segoe UI) with system fallbacks; there is no `@font-face`.
- **Logos, illustrations, images.** None. There is no `public/` folder, no image under `src/`, and `index.html` has no favicon link. The window title and the product name are "FakeFlac" (`tauri.conf.json`).
- **Texts.** Own catalogue, `translations.md`.
- **Colors.** `tokens.css`; spectrogram palette `src/lib/tokens/spectrogram.ts`.
- **`docs/reference/`** (12 reference screenshots and `app-icon-source.png`) is not shipped. The frontend bundle is built by Vite from `index.html` and `src/` into `dist/` (`tauri.conf.json` `frontendDist`), and the installer adds the `src-tauri/icons/` files. Nothing under `src/` or `src-tauri/`, nor `index.html`, `vite.config.ts`, `package.json` or `.storybook/`, imports or names a file of `docs/reference/`; the only mention is a comment in `tokens.css`. The reference screenshots stay in `docs/reference/` and are never copied into `src/` or `src-tauri/` (check A2). A built installer was not inspected in this stage.

### 2.2 Asset register

The data that check A1 reads. A new shipped image, icon or font file, or a new `IconName`, needs a row here in a `spec/…` PR (G1), so a new asset is always seen by the owner before it ships.

**Format (what the test reads).** The test reads this section the way `catalogue.test.ts` reads `translations.md`: it takes the two tables under the heading `### 2.2 Asset register`. In each row, the first cell is one backticked name: a repository-relative file path written with `/` in the first table, one `IconName` in the second. The Origin cell starts with one word or phrase of this closed vocabulary: `own drawing`, `derived from own drawing`, `redrawn`, `third party` (followed by the name and the licence) or `unrecorded`. Anything else fails the test. `unrecorded` passes the test and is counted (K2).

| File | Kind | Origin | Shipped as |
|---|---|---|---|
| `src-tauri/icons/32x32.png` | App icon | derived from own drawing: `docs/reference/app-icon-source.png`; the tool that made the file is not recorded | Installer and window icon |
| `src-tauri/icons/64x64.png` | App icon | derived from own drawing (same) | Same |
| `src-tauri/icons/128x128.png` | App icon | derived from own drawing (same) | Same |
| `src-tauri/icons/128x128@2x.png` | App icon | derived from own drawing (same) | Same |
| `src-tauri/icons/icon.png` | App icon | derived from own drawing (same) | Same |
| `src-tauri/icons/icon.ico` | App icon | derived from own drawing (same) | Same |

| `IconName` (inline SVG in `Icon.svelte`) | Origin |
|---|---|
| `folder-open` | own drawing |
| `cancel` | own drawing |
| `export` | own drawing |
| `clear-cache` | own drawing |
| `minimize` | own drawing (the bar has the same commands as Bootstrap Icons `dash-lg`, with coordinates shifted by 0.5 px; see the note below) |
| `maximize` | own drawing |
| `close` | redrawn (an original filled × in both branches of `Icon.svelte`, #182, merged 2026-11-20; before it the path was the glyph `x-lg` of Bootstrap Icons 1.10 and 1.11.0, MIT) |
| `verdict-real` | own drawing (the check mark has the same commands as Bootstrap Icons `check2`, with other coordinates; see the note below) |
| `verdict-suspicious` | own drawing |
| `verdict-fake` | own drawing |
| `verdict-error` | own drawing |
| `sort-asc` | own drawing |
| `sort-desc` | own drawing |
| `search` | own drawing |
| `settings` | own drawing |

**How the origins were recorded (owner, 2026-11-20, Q3).** The owner accepted the proposal: the app icon files are derived from the owner's own drawing, the tool that made them is not recorded, and 14 of the 15 UI icons are the owner's own drawing. The 15 paths were compared on 2026-11-20 with the 2,050 symbols of the Bootstrap Icons 1.11.3 sprite (MIT, from its public text), by token overlap. Only `close` is identical to a symbol (`x-lg` in 1.10.5 and 1.11.0; 1.11.3 only drops a redundant tail): it is redrawn. The next highest matches were `export` 68 %, `minimize` 62 % and `verdict-real` 59 %, none identical. `minimize` and `verdict-real` follow the construction of two Bootstrap glyphs and the owner kept them; redrawing them is an optional change that needs no spec change. Not compared: Octicons, Lucide, Feather, Material, Fluent, Phosphor and Tabler. A later check can only change a row in a `spec/…` PR.

### 2.3 How a copied asset would be noticed

- **A1 (automated, scenario S5).** The files with extension `png`, `ico`, `icns`, `svg`, `jpg`, `jpeg`, `gif`, `webp`, `bmp`, `woff`, `woff2`, `ttf` or `otf` under `src/`, `src-tauri/` (leaving out `src-tauri/target/` and `src-tauri/gen/`) and `public/` are exactly the file rows of the register. The `IconName` values are exactly the icon rows. Every origin starts with a word of the vocabulary of 2.2. The test prints how many rows are `unrecorded` and does not fail on them: a red test that only the owner can turn green would block every later `just check`.
- **A2 (automated, scenario S6).** No file in those folders has the same SHA-256 (whole file, byte for byte) as a file of `docs/reference/` other than `app-icon-source.png`; the app icon files may equal or derive from that one. Only a verbatim copy is found. No `import`, CSS `url()`, HTML `src` or `href`, or `tauri.conf.json` value names a path under `docs/reference/`; plain comments (the one in `tokens.css`) are not scanned.
- **M1 E7 (manual).** A redrawn copy of a reference logo, icon or text cannot be found by A1 or A2. The owner's visual comparison finds it.

### 2.4 Out of scope

- A legal clearance or trademark search.
- Copying the layout and the handling. That is intended (ADR-0027, D12).
- Dev-only assets that never reach the installer (`node_modules/`, Storybook).
- System UI: WebView2, the Windows folder picker and save dialog (`translations.md`).
- Generic conventions shared with many tools, such as the `Ctrl+F` shortcut of the search field (`sort-filter.md`).

## 3. Acceptance (measurable)

- **K1.** Every scenario of `look-and-feel.feature` except the M1 one has a passing test in `just check` (S1 to S6).
- **K2.** The asset register is complete: A1 passes and no row is `unrecorded`. A1 does not fail on `unrecorded` rows, so K2 is closed by the owner, by recording each origin in a `spec/…` PR (G1). The owner recorded every origin on 2026-11-20 (Q3). The redraw of `close` was merged on 2026-11-20 (#182), so K2 holds.
- **K3.** M1 is recorded on a release build, with each of E1 to E7 marked pass, or marked as a deliberate difference that the owner accepted at G1 (section 4 lists the known ones).

RNF-10 is verified when K1 to K3 hold.

## 4. Manual check M1 (owner, release build)

Setup. Install a release build (`just build` installer or a release asset) on Windows 11 x64. Use a test folder of synthetic fixtures (`specs/detection/fixtures.md`) that gives at least one Genuine, one Fake and one Error row. Window sizes are 1280 × 720 (the default) and 900 × 500, in English. Spanish layout is already covered by RF-13 M2 and RNF-08 U3. Open the reference screenshots in `docs/reference/` side by side with the app (`ftf-main-window-1.png`, `ftf-main-window-2.png`, `pantalla-principal.png`, `vision-espectro.png`, `ajustes-*.png`).

Compare element by element:

| # | Element | Pass when |
|---|---|---|
| E1 | Single window | One OS window with no native title bar. The own dark title bar shows the title and minimize, maximize and close; drag and double-click work. Settings opens inside the window (the reference opens its options in a window of its own, `ajustes-*.png`). The Windows folder dialog is system UI and is allowed |
| E2 | Icon toolbar | Icon buttons in groups with separators, a tooltip on hover, the search field at the right, as in the reference's top bar |
| E3 | Left panel | The Albums panel takes the place of the reference's folder tree, at the full content height |
| E4 | Dense table | Rows as compact as the reference's. State is the first column, with icon, text and color. Sortable headers, horizontal scroll, the selected row highlighted |
| E5 | Detail panel | Spectrogram with the cutoff line and the verdict line, in the place of the reference's player, resizable with the splitter |
| E6 | Windows desktop styling | Dark theme on every surface (title bar, toolbar, panels, table, detail panel, status bar, Settings). Windows system font. It reads as a desktop tool, not a web page (planning 3.14.1) |
| E7 | Own assets, no copied branding | The icon in the taskbar, the title bar and the installer is FakeFlac's. No logo, name, icon, illustration or text of the reference tool appears in the app or the installer. Q6 decides the accent color |

Deliberate differences known before the check (record them, do not fail on them): no folder tree (Q1); no player; no analyze, remove-from-list, help, exit or store buttons; columns in the order State · File · Folder · Format · …; the Albums panel is not collapsible (`resizable-widths.md`); Settings holds only the language (`language-settings.md`); window size and position are not remembered (Q5).

Record only aggregates: candidate commit, date, build (installer type and version), window sizes, and per element pass, fail or deliberate difference with one line of note. **No screenshot is committed, and no name or path of a music file is recorded** (ADR-0022). A failure stays visible as pending.

### 4.1 Result (2026-11-20): M1 passed

Run with a desktop agent for E1 to E6 and by the owner for E7. The owner marked every element pass.

| Item | Value |
|---|---|
| Candidate | Branch `feat/RNF-10` at `398e6ca` (feature PR #182), FakeFlac 0.3.0 |
| Build | Release executable from `npm run tauri build -- --no-bundle` on Windows 11 Pro, build 26300. No installer was built or run by the agent |
| Window sizes | The default size (1296 × 729 outer, as measured) and 900 × 500 outer. English |
| Fixtures | Synthetic, generated for this run (`fixtures.md`): one full-band noise WAV (Genuine), the same noise low-passed at 16 kHz (Fake, cutoff 16.1 kHz), one 1 s WAV (Error) and one corrupt `.flac` (Error) |
| E1 | **Pass.** One window, no native title bar; minimize, maximize and close present; drag and double-click (maximize, restore) work; Settings opens inside the window, dark, with only the language |
| E2 | **Pass.** Icon buttons in groups with separators, search field at the right, as in `ftf-main-window-1.png` |
| E3 | **Pass.** The Albums panel takes the full content height, where the reference has the folder tree |
| E4 | **Pass.** Dense rows; State is the first column with icon, text and color; sortable headers; horizontal scroll; selected row highlighted |
| E5 | **Pass.** Spectrogram with the cutoff line and the verdict text; the splitter resizes it (checked with the keyboard) |
| E6 | **Pass.** Dark on every surface; system font; reads as a desktop tool |
| E7 | **Pass** (owner). FakeFlac's own icon; no mark of the reference tool. The redrawn `close` icon is part of this check |

**Deviations.**
- The agent could not resolve the text of the toolbar tooltips (drawn by a WebView2 process that the screen capture hides) or the end caps of the redrawn `close` (about 10 px on screen). Both are covered by the owner's pass, not by the agent's capture.
- Dragging the splitter and the window edge with the mouse did not register from the agent's synthetic input. The splitter was moved with the keyboard and the 900 × 500 size was set with a window API. This is a limit of the tool, not a finding in the app.
- The first run used the installed app by mistake. It was discarded, that instance was closed, and every result above is from the branch build.
- At 900 × 500 the table shows about three and a half rows until the detail panel is made smaller. Nothing overlaps. The deliberate differences of section 4 were seen as listed, with no new ones.

Result: **M1 passed**. K3 is met.

## 5. Questions for G1 (answered by the owner on 2026-11-20)

The owner accepted every default, after a read-only check of each one against the repository. The facts that check added are in the answers.

| # | Question | Default (and answer) |
|---|---|---|
| Q1 | **Folder tree.** Does the Albums panel (`album-summary.md`) close RNF-10's "folder tree" for v1, leaving the reference's tree (drives and folders, selecting a folder lists its files, collapsible) to a future requirement? Note: the source row of RNF-10 in the planning document (section 2.4) names a single window, a file list with columns, a spectrum panel and drag and drop, but no folder tree. The tree comes from section 3.14.2 and ADR-0027 | Yes. M1 E3 records it as a deliberate difference. This spec creates no new requirement; the owner may open one. **Resolved (owner, 2026-11-20): yes.** The sentence of `requirements/RNF-10.md` now says that the Albums panel takes the place of the folder tree for v1, so the requirement and this spec agree |
| Q2 | Make the asset register checks A1 and A2 automated tests (S5, S6), or rely on M1 only? | Automated. Like `catalogue.test.ts`, which reads `translations.md`, the test reads the register from this spec. **Resolved (owner, 2026-11-20): automated.** Both checks were run by hand on 2026-11-20 and pass: the image and font files under `src/`, `src-tauri/` and `public/` are exactly the six icon files, and none is byte-identical to a file of `docs/reference/` |
| Q3 | **Origin of the icons.** `components.md` says "Own SVG set (no copied icons)", but nothing records who drew the 15 icons. The `close` path appears to match the large "x" glyph of Bootstrap Icons (MIT), and other paths follow that set's 16 × 16 conventions. This impression is from memory and was not checked against that set's source. The tool that produced `src-tauri/icons/` from `app-icon-source.png` is not recorded either | The owner records the origin of each icon and the icon tool in the register (K2). If any glyph is a copy of a third-party icon, it is redrawn (so the approved "no copied icons" stays true) rather than the approved text being amended. Alternative: keep it with its licence notice shipped and an ADR for the new third-party asset. A review that compared the 15 paths from memory found `close` identical to a glyph of Bootstrap Icons (MIT) and `minimize`, `maximize`, `cancel`, `export`, `verdict-real` and `clear-cache` built with the same conventions; `settings`, `sort-asc`, `sort-desc`, `search`, `folder-open`, `verdict-suspicious` and `verdict-error` look hand-built. Not checked against the set's source. Each path appears twice in `Icon.svelte` (labelled and decorative branch): a redraw changes both. **Resolved (owner, 2026-11-20): the proposal was accepted.** `close` is identical to the Bootstrap Icons glyph `x-lg` (verified against the public text of versions 1.10.5, 1.11.0 and 1.11.3) and is redrawn; the other 14 icons are recorded as the owner's own drawing and the app icon files as derived from the owner's own drawing, with the tool unrecorded. The register in 2.2 has every row and the method of the comparison |
| Q4 | Guard the density with a token test (`--row-height: 22px`, `--font-size: 12px`)? | No. No approved spec fixes these numbers as requirements, and a guard would freeze a styling choice. Density stays the visual M1 E4. **Resolved (owner, 2026-11-20): no guard.** `tokens.css` has `--row-height: 22px` and `--font-size: 12px`, and the rows of the reference screenshot sit at the same 22 px pitch, so E4 is expected to pass |
| Q5 | Two `frontend.md` decisions are not built: "size and position remembered" and "light optional (H2)". Are they in RNF-10's scope? | No. M1 records them as deliberate differences. Updating `frontend.md` is left to `docs-keeper`. **Resolved (owner, 2026-11-20): out of scope.** There is no window-state plugin and no light theme today; panel widths are remembered, window size and position are not. `docs-keeper` moves both items of `frontend.md` to the backlog |
| Q6 | **Accent color.** D12 forbids the reference's brand colors. `frontend.md` adopts the orange accent seen in the reference (`--color-accent: #e8892b`), and the app icon is orange on dark. Is a dark theme with an orange accent acceptable as layout and handling rather than branding? | Yes, as a generic accent, if the owner confirms at G1 that the orange is not the reference tool's brand color (the reference's accent was not measured). The answer is recorded in this table; M1 E7 checks it again. Changing it would be a separate styling task. **Resolved (owner, 2026-11-20): yes, as a generic accent.** Measured on the PNGs of 2026-11-20: the accent `#e8892b` is also the orange of the app icon (hue about 30°, lightness 54 %, saturation 80 %). The orange UI pixels of the reference are `#cc802a`, `#c77d2e` and `#c27a2d` (hue about 32°, lightness 48 %, saturation 66 %): the same family, not the same value. The reference tool's own logo is also orange, so this is a choice of an orange accent on a dark theme as layout and handling (ADR-0027), not a copy of a brand value. M1 E7 checks it again |
| Q7 | M1 data and language: synthetic fixtures, English only? | Yes. The Spanish layout is covered by RF-13 M2 and RNF-08 U3. **Resolved (owner, 2026-11-20): yes.** `detection/fixtures.md` holds the synthetic fixtures |

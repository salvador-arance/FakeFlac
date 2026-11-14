# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

## 2026-11-14 - RP-11 v5 corpus acceptance after G2

**Done**
- PR [#132](https://github.com/salvador-arance/FakeFlac/pull/132) is merged as `fac327a`; G2 is closed. Ran `just accept` on that commit with the defaults: calibration config v5 and shipped detection v8. Report id: `20261114T130952Z`; detailed outputs remain in the private calibration directory.
- T1 PASS: 0/78 primary tracks, 0/6 albums (95 % CI 0.0-4.7 %); holdout 0/23 tracks, 0/2 albums (95 % CI 0.0-14.3 %). Primary Suspicious: 8/78 (10.3 %).
- T2 FAIL as accepted: 3 detectable `Real` on all, 1 on holdout; 5 and 4 undetectable transcodes excluded, respectively. Compared track identities against pre-G1 run `20261114T113810Z`: exactly the quiet piano, clipping-masked cutoff and 14450 Hz step, with no new miss.
- T3 PASS: 5/2021 on all (0.2474 %, displayed 0.2 %) and 2/548 on holdout (0.3650 %, displayed 0.4 %), against the 1.0 % bound. Undetectable transcodes: 475 overall (88 extent, 387 hf drop). All 15 lossless confirmed fakes remain `Fake`.
- The 12 Bandcamp WAV tracks (one primary album, tune) were already incorporated by the pre-G1 measurement, despite the older maintenance handoff saying incorporation was pending. All 12 are `Real`, with 108 lossless and 168 lossy generated rows (9 and 14 settings). Relative to the pre-G1 run: 49,443 rows, no verdict, measurement or split changes.
- Cache: 49,443 hits, 0 decoded, 0 transcoded, 0 generation failures and 0 skipped lines; 0 track errors. Every active corpus label and generated source path resolves. Corpus verified before/after: all 12,836 files retain size and mtime, with no additions or removals.
- `just prune-branches` completed: nothing to delete; only the active documentation branch and main remain locally. The formerly carried-over release branch is absent locally and on origin; v0.2.0 publication is already recorded in #120.
- Updated RP-11 traceability for merged G2 and this manual acceptance run. Documentation branch: `docs/RP-11-acceptance-v5`.
- `just check` passed (all Rust suites; 83 frontend files / 1,430 tests), as did `just trace` and `git diff --check`.

**Pending**
- None for the acceptance of #132; no implementation or spec behaviour change is needed.
- No grid search in this session. The last L5 run (`20261102T170123Z`) predates the additional Bandcamp album; rerun `/calibrate` when resuming parameter tuning.

**Blockers / questions for the human**
- None for this acceptance. The existing limitations remain: small primary sample and no confirmed-fake holdout album.

**Next step:** choose the next requirement; RP-11 acceptance for #132 is complete.

## 2026-11-14 - RP-11 acceptance targets implemented (PR #132, G2 pending)

**Done**
- G1 closed: spec PR #131 merged as `7999698`; feature branch `feat/RP-11` starts at `b36f69e`.
- Tests `49a63fc`: exact rate bounds, target exclusions and report cells, undetectable criteria, legacy configs and default v5; authorized existing test updates from calibration.md OQ-31 (d). New tests compiled and failed for the expected reasons before implementation.
- Implementation `8fb40e3`: rate targets compared in integer thousandths, config validation, error/undetectable exclusions, criterion metadata and reports, v5 default. No dependency or app detection changes.
- Fresh review agent found no blocking or improvement findings. Specs and protected tests were unchanged during implementation.
- `just check` and final `just ci` passed (all Rust suites, 83 frontend files / 1430 tests, generated bindings and Tauri adapter checks). `just guard-tests`, `just lint-arch` and `git diff --check` passed; privacy check found 0 matches.
- Traceability updated for the new executable scenarios. No new acceptance or calibration run on the local corpus.
- With the owner's approval, published `feat/RP-11` and opened feature PR [#132](https://github.com/salvador-arance/FakeFlac/pull/132).

**Pending**
- Owner's G2 review and merge of PR #132.
- Carried over: `docs/release-0.2.0-record` (commits after #119).

**Blockers / questions for the human**
- None for implementation.

**Next step:** owner reviews and merges PR #132 (G2).

## 2026-11-14 - RP-11 acceptance-target spec: review fixes (PR #131, G1 pending)

**Done**
- Spec PR #131 (`spec/RP-11-targets`) reviewed; six findings, all in the calibration specs.
- T2's third detectable `Real` row confirmed on run `20261114T113810Z` as the 14450 Hz step: the three rows are the three `cbr128` known limitations. "To confirm" removed from evaluation.md, calibration.md and rules.md.
- `max_rate_pct`: at most 3 decimals, compared in integer thousandths of a percent (`100000 · k ≤ m · n`), bound written from `m`; new config error and scenarios in `calibration.feature`. Recorded in OQ-25 as proposed, for the owner to confirm at G1.
- Written down without behaviour change: a rate target with `n = 0` keeps passing (OQ-25); v1–v4 runs no longer reproduce the old target counts (evaluation.md section 6); an include mechanism for v5 rejected (OQ-29); T2's accepted `FAIL` is the three known rows only.
- `just check` passed.

**Pending**
- Gate G1 on PR #131: the owner delegated the decision to the reviewing agent, which accepted OQ-25's review addition and OQ-26 and found the spec ready. The merge itself stays with the owner.
- After G1: `/feature RP-11` (tests, then implementation; the protected tests listed in calibration.md OQ-31 (d) change in the test stage).
- Carried over: `docs/release-0.2.0-record` (commits after #119).

**Blockers / questions for the human**
- None.

**Next step:** owner merges PR #131, then `/feature RP-11`.

## 2026-11-14 - Local calibration path repair

**Done**
- Repaired the references affected by a renamed library folder in the local lists, labels, feature cache, reports and analysis artifacts (46 files). Originals and a SHA-256 manifest are in the local `_arreglo-rutas-2026-11-14/` backup directory; no corpus paths or names were added to the repository.
- Updated generated recipe ids with their source paths; preserved cached spectra, source size/mtime keys, labels and historical verdicts. Kept labels sorted by the specified columns.
- Added one owner-confirmed Bandcamp purchase (12 WAV tracks) to the local trusted list.
- Active trusted/confirmed references and current label paths resolve. Old reports also reference an album absent from the configured library; those historical references were retained rather than assigned to another recording.
- `just check` passed. No implementation/spec changes or commits were made for this maintenance.

**Pending**
- The next acceptance run will incorporate the new purchase into generated labels/features and recompute the split from the current paths. No new acceptance or calibration run was requested.
- RP-11 acceptance-target spec and the floor measurement remain pending as detailed in the 2026-11-02 block below; the four owner decisions still apply.
- Carried over: `docs/release-0.2.0-record` (commits after #119).

**Blockers / questions for the human**
- None for the active calibration inputs. The absent historical album has no verified replacement path.

**Next step:** `/new-spec RP-11` when the owner resumes the acceptance-target spec.

## 2026-11-02 - RP-11 acceptance targets: owner decisions (spec paused)

**Done**
- Diagnosed the `Real` transcodes behind the T2 and T3 failures with v8 (L5 run `20261102T170123Z`). Two cbr128 cases were re-encoded locally with the config's recipe.
- **T2 (5 `Real`: 4 cbr128, 1 cbr192):**
  - 2 are undetectable under the current definition: the source band ends near 11.85 kHz.
  - 1 is a quiet solo-piano track. Above 15 kHz the source is already at the digital floor (about −91 dB mean level in 15–16, 17–19 and 19.5–21.5 kHz), and the transcode is identical there. It still counts as detectable, because the band measures are relative to a quiet reference band.
  - 1 is a loud track whose cutoff is masked by 16-bit decode clipping. The float decode peaks at +1.8 dBFS and has nothing above 17 kHz; the s16 decode (no dither) fills 17–21.5 kHz at about −78 dB. In tracks.csv: `fs` 16050, `fe` 22050, `D` 5.49.
  - 1 has its step at 14450 Hz, 50 Hz below `lossy_min_cutoff_hz` (`D` 8.07 < `step_low_suspicious_db`). Lowering the threshold to 14400 would catch only this one and change no negative.
- **T3 (8 `Real`, cbr320):**
  - 3 are undetectable.
  - 5 are detectable, with `fs` 19350–20350 Hz, `D` 4.5–11.6 and `G` ≤ 2.3 (3 in tune, 2 in holdout). This is the same zone as the real purchased discs (rules.md, "Why v2 was replaced").
- The owner decided the scope; each answer was the recommended option. It is a calibration spec (RP-11), not detection v9:
  1. **Q9:** T2 and T3 exclude undetectable transcodes, like the rates do. "Undetectable" is widened to a source whose content above the setting's `lowpass_hz` is already at the digital floor, in absolute terms (the piano case). The criterion and its threshold are still to be defined and measured before G1.
  2. **Clipping:** the clipping-masked cbr128 transcode becomes a known limitation in rules.md section 6. A clipping-aware measure is a later detection spec.
  3. **14450 Hz step:** a known limitation. `lossy_min_cutoff_hz` stays 14500, because moving it for one track would be fitting the corpus.
  4. **T3:** it becomes a rate bound, a maximum `Real` rate over detectable cbr320 transcodes (example: 1 %), instead of a count of 0.

**Pending**
- The spec, `/new-spec RP-11`, paused by the owner until about 2026-11-09. Its new OQs follow OQ-24. It covers:
  - evaluation.md sections 2 and 6;
  - the `[[targets]]` rate format, and whether the default config changes in place or gets a new version;
  - rules.md Q9 and Q10 closed, with the two limitations added to section 6;
  - new `@RP-11` scenarios in `calibration.feature`.
- Before G1, measure the floor criterion on the corpus.
- Carried over: `docs/release-0.2.0-record` (it has commits after #119).

**Blockers / questions for the human**
- None.

**Next step:** `/new-spec RP-11` with the four decisions above.

## 2026-11-02 - RP-11 merged; checklist L5 passed

**Done**
- The owner merged #123 (`feb4348`). `just prune-branches` deleted the four merged RP-11 branches. It kept `docs/release-0.2.0-record`, which has commits after #119 was merged.
- L5 (calibration.md checklist) passed. Run `20261102T170123Z`: `just calibrate` with the defaults (grid v3, baseline v8), tool commit `feb4348`. Figures from `summary.md`:
  - Grid search:
    - The baseline has cost 154.100 and is feasible.
    - `- cost excludes the positives of the settings [v0, aac256]` is present.
    - `- must_catch Real at baseline: 1` (the `cbr128` transcode v8 misses).
    - All 27 points were evaluated, with 0 skipped. Result: "no change proposed", and no file was written.
  - Best-points table:
    - It starts with the `baseline` row. `must_catch lost` and `confirmed lost` are 0 on every row.
    - The 9 points that only move `full_band.*` cost exactly the baseline's 154.100. `verdict.gap_fake_db` 6.5 costs 154.500. So the `full_band` values make no difference on this corpus.
  - Confirmed fakes checked by the feasibility constraint: 15, none `Real`, 0 in holdout.
  - Primary figure with v8: 0/66 tracks (95 % CI 0.0–5.5 %), 0/5 albums. On holdout (headline): 0/23 tracks (95 % CI 0.0–14.3 %), 0/2 albums. Targets: T1 PASS (0), T2 FAIL (5), T3 FAIL (8).
  - `git status` was clean after the run.

**Pending**
- Owner's choice: `docs/release-0.2.0-record` (unmerged commits after #119).
- Still open from the RP-11 spec block: the `cbr128` transcode v8 misses (rules.md Q10 alternatives), rules.md Q9 (T2/T3 and undetectable transcodes), and no confirmed fakes in `holdout`.

**Blockers / questions for the human**
- None.

**Next step:** the owner picks the next work. Options: a detection spec for the `cbr128` miss and Q9 (T2/T3 fail), or one of the draft RNFs (RNF-02, RNF-03, RNF-10).

## 2026-11-02 - RP-11 implemented: calibration against the shipped detection v8

**Done**
- `feat/RP-11` (from main ef2999b) follows `/feature RP-11`:
  - `95d2f5c` test: 15 tests in `tools/fakeflac-calibrate/tests/shipped_baseline.rs` for the 11 new `@RP-11` scenarios. No existing test changed (OQ-21 (f)).
  - `bd876d5`: the RP-11 traceability row lists the new test file.
  - `d316917` feat: the defaults are params v8 and grid v3, read from the repository root. `baseline` is optional; new keys `cost_exclude` and `grid_version` 3. Feasibility is non-regression, plus the "confirmed lost" rule. The report has the new lines, the baseline row and the new columns. The calibrate skill names grid v3 (`just agents-sync` run).
- Review: no blocking findings. `just ci` is green and `just privacy-check --base origin/main` finds 0 matches.
- Known limitation, recorded in the PR: the tool cannot tell a default path from the same path typed by hand. So an explicit `--config` or `--grid` equal to the default relative path is read from the repository root. The protected tests fix the `PathBuf` fields of the argument structs.

**Pending**
- G2: the owner reviews and merges the `feat/RP-11` PR.
- L5 on the real corpus: `just calibrate` should show a feasible baseline and the `cost excludes` line.

**Blockers / questions for the human**
- None. Still open from the spec block below: the `cbr128` transcode v8 misses (rules.md Q10 alternatives), rules.md Q9, and no confirmed fakes in `holdout`.

**Next step:** after G2, `just prune-branches`, then run L5 (`just calibrate` on the local corpus).

## 2026-11-02 - RP-11 spec: calibrate against the shipped detection (G1 approved)

**Done**
- Spec PR #121 merged (G1, owner). The decisions are in `specs/calibration/calibration.md`, "Calibrating against the shipped detection", OQ-18 to OQ-24:
  - the default params are the app's (v8 today), checked by content;
  - the new default grid `calibrate-grid.v3.toml` has no `baseline` and searches from those params;
  - `must_catch` is non-regression against the baseline (this unblocks rules.md Q10);
  - confirmed fakes may not be `Real` and may not lose `Fake`;
  - grid v3 searches `full_band.*` and `gap_fake_db` 6.0–7.0 only;
  - the new grid key `cost_exclude` takes `v0` and `aac256` out of the cost.
- Finding: the grid v3 keys only move `Fake` ↔ `Suspicious` and never change a `Real` verdict (rules.md section 3).
- Measured on the owner's revised corpus with v8 (probe grid, aggregate figures): every probe point was infeasible under the old absolute rule, because of 1 `cbr128` transcode that v8 misses in `tune`. Primary figure: 0/66 tracks, 0/5 albums.

**Pending**
- `/feature RP-11` for this change. Test stage: the 11 new scenarios of `calibration.feature` (OQ-21 (d)); no existing test changes (OQ-21 (f)). Implementation:
  - defaults read from the repository root;
  - the default-params constant;
  - optional grid `baseline`, `cost_exclude` and `grid_version` 3;
  - non-regression feasibility for constraints 1 and 2;
  - the new report lines and columns;
  - `.claude/skills/calibrate/SKILL.md` naming grid v3, then `just agents-sync`.
- Then run L5 on the real corpus (`just calibrate`, baseline feasible, `cost excludes` line).

**Blockers / questions for the human**
- None. Left out of #121 for later specs: the `cbr128` transcode v8 misses (rules.md Q10 alternatives), rules.md Q9 (whether T2/T3 count undetectable transcodes), and the confirmed fakes all being in `tune` (no known-truth fakes in `holdout`).

**Next step:** `/feature RP-11` (calibration defaults and grid v3, calibration.md OQ-18 to OQ-24).

## 2026-11-02 - Release 0.2.0 published

**Done**
- The owner published v0.2.0 on 2026-11-02 (M7), after M3-M6; README status updated.
- The owner tagged v0.2.0; the Release workflow (run 41457533514) succeeded and left a draft with three assets.
- M2 (RNF-06) from the `package` log and the draft's assets: `FakeFlac_0.2.0_x64-setup.exe` 2,872,090 bytes (2.87 MB) and `FakeFlac_0.2.0_x64_en-US.msi` 3,981,312 bytes, both far below the 20,000,000-byte limit; `SHA256SUMS.txt` 190 bytes.
- M3-M6 (install, checksums and provenance, uninstall, MSI) were reported completely correct by the owner on Windows 11.

**Pending**
- None for the release.

**Blockers / questions for the human**
- None.

**Next step:** choose the next work (RNF-02, RNF-03 and RNF-10 are still `draft`; RF-12 is H3).

## 2026-11-02 - H2 complete; release 0.2.0 prepared

**Done**
- All H2 requirements are specified and built; RNF-07 (Windows 11 x64 matrix, #117) and RNF-08 (#116) are recorded as verified. Windows 10 stays a stated target, unverified by owner decision. RF-12 is H3; RNF-02, RNF-03 and RNF-10 remain `draft`.
- Branch `chore/release-0.2.0`: `just set-version 0.2.0` (package.json, package-lock.json, tauri.conf.json, Cargo.toml, Cargo.lock); `just ci` passed. Docs brought up to date in the same PR: README status, "implementation pending" wording after #115 in the resize specs/contracts and traceability.

**Pending**
- Human G2: merge the release PR. Then the human follows `specs/contracts/release.md` section 6 (release-check, tag on the merge commit, push the tag). CI leaves a draft release; check it (M2-M6) and publish it (M7). Agents never tag or publish.
- Record the two installer sizes from the `package` log (M2, RNF-06).

**Blockers / questions for the human**
- None.

**Next step:** merge the release PR, tag v0.2.0 and check the draft.

## 2026-11-02 - RNF-08 verdict label contrast correction implemented

**Done**
- Spec PR #111 is merged (G1 for RNF-07/RNF-08). Built the focused U3 correction on `feat/RNF-08`: protected test `7635fef` (`ResultsTable.contrast.test.ts` measures the rendered label colour on normal, hovered and selected rows in both languages; 14 cases failed before the fix), fix `285b7db`, margin/doc follow-up `235b07d`.
- Label text now uses new lighter tokens `--verdict-{real,suspicious,fake,error}-text` (same colour families); icons keep the saturated `--verdict-*` colours. Label contrast is 4.86-9.46 on all three row states (Fake selected 5.09). No spec, existing test, dependency, props or architecture changed.
- Independent review: no blocking findings; both improvements applied. `just check` green (83 files, 1,430 tests). `just ci` passed on `285b7db`; it was not rerun after `235b07d` (owner's instruction).

**Pending**
- Human G2 review/merge of the feature PR.
- RNF-08: the owner reported U3 (900 x 500, English and Spanish, Fake reads as red) and U4 (both languages) passed on 2026-11-02 with a desktop build of `235b07d` in a Windows 11 VM; recorded in `specs/ui/verdict-usability.md`.
- RNF-07: closed for the Windows 11 x64 matrix in merged spec PR #117.

**Blockers / questions for the human**
- None. The owner accepted the salmon Fake label (`#ffb0ab`) in the U3 check.

**Next step:** merge the feature PR at G2, then RNF-07 and RNF-08 are both closed.

## 2026-11-02 - RNF-07 matrix reduced to Windows 11 x64 (proposed)

**Done**
- Branch `spec/RNF-07-windows11-matrix`: the owner chose to drop Windows 10 from the closure matrix. The contract (P2, P-Q1 amendment), feature, RNF-07 text and traceability now name two rows (Windows 11 x64, NSIS and MSI); Windows 10 stays a stated target, unverified.
- Recorded the owner's report: Windows 11 26H2 build 26300.9711 x64 VM, both installers built by `npm run tauri build` from production source `235b07d`, install/launch/analysis/uninstall OK, with the synthetic fixture pack (valid, corrupt, Unicode name, empty folder); result reported as OK, not itemised per case.

**Pending**
- G1: owner approval/merge of this amendment.
- None for RNF-07 once this PR is merged: recorded as verified for the Windows 11 x64 matrix.

**Blockers / questions for the human**
- None. WebView2 Runtime 156.0.4314.58; the owner checked the installer hashes (values not recorded).

**Next step:** merge this spec PR, then both RNF-07 and RNF-08 are closed; Windows 10 remains unverified by choice.

## 2026-11-02 - RNF-07 and RNF-08 closure specifications approved at G1

**Done**
- Branch `spec/RNF-07-RNF-08`, PR [#111](https://github.com/salvador-arance/FakeFlac/pull/111): proposed platform contract, usability spec, scenarios and traceability; the owner approved P-Q1, U-Q1 and U-Q2 in chat; specs are now `approved` and the requirements `specified` (not verified). Independent final review: no blocking findings.
- Confirmed an existing RNF-08 contrast gap: verdict labels inherit colours that fail the already approved 4.5:1 text requirement, especially on selected rows. The proposal requires a focused correction after G1 and an owner walkthrough in both languages.
- `just check` (1,295 frontend tests), `just check-app`, `just build`, `just release-assets`, `just trace` and `git diff --check` passed. Local production source `216c945`, version 0.1.0: NSIS 2,877,203 bytes; MSI 4,661,248 bytes. Candidate hashes are in `specs/contracts/windows-platform.md`; ignored `target/rnf-07-test-pack.zip` contains installers, checksums and synthetic audio fixtures.

**Pending**
- Human merge of #111 (G1 decisions are recorded in the specs).
- The owner can test both Windows versions this session; all four installed-app matrix rows still need results. Historical Sandbox acceptance does not establish both OS builds.
- Feature PR #115 (resizable layout) is merged; this branch now includes `main`, and the only conflict was this file.
- After the merge: `/feature RNF-08` - independent tests and implementation for RNF-08 contrast, then visual/comprehension acceptance. No production code or executable tests changed in this proposal.

**Blockers / questions for the human**
- Neither requirement is verified yet. Platform/runtime observations are outstanding.

**Next step:** merge #111, then execute its closure checks and the focused RNF-08 correction.

## 2026-11-02 - Resizable Albums panel and result columns implemented

**Done**
- Built approved #113/#114 on `feat/RF-06-RF-10-resizing`, feature PR [#115](https://github.com/salvador-arance/FakeFlac/pull/115): test commit `dba2df1`, explicit test-stage corrections `5be122c` (approved searchbox role) and `990682c` (new Tab stops); production `8356e1d`, keyboard-help clipping fix `e700098`, representative story frames `d816d65`. No spec behavior, backend, dependency or architecture decision changed.
- Sidebar and all ten columns resize by pointer or keyboard, preserve independent widths and horizontal overflow, reset individually with double-click/Enter, and remember committed local layout preferences. Storage recovery does not affect analysis; language settings and the existing detail-height preference remain separate.
- Frontend: 1,405 passing tests, zero type errors/warnings; Svelte autofixer clean. `just check`, `just ci`, `just build`, `just release-assets` and Storybook build passed. Independent code review and story follow-up: no blocking findings. Required story mutation checks fail for raw zero-height/unbounded examples and pass with their frames.
- M1 on real Edge with the actual page and a synthetic API: 36 combinations of window size, language, row count and width mode pass pointer resizing, alignment, neighbor stability, persistence and long-table reveal. Native keys/double-click, rendered reload restoration, sorting, full keyboard help and modal interruption pass. Evidence in ignored `target/resizing-{qa-results,extras}.json`, screenshots and `target/story-{frames-geometry,frame-mutation}.json`; production candidate `e700098`.
- Local `target/resizable-layout-test-pack.zip` contains source `e700098` installers (version 0.1.0; distinct from the published release), SHA256SUMS.txt, instructions and synthetic audio. NSIS: 2,879,459 bytes; MSI: 4,661,248 bytes. No agent installation or publication was performed.

**Pending**
- Human G2 review/merge of the feature PR; GitHub's checks are separate from the green local `just ci` run.
- RNF-07/08 and PR #111 remain separate and pending, including the verdict-text contrast correction; this feature does not mark them verified.

**Blockers / questions for the human**
- No technical blocker; human G2 merge remains required.

**Next step:** review and merge the resizing feature after its checks pass.

## 2026-11-02 - Resizable layout approved and merged at G1

**Done**
- The owner approved R-Q1–R-Q3 in chat and merged spec PR #113 (`ad47005`). The sidebar/column resizing extension of RF-06/RF-10 is approved; its implementation remains pending.
- Status-only handoff on `docs/resizing-spec-handoff`: approved metadata and references now reflect G1 in the UI/local-preference specs, requirement links, catalogue supplement and traceability. No behavior, scenario, production code or executable test changed.

**Pending**
- `/feature RF-06/RF-10` for resizable widths: independent test stage, Svelte implementation, review, rendered M1 and `just ci` before G2.
- RNF-07/08 PR #111 and remaining acceptance are separate; header fix #112 is merged.

**Blockers / questions for the human**
- None for the resizing feature; its G1 is complete.

**Next step:** build the approved resizable layout on a feature branch.

## 2026-11-02 - Header fix merged; resizable layout proposed

**Done**
- The owner merged header-fix PR #112 into `main` (`8b24925`), confirmed via GitHub. The new spec branch includes that merge; its header-height correction must be preserved.
- Branch `spec/RF-06-RF-10-resizing`, PR [#113](https://github.com/salvador-arance/FakeFlac/pull/113): drafted the requested draggable Albums panel and all ten result-column widths as a proposed extension of RF-06/RF-10. Covers independent widths, bounds, mouse/keyboard operation, per-target reset, local layout persistence and invalid/unavailable storage recovery. The fixed-240px decision in RF-10 is explicitly proposed for amendment; no backend settings/command, production code, dependency or executable test changed. Independent review: no blocking findings; its displayed-width baseline clarification was incorporated.
- `just check` passed (1,295 frontend tests). Local Edge/Segoe UI measurements and width arithmetic informed the proposed limits and constrained-header policy; these are design measurements, not validation of implemented resizing. Future rendered acceptance remains explicit.

**Pending**
- Human G1 approval/merge of the resize proposal before the test/implementation/review cycle.
- RNF-07/08 spec PR #111 and their remaining acceptance, including verdict-text contrast, are still separate and pending. Header-fix G2 is complete.

**Blockers / questions for the human**
- G1 decisions: sidebar plus every column; remember/reset local widths; proposed bounds and independent-column/default-width rules.

**Next step:** approve and merge the resizing specification at G1, then implement its protected tests and UI changes.

## 2026-11-02 - Correct translated table header overlap; clarify empty-folder acceptance

**Done**
- The owner reported that a post-analysis empty-folder attempt kept previous rows and that Spanish table headers overlapped the first data row. Native UI inspection reproduced the overlap and confirmed that the localized no-audio status text was visible at the bottom left. Keeping previous rows is the approved RF-09 failed-start behavior (`ui/export-csv.md` section 3); the contradictory instruction in our RNF-07 proposal and local test pack was wrong.
- Corrected the proposed acceptance and observations separately in spec PR [#111](https://github.com/salvador-arance/FakeFlac/pull/111), commit `2f6f1b3`; the local test-pack README/ZIP now matches the approved failed-start behavior. G1 remains pending.
- Bugfix branch `fix/results-table-header`, PR [#112](https://github.com/salvador-arance/FakeFlac/pull/112), production commit `9eb67f1`: the header reserves natural translated-label height, and the header/body flex items do not shrink. Data rows remain 22px; no analysis/error logic, protected tests, approved specs, dependencies or architecture changed. Independent review: no blocking findings.
- Baseline `just check`, focused ResultsTable/ColumnHeader tests (110), `npm run check`, Svelte autofixer and post-change `just ci` passed. A local Edge/CDP harness renders the actual Svelte component: 12 combinations (English/Spanish, 900×500/1280×500, 0/3/1,000 rows) pass header-text containment, first-row separation, row-height and scrolling checks. All four long-table cases pass sticky-header and End/Home selection visibility. The original component from `main` fails Spanish label containment in the same harness. Evidence is ignored `target/header-{before,after}.{json,png}`; no browser dependency was added. jsdom cannot validate rendered geometry, so no source-mirroring CSS tests were introduced.

- `just build` and `just release-assets` passed for production source `9eb67f1`. Local `target/rnf-07-header-fix.zip` contains rebuilt NSIS/MSI, SHA256SUMS.txt, synthetic fixtures and corrected instructions; the earlier test pack remains available separately. This candidate includes the header fix and does not include the pending verdict-contrast change.

**Pending**
- Human G2 review/merge of the header correction and installed-app confirmation of the rebuilt candidate.
- G1 for RNF-07/08 in #111; RNF-08 verdict-text contrast correction and bilingual comprehension remain pending. The owner's general report of completed checks does not identify each OS build/installer/candidate hash, so the platform matrix is not recorded as fully verified.

**Blockers / questions for the human**
- No blocker for the header fix. Neither RNF-07 nor RNF-08 is marked verified.

**Next step:** review and merge the header-fix PR at G2, then resume #111 and its remaining acceptance.

## 2026-11-02 - RF-13 merged; status wording updated

**Done**
- The owner merged the RF-13 feature PR #109 into `main` (`52ef742`) after M1–M2 passed.
- Status-only wording, with no behaviour, scenario or test change: "not implemented yet" for RF-13 / contract version 8 replaced with "implemented in #109" in `specs/requirements/RF-13.md` (status stays `specified`, like every built requirement), `specs/contracts/{settings,commands}.md`, `specs/traceability.md`, ADR-0049, `docs/architecture/{overview,data-model}.md`; README status lists RF-13 as merged for the next release. Branch `docs/rf-13-implemented`.

**Pending**
- Next requirement: none chosen; remaining H2 requirements and draft RNF/RP are in `specs/requirements/`.

**Blockers / questions for the human**
- None.

**Next step:** review and merge this docs PR; then choose the next requirement (`/new-spec`).

## 2026-11-02 - RF-13 feature: Spanish/English and persistent language

**Done**
- `/feature RF-13` on branch `feat/RF-13`: `3306c82` test (57 Rust + frontend tests across api, i18n, components, pages), `bf80463` ADR-0049 amendment, `88f8d4d` Rust (`src-tauri/src/settings.rs`, DTOs, `get_settings`/`set_settings`, bindings), `6aec431` frontend (catalogue, reactive locale, `LanguageSelect`, `SettingsPanel`, Toolbar gear, inert background, startup gating, translated components).
- Owner approved in chat on 2026-11-02 the `windows` crate (`cfg(windows)`, 0.62 already in `Cargo.lock` through tauri, features `System_UserProfile` + `Foundation_Collections`) for `GlobalizationPreferences::Languages()`; recorded as an amendment in ADR-0049 and its index row. No new crate; `unsafe_code` stays forbidden.
- Review: no blocking findings. `just ci` green (1295 frontend tests, `check-app` included), `just privacy-check --base main` clean. RF-13 traceability row updated.
- Readings where the spec left room are listed in the test file headers and the PR description (for example: the settings warning survives a run start and is replaced by export/cache notices; title bar stays usable while the modal is open; corrupt BOM/duplicate-field files are invalid).

**Pending**
- G2 review and merge of the RF-13 PR (#109). Manual checks M1–M2 (relaunch persistence, 900×500 in both languages, keyboard-only modal) passed: the owner confirmed in chat on 2026-11-02.
- After merge: status-only wording in `specs/` still says RF-13 is "not implemented" (`requirements/RF-13.md` `status: specified`, `contracts/commands.md`, `contracts/settings.md`); update in a docs PR.

**Blockers / questions for the human**
- None.

**Next step:** review and merge the RF-13 PR #109 (G2); then the docs PR for the RF-13 status wording.

## 2026-11-01 - Documentation audit after the RF-13 spec merge

**Done**
- PR #106 (Codex harness) and PR #107 (RF-13 spec) are merged into `main` (`3626b1f`); the earlier "pending G2/G1" notes for them are historical.
- ADR-0049 set to `accepted` in the ADR and in `docs/adr/README.md`. `overview.md`, `data-model.md`, `frontend.md` and the RF-13 traceability row no longer say "pending G1"; they say approved, not implemented.
- README status rewritten: v0.1.0 pre-release published on 2026-10-31; Opus (detection v8), sort/filter, CSV export, album summaries, the cache and the perf harness listed as merged; RF-13 named as next.
- At the owner's request, split this log: the 48 blocks before the v0.1.0 release moved unchanged to `docs/agents/archive/progress-v0.1.md`; this file keeps the active cycle (v0.1.0 release, Opus RF-14, harness RP-10, RF-13) and links the archive in a footer. ADR-0041's reference to the 2026-10-18 block points to the archive.
- At the owner's request ("si en algún sitio pone que algo está pendiente pero no lo está, actualízalo"), status-only wording in `specs/` was updated in the same PR, with no behaviour, scenario or test change: `RF-13.md` `status: specified`; the 26 merged contract/UI/detection/calibration specs `status: proposed` → `approved`; RF-13 "proposal"/"G1 pending" wording in `commands.md`, `settings.md`, `translations.md`, `language-settings.md`; `ui/album-summary.md` no longer says RF-08 is not built (#76); `lossy-source.md` Q9 ("Opus is pending") marked superseded by #103.
- ADR-0030, ADR-0033 and ADR-0040 now say "amended by ADR-0047" (file and index); ADR-0040 point 4 and ADR-0047's first consequence note that Opus landed (#103, #105).
- Checked: relative Markdown links (none broken in tracked files), `just trace`, `just agents-check` (no drift), `backend.md` (current). Still genuinely pending and left as is: `just fixtures`, the `just bench-windows` window-selection recipe, the future `fakeflac-cli`, RF-12 and the draft RNF/RP requirements. Branch `docs/rf-13-spec-handoff`, PR #108.

**Pending**
- `/feature RF-13` with an independent test stage. Fresh-session harness lifecycle acceptance from #106 remains a manual follow-up.

**Blockers / questions for the human**
- None.

**Next step:** review and commit this docs branch; then `/feature RF-13`.

## 2026-11-01 - RF-13 Q1-Q4 approved in chat

**Done**
- At the owner's request, a read-only subagent evaluated Q1-Q4. The owner approved its four recommendations ("de acuerdo").
- Updated PR #107's specification: valid saved language first; missing file uses the first supported Windows user language preference, with English fallback. Regional tags and unsupported/empty/unavailable lists are defined and covered by deterministic scenario examples. Recovery from invalid, unavailable or newer settings still uses English with a warning.
- Recorded approval of Save/Cancel, JSON persistence and recovery, complete translations and stable technical formats. Clarified that the newer-version check protects the observed file without synchronization between independent processes; ADR-0049 matches this limitation.
- The native Windows language-list source is a separate safe backend adapter; any required dependency needs explicit owner approval and an ADR amendment before implementation. Q1 approval does not implicitly approve a library or runtime helper.
- Independent review found no blocking behavior/scenario issues; its index, traceability and native-adapter clarifications were applied. `just trace`, `git diff --check` and a fresh `just check` passed (757 frontend tests). Commit `282c14c` and PR #107's description record the approved decisions.

**Pending**
- Human review and merge of [PR #107](https://github.com/salvador-arance/FakeFlac/pull/107). ADR-0049 remains proposed until merge. No production code or executable tests changed.
- After merge: handoff/ADR status reconciliation, then `/feature RF-13` with an independent test stage.

**Blockers / questions for the human**
- Q1-Q4 are answered and approved. Human merge remains outstanding.

**Next step:** review and merge PR #107.

## 2026-11-01 - RF-13 bilingual interface and persistent preference specification

**Done**
- Started `/new-spec RF-13` on `spec/RF-13` from `fcee584` (merged #106). The owner selected Spanish/English with a persistent preference; advanced analysis settings are deferred.
- A spec-writer prepared `contracts/settings.md`, `ui/language-settings.md`, `ui/translations.md` and 22 Gherkin scenarios in `features/language-settings.feature`; RF-13, proposed command contract version 8 and traceability are linked.
- Added proposed ADR-0049: language-only JSON in the app configuration directory using existing dependencies; version-aware recovery, explicit saves and preserved previous settings on failure. Architecture links distinguish this proposal from implemented behavior.
- `just check` passed (all Rust suites and 757 frontend tests); `just trace` passed (38 requirements, 28 tagged). All 73 current English catalogue keys are covered and placeholder sets match. The English/Spanish name-order example was verified with the local `Intl.Collator`.
- Independent read-only review identified and prompted corrections to the new AppError union and focus handling during saves/read-only failures. No production code or executable tests were changed.
- Final independent review found no blocking findings. Spec commit `d5e452b`; PR [#107](https://github.com/salvador-arance/FakeFlac/pull/107) opened for G1. Human merge only.

**Pending**
- G1 review of Q1-Q4 in `specs/contracts/settings.md`: initial English default, Save/Cancel interaction, persistence/recovery and translation/numeric conventions. ADR-0049 remains proposed.
- After human approval and merge: reconcile accepted ADR status and handoff, then `/feature RF-13` with an independent test stage. Existing fallback assertions must be updated in that stage per the UI spec.
- Fresh-session harness lifecycle acceptance from #106 remains a separate manual follow-up.

**Blockers / questions for the human**
- No implementation is authorized until G1 approves the proposal. Human merge only.

**Next step:** review the RF-13 specification PR at G1.

## 2026-11-01 - Claude canonical harness; generated Codex compatibility

**Done**
- Reverted the earlier optimization edits at the owner's request; Claude model defaults and workflow remain unchanged.
- The owner clarified the sole objective: maintain Claude configuration once and make Codex follow the existing workflow.
- The owner approved the contract, scenarios and ADR-0048 at G1 in chat ("apruebo todo"). ADR-0048 is accepted; traceability and architecture are linked.
- Implemented `scripts/sync-agents.mjs` and `just agents-sync` / `just agents-check`. It owns 25 derived files: six scoped instruction files, five skills, five roles, eight thin hook entry points and the hook registrations. `check-fast` and therefore existing CI reject drift. Personal settings and unowned files are preserved; no dependency was added.
- The owner approved explicit Codex models and requested a PR. `.claude/codex-models.json` is the sole maintained assignment source: Astra/high for specs and review, Sol/high for tests and Rust, Sol/medium for Svelte. The generator emits supported TOML fields, validates full role coverage/schema and fails before writes on invalid configuration. The host refreshed all five role/model bindings; the final model review ran as Astra/high.
- `.codex/hooks/dispatch.mjs` translates native shell/patch payloads to canonical hooks, checks every patch source/move destination and formats surviving paths. Claude hooks, skills, role instructions and model defaults are unchanged. Copied `.Codex/stage` references disappear because generated skills now preserve the canonical `.claude/stage` path.
- Two fresh test-author iterations produced 34 passing generator/adapter tests. Independent review found and then cleared reserved-adapter and unowned-output collisions. Reverting each fix in isolated `target/` copies makes its covering regression fail. Five generated agent files parse with the existing Rust TOML parser.
- Generated hook commands were exercised from the root and `src/` in Git Bash and PowerShell (the PowerShell runner explicitly forwards `$LASTEXITCODE`); allowed docs and blocked protected edits agree. This is a shell smoke test, not evidence of a live host's hook runner.
- Final `just check` and `just ci` passed, including 757 frontend tests, all 71 focused harness/model tests, native Tauri checks, generated bindings, traceability and protected-test checks. Spec/ADR commit: `e806520`; independent tests: `12dc2a9`; implementation: `7c0e244`. `agents-check` reports no drift. The existing untracked `pnpm-lock.yaml` was untouched.
- Pushed `salvador-arance/chore-optimize-agent-usage` and opened PR #106: https://github.com/salvador-arance/FakeFlac/pull/106. The owner explicitly requested publication. The description records the model table, G1 approvals, independent review and remaining manual host checks. Human merge only.

**Pending**
- Hosted CI and human G2 review of PR #106.
- Fresh-session lifecycle hook enforcement remains unverified. Claude tool metadata, permission settings and native Codex transcript enforcement are explicitly not claimed to be portable runtime controls. Role/model discovery has been observed in the host.
- Opus PR #105 is already merged in the branch's baseline `d4998f3`; its earlier pending G2 note is historical.

**Blockers / questions for the human**
- No remaining automated-review blocker. Full live-host acceptance requires fresh local sessions; no host permissions or personal configuration were changed.

**Next step:** inspect hosted CI and review PR #106 at G2; verify fresh-session lifecycle hooks before claiming complete runtime parity.

## 2026-11-01 - Opus corpus acceptance completed; O1-O5 pass

**Done**
- The owner asked the agent to execute the acceptance steps using the existing configured roots, authorizing the calibration outputs outside the repository while keeping corpus inputs read-only.
- Reproduced the startup issue in Windows PowerShell 5.1: default ANSI JSON reading corrupts the corpus path; explicit UTF-8 resolves it. Documented the command in the workflow.
- Ran v4 acceptance with v7 baseline and v8 params: run `20261101T143541Z`, exit 0, 18,360 new transcodes, 57,325 cache hits, zero generation failures. O1-O5 pass; O6 n/a (no actual Opus corpus files). Verified equal verdicts/reasons/cutoffs/estimates for non-Opus rows locally.
- O3 maximum Fake rate 0.7% (gate <=2%); O4 MP3/AAC cascades caught 92.2%/99.2%, Fake 90.9%/95.0%. O5 counts are recorded in `docs/agents/opus-acceptance-2026-11-01.md`, using only aggregate summary data.
- O8 report on the prepared synthetic 360 s stereo Opus: median/max 630/630 ms per track, zero errors. O7 and hosted CI at `6510921` are green, including supply-chain and Windows build.
- The generic T2/T3 targets retain their reference failures (5/8) identically in v7 and v8; they are not resolved by this feature. No detection parameter was changed.
- `just privacy-check --base main` passes: 54 changed files checked, zero matches.

**Pending**
- Owner G2 review of draft PR #105 and its acceptance results; human merge only.
- Real-world Opus material remains unmeasured; generated encodes and synthetic timing are explicitly identified as such.
- Earlier follow-ups, including the unchanged generic T2/T3 acceptance targets, remain separate.

**Blockers / questions for the human**
- No new Opus acceptance blocker. No input lists or detection parameters were edited.

**Next step:** review #105 at G2 with the completed acceptance evidence.

## 2026-11-01 - Opus draft PR #105 opened

**Done**
- The owner authorized publication. Pushed `codex/feat-RF-14-opus` and opened draft PR #105: https://github.com/salvador-arance/FakeFlac/pull/105.
- Its description includes the approved scope, local green `just check` / `just ci`, independent review, installer sizes and the full quiet performance report. Manual corpus acceptance is explicitly pending; no merge was performed.

**Pending**
- Hosted CI, including the supply-chain job unavailable locally.
- Manual corpus acceptance O1-O6 and the owner's O8 folder, as detailed below. G2 remains with the human.

**Blockers / questions for the human**
- No remaining code/test blocker. Running acceptance through the agent still needs an explicit exception for writes to external `FAKEFLAC_CALIBRATION`.

**Next step:** inspect the hosted CI results of #105 before proceeding with the manual acceptance gates.

## 2026-11-01 - Opus implemented; local CI, review and installers verified

**Done**
- The owner approved the fixture correction and return to tests in chat. `aaca164` applies the G1 recipe: 255-byte split pages guarantee continued packets.
- Fresh test iterations `9e84ae7`, `7a925c7` and `b7b9ae9` fix concurrent fixture publication and stale calibration/native assertions, and add C14/C15/C17 regressions. The native VBR check uses the independent packet-byte/duration formula, the approved ±1 kbps tolerance and real estimate = actual declared bitrate.
- `d159d7f` and `07f7a1b` complete the Opus checkpoint `83bb40b`: calibration folders include Opus, params without Opus rows exclude even failed Opus extraction, and configuration v4 loads. Production never changed protected tests or specs.
- `just check` and final `just ci` are green, including 18 audio Opus tests, 757 frontend tests, all native Tauri tests, Clippy and generated bindings. `just build` succeeds: NSIS 2,854,359 bytes, MSI 4,628,480 bytes, both under 20 MB.
- Independent full review and the final native-test review found no blocking findings. The quiet `just perf` report passes: T1 off/cold 172/197 ms, T2 off/cold 557/606 ms (all less than 8 % above baseline). This resolves the conditional performance note from the earlier contended run.
- Supplementary release timing: a generated 360 s stereo libopus file (3.9 MB), median/max 612/612 ms per track, zero errors. It does not replace the owner's O8 folder.
- README, architecture, ADR-0047 implementation notes, traceability and lessons updated. Reviewable PR body with the complete quiet timing report: ignored local `target/opus-pr.md`.

**Pending**
- Owner authorization to open the prepared draft PR from `codex/feat-RF-14-opus`; no branch push or PR has been made yet. G2 stays with the human.
- Manual corpus acceptance O1-O6 and the owner's O8 folder. Do not claim those gates passed: the agent's repository-only write rule prevents running `just accept` against external `FAKEFLAC_CALIBRATION` without an explicit exception.
- Cargo deny/audit are not installed locally; the hosted supply-chain job must pass after publication.
- Earlier optional/manual follow-ups remain unchanged.

**Blockers / questions for the human**
- No remaining code/test blocker. Approve opening the draft PR; corpus acceptance remains explicitly pending.

**Next step:** after authorization, push `codex/feat-RF-14-opus` and open the draft PR using `target/opus-pr.md`.

## 2026-11-01 - Opus implementation checkpoint; G1 needed for fixture corrections

**Done**
- #102, #103 and #104 are merged; `just prune-branches` removed the merged Codex setup branch.
- Started `/feature RF-14` / RF-15 on `codex/feat-RF-14-opus`. Baseline `just check` passed (752 frontend tests).
- Approved pinned dependency: `4bfe545`; the same package is available to Tauri/calibration fixture tests through dev-dependencies (`d09884d`), with no additional package.
- Test stage `54b5558`: independent Opus writer, fixture preconditions, scalar/deterministic subprocesses, per-channel decoder seam, rules, scan, native events, UI and calibration tests. Exact protected files are listed in the commit. Independent fixture preconditions and exhaustive non-Opus v7/v8 regression pass.
- Implementation checkpoint `83bb40b` (incomplete): streaming Ogg validation/packet indexing, 48 kHz decode shared by analysis and spectrogram, pre-skip/end trim/gain, v8 defaults, Opus estimates and calibration support. Focused core suites pass; audio Opus suite: 16 pass, 2 fixture failures. Compilation and test-target Clippy passed.

**Pending**
- G1: correct the split-fixture recipe in `lossy-fixtures.md` 1.4/2.2 and `opus.feature`: the approved 1000-byte cap can fit three whole 320-byte packets, producing no continued packet. Proposed cap: 255 bytes; packets longer than 255 bytes necessarily span pages. Reviewable proposed spec diff: local ignored `target/opus-g1-proposal.patch` (not applied).
- After G1, return to the test stage with a fresh test-author: update the split writer/continuation preconditions, synchronise same-process fixture creation (the current PID-based temporary name races), and align `lossy_rows.rs` with approved C14 (v5-v7 leave Opus unevaluated; the old test still expects `unsupportedFormat`). Never edit protected tests in the implement stage.
- Resume implementation with a fresh implementer: finish `tools/fakeflac-calibrate/src/lists.rs` `Formats::Lossy` filtering and its folder-empty message, then fix any remaining production failures.
- Full `just check` exits 1 at `rf14_broken_opus_streams` (17 audio Opus tests pass, 1 fails at the missing continuation page). The temporary-file race was reproduced in the earlier focused run. `guard-tests` is green; no spec or protected test changed after `54b5558`. `just check-app`, `just ci`, fresh `just perf`, installer size check, independent review and PR are pending. No PR opened; do not treat this checkpoint as completed.
- Corpus acceptance O1-O6 and timing O8 are pending. `just accept` writes to external `FAKEFLAC_CALIBRATION`; reconcile that with the repository-only write rule before running it. O8 requires the owner's Opus folder.
- Earlier optional/manual follow-ups remain unchanged.

**Blockers / questions for the human**
- Approve the 255-byte split-fixture recipe at G1 and the return to test stage for the two fixture defects and the C14 test correction. Production work stopped; no protected test or spec was changed during implementation.

**Next step:** G1 on the split-fixture correction, then a fresh test-author iteration.

## 2026-11-01 - Portable Codex hook commands (#104)

**Done**
- Replaced the seven machine-specific paths in `.codex/hooks.json` with quoted paths resolved by `git rev-parse --show-toplevel`, following the official hooks guidance: https://learn.chatgpt.com/docs/hooks.
- Verified every configured command in PowerShell from the root and a subdirectory, and in Git Bash from a subdirectory; verified that guard-bash receives stdin and preserves Node's blocking exit code.
- Documented path resolution in the agent workflow. `just check` is green (including 60 frontend test files, 752 tests).

**Pending**
- G2 for #104. The Opus work and other pending items in the previous blocks remain unchanged.

**Blockers / questions for the human**
- None.

**Next step:** review and merge #104 at G2.

## 2026-11-01 - Opus decoder spike; ADR-0047 accepted at G1 (#102)

**Done**
- #101 merged. No branch was left to prune.
- The owner picked Opus. The ADR-0030 spike ran in a throwaway crate under `target/opus-spike/` (not committed). It compared five decoders against libopus's float decode on 10 synthetic Ogg Opus files, plus 72 020 corrupt packets per decoder and the binary size of each. Results are in ADR-0047.
  - `rusty-opus` 1.0.2, `opus-rs` 0.1.40 and `opus-pure` 0.2.2 match libopus (about 126 dB SNR) and never panicked.
  - `opus-decoder` was rejected: 16-bit output and 8.6 s per 30 s.
  - libopus in C needs cmake.
- The owner chose **`rusty-opus` `=1.0.2`**. #102 (`spec/RF-14-opus`) holds:
  - ADR-0047 (proposed): decoder; the Ogg layer read by the adapter; Opus fixtures encoded in test support with the same crate; family 0 only; `EXTRACTION_VERSION` stays 2;
  - the ADR index;
  - the `overview.md` and `backend.md` rows.

  `just check` is green.
- G1: the owner accepted every default of #102 (questions 1–3). ADR-0047 is set to `accepted` on the branch, in the ADR and in the index, with a "G1 decisions" section.

**Pending**
- The owner merges #102.
- After G1, the Opus spec PR (`/new-spec`, RF-14/RF-15). It:
  - moves `specs/pending/opus.feature` to `specs/features/`;
  - adds the Opus `[lossy.cutoffs]` rows, which means a new params version, calibrated with the calibration tool's libopus cascades;
  - fills in the `<to be specified>` values;
  - specifies the Ogg reader's `corruptStream` cases and the fixture writer.
- `target/opus-spike/` can be deleted after G1.
- Carried over: the same list as the 2026-10-31 block.

**Blockers / questions for the human**
- None.

**Next step:** after #102 is merged, `just prune-branches`, then `/new-spec RF-14` for Opus.

## 2026-10-31 - First release v0.1.0 published; RNF-11 M1–M7 all passed

**Done**
- #99 merged; its branch was already pruned.
- #100 `chore(release): 0.1.0`: `just set-version 0.1.0` and `just ci` green. No new rehearsal: M1 had passed on 2026-10-30, and #94, #95 and #99 do not touch the release wiring.
- The owner tagged `v0.1.0`. Release run 41196296529 went green and left a draft. The owner then ran M2–M7 and published it. It is marked as a GitHub **pre-release** on purpose (the owner's choice for 0.x), so it is not "Latest".
  - M2: NSIS 2 738 475 bytes, MSI 3 821 568 bytes.
  - M3–M6 passed in Windows Sandbox. The NSIS installer needed no administrator rights; the MSI asked for them.
  - M7: the fixed text comes before the PR list, and there are exactly three assets.
- The owner's manual checks for RNF-01 (M1: cold album 381 ms; M2: 210 real tracks, every median under 300 ms per track) and for RF-08 (M2 Ctrl+F, M3 look) passed. RF-08 P1, P2 and M1 were not done; they need the 10 000-fixture folder.
- On `docs/release-0.1.0-results`: the RNF-11 row of `traceability.md` records M1–M7. The RNF-01 row now points to #99 instead of the branch. The results are also posted as a comment on #100.

**Pending**
- G2 for the docs PR.
- RF-08 P1, P2 and M1 (optional; they need the 10 000 synthetic fixtures).
- Carried over:
  - the spectrogram of very long files (3.2 s against RF-07's 2 s);
  - cancel latency on long files;
  - RF-13 and the draft RNFs;
  - the RF-16 K5 wording;
  - the 48 kHz MP3 check;
  - Dependabot alert #2;
  - `target/perf-harness/`, which the owner may delete.

**Blockers / questions for the human**
- None.

**Next step:** after G2, `just prune-branches`; then the owner picks the next requirement (Opus after the ADR-0030 spike, RF-13 i18n, or another draft RNF).

---

Blocks before v0.1.0: see [`archive/progress-v0.1.md`](archive/progress-v0.1.md).

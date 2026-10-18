# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

## 2026-10-18 - RP-11 real-corpus acceptance run and follow-up plan

**Done**
- First real-corpus acceptance run executed by owner on 2026-10-18 (checklist L1–L7, `specs/calibration/calibration.md` §7):
  - Spectra cache warm (~3 s per accept/calibrate run; cold run 82 min for 1425 corpus tracks + 12825 transcodes).
  - Owner provided `confirmed.txt` (15 confirmed fakes, 4 confirmed false positives W1–W4) and updated `trusted.txt` (`revisado` for known-truth lossless tracks).
- Checklist L1–L7 results:

| Check | Result |
|---|---|
| L1 `just accept` | ⚠ Report OK, 9 settings × 1425 rows, 0 generation failures. `encoder_version` is garbage (Task 3). |
| L2 second run | ✅ 14250 hits, 0 decoded, 0 transcoded, `labels.csv` byte-identical |
| L3 known-truth album | ✅ 15/15 confirmed fakes are Fake (detection rate `confirmed_fake` 100 %); 9/9 `revisado` tracks are Real |
| L4 watch list | ✅ W1–W4 listed one by one, all Fake under v2 |
| L5 `just calibrate` | ⚠ Repo stays clean, but no proposal: no grid point is feasible (Task 2) |
| L6 `privacy-check` | ❌ `just privacy-check --base c6f039e` gives 1 match: `docs/agents/progress.md:104` holds the calibration folder's absolute path (Task 1). |
| L7 corpus untouched | ✅ All 6613 files have the same size and mtime, and no new file appeared |

- Detection v2 aggregate metrics (split `all`):
  - Primary (purchased, 67 tracks / 5 albums): 4 Fake (6.0 %, 95 % CI 2.3–14.4 %), one in each of 4 albums (exactly W1–W4).
  - Clean: 0/546 Fake, 2.2 % Suspicious. Reference: 3/812 Fake.
  - Detection rate per setting: cbr128 99.5 %, cbr192 99.9 %, cbr256 98.0 %, cbr320 91.6 %, v0 18.2 %, v2 99.6 %, aac128 99.0 %, aac256 0.6 %, opus128 96.8 % (FN rate 19.3 %, driven by v0 and aac256).
- Watch list (confirmed false positives W1–W4 under v2):

| id | G (dB) | D (dB) | fs (Hz) | fe (Hz) | Rule that fires |
|---|---|---|---|---|---|
| W1 | 18.39 | 8.93 | 20450 | 20900 | V3 (16 kHz gap) |
| W2 | 13.18 | 14.79 | 21450 | 21400 | V3 |
| W3 | 10.26 | 14.70 | 21250 | 21100 | V3 |
| W4 | -0.25 | 14.22 | 19450 | 22050 | V4 (cutoff step) |

  Empty reasons for Fake is correct per `specs/detection/rules.md` (V3 and V4 carry no reasons).
- Detection v3 findings recorded (for later spec):
  - W1–W3 are Fake by rule V3 (16 kHz gap G 10–18 dB, content up to ~21 kHz). The v3 candidate rule in `evaluation.md` §7 ("16 kHz gap with full band → Suspicious, not Fake") would clear them.
  - W4 is Fake by rule V4 (D 14.2 dB, fs 19450 Hz) although it has content up to 22050 Hz. It requires an analogous full-band guard for V4.
  - Blind spots: LAME V0 (18.2 % detected) and AAC 256 (0.6 %). No v2 threshold reaches them; they need new measurements or rules, not tuning.

**Pending**
- Task 1: docs PR `docs/rp11-acceptance-results` (scrub absolute path from `progress.md:104`, record L1–L7 run).
- Task 2: spec PR `spec/RP-11-undetectable-transcodes` (undetectable transcodes from band-limited sources, spec alignment for kept audio, `--no-generate`, and `privacy-check`), followed by implementation and recalibration.
- Task 3: fix PR `fix/RP-11-lame-tag` (parse first MPEG audio frame Xing/Info header for LAME tag, fallback to spec-conformant string).

**Blockers / questions for the human**
- None.

**Next step:** Merge `docs/rp11-acceptance-results` (Task 1); implement `fix/RP-11-lame-tag` (Task 3) and `spec/RP-11-undetectable-transcodes` (Task 2).

## 2026-10-18 - RP-11 calibration tool implemented (feat/RP-11)

**Done**
- Spec fixes at G1 found during the cycle: #29 (split rule for holdout_fraction 0.0/1.0, stability wording), #30 (mixed-class albums take the strongest class; hard constraint (2) covers holdout confirmed fakes).
- `tools/fakeflac-calibrate` (ADR-0036): env/containment checks, trusted/confirmed lists, stratified split, ffmpeg transcoder, JSON Lines features cache, evaluation and reports, grid search with feasibility and proposal file, privacy check. `fakeflac-audio::EXTRACTION_VERSION = 2`. No new crate in `Cargo.lock`.
- `just accept`, `just calibrate`, `just privacy-check` (outside `check`/`ci`) and the `/calibrate` skill.
- Tests: 8 test files under `tools/fakeflac-calibrate/tests/`, all @RP-11 scenarios. Owner-approved fix of a time-dependent protected test (target filter matched the run id).
- Three review rounds; all blocking findings fixed. `just check` and `just ci` green.

**Pending**
- Gate G2 on the `feat/RP-11` PR.
- Local checklist L1–L7 on the real corpus with real ffmpeg (owner, manual).
- Next spec PR should record three behaviours the code has but the spec does not state: the kept-audio sidecar `generated/<id>.json` (generation.md §3); `--no-generate` leaves `labels.csv` untouched (calibration.md §2); `privacy-check` exits 1 when a file could not be checked (calibration.md §6).
- Known limits: no progress output during a long run; on ffmpeg 8.1 most MP3/Opus settings fall back to "version not readable" for `encoder_version`.
- ADRs 0033–0037 still say `proposed` although merged at G1.

**Blockers / questions for the human**
- None.

**Next step:** Owner reviews and merges the `feat/RP-11` PR (G2), then runs `just accept` on the real corpus (checklist L1–L7).

## 2026-10-18 - Calibration tool spec (RP-11) updated from G1 review answers

**Done**
- Incorporated the owner's answers to the 17 open questions for Gate G1:
  - OQ-1/OQ-2: v1 is cited from ADR-0035; tool evaluates v2 family only; reproduces v2 on known-truth album.
  - OQ-3: `EXTRACTION_VERSION` in `fakeflac-audio` must also be bumped when `symphonia` version is updated.
  - OQ-4: CI uses synthetic fixture transcoder; no ffmpeg in CI.
  - OQ-5: `confirmed.txt` for confirmed fakes and false positives (`W1`...); `labels.csv` is 100% derived. Documented known-truth tracks to be added by owner once spec is approved.
  - OQ-6: Multi-disc folders (`CD1`, `Disc 2`) merged into parent album.
  - OQ-7: Stratified split per class (stable hash order within class, bottom 30% to holdout with guaranteed minimum of 2 albums per class when $N \ge 2$).
  - OQ-8: Opus FLAC transcodes stay at 48 kHz.
  - OQ-9: `keep_audio = false` by default; added `keep_misclassified = false` option to retain generated FLACs for misclassified tracks for spectrogram inspection.
  - OQ-10: Configurable `fragment_s` with **full track as default** (`null` / omitted) to avoid optimistic detection bias on skits/intros.
  - OQ-11: Approved weighted cost (`fp: 10`, `fn: 1`, `suspicious_neg: 0.5`, `suspicious_pos: 0.1`). Hard constraint in feasibility: no confirmed fake may be `Real`. Rule families evaluation support. Documented candidate rule for detection v3 ("16 kHz step with full band -> Suspicious, not Fake").
  - OQ-12/OQ-13: Acceptance targets report-only (exit 0); primary false positive metric counts `Fake` only.
  - OQ-14: Dev-only tools under `tools/`, checked by `scripts/lint-arch.mjs` in this PR (tools depend only on `core`, `audio`, `fs`; product crates never depend on tools).
  - OQ-15: Binary strictly refuses to start if `FAKEFLAC_CALIBRATION` is inside corpus or repo (or vice versa); opens `FAKEFLAC_CORPUS` strictly read-only.
  - OQ-16: `just privacy-check` supports unversioned `privacy-exceptions.txt` for common words.
  - OQ-17: Best-effort encoder version parsing with fallbacks.
- Updated `specs/calibration/calibration.md`, `inputs.md`, `generation.md`, `evaluation.md`, `calibration.v1.toml`, `calibrate-grid.v1.toml`, `specs/features/calibration.feature`, `crates/fakeflac-audio/CLAUDE.md`, and `scripts/lint-arch.mjs`.
- All checks pass (`just check-fast`, `just trace`).

**Pending**
- Gate G1 approval on spec PR.
- After G1: implementation of RP-11 (`tools/fakeflac-calibrate`, `just accept`, `just calibrate`, `just privacy-check`, `.claude/skills/calibrate`).

**Blockers / questions for the human**
- None.

**Next step:** Open PR for `spec/calibration-g1-answers` for G1 sign-off.

## 2026-10-18 - Main-screen UI implemented (RF-01, RF-05, RF-06)

**Done**
- Branch `feat/RF-01-05-06-main-screen`:
  - `chore`: component test stack (jsdom, Testing Library, Storybook 10, ADR-0037) and component stubs.
  - `test`: 179 component and page unit tests across 17 test files, plus Storybook CSF stories (`*.stories.ts`) for atoms, molecules, organisms, and templates.
  - `feat`: Svelte 5 implementation of all main-screen UI components (Icon SVG set, IconButton, ProgressBar, VerdictCell, ColumnHeader, CutoffValue, WindowControls, TitleBar, Toolbar, FolderDropZone, StatusBar, ResultsTable with virtualisation and keyboard navigation, MainLayout template, `analysis` store with event batching, and `AnalysisPage` wiring the Tauri API client).
  - All 179 vitest tests and backend tests pass; `just ci` and `npm run build-storybook` green.

**Pending**
- Gate G2: review and open PR for `feat/RF-01-05-06-main-screen` (human merges).
- Gate G1: calibration tool spec (RP-11) open questions from previous block.

**Blockers / questions for the human**
- Ready to open PR for `feat/RF-01-05-06-main-screen`?

**Next step:** Open PR for `feat/RF-01-05-06-main-screen` (G2); next feature is RF-07 (spectrogram viewer in the detail area).

## 2026-10-18 - Calibration tool spec (RP-11)

**Done**
- Branch `spec/calibration`, PR for G1: new process requirement RP-11, `specs/calibration/` (inputs, generation, evaluation, sample corpus, config and grid files), `specs/features/calibration.feature` (CI-runnable on a synthetic sample corpus), ADR-0036 (dev-only crate `tools/fakeflac-calibrate`, JSON Lines features cache keyed by an extraction fingerprint, ffmpeg as an external process, no new crate in `Cargo.lock`). `dependencies.md` rule 3 and the overview table amended. Source: the owner's issue in `FAKEFLAC_CALIBRATION` (not in the repo).

**Pending**
- G1: owner answers the 17 open questions at the top of `specs/calibration/calibration.md` (v1 is not re-run: OQ-1/OQ-2; `confirmed.txt` for confirmed fakes and false positives: OQ-5; `/calibrate` cost weights: OQ-11) and approves the dependency uses in ADR-0036.
- #24 (`just prune-branches`) and #25 (RF-03 undeclared length) merged; `just prune-branches` run after them.
- After G1: `/feature RP-11` (lint-arch checks for `tools/`, `EXTRACTION_VERSION` in `fakeflac-audio`, the tool, `just accept`/`calibrate`/`privacy-check`, the `/calibrate` skill).

**Blockers / questions for the human**
- The 17 open questions above.

**Next step:** G1 on the calibration spec; meanwhile the main-screen UI `/feature`.

## 2026-10-18 - Undeclared stream length (RF-03) and branch clean-up

**Done**
- #22 (detection v2) and #23 (spec follow-ups) merged.
- PR #24 `chore/prune-branches`: `just prune-branches` deletes merged local/remote branches and clean agent worktrees (squash-merge aware, keeps and reports commits pushed after a merge). Already run: 11 stale local branches and the old worktree removed (its empty folder stays: another process holds it open).
- Branch `feat/RF-03-unknown-length`: a FLAC with `total_samples = 0` (symphonia gives `num_frames = None`) is decoded once to count frames, then analysed; test for the silent-track `cutoff: null`. Review: no blocking findings; the second pass now checks the counted length.

**Pending**
- G2: PR #24 and the RF-03 undeclared-length PR.
- Review improvements not done: count frames from packet durations instead of a full decode (RNF-01, only for undeclared-length files); a truncated undeclared-length FLAC is analysed as a shorter track (spec could say so, G1); no tests for corrupt/short undeclared-length files.

**Blockers / questions for the human**
- None.

**Next step:** the human's next task: `FAKEFLAC_CALIBRATION/issue.md`; then the main-screen UI `/feature`.

## 2026-10-18 - Detection v2 implemented (RF-03, RF-04)

**Done**
- Detection v2 spec approved at G1 (#21).
- Recovered `327311e` (decision: the main-screen UI is written by the agents; environment notes). It was pushed to `feat/RF-01-not-a-folder` after #20 was squash-merged, so it never reached `main`. Deleted that remote branch.
- Branch `feat/RF-03-04-detection-v2`:
  - `chore`: `BandSpectra` (mean `M` and median `Q`), `DetectionParams` v2, `HighBandGaps` (core, DTO, bindings).
  - `test`: 64 rule tests in `detect_rules.rs`; 34 scenario tests in `detection_lossless.rs`, with the 5 new fixtures.
  - `feat`: `measure`, `decide`, `cutoff_estimate` and `detect` per rules.md sections 2 and 3; the adapter samples 64 x 4 frames (N = 4096 at 44.1 kHz) and builds 100 Hz bands.
  - Review: no blocking findings; wording and invariant improvements applied. `just ci` green.

**Pending**
- G2: human merges the detection v2 PR. ADR-0035, `rules.md`, ADR-0033 and ADR-0034 are still `status: proposed`.
- Spec PR (G1) before the UI:
  - `specs/ui/main-screen.md` has no text for `reason.highBandGaps`.
  - `specs/traceability.md` and `specs/requirements/RF-03.md` still say "average spectrum".
  - Not specified: the cutoff reported for a silent track (code returns `None`); a FLAC with `total_samples = 0` is reported as silence (`LowHighFrequencyContent`).
- Calibration notes (phase 5):
  - `sfb21_gaps` reports cutoff = Nyquist only because D = 6.15 dB is below the 7 dB threshold.
  - Medians use the upper middle value for even counts; the prototype probably used `np.median`.
- Local clean-up blocked by auto mode (the human can run it): remove the worktree `.claude/worktrees/spec-detection-v2-aa8ae3` and the stale local branches (`git branch -D` on the 9 squash-merged branches).
- Earlier pending items below are unchanged.

**Blockers / questions for the human**
- None.

**Next step:** spec PR with the points above (`reason.highBandGaps` label first), then `/feature` for the main-screen UI (RF-01 dialog + drop, RF-05 status bar and cancel, RF-06 table).

## 2026-10-18 - Detection v2 spec (RF-03, RF-04)

**Done**
- Branch `spec/RF-03-04-detection-v2`, PR for G1. On a local corpus, v1 returns `Real` for real MP3 → FLAC transcodes: the noise or dither above the low-pass sits less than 60 dB below the reference level. The Rust v1 code returned Real for all 16 tracks of an album with known truth, 7 of them transcodes. v2: relative cutoff step on the mean band spectrum + gap step at 16 kHz on the per-frame median spectrum; new params file (`algorithm_version` 2), 5 new fixtures, reason `highBandGaps`, ADR-0035. Evaluation figures are in the PR.

**Pending**
- G1: human reviews the detection v2 spec PR (thresholds, verdict table, new reason).
- After G1: `/feature RF-03,RF-04` again: the test author rewrites `detect_rules.rs` / `detection_lossless.rs` and adds the new fixtures; the implementer changes the analyzer output (mean + median band spectra) and `DetectionParams`; `just gen`.
- Follow-ups noted in the PR, not specified: integrity finding instead of `Error` for partially corrupt tracks; FLAC MD5 check; CSV for Excel in the Spanish locale (revisit ADR-0012).
- Earlier pending items below are unchanged.

**Blockers / questions for the human**
- Thresholds were calibrated on the same corpus. Before release, phase 5 (`just accept`) needs more labelled albums, especially intros/skits flagged Fake inside real albums.

**Next step:** G1 on the detection v2 PR; meanwhile the main-screen UI (see the next block).

## 2026-10-15 - H1: contract v2 in the backend (RF-01)

**Done**
- Merged: #18 (Tauri adapter, bindings, API client; CI Rust tests on Windows only) and #19 (contract v2: dropped files, `NOT_A_FOLDER`, `INTERNAL` terminal, rows keyed by index, absolute root).
- Branch `feat/RF-01-not-a-folder`: `NOT_A_FOLDER` for a file root, relative roots rejected first (`FOLDER_UNREADABLE`), client helper `startAnalysisOfDropped` (tries dropped paths in order, skips files, `null` if all are files). CI: runs on `main` are never cancelled (one concurrency group per commit); PR runs still replace each other. Review: no blocking findings; 4 improvements applied.

**Pending**
- G2: human merges the `feat/RF-01-not-a-folder` PR.
- Main-screen UI (RF-01 dialog + drop, RF-05 status bar and cancel, RF-06 table, `INTERNAL` scenario): ignore drops while a run is in progress (a drop of files during a run would otherwise show `ANALYSIS_RUNNING`).
- Earlier follow-ups (RF-02/RF-05 block) still open; `build-windows` takes ~17 min (Tauri built twice).

**Blockers / questions for the human**
- None. Decided: the main-screen UI is written by the agents (normal `/feature` cycle), in a new session.

**Next step:** after #20 is merged, `/feature` for the main-screen UI (RF-01 dialog + drop, RF-05 status bar and cancel, RF-06 table): UI specs in `specs/ui/`; atoms/molecules with stories first (S6), then `AnalysisPage` wiring `src/lib/api/` (`startAnalysisOfDropped` for drops). Environment notes for this machine: in Git Bash every command ends with a harmless "cwd file" error and exit code 1; run frontend tests via `just`/PowerShell (`npx vitest` fails); `git push` needs `-c credential.helper= -c "credential.helper=!gh auth git-credential"`.

## 2026-10-15 - H1: Tauri adapter and API client (RF-01, RF-05)

**Done**
- RF-02/RF-05 analyze-folder backend merged (#17, ADR-0034 approved with it).
- Branch `feat/RF-01-05-adapter`: commands `pick_folder` (dialog from Rust, modal), `start_analysis` (Channel), `cancel_analysis`; contract DTOs in `src-tauri/src/dto.rs`; generated `src/lib/api/bindings.ts` (`just gen`); API client `src/lib/api/index.ts` (resolves only after the terminal event; non-contract rejections → `INTERNAL`); a page reload cancels the running analysis. New recipes `gen`, `gen-check`, `check-app` (CI: Windows build job). 26 adapter tests (17 Rust, 10 vitest, incl. 1 bindings drift). Review: 1 blocking (ADR-0033 out of date) fixed with implementation notes; improvements applied.

**Pending**
- G2: human merges the `feat/RF-01-05-adapter` PR.
- Spec clarifications for G1 before the UI (from the review): dropped items that are files, not folders (frontend can't tell them apart; `start_analysis` on a file gives `FOLDER_UNREADABLE`); `INTERNAL` as a possible `start_analysis` error with no terminal event; paths are lossy for non-UTF-8 names (rows must be keyed by `index`, not `path`); relative `root`.
- Earlier follow-ups (RF-02/RF-05 block) still open.

**Blockers / questions for the human**
- None. Note: in this Git Bash, `npx vitest` fails ("failed to find the runner", likely drive-letter case); run frontend tests through `just` or PowerShell.

**Next step:** `spec/H1-contract-clarifications` (the four points above) → G1; then the main-screen UI (RF-01 dialog + drop, RF-05 status bar and cancel, RF-06 table) with UI specs, stories and component tests.

## 2026-10-15 - H1: analyze-folder backend (RF-02, RF-05)

**Done**
- H1 specs approved at G1 (#14); fixture wording clarified (#16).
- RF-03/RF-04 detection engine for WAV/FLAC merged (#15).
- Branch `feat/RF-02-05`: `FileScanner` port + `WalkDirScanner` (`fakeflac-fs`), `analyze_folder` use case with progress events, `CancelToken`, `AnalysisSlot` (one run at a time), `AppError`. 24 new tests (17 core with fakes, 7 fs on temp folders). Review found 3 blocking issues, all fixed with tests: decoder panic isolated per track, unreadable root → `FOLDER_UNREADABLE`, std threads instead of rayon recorded in ADR-0034 (proposed).

**Pending**
- G2: human merges the `feat/RF-02-05` PR (it also approves ADR-0034).
- Follow-ups from the review (not blocking): cancel inside the analyzer and the scan if per-track time can exceed ~400 ms (measure in release, RNF-01); Windows hidden/system attributes and `$RECYCLE.BIN` (spec change at G1); a supported file with unreadable metadata is skipped instead of shown as `Unreadable`; no symlink test (needs Developer Mode on Windows).
- `cargo clippy --tests` is not clean on pre-existing tests (`detect_rules.rs`, `domain.rs` unit test use `expect` without an allow); `just check` does not lint tests.
- Rest of the harness listed in the bootstrap block.

**Blockers / questions for the human**
- Next PR needs a decision on how contract types get `specta::Type`: mirror DTOs in `src-tauri` (no ADR) or an optional `specta` feature in `fakeflac-core` (ADR). Default: DTOs in `src-tauri`.

**Next step:** `feat/RF-01` Tauri adapter: `pick_folder`, `start_analysis` (Channel; cancel the token if a send fails), `cancel_analysis`, tauri-specta bindings (`just gen`, `BigIntExportBehavior::Number` for u64/i64), dialog plugin with `dialog:allow-open`, then the UI (RF-01 drop, RF-05 status bar, RF-06 table) and RF-07 spectrogram.

## 2026-10-15 - H0 smoke test: RF-10 through the full cycle

**Done**
- CI green on `main` (PR #8: supply-chain check limited to the Windows target; dependency graph enabled).
- Dependabot: updates grouped (PR #9); closed #4 (`@types/node` 26 must match Node 24) and #7 (TypeScript 7 is not supported by `svelte-check` 4.7).
- RF-10 album summary: spec approved at G1 (PR #10); branch `feat/RF-10` with stub, `test(RF-10)` (11 failing tests), `feat(RF-10)` (all green), review without blocking findings.

**Pending**
- G2: human merges the `feat/RF-10` PR. That closes step 0.13 and the H0 acceptance criteria can be checked.
- Dependabot actions group PR #11 (asked to rebase; its CI had not run because of a conflict).
- Rest of the harness listed in the previous block.

**Next step:** H1 specs, grouped in one `spec/H1` PR for gate G1: detection rules and `DetectionParams` v1, contract, `analyze-folder.feature`, `inspect-track.feature` and UI specs for the MVP (RF-01..07, RF-14 for WAV/FLAC).
## 2026-10-15 - Repository bootstrap (H0, in progress)

**Done**
- Repository created at `C:\Workspaces\FakeFlac` (git, MIT, English docs, public-repo ready).
- Design migrated from the Spanish planning document: `docs/architecture/*`, `docs/adr/0001-0032`, `specs/requirements/` (RF/RNF/RP), reference UI screenshots in `docs/reference/`.
- New requirements captured: formats WAV/FLAC/MP3/M4A/Opus (RF-14) and real-bitrate detection for lossy files (RF-15, e.g. 320 kbps MP3 from 128 kbps).
- Cargo workspace skeleton (`core`, `audio`, `fs`, `store`, `export`, `src-tauri`) with the domain model in `fakeflac-core`.
- Frontend scaffold (Svelte 5 + Vite + TS), atomic design folders and dark design tokens. `npm run check` and `npm run build` pass.
- Harness: `CLAUDE.md` (root and nested), `justfile`, `.claude/settings.json` (permissions, hooks), hooks with 24 passing tests, 4 subagents, 4 skills, `.mcp.json`.
- `scripts/lint-arch.mjs`, `scripts/trace.mjs`, `scripts/guard-tests.mjs`.
- CI: `ci.yml`, `ci-nightly.yml`, `release.yml`, Dependabot, CODEOWNERS, PR/issue templates, `deny.toml`.

**Verified on this machine (after installing VS Build Tools and just):** `just check` green; `cargo check -p fakeflac` (Tauri) compiles; `npm run tauri build` produces NSIS (1.44 MB) and MSI (2.12 MB), well under RNF-06's 20 MB. CI workflows are still unrun.

**Pending**
- Verify `.mcp.json` package names, the GitHub Actions versions/inputs and pin actions by commit SHA (Dependabot can maintain them).
- Create the GitHub repository (public), push, protect `main`, require the `ci-ok` check.
- Step 0.13: smoke test of the full cycle with a minimal pure feature (RF-10 worst-verdict rule).
- Remaining harness pieces from the design: `spec-reviewer`, `architecture-guardian`, `docs-keeper`, `/new-component`, `/calibrate`, `/release`, `notify` hook, `settings.local.json` with `FAKEFLAC_CORPUS`.

**Blockers / questions for the human**
- The first push to `main` must be done by the human (`guard-bash` blocks agents pushing to `main`). The GitHub repo exists but is currently PRIVATE; make it public when ready.
- Keep the reference tool's player out of v1? (default: yes, see docs/architecture/frontend.md)

**Next step:** after the first push and branch protection (require `ci-ok`), start H1 specs (`/new-spec RF-01` … `RF-07`, `RF-14` for WAV/FLAC) following `docs/planning/fakeflac-phases-1-3.es.md` section 3.12, stage 1.
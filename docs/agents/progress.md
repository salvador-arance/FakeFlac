# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

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

**Next step:** the human's next task: `C:\Workspaces\FakeFlac-calibration\issue.md`; then the main-screen UI `/feature`.

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
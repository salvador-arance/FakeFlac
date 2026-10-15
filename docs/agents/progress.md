# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

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
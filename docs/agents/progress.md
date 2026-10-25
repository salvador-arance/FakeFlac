# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

## 2026-10-25 - After RF-08: Dependabot alert dismissed, agent models committed

**Done**
- #76 (RF-08 feature) merged. `just prune-branches` deleted `feat/RF-08` and `spec/RF-08`.
- Dependabot alert #1 (`glib` 0.18, GHSA-wrw7-89jp-8q8g, medium) dismissed as "vulnerable code not used":
  - `glib` only comes in through Tauri's Linux backend (gtk/webkit2gtk via tao, wry, muda). `cargo tree -i glib` for the Windows host target prints nothing; only `--target all` shows it.
  - It can't be fixed by an upgrade: glib >= 0.20 needs gtk-rs 0.20, and Tauri 2.12 uses 0.18.
  - Revisit if a Linux build is ever added (RNF-07).
- `.claude/agents/{rust-implementer,test-author}.md` now run on `opus` (`6dd1662`): the owner's local edit, committed at the owner's request.

**Pending**
- H2 requirements still `draft`:
  - RF-11 (result cache, medium).
  - RF-13 (Spanish/English interface, low). It carries the #76 review notes: the locale-dependent Format sort key cache, and `getLocale()` inside `ResultsTable`'s `$derived`.
  - RNF-01, 06, 07, 08, 10 and 11 (high).
- RF-08: manual checks M1–M3 and P1–P2, if they were not run before the merge.
- There is still no frontend implementer agent type. RF-08 and RF-10 used `general-purpose` with a brief. Consider a `frontend-implementer.md` before the next UI feature.

**Blockers / questions for the human**
- Which requirement comes next.

**Next step:** the owner picks the next H2 requirement, then `/new-spec RF-xx`.

## 2026-10-25 - RF-08 (sort and filter the results table) implemented; feature PR #76 waiting for G2

**Done**
- #74 (RF-10 feature) and #75 (RF-08 spec) merged.
- `/feature RF-08` on `feat/RF-08`, PR #76 (frontend only, contract v5 unchanged):
  - Tests:
    - `1f66eee`: `build(RF-08)` stubs.
    - `7d87244`: `test(RF-08)`, 239 tests and stories.
    - `b5e573d`: owner-approved edit to the existing Toolbar separator test (section 2 adds a second separator).
    - The owner accepted the six readings the test author made, listed in the PR.
  - Implementation: `248a0f4` `feat(RF-08)`, in `src/lib/` only.
  - Review round 1: nothing blocking. Improvement 1 (literals instead of tokens) fixed in `511f2e8`.
  - Traceability: `f525c5d`.
  - `just ci` (675 vitest), `trace`, `privacy-check`, `guard-tests`: green.

**Pending**
- G2 of #76 (owner): manual checks M1–M3 and timing P1–P2 on a release build first. Then `just prune-branches`.
- Review notes for RF-13, written in #76:
  - the Format sort key cache in `table-view.ts` is locale-dependent (ALAC label);
  - `ResultsTable` reads `getLocale()` inside a `$derived`.
- If P1 is tight: make the store's `rows` a `$state.raw` array.
- Dependabot alert #1 is still open and unexamined.
- Harness gaps:
  - The RF-10 one (writes to protected files through Bash) did not repeat.
  - New, seen once: the test author wrote temporary logs to the session scratchpad through Bash redirects. That is outside the repo, against RP-06, because `guard-protected` only covers Edit/Write. If it repeats, record it in `lessons.md`.

**Blockers / questions for the human**
- None.

**Notes for the next session**
- In this session's Bash tool, vitest failed with "Vitest failed to find the runner" (all suites), while the same command passed from PowerShell. The likely trigger was an `npx vitest run <file>` call; the cause is unknown.
  - Workaround: `powershell.exe -NoProfile -Command "Set-Location C:\Workspaces\FakeFlac; just check"`.
  - Use `npm run test -- <pattern>`, never `npx vitest`.
- Temporary files: the session scratchpad is outside the repo and blocked by the hook. Use the ignored `target/` instead (the PR body went through `target/pr-body.md`).
- One RF-10 page test (`AnalysisPage.albumReveal.test.ts`) timed out once at 5 s under load. It passed in every other run.
- The owner's local edits in `.claude/agents/{rust-implementer,test-author}.md` are still uncommitted. Never commit them.

**Next step:** after #76 is merged, `just prune-branches`, then the owner picks the next H2 requirement.

## 2026-10-25 - RF-10 (album summary in the app) implemented; feature PR #74 waiting for G2

**Done**
- #73 (RF-10 spec) merged at G1.
- `/feature RF-10` on `feat/RF-10`, PR #74:
  - Tests:
    - `1fde168`: the first `test(RF-10)` commit.
    - `92cc5d2`: the A1 page test fixed with the owner's approval. It expected plural-only counts ("1 errors") and counted DOM rows, but the non-virtualised jsdom table renders at most 50.
  - Implementation:
    - `86ad2f3` (core and src-tauri): `AlbumUpdate`, album lock, name rule and the `AlbumTally` shared with `summarize_album`.
    - `0b698bb` (frontend): `AlbumSummaryList`, store, sidebar, table scroll-to-selection.
  - Review round 1: nothing blocking; improvements in `e406e8c`.
  - Its finding 2 led to Q16: selecting the current album again brings its first row back into view.
    - `05805d5`: spec. A delegate answered the G1 questions and the owner approved.
    - `885fa98` and `1c2667c`: tests and story.
    - `70d5a9f`: `ResultsTable.revealRequest`.
    - Review round 2: nothing blocking; improvements in `921af6c`.
  - `just ci` (436 vitest), `trace`, `privacy-check`, `guard-tests`: green.
  - Manual checks M1 (1000 albums, release build) and M2 (visual) passed (owner).

**Pending**
- G2 of #74 (owner), then `just prune-branches`.
- Dependabot alert #1 is still open and unexamined.
- Harness gap, seen once: the frontend implementer (a `general-purpose` agent) appended a story to the protected `ResultsTable.stories.ts` in stage `implement`.
  - `guard-protected` only covers Edit/Write/NotebookEdit, so it was presumably done through Bash.
  - CI `guard-tests` caught it. The orchestrator amended the commit before the push and moved the story to its own `test(RF-10)` commit.
  - If it repeats, record it in `lessons.md` and close the Bash path, or add a frontend implementer agent definition.
- Optional: name the type-forced test edits (`ResultsTable.test.ts` and stories, `albumIndex: 0`) in commands.md's declared list. They are disclosed in #74.

**Blockers / questions for the human**
- None.

**Notes for the next session**
- `just test` excludes the src-tauri package (`fakeflac`). Its tests run in `check-app` / `just ci` or with `cargo test -p fakeflac`.
- There is no frontend implementer agent type. RF-10 used `general-purpose` with a brief, and it does not inherit `rust-implementer`'s rules.
- The owner has uncommitted local edits in `.claude/agents/{rust-implementer,test-author}.md` (model sonnet → opus). Never commit them.

**Next step:** after #74 is merged, `just prune-branches`, then the owner picks the next H2 requirement.

## 2026-10-25 - RF-10 (album summary in the app) specified; spec PR #73 waiting for G1 merge

**Done**
- #71 (RF-09 CSV export) merged at G2.
- The owner chose RF-10 as the next H2 requirement.
- RF-10 spec, PR #73 (`spec/RF-10`):
  - 4c3c6da: `commands.md` version 5 (`trackDone.album: AlbumUpdate`, computed in core with `summarize_album`, kept in order by a separate album lock), the left Albums panel (`specs/ui/album-summary.md`), and `features/album-summary-app.feature` with scenarios for core, src-tauri and the UI. The approved H0 rule files are untouched.
  - code-reviewer readiness review: nothing blocking; its 9 improvements are applied.
  - 5f0ef58: G1 questions answered by a delegate on the owner's behalf. Q1–Q14: defaults kept. Q15: the core sink-panic scenario is dropped. The decisions are in "G1 decisions" of `ui/album-summary.md` and summarised in a PR comment.
  - `just trace` and `just privacy-check` are green.

**Pending**
- The owner merges #73 (G1), then `just prune-branches`.
- `/feature RF-10`. The `test(RF-10)` commit must update the existing tests that build or match `trackDone`; they are listed in `commands.md` version 5 and in the RF-10 row of `traceability.md`. The implementer must not touch them.
- Dependabot alert #1 is open on the repo (seen on push); not looked at.

**Blockers / questions for the human**
- None.

**Notes for the next session**
- The name-rule and Windows-path scenarios are `#[cfg(windows)]`; CI runs Rust tests on `windows-latest`.
- `StatusBar.svelte` has hard-coded English strings (existing debt, for RF-13).

**Next step:** after #73 is merged, run `/feature RF-10`.

## 2026-10-25 - RF-09 (CSV export) built; feature PR #71 waiting for G2

**Done**
- #66 merged: README says v6 is accepted.
- The owner rewrote `main` (05ce393) to drop a Claude `Co-authored-by` trailer.
- #67 merged: the no-attribution rule is in `CLAUDE.md`.
- RF-09 spec:
  - #68 merged (G1), every default kept; no ADR (owner, Q15);
  - #69 merged: `NOT_A_FOLDER` from Open folder shows "Ready" without the notice.
- #70 merged: `check-fast` runs clippy with `--all-targets`; the 31 lints it found in test code are fixed.
- RF-09 feature, PR #71 (`feat/RF-09`):
  - commits 699541f (tests), befc528 (backend), f1b9ae3 (frontend), 06e396e (review fixes);
  - `just ci` green: 702 Rust tests, 324 frontend tests;
  - review: nothing blocking.

**Pending**
- G2 for #71. The manual checks M1–M7 pass (owner, `npm run tauri dev`). M6(b): a typed `results.txt` is refused ("Cannot save the CSV file") and no file is written. M7: 10 000 rows in about 0.5 s.
- #72 merged: `App.svelte` mounts `AnalysisPage`. Until then the window showed the H0 placeholder, and no test rendered `App`.
- `CLAUDE_CODE_SUBAGENT_MODEL=sonnet` in `~/.claude/settings.json`: the owner adds it (it is outside the repo).

**Blockers / questions for the human**
- None. (`src-tauri/examples/cli.rs`, an untracked file no agent wrote, was removed by the owner.)

**Notes for the next session**
- Bash may start in `/cygdrive/c/...`; use `cd "C:/Workspaces/FakeFlac"` there, or vitest fails to start.
- A worktree under `target/tmp/` has no `node_modules`: `npm run check` and vitest cannot run there, only the Rust checks.
- A subagent can stop when the session ends, leaving uncommitted work. Check `git status` and run the checks before you assume it is lost or done.

**Next step:** after G2 of #71, choose the next H2 requirement: RF-08 (sort and filter), RF-10 (album summary in the UI), RF-11 (cache) or RF-13 (Spanish UI).

## 2026-10-25 - #65 merged (G1); detection v6 accepted

**Done**
- #65 merged (098ae18): E6 `holdout` gate on the Wilson lower bound. With it, v6 passes E1–E7 on run `20261025T010130Z` and is accepted.
- README status: v6 accepted on the local corpus.

**Pending**
- G2: merge this docs PR.
- Choose the next requirement. H2 still has RF-08 (sort and filter), RF-09 (CSV export), RF-10 (album summary in the UI; the core use case exists), RF-11 (cache), RF-13 (Spanish and English UI), Opus (ADR-0030 spike), RNF-01, RNF-06, RNF-07, RNF-08 and RNF-11. Open detection question: Q15 (W6, via per-frame statistics).

**Blockers / questions for the human**
- Which requirement comes next.

**Notes for the next session**
- Same as the block below: temporary files in `target/tmp/`; keep Windows awake during corpus runs.

**Next step:** `/new-spec` for the requirement the owner chooses.

## 2026-10-25 - Corpus run of v6 done; E6 `holdout` gate changed (spec PR, waiting for G1)

**Done**
- Owner's answer to the block below (2026-10-24): judge E6 on `all` and on the new, contaminated `holdout`, with no spec change.
- `just accept` (v3 config, v5 and v6), run `20261025T010130Z`:
  - cold run, 18:11–02:01 local time; the PC slept 20:57–01:43, so about 2 h 50 min of work;
  - 38,305 entries decoded and 36,864 transcoded, 0 failures;
  - the work also includes the 14 RF-15 lossy groups (19,950 files), not only the 13 lossless settings.
- Results (rules.md section 8.3):
  - E1–E5 and E7 pass. W4 is `Suspicious` (`AmbiguousCutoff`) and T1 passes. E7: `just test`, 624 Rust tests and 279 frontend tests.
  - E6 on `all` passes: cbr192 0.28 pp, exactly as predicted.
  - E6 on `holdout` failed the original gate by one transcode: cbr192 and cbr192_s24 each lose 2 of at most 392 (0.51 pp). They are the same two source tracks at 16 and 24 bits.
- The owner chose to keep v6 and its values and to change the `holdout` gate: E6 now fails there only when the lower bound of the 95 % Wilson interval of the drop exceeds 0.5 pp. Q23 records that the choice was made after seeing the result.
- Spec PR `spec/RF-04-e6-holdout-gate`:
  - rules.md: Q19 resolved, Q23 added, E6 row and its rationale, a note in 8.2, results in 8.3;
  - ADR-0042: amended;
  - README status.
  - No code or test change: the tool does not compute E6.

**Pending**
- G1: review and merge the spec PR. After the merge, the README can say that v6 is accepted.

**Blockers / questions for the human**
- None.

**Notes for the next session**
- Temporary files go in `target/tmp/` inside the repo, never in the session scratchpad. RP-06 also covers shell redirects, which the `guard-protected` hook does not see: this session wrote to the scratchpad by mistake and cleaned it up. The hook also blocks Claude's memory directory.
- A corpus run can take hours. Keep Windows awake for the whole run: this one stopped for almost 5 h while the PC slept.

**Next step:** after G1, v6 is accepted on the corpus. Choose the next requirement from `docs/planning/` or the open questions (Q15: W6, via per-frame statistics).

## 2026-10-24 - #63 merged (G2); the corpus was reorganised

**Done**
- #63 merged (03fdfdc); `just prune-branches` run.
- `confirmed.txt` checked (read-only):
  - W5 and W6 are the 5th and 6th `false-positive` lines, after W4;
  - they match the measurements of the v4 run: W5 `fs` 20150 and `D` 9.88, W6 `G` 5.65;
  - `trusted.txt` covers them.
- `.claude/agents/code-reviewer.md`: `model: opus` → `sonnet` (owner, to cut usage).

**Correction to the block below: the next run is a cold run with a new split.**
- **Paths changed.** The owner reorganised the library after the v4 run (`20261019T141953Z`). The relative paths changed, folders and file names included, and `trusted.txt` and `confirmed.txt` already use the new paths. No run has used them yet.
- **Cold run.** Corpus cache entries are keyed by path, size and mtime, and transcode recipes contain the source path (generation.md sections 3 and 4). The next `just accept` re-extracts the whole corpus and regenerates every transcode, not only the 24-bit ones. Expect a run much longer than the earlier "about 5,700 transcodes".
- **New split.** `album_id` is the folder path (inputs.md section 4), so `tune`/`holdout` is drawn again.
  - Verdicts on `all` should not change, since the audio is the same.
  - The `holdout` figures of rules.md section 8 (Q19) will not reproduce.
  - The new `holdout` mixes in albums from the old `tune`, where the v6 values were chosen, so it is no longer a clean holdout for E6.

**Blockers / questions for the human**
- Before `just accept`: is E6 judged on `all` plus the new, contaminated `holdout`? Or does Q19's evaluation go back to G1, for example by pinning the old split or re-choosing on the new `tune`?

**Next step:** answer the question above, then run `just accept --config specs/calibration/calibration.v3.toml --params specs/detection/detection-params.v5.toml --params specs/detection/detection-params.v6.toml`.

## 2026-10-24 - Detection v6 implemented (#63, waiting for G2)

**Done**
- #62 merged (G1). Q19 and Q22 were still "Open, default yes"; the merge was taken as approval of the defaults.
- `/feature RF-04` on `feat/RF-04-detection-v6`, PR #63. `just ci`, `just trace` and `privacy-check` are green.
  - Tests: 80e83f7 (with compile stubs) and c1da438 (rustfmt; the test stage skips `check-fast`, so fmt slipped through).
  - Implementation: 6bd0830, covering v6 core, `[edge]` and calibration v3.
  - e57e7f6: protected-test fix in `params_files.rs` (`edge: None`). Inside ADR-0042's list, but made after the feat commit; the PR says so.
  - Review round 1 found one blocking issue: the v6 near-threshold rule (evaluation.md section 8) was missing. Fixed by 8c070ef (tests) and fcd3b93 (the near rule, grid points filtered by `validate()`, reuse of `is_edge_step`).
  - Review round 2 approved.
- `.claude/agents/code-reviewer.md`: `model: opus` → `sonnet` (owner, to cut usage). **Uncommitted** in the working tree, kept out of #63; commit it in its own docs/chore PR.

**Pending**
- G2 on #63.
- Then the owner adds W5 and W6 to `confirmed.txt` (the draft is in `target/v6-analysis/confirmed.txt`; check that the rip `.m4a` files are stream copies).
- Then `just accept --config specs/calibration/calibration.v3.toml --params specs/detection/detection-params.v5.toml --params specs/detection/detection-params.v6.toml`. The first run generates about 5,700 new 24-bit transcodes. If E1, E2, E3 or E6 fails, the figures go back to G1, not to tuning.
- Follow-ups listed in #63:
  - add a test for the edge-aware list in `render.rs`;
  - add a test for the `validate()` grid filter;
  - G1 clarifications: lossy rows' edge columns (lossy.md 6.4), the `floor_min_db` comment in `detection-params.v6.toml`, the p95 method;
  - `clippy --all-targets` `unwrap_used` in the `fakeflac-audio` tests.
- Unchanged from the block below: the review-3 improvements of #58, Q16/Q17 in `mp3.rs`, the #55 leftovers, B1–B11.

**Blockers / questions for the human**
- None for #63 itself. Before the corpus run: W5/W6 in `confirmed.txt`.

**Next step:** after G2 merges #63, run `just prune-branches`, then the `just accept` run above.

## 2026-10-24 - Detection v6 spec PR (#62, waiting for G1)

**Done**
- **Corpus analysis** (offline port of `measure()`/`decide()`, 0 mismatches on 14,241 rows; cache of run `20261019T141953Z`; detectable transcodes only):
  - values chosen on `tune` only, then checked on `holdout`: `EdgeStep` = `InRange(fs)` ∧ `D_Q < 3` ∧ `R ≥ 15` ∧ `D < 16`;
  - `holdout` passes E6 with a thin margin: worst cbr192 0.41 pp (2/484);
  - W4 not `Fake`, 15/15 confirmed fakes `Fake`, `primary` 0 `Fake`.
- **24-bit correction:** `F_top` is never below −104 dB absolute (the 16-bit floor), whatever the bit depth (owner, Q20).
  - Tested on 773 corpus transcodes decoded to both 16 and 24 bits (scratch only): no 24-bit-only release with it (4 without), W4 still released, no 16-bit verdict changes.
- **#62 (`spec/RF-04-detection-v6`, debdc6b):**
  - rules.md v6 (sections 2–4, 6, and 8: E1–E7; Q12–Q18 resolved, owner delegated; Q19–Q22 open);
  - `detection-params.v6.toml` (default, `[edge]`), fixtures `edge_bump19k3`, `edge_bump19k3_lp19k5` and `holes_lp19k5`, `detection-edge-step.feature`;
  - evaluation.md edge report, per-setting `decode_args`, `calibration.v3.toml` (4 settings `_s24`), version references (next free 7);
  - ADR-0042 amended; pending file removed.
- **G1 review by a subagent** (album bootstrap, segment-shift variability, 773 pairs at 24 bits, numpy synthesis of the fixtures; scripts in `target/v6-analysis/g1/`). Applied to #62:
  - `edge_bump19k3` as first written failed `D_Q < 3` (2.97–3.48 dB); it now uses a 0.25 s gate at +15 dB (6/6 seeds pass);
  - the all-fixture scenarios of `detection-lossless`/`detection-lossy` use `detection-params.v5.toml`;
  - Q20 and Q21 resolved by the owner (floor always at −104 dB; `_s24` settings cbr128, cbr192, cbr256, cbr320);
  - rules.md 8.2 statistical caveats (about a 30 % chance that a new corpus of this size fails E6), the selection grid, corrected figures, dual-mono.
  - `just check`, `just trace` and `privacy-check` are green.
- Throwaway scripts (aggregates only, not versioned): `target/v6-analysis/` (git-ignored). Copy them to the calibration folder if they should be kept.

**Pending**
- G1 on #62 (Q19 and Q22 still open, default yes).
- Then `/feature RF-04` (detection v6).
- Then `just accept --config specs/calibration/calibration.v3.toml --params specs/detection/detection-params.v5.toml --params specs/detection/detection-params.v6.toml`. The first run generates about 5,700 new 24-bit transcodes.
- Owner, before the run: add W5 and W6 (the proposal's N1 and N2) as `false-positive` lines of `confirmed.txt` after W4.
- Unchanged from the block below: the review-3 improvements of #58, Q16/Q17 in `mp3.rs`, the #55 leftovers, B1–B11 on the corpus, `clippy --all-targets` in `full_band.rs:892`.

**Blockers / questions for the human**
- Q19: the values rest on `tune`, and `holdout` cbr192 is one transcode from the 0.5 pp gate.
- Q22: the list of changed approved specs and protected tests.
- Owner, before the run: the rewritten `confirmed.txt` (W5, W6 and the RF-15 rips) is in `target/v6-analysis/confirmed.txt`; check that the rip `.m4a` files are stream copies (`real`).
- Environment: the `guard-protected` hook blocks Write/Edit outside the repo, the scratchpad included. Put throwaway scripts in `target/` (ignored), and write audio outside the repo only through ffmpeg.

**Next step:** after G1 merges #62, run `/feature RF-04`.

## 2026-10-23 - Detection v6 proposal (edge step on V4): merged at G1 (#60)

**Done**
- The owner tested the app on one album held in three copies: store FLAC, store 320 kbps MP3, and a rip (AAC 128 plus MP3 320 transcoded from it). The test ran the engine headless with `DetectionParams::default()` from a scratch crate outside the repo.
  - MP3 and M4A (v5) are all correct: the store MP3 is `Real` at 320, the rip's AAC is `Real` at 128, and the rip's MP3 is `Fake` with an estimate of 128 (112 on 4 tracks).
  - The FLAC copy gives 1 `Fake` and 2 `Suspicious`, all false positives. The `Fake` is **W4 (FP4)**: same `D`, `fs`, `G`, `U` and `B` as rules.md. The other two, named **N1** and **N2** in the proposal, are the `primary` tracks rules.md section 6 already mentions (N1 0.12 dB below V4 by V6, N2 by V5 with `G` = 5.65).
- Root cause of W4 and N1: a short narrow-band sound just below a natural roll-off. It makes a step on the mean spectrum, but the median spectrum is smooth across it and the level above it is far from the floor. The album was transcoded with 7 `calibration.v2.toml` settings (91 transcodes, scratch only). Neither the median step nor the residual separates them alone; together they separate W4 and N1 from the 78 low-pass transcodes.
- **#60 merged at G1 (690b7dd):**
  - `specs/pending/detection-v6-edge-step.md`: `D_Q`, `R` and `EdgeStep`; V4 gains "and not `EdgeStep`"; an optional `[edge]` section, `algorithm_version` 6; acceptance E1–E7; questions Q1–Q7.
  - ADR-0042, now `accepted` (direction only; values open, see below).

**Pending**
- **Follow-up spec PR (stage `spec`)** to make the proposal an approved spec:
  - move the rules into `specs/detection/rules.md` (sections 2–7);
  - write `specs/detection/detection-params.v6.toml` (v6 becomes the default);
  - write the fixtures `edge_bump19k3` and `edge_bump19k3_lp19k5` in `fixtures.md`;
  - write the scenarios (`@RF-04`), and the `D_Q` / `R` / `EdgeStep` reporting in `specs/calibration/evaluation.md`;
  - delete the pending file.
- Then `/feature RF-04` (detection v6) and `just accept --params …v5.toml --params …v6.toml` on the corpus. The indicative values (3 dB, 12 dB) are decided there; a failed E1, E2, E3 or E6 goes back to G1.
- Owner, in the local corpus (not the repo): add N1 and N2 as `false-positive` lines of `confirmed.txt` after W4 (Q7), so the tool lists them.
- Unchanged from the block below: the review-3 improvements of #58, the Q16/Q17 round in `mp3.rs`, the #55 leftovers, acceptance B1–B11 on the corpus, and the `clippy --all-targets` error in `full_band.rs:892`.

**Blockers / questions for the human**
- **Q1–Q7 of the proposal.** #60 was merged without explicit answers. The next spec PR assumes the defaults unless the owner says otherwise:
  - Q1: V4 only, not V6;
  - Q2: no new reason;
  - Q3: 3 / 12 dB as starting values;
  - Q4: N2 accepted as `Suspicious`;
  - Q5: not for lossy L3;
  - Q6: absolute floor window;
  - Q7: the owner adds N1 and N2.
- Unchanged: Dependabot alert #1 (`glib`), the `specs/CLAUDE.md` line about `specs/pending/`, the splitter accessibility gap.
- **Environment:**
  - `privacy-check` reads the committed branch diff only: run it after the commit (it reports "0 files checked" before).
  - From PowerShell, `just` recipes with a shebang (`accept`, `calibrate`, `privacy-check`) fail with "could not find `cygpath`". Run the binary directly: `cargo run --release -q -p fakeflac-calibrate -- privacy-check --base main`.

**Next step:** the follow-up spec PR that turns `specs/pending/detection-v6-edge-step.md` into rules.md v6 and `detection-params.v6.toml`, with the Q1–Q7 defaults unless the owner answers them first.

## 2026-10-23 - RF-15 part 2 (lossy calibration): merged (#58)

**Done**
- #57 is merged into `feat/RF-15-calibration` (b53373e).
- Round 2:
  - 6486941 test: #57 follow-ups and the mutants that survived round 1.
  - a9fdf10 feat: the shared pure `fakeflac_core::detect::track_guard`, used by `analyze_track` and by the tool. It fixes round-1 blocking 2 with no change in the app. Also the #57 items: `real applies to lossy files only`, the album-units `file:line` refs, the `upstream_by_rate` checks, the Fake-rows median and `within_source` for positive groups only. Also the watch list for every params file, `real_excluded` printed once, the ALAC text derived from data, and the `--no-generate` count.
- Review 2: 1 blocking finding, the holdout warning per stratum. The owner decided:
  - add the per-stratum warning for every stratum and keep the album-class warning;
  - keep the `--no-generate` change, so uncached labelled tracks are not "entries not generated".
- Round 3: f5f7507 test, ba0636c fix (per-stratum warning, lossy files skipped by extension under `--no-generate`, `source_max_kbps` bounds with the spec message, a single watch list). Review 3 found no blocking findings and killed every earlier survivor.
- `just ci` is green, `privacy-check` has 0 matches. **#58 is merged** (squash, 09fd253). Branches pruned after G2.

**Pending**
- Improvements left from review 3, for a small follow-up PR:
  - tests for a `lossy:*` stratum warning, for the `<` boundary of the per-stratum warning (`render.rs:810`), and for upper-case extensions under `--no-generate` (survivors N1b, N2, N3c);
  - one `pub(crate) is_lossy_path` instead of three extension lists (`run.rs:356`, `extract.rs:61-70`, `extract.rs:336-341`);
  - `UpstreamRate.source_max_kbps_invalid` as an enum or a raw value;
  - `upstream = 1` still gives a TOML type error.
- Spec notes for the next G1:
  - lossy.md section 11 should record that an uncached ALAC `.m4a` under `--no-generate` is skipped by extension (undercount);
  - `tests/lossy_inputs.rs:903, :939, :1030` cite scenarios that do not exist ("The holdout warning applies per stratum", "Lossy watch tracks are listed"): add them to `calibration-lossy.feature` or cite the spec section.
- Unchanged from the previous block: the Q16 and Q17 round in `mp3.rs` (`EXTRACTION_VERSION` 3), the #55 leftovers, and acceptance B1–B11 on the real corpus (`just accept`, now possible).
- `cargo clippy --all-targets` fails on `tools/fakeflac-calibrate/tests/full_band.rs:892` (type_complexity). The error predates this PR, and the justfile does not run that form.

**Blockers / questions for the human**
- Unchanged: Dependabot alert #1 (`glib`), the `specs/CLAUDE.md` line about `specs/pending/`, the splitter accessibility gap.
- **Environment:** the Git Bash cwd noise and the Vitest-from-Git-Bash note of the previous block still apply. The `guard-protected` hook blocks Write outside the repo, the session scratchpad included; pass PR bodies to `gh` on stdin (`--body-file -`).

**Next step:** `just accept` on the corpus (B1–B11) or the Q16/Q17 `/feature` round, whichever the owner prefers.

**Note:** squash-merging a PR before its handoff commit is pushed leaves the handoff out of `main`. Push the handoff before asking for G2.

## 2026-10-23 - RF-15 part 2 (lossy calibration): review round 1 done, round 2 waits for spec PR #57

**Done**
- #55 (RF-15 part 1) and #56 (spec follow-ups: L1a/L0/L1b order, MP3 duration tolerance, Q16 and Q17) are merged. Q16 and Q17 were accepted with their defaults: treat a tag frame count or bytes field of 0 as absent, and make free-format MP3 `unsupportedFormat`.
- `feat/RF-15-calibration`, local only, not pushed, no PR:
  - ab7ebb5 `test(RF-15)`: the 25 scenarios of `calibration-lossy.feature`, the C11 messages, the v2 default config. New files `tools/fakeflac-calibrate/tests/{lossy_inputs,lossy_acceptance,lossy_sample}.rs` plus `common/mod.rs` helpers.
  - cf439c0 `feat(RF-15)`: the attribute grammar, `real`, cascades, split units with `lossy:*` strata, the lossy tables and chains, the v2 default. `just check` is green.
- **Review round 1: 2 blocking findings.**
  1. A `real` folder defaulted to `lossy` against spec 1.2. The owner chose `lossy`, and the spec now says so in #57.
  2. `tools/fakeflac-calibrate/src/lossy.rs:89-99` re-sequences L0 and L1 itself and drops the "no declared bitrate" guard. The core's `analyze_track` does L1a (decode failed), then L0, then too short, then no bitrate. The tool must not reimplement this (ADR-0036).
- Spec PR **#57** (`spec/RF-15-calibration-followups`) is open for G1, and the owner has answered its questions:
  - `real` defaults to `lossy` and rejects `formats=lossless|all`;
  - the split domain goes by extension;
  - the album-in-several-units message lists `file:line` refs, `trusted.txt` first;
  - checks and messages for `[labels.upstream_by_rate]`;
  - 6.3 adds the median estimate of the Fake rows (B8) for positive groups only, and `within_source` applies to positive groups only;
  - chain keys, with disc-number stems kept as a known limitation.

**Pending: round 2 of `/feature RF-15` part 2, after #57 is merged**
1. `git merge main` (or rebase) into `feat/RF-15-calibration` to bring in #57.
2. **Stage `test`** (fresh test-author). Tests for the #57 changes:
   - two new Examples rows;
   - the exact album-units message;
   - Fake-rows median 128 for `l128_320`;
   - the `upstream_by_rate` checks;
   - `real formats=all`.

   Also tests that kill the mutants that survived round 1:
   - `real` folder default;
   - the source-limited threshold (`c − suspicious_shortfall`), with a unit test of `evaluate_lossy_row` placing the source cutoff between `c − fake_shortfall` and `c − suspicious_shortfall`;
   - source-limited only on honest generated rows;
   - weak fakes kept out of the strong `fake` row;
   - a Suspicious lossy row not counted as Fake;
   - cascade truth at equal bitrates (`low < last`);
   - `weak=x` rejected;
   - the "already listed" error for a track in both a `real` and a `fake` line, de-duplicated;
   - a C7 legacy cache line without `stream` (`cache.rs:209`);
   - "also trusted" on a `real` line;
   - a lossy stream with no declared bitrate giving `Error`.
3. **Stage `implement`** (fresh implementer):
   - blocking 2: ideally one pure core function, used by both `analyze_track` and the tool, that takes the probe data and returns the decision, with no behaviour change in the app;
   - the four #57 items not yet implemented: the album-units refs, the `upstream_by_rate` validation, the Fake-rows median and `within_source` on positives only, rejecting `real formats=all`.

   Spec-backed improvements from the review:
   - the lossy watch list for every params file (6.7);
   - the `--no-generate` `not_generated` count (`extract.rs:336`);
   - `real_excluded` lines repeated per extraction group (`render_lossy.rs:62-68`);
   - the holdout warning per stratum, not per album class (`render.rs:796`);
   - the "holds ALAC" text hard-coded (`render_lossy.rs:66`).
4. **Review round 2** with a mutation check. Expect it to be slow: each mutation recompiles and reruns `lossy_acceptance` (round 1 took about 28 min). Then `just ci`, PR, `/handoff`.

**Afterwards**
- `/feature` round for **Q16 and Q17** in `fakeflac-audio` `mp3.rs`. It bumps `EXTRACTION_VERSION` to 3, so do it after part 2, since C7 assumes 2. Also check `find_first_frame` accepting a bitrate-index-0 header without the next-frame check.
- From #55: a direct `detect_track` test for L0 before silence; `run_analysis_lossy_v5.rs` uses a WAV where the scenario says FLAC; `unwrap_or(AudioCodec::Pcm)` in `analysis.rs`; the alignment of the ColumnHeader bubble.
- Acceptance B1–B11 on the real corpus (`just accept`) after part 2. The installer size against RNF-06. The `confirmed.txt` lines and the splitter leftovers.

**Blockers / questions for the human**
- G1 of #57.
- Dependabot alert #1 (`glib`), the `specs/CLAUDE.md` line about `specs/pending/`, and the splitter accessibility gap: unchanged.
- **Environment:** Git Bash prints a spurious "cwd: No such file or directory" and exit 1 after every command; read the real status with `echo $?`. Vitest from Git Bash fails with a lowercase `c:` cwd; use PowerShell (`Set-Location C:\Workspaces\FakeFlac`). Subagents without Bash (spec-writer) cannot `git show` another branch; give them the file paths in a checked-out tree.
- **Harness:** a mutation check that disables checks in a scratch copy triggered a "Security Test Removal" warning. It was a false positive: the repo was untouched.

**Next step:** once #57 is merged: check out `feat/RF-15-calibration`, `git merge main`, then round 2 as in "Pending", starting with the test stage.

## 2026-10-23 - RF-15 part 1 (detection v5, contract v3, lossy UI): PR #55 open (awaiting G2)

**Done**
- #52 (RF-14) was merged. `just prune-branches` found nothing to delete.
- **Scope (owner):** RF-15 is split into two PRs.
  - Part 1 is `feat/RF-15`, PR #55: detection v5, `[lossy]` params, v5 default, contract v3, the UI, and the `decode-lossy.feature` scenarios RF-14 had deferred.
  - Part 2 is lossy calibration: `calibration/lossy.md`, `calibration-lossy.feature`, `calibration.v2.toml` as the default config, and the `inputs.rs` messages C11.
- PR #55 has 9 commits and `just ci` is green.
  - Tests: 39c0692 and 0445ada.
  - Implementation: e192be6 and 383abba.
  - Review round 2: tests in 361d500, fixes in d6145b0, 384087f and d34fc8e.
- Two review rounds, neither with blocking findings. Round 2's mutation check killed every requested mutation.

**Pending**
- G2 of #55 (the owner merges it). Its "Follow-ups" section lists these items:
  - **Spec (G1):** should L0 come before "decoding failed" for `.mp3` files? The codec is known before the probe.
  - **Spec (G1):** wording of `bitrate.noneLabel`, of "the row tooltip" for `error.unsupportedFormat`, and approx taking precedence over a mode.
  - **Tests:** a direct `detect_track` test for L0 before silence.
  - **Tests:** `run_analysis_lossy_v5.rs` uses a WAV where the scenario says FLAC.
  - **Code:** `unwrap_or(AudioCodec::Pcm)` in `analysis.rs`.
  - **Code:** the ColumnHeader tooltip bubble is right-aligned.
- `/feature RF-15` part 2: lossy calibration.
- The spec follow-ups from the 2026-10-19 block (MP3 shortfall wording, tag frame count 0, free-format, implicit HE-AAC), the installer size against RNF-06, B1–B11 with `just accept`, the `confirmed.txt` lines and the splitter leftovers: all unchanged.

**Blockers / questions for the human**
- `trusted.txt` was fixed by the owner; `just privacy-check --base main` on #55: 41 files checked, 0 matches.
- Dependabot alert #1 (`glib`), the `specs/CLAUDE.md` line about `specs/pending/` and the splitter accessibility gap: unchanged. The `guard-tests` stub gap was closed by #53.
- **Environment:** in this session, Git Bash prints a spurious "cwd: No such file or directory" and exit 1 after every command. Read the real exit code with `echo $?` inside the command. Subagents saw vitest fail from Git Bash when the cwd was `c:` (lowercase); from PowerShell with `Set-Location C:\Workspaces\FakeFlac` it works.

**Next step:** after #55 is merged: `just prune-branches`, then `/feature RF-15` (part 2, lossy calibration) on a new branch from `main`.

## 2026-10-19 - RF-14 H2 decoding: PR #52 open (awaiting G2)

**Done**
- `/feature RF-14` on `feat/RF-14`, PR #52 (10 commits, `just ci` green). MP3 and M4A (AAC-LC, ALAC) are decoded with symphonia 0.6.1, using the `mp3`, `aac`, `isomp4` and `alac` features. The adapter reads stream information itself in `crates/fakeflac-audio/src/{mp3,mp4}.rs`. Contract v3 adds `codec` and `bitrateMode`. `algorithm_version` stays 4.
- **Scope (owner):** decoding only. These `decode-lossy.feature` scenarios need v5 and are left to `/feature RF-15`:
  - "A lossy row has the same fields…"
  - "A variable bitrate is marked…"
  - the four `@RF-15` presentation scenarios
  - "Unicode names of lossy files"
- The fixture checks of `lossy-fixtures.md` section 3 passed on every fixture.
- Four review rounds, with fixes in `14e752f`, `fdce760`, `c7da1d0` and `9aee8ea`. The corpus probe (read-only, aggregate counts, owner-approved) on a 500-file sample: MP3 413/471 → 471/471, M4A 29/29.
- **Licensing (owner):** the AAC tables copied from symphonia live in the MPL-2.0 file `crates/fakeflac-audio/tests/common/aac_tables.rs`.

**Pending**
- G2 of #52 (owner merges it alone, squash). Until RF-15 lands, `main` judges MP3/M4A by the lossless rules: do not release from `main` in between.
- **Spec follow-ups for G1** (one `spec/` PR):
  - `lossy-source.md` line 82: the MP3 shortfall is `enc_delay + enc_padding` (up to about 2.5 frames), not "up to two frames". The code uses delay + padding + 2 frames, or 3 frames without a LAME/Lavf/Lavc tag.
  - Should a tag frame count of 0 count as absent?
  - Free-format MP3.
  - Implicit HE-AAC (ASC type 2 with SBR) passes as LC.
- **For `/feature RF-15`:**
  - M4A error rows keep the info read before the error (needs a port change).
  - `makeTrack` in `src/lib/testing/fixtures.ts` derives `codec` from `format`.
  - `error.NO_AUDIO_FOUND` text.
  - The ten-column `ResultsTable` and the rest of ADR-0041's protected-test list (v5 items).
- **Installer size against RNF-06:** not measured. ADR-0040 says "before release".
- Unchanged from the block below: real-corpus `just accept` for B1–B11, the `confirmed.txt` lines, the splitter leftovers.

**Blockers / questions for the human**
- The harness gap in `lessons.md` (compile-only stubs inside `test(` commits break `guard-tests`) needs a decision on `test-author.md` or `guard-tests.mjs`.
- Dependabot alert #1 (`glib`), the `specs/CLAUDE.md` line about `specs/pending/`, and the splitter accessibility gap: unchanged.

**Next step:** after #52 is merged, `just prune-branches`, then `/feature RF-15` on a branch from `main`.

## 2026-10-19 - RF-15 spec approved (G1, #49)

**Done**
- `/new-spec RF-15`, PR #49 merged at G1 (56bfb1a), `spec/RF-15` pruned. Owner scope: RF-15 plus RF-14 H2 decoding of MP3 and M4A (AAC; ALAC is lossless). Opus scenarios are parked in `specs/pending/opus.feature`, which neither `just trace` nor the test stage reads, until the ADR-0030 spike.
  - Detection v5 (`specs/detection/lossy-source.md`, `detection-params.v5.toml`): shortfall `s = c_exp − fc`, rules L0–L6, acceptance B1–B11. The v4 lossless values are unchanged; the next free `algorithm_version` is 6.
  - Calibration (`specs/calibration/lossy.md`, `calibration.v2.toml`), contract v3, UI (`specs/ui/lossy-formats.md`).
  - ADR-0040 and ADR-0041 set to `accepted` in this handoff.
- The readiness review had 4 blocking findings and 18 improvements, all applied before the PR.
- No comments at G1, so every default applies (`lossy-source.md` Q1–Q15, `lossy.md` C1–C12).
- PR #50 (`docs/inputs-privacy`, awaiting merge): `specs/calibration/inputs.md` named two corpus albums (ADR-0022); they are now `album F1` / `album F2`. The names stay in git history.

**Pending**
- `/feature RF-14` (H2 decoding, `decode-lossy.feature`), then `/feature RF-15` (`detection-lossy.feature`, `calibration-lossy.feature`). `/feature` takes one requirement and RF-15 depends on the decoders.
  - The test stage first confirms the fixture byte arithmetic (`lossy-fixtures.md` section 3) and the symphonia 0.6.1 feature names (ADR-0040). If either fails, go back to G1.
  - Protected tests that change are listed in ADR-0041 "Consequences".
- After implementation: real-corpus `just accept` for B1–B11. The MP3 and AAC cutoff tables are indicative (Q2): the MP3 rows were written from memory and differ from `calibration.v1.toml` (cbr128 17000 vs 16000).
- Uncomment the 20 RF-15 `fake` lines in `confirmed.txt` only after `/feature RF-15` lands the new label grammar.
- The splitter and v3 review leftovers from the block below, unchanged.

**Blockers / questions for the human**
- Dependabot alert #1 (open, medium): `glib` < 0.20.0 (`VariantStrIter` unsoundness) in `Cargo.lock`. It is a transitive Tauri/GTK dependency, probably Linux-only; not looked at.
- Optional, from RF-15 Q9: a line in `specs/CLAUDE.md` saying `specs/pending/` holds parked features (agents do not edit CLAUDE.md).
- Accessibility gap of the splitter and Q9–Q11 in `rules.md`, unchanged.

**Next step:** `/feature RF-14` (H2 decoding of MP3 and M4A).

## 2026-10-19 - H1 UI: MainLayout splitter merged (#46)

**Done**
- #44 merged at G2 (ca69bb0). `prune-branches` had nothing left to delete, and `feat/RF-04-v3-backup` is gone.
- `/feature RF-07` (splitter, already in the approved UI specs, so no G1 was needed), PR #46 merged at G2 (689189a). `just prune-branches` done. The H1 UI is complete:
  - test: 33fcabc, 19 tests plus `Splitter.stories.ts`;
  - implement: 5f43050;
  - review fix: 2b6ee69. The blocking finding was that sub-pixel deltas were rounded away at each step. The same commit adds primary button only, ending the drag on blur, and re-clamping after the window shrinks;
  - traceability row: a94a81c.
- Test contract:
  - `role="separator"` with `data-state` `idle` or `dragging`;
  - the delta is incremental per event, positive downward;
  - localStorage key `fakeflac.layout.detailHeight`;
  - a saved value below 160 is raised to 160.
- `just ci` is green (232 frontend tests), and `privacy-check` finds 0 matches.

**Pending**
- Drag the splitter slowly at 150 % / 200 % display scaling; this has not been checked in the app.
- Splitter improvements listed in #46:
  - the minimum heights and the bar height are written both as TS constants and in CSS;
  - the height is saved on every pointer move;
  - there is no test for a saved value below 160;
  - the hover state is not in the spec;
  - the story is minimal.
- Review improvements left over from v3 and the RF-07 known deviations, both unchanged (see the blocks below).

**Blockers / questions for the human**
- Accessibility spec gap, needs G1: a focusable separator should have `aria-valuenow` and an accessible name (prop and i18n key in `components.md`).
- Q9–Q11 in `rules.md`, unchanged.
- Bash tool: `cd /c/Workspaces/FakeFlac` makes vitest fail with "failed to find the runner"; use `cd "C:/Workspaces/FakeFlac"`. Every Bash call ends with exit 1 because of a missing harness cwd file. Read the command's own result, not the exit code.

**Next step:** `/new-spec RF-15` (input in the 2026-10-18 block).

## 2026-10-19 - RF-04 detection v4 implemented (PR #44, awaiting G2)

**Done**
- `feat/RF-04` rebased onto main. The v3 traceability commit became empty and was dropped. The pre-rebase state is kept locally as `feat/RF-04-v3-backup`.
- `/feature RF-04` (v4), PR #44:
  - test: 4b65425 and 97335dc;
  - implement: a3d09aa and e4d63ce;
  - traceability row: a206757.
- Review: no blocking findings. All seven improvements were applied. Owner decision: `grid_version` stays `1 | 2`, and the test uses 2, with no spec change.
- `just ci` is green, and `privacy-check` finds 0 matches.
- Real-corpus `just accept` (v2 vs v4): A1–A8 all PASS, and every figure matches the §7.1 prediction. Aggregates are in the PR body.
  - `primary` `Fake` goes from 4 to 1 (W4).
  - Fake → Suspicious changes: 160.
  - A8: the maximum is cbr320, 1.0 %.
  - v0 detection rate drops 9.9 pp.

**Pending**
- G2 on #44. After the merge, run `just prune-branches`, then delete `feat/RF-04-v3-backup` by hand (it is not merged).
- Review improvements left over from v3 (see the RF-04 v3 block): the full-band per-setting table appears only on split `all`; `calibrate` exits 2 outside a git checkout; a malformed `detection-params.v*.toml` is skipped silently when allocating a version.
- The RF-07 known deviations, the MainLayout splitter (H1 UI) and the RF-15 spec, all unchanged.

**Blockers / questions for the human**
- Open questions in `rules.md`: Q9 (T2/T3 targets and undetectable transcodes), Q10 (`/calibrate` has no feasible point), Q11 (A8 bound 5 %).
- The auto-mode classifier blocks `git rebase --continue` and `--skip`, and the owner had to run them by hand. Writes outside the repo, including the session scratchpad, are blocked by `guard-protected.mjs`. Use `target/` for temporary files.

**Next step:** G2 on #44. After that, the remaining H1 UI (MainLayout splitter) or the RF-15 spec.

## 2026-10-19 - RF-04 Detection v4 spec approved (G1, #42)

**Done**
- Measured analysis of the failed v3 acceptance run (aggregates only): no `(U, B)` setting works; the deepest step's position separates W1–W3 from transcodes and confirmed fakes. Proposal document (git-ignored, local only): `target/rf04/rf04-v3-threshold-proposal.md`.
- PR #42 `spec(RF-04)` merged at G1 (56a15b1): detection v4 = v3 window and values plus `full_band.max_lossy_step_db = 3.0` (no `FullBand` with a step of 3 dB or more in the lossy range, so V4 is never guarded). `detection-params.v4.toml` (default, `algorithm_version` 4); v2/v3 files stay loadable. W4 (FP4) stays Fake (Q7 accepted). Acceptance v4 in `rules.md` §7 (A6 "not worse than v2", A8 gate ≤ 5 % FullBand on protected settings; predictions in §7.1). ADR-0039 accepted (amends ADR-0038).
- Fixture steps measured once before G1 (recorded in `fixtures.md`): `sfb21_gaps` D 6.15 dB at 16050 Hz (Fake again with v4), `shelf19k5_bursts` D 20.8 at 19550, `sfb21_gaps_aa21k` step at 21050 (stays guarded), `hf_bursts` D 0.48 at 20950.
- PR #41 (handoff) merged; merged branches pruned; analysis worktrees removed.

**Pending**
- `/feature RF-04` (v4) on the local branch `feat/RF-04` (v3 implementation, 7 commits, never pushed): rebase onto main (expect conflicts in `specs/traceability.md` row RF-04 — keep main's — and `docs/architecture/backend.md`); test stage updates the protected tests listed in ADR-0039 "Consequences" (incl. `default_params_equal_versioned_file`, `every_analysis_carries_algorithm_version_3`, FP4/`sfb21_gaps` tests, next free version 5) and derives v4 tests; implement `max_lossy_step_db`, `validate()` bounds, `m_full_band_step_db`; update `backend.md` (v4 default); review; `just ci`; then real-corpus `just accept --params …v2.toml --params …v4.toml` for A1–A8 before the PR.
- Review improvements left from the v3 work (see the RF-04 v3 block below): full-band per-setting table only on split `all`; `calibrate` exits 2 outside a git checkout; malformed `detection-params.v*.toml` skipped silently when allocating a version.
- RF-07 known deviations (block below); MainLayout splitter (H1 UI); RF-15 spec input (2026-10-18 block).

**Blockers / questions for the human**
- Open questions in `rules.md` with defaults: Q3, Q5, Q6, Q8 (defaults apply); Q9 (targets T2/T3 counting undetectable transcodes; calibration spec), Q10 (`/calibrate` has no feasible point: 2 detectable cbr128 tune transcodes are Real), Q11 (A8 bound 5 %).

**Next step:** `/feature RF-04` (detection v4), starting from the local branch `feat/RF-04`.

## 2026-10-19 - RF-07 spectrogram merged (#39, #40)

**Done**
- PR #39 `spec(RF-07)` merged at G1 (f540a2c): owner decisions taken while implementing. The Fake detail line has no reason ("Cutoff 16.0 kHz · Fake"; " · <reason>" only when the track has reasons); `Spectrogram.data` documented as a JSON array of numbers; detail-panel i18n keys listed; new state/scenario "The spectrogram cannot be read" (`spectrogram.unavailable`).
- PR #40 `feat(RF-07)` merged at G2 (938d08e): `fakeflac-audio::spectrogram` (STFT from `[spectrogram]` params, dBFS → u8, bounded memory), `get_spectrogram` command, `SpectrogramView` and `DetailPanel` with stories, `AnalysisPage` wiring (stale responses discarded, Error rows request nothing, failed request shows `spectrogram.unavailable`). `just ci` green (213 frontend tests), two review rounds, blocking findings fixed.
- `gh auth setup-git` done: `git push` over HTTPS works now.

**Pending**
- `just prune-branches` after G2 (merged branches; `feat/RF-07-backup` is not merged and must be deleted by hand).
- RF-07 known deviations (listed in #40): 60-minute file takes 2.37 s (budget 2 s; owner accepted, mostly decode); `DISPLAY_KAISER_BETA = 10.0` constant until `SpectrogramParams` gets a beta field (next params version); no `spectrogram` method on the `SpectrumAnalyzer` port; superseded requests not cancelled/debounced; re-clicking the selected row does not retry; MainLayout splitter still pending (H1 UI).
- RF-04: unchanged, see the next block.

**Blockers / questions for the human**
- RF-04 G1 decision (next block).

**Next step:** owner G1 decision on the RF-04 v3 values; meanwhile, the remaining H1 UI (MainLayout splitter) or the RF-15 spec.

## 2026-10-19 - RF-04 Detection v3 implemented; acceptance fails (back to G1)

**Done**
- Branch `feat/RF-04` (local, not pushed; 7 commits 02b9919..7f9ff67): tests from `detection-full-band.feature` and the @RF-04 scenarios of `calibration.feature`, the protected tests listed in ADR-0038, `validate()` (incl. NaN); core v3 (U, B, `is_full_band`, guard on V3/V4, `Default` = v3) and calibration tool support (`[full_band]`, grid v2, next-free proposal version, U/B columns and tables). `just ci` and `just privacy-check` green; two review rounds, no blocking findings. Traceability row updated on that branch.
- Real-corpus `just accept` v2 vs v3 (run 20261019T091918Z, report in `$FAKEFLAC_CALIBRATION/reports/`): **A1, A2, A3 fail; A8 shows the guard firing on many transcodes.** A4, A5 (gate), A7 pass.
  - A1: W1–W3 Suspicious; W4 (FP4) stays Fake (B 2.86 < 3; Q7 case).
  - A2: `confirmed_fake` 8/15 Fake (7 moved to Suspicious). A3: primary 1 Fake (FP4), T1 FAIL.
  - A8: FullBand share of transcodes: cbr128 44 %, cbr192 28 %, cbr256 41 %, cbr320 29 %, v2 60 %, aac128 59 %, opus128 73 %, v0 ~92 %. Fake rate drops 14–74 pp per setting (caught rate unchanged).
  - Cause (thresholds, not code): U median of real tracks ≈ −36 dB (p5 −55); of low-passed transcodes −65 to −75 dB (p95 ≈ −45 to −50), so `min_level_db` −70 sits inside the transcode distribution; B ≥ 3 does not filter it. FP1–FP3 have U −57, −45, −51: no U threshold alone separates them.
  - A6: T2 (4) and T3 (8) fail on split `all` for **v2 too** (holdout passes), so A6's premise does not hold in this run.

**Pending**
- RF-04: G1 decision (below), then a new params version (or a new discriminator) through a spec PR; `feat/RF-04` is rebased on it, re-run `just accept`, then PR (G2). Do not retune in implement (`rules.md` §7).
- Review improvements left on `feat/RF-04` (need spec lines first): full-band per-setting columns only on split `all` (evaluation.md §4 asks per split); `calibrate` exits 2 outside a git checkout even with no proposal; a malformed `detection-params.v*.toml` is skipped silently when allocating the next version.
- RF-15 spec input: unchanged from the 2026-10-18 block.
- RF-07 (spectrogram): spec approved, tests and UI pending.

**Blockers / questions for the human**
- G1 on RF-04: new v3 values (e.g. a much stricter `min_level_db`, measured on this run's `tracks.csv`) or another discriminator; accept FP4 staying Fake (Q7)?; why T2/T3 already fail with v2.

**Next step:** owner G1 decision on the v3 values (RF-04).

## 2026-10-18 - RF-04 Detection v3 spec approved (G1)

**Done**
- PR #35 (RP-11 undetectable transcodes) merged at G2; merged branches pruned.
- PR #36 `spec(RF-04)`: Detection v3 spec merged at G1 (2fd4114).
  - Full-band guard: `U`/`B` over bands in [20.7, 21.0) kHz; V3 and V4 apply only when not `FullBand`, so FP1–FP4 (W1–W4 of the acceptance run) become Suspicious.
  - `specs/detection/detection-params.v3.toml` (optional `[full_band]`; v2 files reproduce v2); `DetectionParams::validate()` in core; ADR-0038 (proposed): v3 becomes the default.
  - Acceptance A1–A8 (`rules.md` §7). Owner decisions: Q1 approve without pre-measuring (A1/A5/A8 decide); Q2 A5 gates the caught rate (Fake + Suspicious) vs the v2 column of the same run.
  - V0 and AAC 256 false negatives documented as known limitations (`rules.md` §6).
  - Protected tests that v3 changes are listed in ADR-0038 "Consequences" (incl. `tools/fakeflac-calibrate/tests/sample_corpus.rs:35–56`).

**Pending**
- `/feature RF-04` (Detection v3): test stage updates the listed protected tests and derives tests from `detection-full-band.feature`; then implement, review, PR (G2).
- After implementation: real-corpus `just accept --params …v2.toml --params …v3.toml` to check A1–A8. If A1 or A5 fails, back to G1 (no retuning in implement).
- RF-15 (lossy-source detection) spec input, from a parallel session (owner labels in `$FAKEFLAC_CALIBRATION/confirmed.txt` lines 18–39; no names or paths in the repo, ADR-022):
  - ~170 owner-ripped YouTube MP3 320 tracks (yt-dlp, `Lavf`). Sample rate tells the stream: 48 kHz ← Opus, 44.1 kHz ← AAC 128. The label is an upper bound (source ≤ stream; chains may have 3+ lossy stages). One bootleg tape compilation is a weak label.
  - 14 `.m4a` are YouTube AAC 128 remuxed without re-encode: honest lossy negatives; the label scheme needs a kind for them (one is mislabelled `fake`).
  - One album has the full chain from one master (trusted FLAC → YouTube AAC 128 → MP3 320); its FLAC holds FP4. The spec should pair tracks across versions.
  - `confirmed.txt`/`trusted.txt` grammar must accept MP3/M4A/Opus; new cascade setting kind (incl. Opus 48 kHz → MP3); consider grouping the split by main artist (several albums share artists).
- Pending UI of H1: RF-07 (spectrogram: `SpectrogramView`, `DetailPanel`). RF-01, RF-05 and RF-06 UI shipped in #27.

**Blockers / questions for the human**
- Open questions in `rules.md` ("Open questions for G1"), defaults apply unless changed: Q3 (new reason for guarded tracks; default no), Q4 (changes to approved specs; default yes), Q5 (V0 has no low-pass; may be checked from the LAME tag low-pass byte), Q6 (v2 caught rates for v0/aac256 from `summary.md`), Q7 (FP4 stays Fake if its upper content is steady; default accept).
- The 20 RF-15 `fake` lines in `confirmed.txt` must stay commented out, or `just accept`/`just calibrate` fail.
- `git push` over HTTPS has no credential helper on this machine; `gh auth setup-git` would fix it (owner's choice).

**Next step:** `/feature RF-04` (Detection v3).

## 2026-10-18 - RP-11 undetectable transcodes and follow-ups implemented (feat/RP-11-undetectable)

**Done**
- Follow-ups from PR #34 implemented:
  - Undetectable transcodes on band-limited sources (`source.fe <= setting.lowpass_hz` where `lowpass_hz < nyquist` or HF drop below `min_removed_hf_db`).
  - Added `undetectable` field to `LabelRow` and `PointRow`.
  - Excluded undetectable transcodes from `must_catch` feasibility and grid cost in `grid.rs` and `run.rs`.
  - Updated confusion matrices and per-setting tables with `Undetectable` column; rate denominators exclude undetectable transcodes; added `### Undetectable transcodes` section to `report.md`.
  - Privacy check writes unreadable files to stderr with exit code 1.
  - `--no-generate` verified leaving `labels.csv` byte-identical.
- All tests pass: 24/24 acceptance tests, 7/7 grid tests, all workspace unit and integration tests.
- `just check` and `just ci` pass with 0 errors, 0 warnings, guard-tests ok.

**Pending**
- Gate G2 review and merge of `feat/RP-11-undetectable` by owner.

**Blockers / questions for the human**
- None.

**Next step:** Owner review and merge of PR for `feat/RP-11-undetectable`.

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
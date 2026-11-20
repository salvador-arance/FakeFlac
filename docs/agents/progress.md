# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

## 2026-11-20 - H3 widened: MDCT fingerprints, competitor benchmark, speech codecs out (`spec/h3-scope-mdct-benchmark`, G1 pending)

**Done**
- The owner widened H3 again and accepts that it takes several months. The plan in #184 is updated.
  - **MDCT quantisation fingerprints move into H3** (until now "after H3", #176). New draft requirement RF-21 and issues #214 (read-only spike), #215 (spec), #216 (core and adapter) and #217 (UI, CSV and calibration).
  - **Benchmark against comparable detectors**: new draft requirement RP-13 and issue #213.
  - **Speech codecs out of the format scope**: AMR (`.amr`, `.3gp`, `.3g2`) and Speex (`.spx`) give no detection value. RF-19 now covers 31 of the 35 extensions, 26 to add. #204 and #210 are updated.
- **Competitors are anonymised in issues** (owner's decision): tool A to tool E. The bodies of #184 and #198 to #210 were rewritten without names, source file names, identifiers or URLs. The mapping is kept outside the repository. New text in the repository uses letters too; the existing requirement texts (RF-19, RF-20, RNF-12, RNF-13) still name that tool, pending the owner's answer.
- This branch: RF-21 and RP-13 (draft), RF-19 scope, a pointer note in RF-15, and three traceability rows. No detection, contract, params or code change.

**Pending**
- Owner, at G1: review and merge this branch's PR. Priorities to confirm: RF-21 and RP-13 medium.
- Owner: whether to anonymise the competitor names already in the repository (requirements and earlier PR descriptions). GitHub keeps the previous issue bodies in each issue's edit history; only a maintainer can delete those revisions.
- Everything in the previous block.

**Next step:** unchanged for the original line (#188, with #186 in parallel). The MDCT spike #214 is read-only and can start now, in parallel with #188 and #203.

## 2026-11-20 - H3 extended: format parity, CLI, website and real-library findings (`spec/h3-extension`, G1 pending)

**Done**
- The owner compared FakeFlac with AudioAuditor 2.0.0, both on a sample of the owner's library and in AudioAuditor's source, and extended H3 beyond RF-12. The plan is in #184 (retitled "Fine detection, format parity, CLI and website"), with a second issue table, the updated graph, order, owner inputs, risks and definition of done.
- New issues:
  - Real-library detection findings, with measurements only (ADR-0022): #198 (one damaged MP3 packet makes the track `corruptStream`), #199 (an `EdgeStep` false positive missed by 0.04 dB of `R`), #200 (a cbr320 transcode just above `fake_max_cutoff_hz`), #203 (spike: per-channel spectra against the `(L+R)/2` mono mix).
  - #201 (source bitrate of fake lossless files).
  - Format parity: tracking #204 (35 extensions), with #206 to #210 by group.
  - #202 (user CLI), #205 (website), #211 (this record).
- This branch records the extension at G1:
  - draft requirements RF-18 (CLI), RF-19 (format parity), RF-20 (source bitrate of lossless files with lossy evidence), RNF-12 (damaged streams) and RNF-13 (website);
  - pointer notes in RF-14, RF-15 and RNF-04, which stay as specified;
  - one traceability row each.
- No detection, contract, params or code change. #199, #200 and #203 need no requirement change: they go through detection specs and calibration as usual.

**Pending**
- Owner, at G1: review and merge this branch's PR (#211). Priorities to confirm: RF-19 high; RF-18, RF-20, RNF-12 and RNF-13 medium.
- Release split: one release for all of H3, or detection and RF-12 first, then formats, CLI and website (#184, risks).
- Everything in the previous block and in #184.

**Blockers / questions for the human**
- The owner inputs of #184, items 3 and 7 to 9: #211 at G1, the open questions of #198, #201, #202 and #205, and the per-codec decisions of #209 and #210 once their surveys exist.

**Next step:** unchanged for the original line (#188, with #186 in parallel). Read-only work of the extension that can start now: the spike #203, the surveys of #209 and #210, and the measurements of #200.

The planning document (section 1.9) is historical and is not edited. For H3, #184 and this file supersede it.

## 2026-11-20 - H3 planned as issues (#184 to #196)

**Done**
- H3 is split into GitHub issues: the tracking issue #184 with sub-issues #185 to #196. #184 holds the dependency table and graph, the critical path, the owner inputs in order, the risks and the definition of done. No repo change other than this block.
- Exit criteria (planning section 1.9, decisions of #176): RF-12 `verified`; at most 1 % of authentic lossless corpus tracks `Fake` on the default params, with population and counting rule fixed at G1 (#186); v0.4.0 published.
- Order: #185 (H2 carry-overs), #186 (false-positive target, then v8 measured against it) and #188 (read-only RF-12 spike) can start now, in parallel. Critical path: #188 → #189 (RF-12 spec, detection v9) → #192 (core and adapter) → #193 / #194 → #195 (acceptance, census against `20261119T220711Z`) → #196 (release 0.4.0, milestone close). #187 (false-positive reduction) only has work if v8 fails the target of #186.

**Pending**
- Everything in #184. The carry-overs of the previous block (#183 at G1, the `WM_CLOSE` finding, `just prune-branches`) are now tracked in #185.

**Blockers / questions for the human**
- The decisions of #186 (population, unit, statistic, split, lifetime of the gate).
- The verdict of an upsample (#189, first open question): it decides how a label-upsampled release counts in #186.

**Next step:** #188 (spike: census aggregates by sample rate and bit depth, hi-res negatives in the labelled corpus, LSB and sample-rate signals). #186 can be specified in parallel. `/new-spec RF-12` waits for both.

## 2026-11-20 - RNF-10 verified, H2 closed (#158)

**Done**
- #182 merged. M1 passed: the owner recorded it in `look-and-feel.md` 4.1 within #182. K1 to K3 hold, so RNF-10 is verified.
- PR #183 (`spec/RNF-10-m1`) brings the `close` row note in 2.2, the K2 sentence and the traceability row up to date after the merge.
- #158 is closed, with a comment mapping each step to its PRs. Every item of its definition of done holds, and H2 is closed.

**Pending**
- Owner, at G1: review and merge #183.
- Carried over:
  - The minimized-window WM_CLOSE finding from RNF-03 M1: the owner decides whether it becomes an issue.
  - `just prune-branches`: `spec/RNF-03-privacy` and `feat/RNF-10` are still local.

**Blockers / questions for the human**
- None.

**Next step:** H3 begins with `/new-spec RF-12` (upsamples). It is the only H3 requirement. The decisions it needs are in the 2026-11-19 block (#176): the false-positive target is at most 1 %, and the census baseline is `20261119T220711Z`.

## 2026-11-20 - RNF-10 look and feel implemented, M1 passed (`feat/RNF-10`, G2 pending)

**Done**
- RNF-03 closed: #181 merged on 2026-11-20.
- `/feature RNF-10` on `feat/RNF-10`, PR #182:
  - `9f09ebd` test: S1 and S2 in `src/App.lookAndFeel.test.ts`, S3 in `Toolbar.lookAndFeel.test.ts`, S4 to S6 in `scripts/look-and-feel.test.mjs`. S1 to S6 already held. An extra `close` check (no shipped source draws the Bootstrap `x-lg` glyph, from the register note and K2) was the only red test.
  - `285f036` feat: the `close` path in `Icon.svelte` (both branches) is redrawn as an original filled ×.
  - `6ae120f` docs: RNF-10 row in `traceability.md`.
- Review: no blocking findings. `just ci` green locally.
- M1 passed on 2026-11-20 on the branch release build, recorded in `look-and-feel.md` 4.1. A desktop agent covered E1 to E6 and the owner E7, and every element is pass. The agent could not read the tooltip text or the end caps of the new `close`; both are covered by the owner's pass. K1 to K3 hold.

**Pending**
- Owner, at G2: review and merge #182.
- After merge, in a `spec/…` PR: refresh the `close` row note in `look-and-feel.md` 2.2 and the K2 sentence. Both still say the path is `x-lg` until the PR merges. 2.2 allows changing a row only in a spec PR.
- Unchanged from the previous block:
  - The minimized-window WM_CLOSE finding (owner decides on an issue).
  - Close #158 once RNF-10 is merged.
  - The optional items.
  - Worktree and branch cleanup (`just prune-branches`; `spec/RNF-03-privacy` is still local).

**Blockers / questions for the human**
- None.

**Next step:** G2 of #182, then the `spec/…` PR for the 2.2 and K2 notes and the close-out of #158.

## 2026-11-20 - RNF-03 privacy lint implemented (`feat/RNF-03`, G2 pending)

**Done**
- The owner answered the open G1 questions on 2026-11-20 and the answers are merged: RNF-03 Q1 to Q10 (#178), RNF-10 Q1 to Q7 (#179), and RP-02/07/08 (#180).
- `/feature RNF-03` on `feat/RNF-03`, in four commits:
  - `3b79001` test: `scripts/lint-network.test.mjs`, 80 tests.
  - `55f467f` feat: new `scripts/lint-network.mjs` with rules T1 to T3, R1, R2, R4, F1 and F2; a call from `lint-arch.mjs`; the 30 NET crates in `deny.toml` `[bans] deny`.
  - `1aa6840` docs: "Network (RNF-03)" section in `dependencies.md`, and in `overview.md` the RNF-03 row plus the Q10 sentence.
  - `ec24ffd` fix: review improvements. A checker that throws now fails closed; `window . open (` is caught; URL matches stop at `;`, `,` and `}`.
- Review: no blocking findings. The repository already meets every rule, so no production code changed (D1, P5).
- R3 precondition: on 2026-11-20 at `bb6db23`, with tauri 2.12.6, `cargo tree --workspace --target x86_64-pc-windows-msvc -e normal,build,dev -i` gives "nothing to print" for `reqwest`, `hyper` and `hyper-util`. The output is in the PR.
- `just ci` green locally. `specs/traceability.md` RNF-03 row updated. PR #181 opened.
- M1 passed on 2026-11-20 on the installed 0.3.0, recorded in `privacy.md` 6.1:
  - Offline: every step worked.
  - Online: `fakeflac.exe` had 0 remote addresses.
  - Finding (Q7): WebView2 keeps a TLS connection to Microsoft (52.98.200.210:443) open for the whole session, and briefly used 150.171.27.11:443. Restricting that would be a spec change.

**Pending**
- Owner, at G2: review and merge #181. RNF-03 closes with it, since P1 to P5 are green and P6 is passed. CI's `supply-chain` job is the first real `cargo deny check` with the bans, since cargo-deny is not installed locally.
- Side finding from M1, not RNF-03: a minimized FakeFlac window did not exit on `CloseMainWindow()` (WM_CLOSE) and had to be force-closed. The cause the agent gave (tao 0.37.1 hidden event window) is unverified. The owner decides whether it becomes an issue.
- `/feature RNF-10`: its G1 answers are merged (#179). It needs tests for S1 to S6 and a redrawn `close` icon.
- Unchanged from the previous block: close #158, the optional items, and the worktree cleanup.

**Blockers / questions for the human**
- None.

**Next step:** after G2 of the RNF-03 PR, `/feature RNF-10`.

## 2026-11-19 - #158 step 8: the two cases measured, no rule change proposed

**Done**
- Both cases were measured on the existing per-track report of the labelled corpus, `20261114T153457Z` (`just accept` at tool commit `701336f`, defaults: calibration v5 and detection v8, 49,443 rows, read-only). No new `just accept` run, so nothing was written under `FAKEFLAC_CALIBRATION`. The report predates #137, #145 and #161, but none of them changes detection, and the census at the 0.3.0 commit (previous block) reproduces the totals of the earlier one exactly. The scripts stayed under the ignored `target/`; the corpus has no path or name in this diff (ADR-022).
- **The 17 kHz AAC edge: no rule change; it cannot be measured on the labelled corpus.**
  - The labelled corpus holds no AAC at or near 320 kbps. Its AAC rows are 14 real corpus files (all 128 kbps, cutoff 15,750 to 16,650 Hz, all `real`), 2,052 generated `aac128` and 2,052 generated `aac256` transcodes of lossless (`real`), and 2,052 generated 128 kbps MP3 to AAC 256 chains (fake, margin of `cutoffBelowDeclared`). The highest declared bitrate among them is 280 kbps.
  - So an exemption such as "AAC declaring 320 kbps with a cutoff in [17,000, 17,250) Hz is not suspicious" would touch 0 labelled rows: the E6 gate cannot be broken by it, and it cannot be shown to be right either. The only labelled cutoffs in or near that edge are the native ffmpeg encoder at 128 kbps (median 17,350 Hz, 1 row in [17,000, 17,250)) and the MP3 to AAC chain (median 16,650 Hz, none in the edge).
  - What would settle it: a labelled sample of the 133 census tracks, i.e. the owner knows which of them are the platform's own 320 kbps encoding (`real` lines in `confirmed.txt`, `lossy.md` section 1.3) and which are transcodes of a lower bitrate. Without that truth, a rule would be fitted blind.
- **The V3-only fakes: no rule change.**
  - In the labelled lossless set, 0 of the 2,052 authentic corpus tracks is decided by V3 (`gapStep`) alone; 4 of them reach the gap threshold and all 4 are guarded by `FullBand`. The 17 authentic tracks that are `Fake` (nine albums: 15 of the `reviewed` class, 2 of `reference`, 0 of `primary` and `clean`) are all decided by V4 (`cutoffStep`), with margins from 0.1 to 7.7 dB.
  - The 419 V3-only `Fake` tracks are all true fakes: 7 of the 15 confirmed corpus fakes and 412 generated ones (MP3 V0 208, V2 103, CBR 128 to 320: 26, 38, 34, 3). 35 of the 419 are within 3 dB of `gap_fake_db` (12 under 1 dB); no corpus negative sits there. Raising `gap_fake_db` to move those would lose true fakes and remove no false positive from the corpus.
  - The case the census cannot see is an authentic release whose own master carries lossy traces (the 3 database-verified rips that the second-agent labelling found marked `Fake`). By `rules.md` section 6 and `calibration.md` OQ-32 that is the `lossy-master` class, where `Fake` reports what is in the audio and is not a false positive. `confirmed.txt` has no `lossy-master` line yet (0 of them), so those rips are not measured by the report; the owner adds them with the proof of `inputs.md` section 2.1 if they want them counted.
- **For the false-positive target of step 7:** on this report the `reviewed` class has 15 `Fake` of 626 authentic lossless tracks (2.4 %), above the 1 % the owner chose; `primary` and `clean` are at 0 %. H3's first spec must define the population of that target before using it as an exit criterion. The primary figure is 0/78 tracks from 6 albums (95 % CI 0.0 to 4.7 %, small sample).

**Pending**
- Owner, at G2 of this PR: confirm the two outcomes (no rule change now), or ask for the labelled sample that would let the AAC edge be measured.
- Optional, owner's call: `lossy-master` lines in `confirmed.txt` for the 3 verified rips, and a few `real` lines for streaming-platform AAC tracks the owner knows to be genuine; then `just accept` would measure both.
- #158 can be closed: steps 1 to 8 have an outcome. Its checklist is not ticked here (public issue).
- `/feature RNF-03` (small: a lint module, tests and `deny.toml` bans; Q1 to Q10 of `privacy.md`) and `/feature RNF-10` (Q3 icon origins and Q6 accent first).
- Optional: `agent-process.md` Q11 (`guard-bash` rules for `gh api` merges and `git -C … push … main`); skipping `check-app` in the `verify` job of `release.yml`; `crates/fakeflac-core/CLAUDE.md` line 4 "(dev-dependencies: ADR-0051)" then `just agents-sync`.
- Cleanup on the owner's machine: the folders `.claude/worktrees/next-steps-github-issues-e67b04`, `calibrate-62fdf7`, `h2-closeout-158` and `priceless-germain-0d699b`.

**Blockers / questions for the human**
- Whether to accept "no change" for both cases, and whether the labelled samples above are worth adding.

**Next step:** close #158 once this PR is merged, then `/feature RNF-03`.

## 2026-11-19 - #158 step 7: decisions for H3 and the census baseline

**Done**
- The owner decided the five points of #158 step 7 (no spec changes; H3's first spec starts from these):
  - **False-positive target:** at most 1 % of the authentic lossless tracks of the labelled corpus (the one `just accept` uses) get a `Fake` verdict, measured with the same E6 gate. The corpus population and the exact counting rule are for H3's first spec to define at G1.
  - **Census baseline:** run `20261119T220711Z`, `just census` on the owner's library at `e055d50` (`origin/main`, code of v0.3.0 plus docs), detection 8, default params, 647.6 s, read-only. Aggregates only; per-track files stay under the ignored `target/census/` (ADR-022).
  - **Scope of H3:** RF-12 (upsamples) only. The MDCT double-compression fingerprints stay after H3, with their own feasibility spike (ADR-0043).
  - **Carried-over detection candidates** (encoder-specific AAC cutoffs for the 17 kHz edge, L6a on 48 kHz files): decided after step 8, with its result. A proposed rule goes to G1 before H3; otherwise the reason for no change is recorded here.
  - **Windows 10:** stays as it is, a stated target not verified at runtime (decision of 2026-11-02 confirmed).
- The baseline equals the census of run `20261119T155033Z` (main at `91e90cb`), as expected: the detection and the extraction did not change in the version bump. 11,420 tracks, 904 albums, 150,718.4 MB; 9,027 real, 1,173 suspicious, 1,156 fake, 64 error (55 FLAC and 9 MP3 `corruptStream`). By codec: FLAC 4,268 (3,382 / 336 / 495 / 55), PCM 30 (23 / 1 / 6 / 0), MP3 6,946 (5,579 / 703 / 655 / 9), AAC 162 (29 / 133 / 0 / 0), Opus 14 (all real). Lossless fakes by evidence: `gapStep` 95, `cutoffStep` 188, both 218. Albums with exactly one `Fake` among two or more analysed tracks: 92.

**Pending**
- #158 step 8: `just accept` on the labelled corpus (`FAKEFLAC_CORPUS`) for the 17 kHz AAC edge (133 `suspicious` AAC tracks, all 320 kbps, `cutoffBelowDeclared`) and the 95 V3-only fakes, to end in a rule proposal at G1 or a recorded reason for no change. The owner has to say how the corpus is launched.
- `/feature RNF-03` (small: a lint module, tests and `deny.toml` bans; Q1 to Q10 of `privacy.md`) and `/feature RNF-10` (Q3 icon origins and Q6 accent first).
- Optional: `agent-process.md` Q11 (`guard-bash` rules for `gh api` merges and `git -C … push … main`); skipping `check-app` in the `verify` job of `release.yml`; `crates/fakeflac-core/CLAUDE.md` line 4 "(dev-dependencies: ADR-0051)" then `just agents-sync`.
- Cleanup on the owner's machine: the folders `.claude/worktrees/next-steps-github-issues-e67b04`, `calibrate-62fdf7` and `h2-closeout-158`.
- #158 itself: its checklist (steps 1 to 7) can be ticked and the issue closed when step 8 has its outcome.

**Blockers / questions for the human**
- How the labelled corpus is launched for `just accept` (step 8).

**Next step:** step 8 with `just accept`, or `/feature RNF-03`.

## 2026-11-19 - Release 0.3.0 published (#158 step 6)

**Done**
- The owner published v0.3.0 on 2026-11-19 (M7), after M3 to M6. The tag is on `6cb702c`, the merge commit of #170.
- The first tag (on `e0e3cc2`, #169) had its `verify` job cancelled at the 40-minute limit (v0.2.0's `verify` took 38 min). #170 raised the limit to 60 minutes; the owner deleted the old tag (no draft or release existed) and tagged `6cb702c`. The Release workflow (run 44601724510) succeeded and left a draft with three assets.
- M2 (RNF-06), from the draft's assets: `FakeFlac_0.3.0_x64-setup.exe` 2,937,336 bytes (2.94 MB) and `FakeFlac_0.3.0_x64_en-US.msi` 4,079,616 bytes, both far below the 20,000,000-byte limit; `SHA256SUMS.txt` 190 bytes. Against v0.2.0 the installers grew by 65,246 and 98,304 bytes.
- M3 to M6 on a clean Windows 11 (Windows Sandbox and the owner's machine), run by an agent with computer use and reported to the owner:
  - M4: `Get-FileHash` of both installers equals `SHA256SUMS.txt` (setup.exe `a672f048…`, msi `7b191246…`). Both `gh attestation verify … --repo salvador-arance/FakeFlac` exit 0; one attestation covers both installers (`release.yml@refs/tags/v0.3.0`, `6cb702c`). This confirms the `publish` job's permissions are enough for `attest-build-provenance@v4`.
  - M3: SmartScreen warned (unknown publisher; More info, Run anyway); the NSIS installer put the app in `%LOCALAPPDATA%\FakeFlac` with its uninstall entry in HKCU and asked for no administrator rights. The app started from the Start menu and analysed a small folder: two tracks `suspicious` and a 1 s file reported as too short to analyse.
  - M5: uninstall from Settings completed; the three test WAV files kept their size, date and SHA-256; the install folder, shortcuts and uninstall entry are gone. The uninstaller offered "Delete the application data" (unticked); the owner left it unticked and `io.github.fakeflac.app` was kept.
  - M6: the MSI installed, the app started and the uninstall completed; the test files were unchanged. The report said it passed "after starting a service by hand"; the owner confirmed that service was a Windows Sandbox matter, not a defect of the MSI or the app.
- M7: the draft's notes have the section 3.5 text first and GitHub's generated list after it (46 PRs, #119 to #170, from the `v0.2.0` tag), so the `--notes-file` + `--generate-notes` ordering of `release.md` 3.6 is confirmed by a real draft. Three assets.
- Paired `just perf` (in #169, same machine, base v0.2.0 against head): no gated median above its base by 10 %; T1 off 175 to 169 ms, T2 off 650 to 537 ms. No `just memory` report: #169 touched no `crates/fakeflac-audio/src/` file.

**Pending**
- #158 step 7, the owner's decisions for H3: the false-positive target, the census baseline at the 0.3.0 commit (the detection and the extraction did not change in the version bump, so the aggregates of run `20261119T155033Z` should be equal), the H3 scope, the carried-over detection candidates, and Windows 10.
- #158 step 8: `just accept` on the labelled corpus (`FAKEFLAC_CORPUS`) for the 17 kHz AAC edge and the V3-only fakes, to end in a rule proposal at G1 or a recorded reason for no change (the census aggregates are in the block of "#158 step 1").
- `/feature RNF-03` (small: a lint module, tests and `deny.toml` bans; Q1 to Q10 of `privacy.md`) and `/feature RNF-10` (Q3 icon origins and Q6 accent first).
- Optional: `agent-process.md` Q11 (`guard-bash` rules for `gh api` merges and `git -C … push … main`); skipping `check-app` in the `verify` job of `release.yml` (it ran for 38 min in v0.2.0 and cancelled at 40 in the first 0.3.0 tag); `crates/fakeflac-core/CLAUDE.md` line 4 "(dev-dependencies: ADR-0051)" then `just agents-sync`.
- Cleanup on the owner's machine: the folders `.claude/worktrees/next-steps-github-issues-e67b04`, `calibrate-62fdf7` and `h2-closeout-158`.

**Blockers / questions for the human**
- The step 7 decisions above.

**Next step:** the owner answers step 7, then step 8 with `just accept`; or `/feature RNF-03`.

## 2026-11-19 - #158 steps 4 and 5: RNF-03 and RNF-10 specified, requirement statuses

**Done**
- #165 (RF-17 guard tests, closes #146 and #147) is merged at G2. ADR-0051 is `accepted` here (ADR file and `docs/adr/README.md`).
- Step 4, merged at G1: RNF-03 (#166, `contracts/privacy.md`: Tauri surface, Rust dependencies, sources, frontend, owner check M1; `reqwest`, `hyper` and `hyper-util` checked absent from the Windows graph) and RNF-10 (#167, `ui/look-and-feel.md`: element map, asset register, M1). Both are `specified`; their `/feature` rounds are still to do.
- Step 5, this PR: RNF-04, RNF-05 and RNF-09 are `specified`. The owner chose option A for RP-01 to RP-10: `contracts/agent-process.md` and `features/agent-process.feature` give RP-02, RP-05, RP-07 and RP-08 the criterion that exists today (what enforces each, owner checks with a milestone-close cadence, the gaps). RP-01 to RP-08 and RP-10 are `specified` (Q14 decided by the owner: they stay). The owner removed RP-09 (the agent usage budget is not a project requirement): `requirements/RP-09.md` is deleted, nothing represents it. `just check-fast` took 24 s on 2026-11-19 (warm, one run).

**Pending**
- G1 on this PR. The questions of `agent-process.md` section 6 have defaults; the owner has answered Q14.
- Open G1 questions of the merged specs, answered by the owner when the `/feature` rounds start: Q1 to Q10 of `privacy.md`; Q1 to Q7 of `look-and-feel.md`, above all Q3 (who drew the 15 icons; the `close` path looks like a third-party MIT glyph) and Q6 (accent color).
- #158 steps 6 to 8: step 6 (release 0.3.0: `just set-version 0.3.0`, paired `just perf` against v0.2.0, README) can start now that #165 is merged; step 7 are the owner's decisions; step 8 needs `just accept` on the labelled corpus for the 17 kHz AAC edge and the V3-only fakes (the census aggregates are in the block below).
- Follow-up from `agent-process.md` Q11 (default yes): `guard-bash` rules for `gh api` merges and `git -C … push … main`, in a separate harness PR.
- Cleanup on the owner's machine: the folders `.claude/worktrees/next-steps-github-issues-e67b04` and `.claude/worktrees/calibrate-62fdf7`, and the worktree folders of this work.

**Blockers / questions for the human**
- Icon origins (look-and-feel Q3).

**Next step:** merge this PR at G1, then release 0.3.0 (step 6), or `/feature RNF-03` (small: a lint module and `deny.toml` bans).

## 2026-11-19 - #158 steps 2 and 3: RF-17 guard tests, M1 reading accepted

**Done**
- Spec PRs #162 (step 1), #163 (#146 and #147, criteria X8 and X9) and #164 (step 3) are merged at G1. Step 3: the owner accepted the 12.7 reading as the final state of F7 (`contracts/memory.md` section 12.8); the RNF-02 row of `specs/traceability.md` says so now.
- `/feature RF-17` for X8 and X9, branch `feat/RF-17-guard-tests`: tests `821f6c1` in `crates/fakeflac-core/tests/fake_evidence.rs` (three tests, about 3 s in the dev profile; they pass on the code as it is, because the spec says no production change), ADR-0051 `df5952e` (`toml` as a dev-dependency of the core's tests, amends ADR-0033; the owner chose this over moving X8 to `fakeflac-audio`). Review: no blocking findings; the reviewer's mutations (a `Fake` path without evidence, a changed `gaps.edge_hz`, a text without "16 kHz") each fail the intended test. `just ci` exit 0. No perf or memory report: the diff touches no `crates/fakeflac-audio/src/` file.
- Library census of 2026-11-19: see the block below.

**Pending**
- G2 on the feature PR. It closes #146 and #147 and takes ADR-0051 from `proposed` to `accepted` (ADR file and `docs/adr/README.md`; ADR-0033 already names it).
- `crates/fakeflac-core/CLAUDE.md` line 4 (and the generated `AGENTS.md`) says "Only `serde` and `thiserror`". It stays true for the shipped core; a human may add "(dev-dependencies: ADR-0051)" and run `just agents-sync` (agent configuration, ADR-0048).
- #158 steps 4 to 8: step 4 (RNF-03, RNF-10 specs) and step 5 (RNF-04, RNF-05, RNF-09 to `specified`; RP-01 to RP-10 need the owner's option) are independent specs; step 6 (0.3.0) comes after this PR; step 7 are the owner's decisions; step 8 needs `just accept` on the corpus.

**Blockers / questions for the human**
- G2 on the feature PR.
- Step 5: RP-01 to RP-10, one short spec pointing at what enforces each, or a status for process requirements.

**Next step:** merge the feature PR at G2, then the `/new-spec` rounds of steps 4 and 5.

## 2026-11-19 - #158 step 1: handoff and traceability

**Done**
- #161 is merged at G2 (`91e90cb`) and #159 is closed, so the "G2 on #161" of the block below is closed. The three RF-14 branches are deleted.
- `specs/traceability.md`: the RNF-01 and RNF-02 rows say "approved at G1 (#149)" instead of "Draft revision for G1"; the RNF-02 row names the reading recorded in #151 (`contracts/memory.md` section 12.7, inconclusive) and that the F7 row has no final state until #158 step 3; the "other RF" row is now the RF-12 row (H3, no spec yet, the only RF without one).
- #158 step 2: the owner delegated the option of #146 and #147; the cheapest of each was taken and is in spec PR #163 (criteria X8 and X9). The tests follow with `/feature RF-17` after G1.
- Census of the whole library (owner's folder, run `20261119T155033Z`, main at `91e90cb`, detection 8, default params, 409 s, read-only). Aggregates only; per-track files stay under the ignored `target/` (ADR-022). 11 420 tracks, 904 albums, 150 718 MB: 9 027 real, 1 173 suspicious, 1 156 fake, 64 error.
  - **The 64 errors are unchanged by #159.** 55 FLAC and 9 MP3 `corruptStream`, no Opus error. The library has 14 Opus tracks and all are `real`. The 64 are therefore the same corrupt files as in the M1 reading of 2026-11-15 (step 3, `contracts/memory.md` section 12.7).
  - **The 17 kHz AAC edge (step 8, first case).** 162 AAC tracks (all 44.1 kHz): 133 `suspicious` and 29 `real`, none `fake`. All 133 have the reason `cutoffBelowDeclared`, declare 320 kbps, have a cutoff in [17 000, 17 250) Hz and lie in 14 albums. No `real` AAC track declares 320 kbps (22 at 128 kbps, 7 at 259 to 288 kbps), and the `real` cutoffs are 15 750 to 18 750 Hz or full band. So the 133 are the whole 320 kbps AAC population, as the 2026-11-14 diagnosis said. Not done: whether an encoder-specific cutoff rule separates them from real low-bitrate transcodes without breaking the E6 gate; that needs `just accept` on the corpus.
  - **V3-only fakes (step 8, second case).** 95 lossless fakes are decided by `gapStep` alone (188 by `cutoffStep` alone, 218 by both). By album stratum: 22 in whole, 38 in mixed, 35 in isolated albums. Gap margin bands (below 1, 1 to 3, 3 to 6, 6 to 12, 12 or more dB): isolated 5, 10, 6, 5, 9; mixed 5, 3, 2, 8, 20; whole 0, 2, 2, 6, 12. 44 of the 95 have a full-band cutoff (21 kHz or more). 15 of the 35 isolated ones are under 3 dB. Nothing in the census says which of them are real transcodes: that needs the labelled corpus (`just accept`), crossed with these bands.
  - This run is not the step 7 baseline: that one is taken at the 0.3.0 commit (the detection and the extraction do not change in a version bump, so the aggregates should be equal).

**Pending**
- #158 steps 3 to 8. Step 3 now has its evidence: the 64 are the corrupt files, so the owner picks "accept the reading" or "repeat M1 on a folder without error tracks". Step 8 needs `just accept` on the labelled corpus to finish both cases (a rule proposal at G1, or a recorded reason for no change).
- Cleanup left on the owner's machine: the folders `.claude/worktrees/next-steps-github-issues-e67b04` (no longer registered in git) and `.claude/worktrees/calibrate-62fdf7`; `just prune-branches` failed on the first while it was in use.

**Blockers / questions for the human**
- G1 on spec PRs #162 (this one) and #163.
- Step 3: which of the two options.

**Next step:** G1 on #162 and #163, then `/feature RF-17` for the two guard tests; in parallel, `just accept` on the corpus for step 8.

## 2026-11-19 - RF-14: Opus start offset (#159), spec merged and feature PR open

**Done**
- #157 is merged (`a11068a`), so the "G2 on #157" of the block below is closed. RP-12 has nothing open.
- Spec PR #160 merged at G1 (`a7f1efb`): `lossy-source.md` sections 2.4.3 and 2.4.4 (`E = max(0, F − S₁)`, R20 and R21 on `G − E`, `L = G − E − pre_skip`), Q43 and Q44, three fixtures in `lossy-fixtures.md`, two scenarios in `opus.feature`, the amendment of ADR-0047 point 5. The owner accepted both defaults on 2026-11-19: a first page below its samples is `E = 0` and not an error, and `EXTRACTION_VERSION` goes to 3.
- Feature PR [#161](https://github.com/salvador-arance/FakeFlac/pull/161) (`feat/RF-14-opus-start-offset`): tests `2fa54f4`, implementation `29ca4f6` (`OpusStream::open` in `crates/fakeflac-audio/src/opus.rs`, `EXTRACTION_VERSION` 3, one comment in `tools/fakeflac-calibrate/src/cache.rs`). Review: no blocking findings; three mutation checks fail the tests as expected. `just ci` exit 0. Paired `just perf` (base `a7f1efb`, head `29ca4f6`): no gated median above its base by 10 %, figures in the PR. `just memory` passes (`opus_1h` 3.5 MB over the baseline, ceiling 32 MiB).
- `specs/traceability.md`: the RF-14 / RF-15 Opus row now records the tests of #161 instead of "tests pending".

**Pending**
- G2 on #161. Then `just prune-branches`.
- #158 step 1 is still open apart from the first bullet above: the RNF-01 and RNF-02 rows of `specs/traceability.md` ("Draft revision for G1") and the "other RF" row (only RF-12 is left). It goes through G1 as a `spec/` PR.
- #158 steps 2 to 8 are unchanged (#146 and #147 need the owner's option; step 3, 4, 5, 6, 7 as listed).
- Not verified: whether #159 explains part of the 64 error tracks of the owner's library (they were taken to be corrupt files). Check it with `just census` after #161 is merged and before the census baseline of #158 steps 7 and 8, and before choosing the M1 option of step 3.
- Worktrees start with an empty `node_modules`: run `npm ci` before `just check` there, or every frontend test file fails with a missing `@testing-library/svelte`.

**Blockers / questions for the human**
- None.

**Next step:** Review #161 at G2, then run `just census` on the library and see whether the Opus errors drop; after that, step 1 of #158 (traceability) or step 8.

## 2026-11-15 - RP-12 closed in PR #157 (branch `feat/RP-12-folder-guard`)

**Done**
- #155 (`test-rust` timeout 30 min) and #156 (the guard spec) are merged on `main` (`275d51d`).
- Guard: tests `845247f`, implementation `cf17bcd` in `tools/fakeflac-calibrate/src/census.rs` (`canonical_report_folder`, `contains_report_folder`): a `FOLDER` that contains `<repository root>/target/census/` is refused with exit 2, an empty stdout, nothing written and a fixed line. Review round 1: no blocking findings; the reviewer ran the real binary on synthetic repositories.
- Everything left open about RP-12 is closed in this PR, with the owner's delegation: spec `5dc6ab3` and `9328467` (four refused cases in section 9, the redirected-`target` clause, C11 cases (a) to (d), "empty means not measured covers error tracks only", `@RNF-07` on the Windows examples); tests `43a605d` (five tests: `..` segment, other letter case, a missing repository root, a junction on `target`, plus the typed `pick` helper); `40c632b` (one empty arm in `probed_field`). Review round 2: no blocking findings.
- `just check` and `just guard-tests` exit 0. `just ci` was not run locally, on the owner's instruction; the CI of the PR runs it. The initial `just check` before the test stage was also skipped.
- RP-12 earlier: census #152, spec clarification #153, numeric blanking and tests #154, MC1 and MC2 passed on the owner's library (recorded in #152).

**Pending**
- G2 on #157 (the spec edits ride in this feat PR by the owner's delegation; keep that in the PR description). Then `just prune-branches`.
- Nothing else is open for RP-12. Optional, not done: the C11 junction cases print a reason and pass when `mklink /J` cannot create the link, so a skip is silent unless the CI log is read; a dangling `target` junction makes the run fail later at `create_dir_all` (nothing is written).

**Blockers / questions for the human**
- None.

**Next step:** Review #157 at G2.

## 2026-11-15 - Report-folder guard specified; CI timeout raised

**Done**
- The census follow-up merged as #154 (`ca67f22`); the merged local branches were pruned.
- Spec PR `spec/RP-12-folder-ancestor-guard`: a `FOLDER` that contains the report folder `<repository root>/target/census/` (the repository root, an ancestor, `target/` or `target/census/`) is refused with exit 2, an empty stdout, nothing written and the fixed line `error: census: the folder contains the report folder (target/census/ of the repository)`. Canonical forms are compared, canonicalizing the deepest part of the report folder that exists. New acceptance case C11, a Scenario Outline in `census.feature` and the traceability row. The stale "tests pending" of the empty-field scenario was corrected (tests landed in #154).
- CI PR [#155](https://github.com/salvador-arance/FakeFlac/pull/155): `test-rust` `timeout-minutes` from 20 to 30 (the `nextest` step takes 12 to 17 minutes on `main`).

**Pending**
- G1 on the spec PR, then a `/feature RP-12` round: the C11 test, then the guard in `tools/fakeflac-calibrate/src/census.rs` (between finding the repository and the run).
- G2 on #155.
- Not specified: the sentence in census.md section 5 saying the "empty means not measured" rule covers error tracks only (pinned by a test today).

**Blockers / questions for the human**
- None. Decision taken for G1 (delegated): the guard canonicalizes the deepest existing part of the report folder, so it also holds on a fresh checkout where `target/census/` does not exist yet. The C11 allowed cases are not tested with other letter case or the `\\?\` prefix.

**Next step:** Review the spec PR at G1.

## 2026-11-15 - RP-12 follow-up: tests and numeric blanking (branch `feat/RP-12-census-empty-fields`)

**Done**
- The spec clarification merged as #153 (`37d06c6`). Decision taken for G1 (delegated by the owner): "empty means not measured" covers error tracks only; a 0 on a track that is not an error is written as `0`.
- Test stage `24496da`: 7 new tests in `tools/fakeflac-calibrate/tests/census.rs` (the new scenario on the sample folder, the rule from given analyses, one blank line after every block of `summary.md`) and the header comment of the file updated. All 42 census tests pass.
- Implement stage `52e3a7b`: the blanking of the five probe columns of `tracks.csv` is decided from the numeric values (`probed_field`) instead of comparing the formatted string with `0`. Output unchanged; mutation checks fail the tests as expected.
- Review: no blocking findings. `just check`, `just guard-tests`, `just ci` and `just privacy-check --base origin/main` exit 0.

**Pending**
- G2 on the PR of this branch.
- Review improvements not applied: the "error tracks only" decision is not written in census.md section 5 (one sentence in a `spec/` PR would make it explicit); the `as` casts of the `pick` helper in the zero-from-probe test; the two equal arms of `probed_field`.
- Not specified: a guard for FOLDER being an ancestor of the repository (D1). Consider raising `timeout-minutes` of `test-rust` in `ci.yml` if it is cancelled again (owner's decision).
- Run `just prune-branches` after the merges.

**Blockers / questions for the human**
- None.

**Next step:** Review the PR at G2.

## 2026-11-15 - RP-12 merged; spec clarification opened for G1

**Done**
- The owner merged the census as #152 (`5404294`). MC1 and MC2 passed (aggregates in a comment on #152). The `census` line is in `CLAUDE.md` and `AGENTS.md` (`cd97640`).
- Spec PR [#153](https://github.com/salvador-arance/FakeFlac/pull/153) (`spec/RP-12-census-clarifications`, `3ecdfce`): statuses `specified` and `approved`, the blank lines of `summary.md` as layout, "empty means not measured" for unprobed error tracks in `tracks.csv`, and a new scenario for it. `just check-fast`, `just trace` and `just privacy-check --base origin/main` pass.
- On #152 the `test-rust` job was cancelled once at its 20-minute limit (the `nextest` step takes 12 to 17 minutes on `main`); a rerun was in progress when it merged. Not investigated further.

**Pending**
- G1 on #153.
- After G1, a new `feat` branch with a test stage: the test for the new scenario in `tools/fakeflac-calibrate/tests/census.rs` (and its header comment about blank lines), then two small code improvements: decide the blanking from the numeric fields instead of the formatted string, with a unit test.
- Not specified: a guard for FOLDER being an ancestor of the repository (D1). Consider raising `timeout-minutes` of `test-rust` in `ci.yml` if it is cancelled again (owner's decision).
- Run `just prune-branches` after the merges.

**Blockers / questions for the human**
- None blocking. The empty-field rule covers error tracks only, as the code does; extending it to every track would be a code change.

**Next step:** Review #153 at G1.

## 2026-11-15 - RP-12 library census implemented (PR `feat/RP-12`)

**Done**
- #148 to #151 are merged on `origin/main` (`3689cd4`); `feat/RP-12` is based on it.
- Test stage `7cd7618`: 35 tests in `tools/fakeflac-calibrate/tests/census.rs` (C1-C10 and every scenario of `census.feature` except MC1/MC2), plus the `shelf19k5` fixture in `tests/common/mod.rs`.
- Implement stage `8091ad7`: new `tools/fakeflac-calibrate/src/census.rs` (report and CSV builders, the run through `analyze_folder`), the `census` command in `run.rs` (accept's params helpers made `pub(crate)`, behaviour unchanged) and the opt-in `census *args` recipe in `justfile`. No new dependency, no ADR, no change to core, adapters, app or specs.
- Review round 1 (no blocking findings) improvements applied in `12fc3af`: the shared USAGE lists `census`; error tracks that could not be probed write empty `sample_rate`, `bit_depth`, `channels`, `declared_bitrate_kbps` and `duration_ms` in `tracks.csv`; `summary_md` is split into one builder per section with byte-identical output. Review round 2: no blocking findings.
- `just check`, `just trace`, `just guard-tests` and `just ci` exit 0 (exit code of `just` itself, not of a pipe). On the synthetic sample the figures match section 8 of `census.md`.

**Pending**
- G2 on the PR (the owner merges).
- MC1 and MC2 passed on 2026-11-15 and are recorded in a comment on the PR (aggregates only): `just census` on the library of 2026-11-14 gave 11 396 tracks (9 003 real, 1 173 suspicious, 1 156 fake, 64 errors), lossless `gapStep` 95, `cutoffStep` 188, both 218, in 345.8 s with no cache; the library's 13 203 files, sizes and modification times were identical before and after.
- Owner's agent-configuration edit, then `just agents-sync`: the `census` line in the `CLAUDE.md` command list.
- Needs a spec line first (G1): a guard for FOLDER being an ancestor of the repository, which would put the report inside FOLDER (D1). Also census.md section 5 / C2 should state that an empty measurement field means "not measured", and the blank lines of `summary.md` (between blocks and at the end, so `elapsed:` is not read as a table row) are unspecified.
- Small round 2 improvements, not applied: a unit test in `census.rs` for the blanking of placeholder fields (the protected test checks only six columns), and deciding the blanking from the numeric fields instead of comparing the formatted string with `0`.
- Spec bookkeeping for a `spec/` PR: `specs/requirements/RP-12.md` still says `status: draft` and "pending G1", and `census.md` says `proposed`.

**Blockers / questions for the human**
- None blocking. A probed `tooShort` track with a 0 ms duration also shows an empty `duration_ms`; the reviewer judged it acceptable.

**Next step:** Review the PR at G2.

## 2026-11-15 - Revised spectrogram reading and agent configuration

**Done**
- Recorded the revised RNF-02 M1 spectrogram reading in PR #151 (`5aa0b35`). In a fresh release session on `faa9fcf`, 11 332 of 11 396 tracks came from the cache and 64 returned errors. The peak increases were zero, and the upper bounds were 49 270 784 private bytes and 44 670 976 working-set bytes. Result: inconclusive. File counts, sizes and modification times matched before and after.
- Opened agent configuration PR #150 (`9f39e0d`) with the memory recipe and review rule, the paired performance comparison, and regenerated Codex files. The census recipe is pending RP-12 implementation.
- `just check` and `just trace` passed on both PR branches. `just agents-check` passed on #150, and the committed diff on #151 passed `just privacy-check --base origin/main`.

**Pending**
- Owner review of #150 and G1 review of #151.
- RP-12 implementation after the census spec merged in #148.

**Blockers / questions for the human**
- The full-library M1 run had 64 cache misses after the preparatory pass. The reading documents that limitation.

**Next step:** Review #151 at G1.

## 2026-11-15 - Census and manual-check specs opened for G1

**Done**
- RF-17 merged as #145 (`13a666c`). The three items left unspecified by the library diagnosis of 2026-11-14 now have spec PRs.
- Spec [PR #148](https://github.com/salvador-arance/FakeFlac/pull/148) (`spec/RP-12-library-census`): new requirement RP-12, `specs/calibration/census.md` (CQ1-CQ9, C1-C10, MC1-MC2) and `specs/features/census.feature`. `just census FOLDER [--params FILE]` is a subcommand of `fakeflac-calibrate` that runs the app's own use case and reports aggregates (verdicts by codec, evidence kinds, album strata, margin bands, suspicion reasons, errors), with a private per-track CSV under `target/census/<run>/`.
- Spec [PR #149](https://github.com/salvador-arance/FakeFlac/pull/149) (`spec/RNF-01-02-manual-checks`): memory.md M1 reads the app's spectrogram in a fresh session as an upper bound (Q10) and records the F7 figures (12.6); performance.md 5.4 replaces the fixed reference with a paired base/head comparison and a 10 % bound (Q15). Specs only.
- `just check`, `just trace` and `just privacy-check --base origin/main` pass on both branches. The two branches merge with each other without conflicts.

**Pending**
- G1 on #148 and #149. After #148: `/feature RP-12`. After #149 nothing is implemented; the owner repeats the spectrogram part of M1 with the revised steps.
- Owner's agent-configuration edits, then `just agents-sync`: the `memory` and `census` recipes in the `CLAUDE.md` command list, and in `.claude/agents/code-reviewer.md` the memory rule of memory.md section 13 and the paired `just perf` rule of performance.md section 14.
- Still measured but not proposed: encoder-specific AAC cutoffs for the 17 kHz platform edge, and the share of legitimate 48 kHz files under L6a.

**Blockers / questions for the human**
- None beyond G1.

**Next step:** Review #148 and #149 at G1.

## 2026-11-15 - RF-17 full-library CSV comparison passed

**Done**
- M2 passed. The 2026-11-14 export was unavailable locally, so rebuilt the same release revision (`2f800f2`) and exported directly from both native apps with isolated cache and WebView2 profiles. Both exports have 11,396 rows and are byte-identical (4,427,958 bytes, SHA-256 `6cc29a063bd825d980e570481f1c28e9c6ff348df029b9f195134a286f7af1d2`). Both apps reported 9,003 real, 1,173 suspicious, 1,156 fake and 64 errors.
- Before the runs, all 13,203 library paths, sizes and modification times matched the November 14 snapshot. The same inventory matched after the two exports. Private CSVs remain under `target/`; the PR draft records aggregates only.

**Pending**
- [PR #145](https://github.com/salvador-arance/FakeFlac/pull/145) is open for the owner's G2 review and merge.

**Next step:** Review PR #145 at G2.

## 2026-11-15 - RF-17 native walkthrough completed

**Done**
- At the owner's request, exercised the native WebView2 release build in English and Spanish on anonymized copies of 35 real library tracks. Verified six selected cases: gap-only with a 22.1 kHz cutoff and a 0.5 dB margin, step-only with a 0.6 dB margin, both lossless conditions, a lossy track with shortfall and step, the only Fake in an album, and an all-Fake album. The mixed albums show their counts, the all-Fake and single-track albums omit the album note, and the audio note explains the provenance limit in both languages. The State cell exposes both evidence items for the two-rule case. This was an engineering walkthrough rather than an unfamiliar user's comprehension test.
- Native MU1 passed at 900 × 500 using the default and 160 px detail heights. The evidence stayed readable without overlapping the graph or status bar. At minimum height in Spanish, the graph remained visible and its frequency labels crowded together.
- The scratch build used the same application code and web assets with test-only window and storage paths. The initial launch used the normal WebView profile before the test was repeated with an explicit scratch WebView2 profile. Language and panel-height changes were made in the isolated session.

**Pending**
- M2 full-library app CSV comparison and the feature PR for G2.

**Next step:** Open the RF-17 feature PR after the owner's go-ahead.

## 2026-11-15 - RF-17 implemented and verified

**Done**
- G1 approved in #139, #143 and #144. Protected tests are in `6a30140`; Rust, generated v9 bindings and cache compatibility are in `35984e1`; the English and Spanish UI is in `34994c7` and `faa9fcf`. Both independent reviews found no issues after the layout adjustment.
- `just check ci`, `just check-app`, `just trace`, `just privacy-check --base origin/main` and `just build` passed. `just perf` passed with T1 medians 165/196 ms and T2 medians 535/546 ms (cache off/cold), within the reference bound. The full performance report is saved under `target/`.
- X3: same local corpus, calibration v5 and detection v8. Both CSV outputs are byte-identical and both Markdown reports match after removing run metadata. The production browser check at 900 × 500 passed in both languages with the detail panel at 260 and 160 px. The Windows release executable and installers are available under `target/release/`.

**Pending**
- Approval to open the feature PR at G2, per `/feature`. The PR body and full performance report are ready under `target/`. Owner walkthrough M1 and real library app CSV comparison M2 remain pending.

**Next step:** Open the RF-17 feature PR when the owner authorizes it.

## 2026-11-15 - RF-17 evidence representation needs G1 clarification

**Done**
- The shortfall margin correction merged in #143 (`bd6353f`). Rebased `feat/RF-17` on it and started the test stage. Baseline probes matched all 25 default rule-level cases and the specified fixture evidence kinds.
- Captured the pre-feature v5/v8 corpus acceptance in repository scratch storage (run `20261115T133313Z`). The original corpus and calibration data were read only.

**Blocker / question for the human**
- The evidence spec §2.4 and contract v9 require finite `f32` fields. The existing validator accepts finite v8 `gap_suspicious_db` and `gap_fake_db` values of `-1e40`. The gap-only FLAC measurement still returns `Fake`, but a direct `f32` conversion produces `-inf` for the threshold and `inf` for the margin. A proposed G1 resolution is to saturate evidence fields to the finite `f32` range after the margin calculation in `f64`. The detector and params validation would retain their existing behavior.

**Pending**
- The test stage is paused with its drafts uncommitted. No implementation or spec changes were made in this stage.

**Next step:** Resolve the evidence conversion rule at G1, then finish the drafted RF-17 tests.

## 2026-11-15 - RF-17 lossy shortfall margin needs G1 clarification

**Done**
- RF-17 spec PR #139 merged at `298138f`. Created `feat/RF-17` from `b645d54`, which includes the merged RP-11 feature PR #142.
- A Rust probe against `decide_track` returned `Fake` for an MP3 with a 2450 Hz shortfall and a `2450.0000000005` Hz threshold. The raw margin remains negative after conversion to `f32` (`-5.002221e-10` Hz).

**Blocker / question for the human**
- The existing L3 comparison allows a shortfall up to `1e-9` Hz below the configured threshold. `specs/detection/fake-evidence.md` sections 2.2 and 2.4 and the v9 contract require both the exact difference and a nonnegative margin while preserving that verdict. A proposed resolution is to keep the configured threshold and measured value and report a zero margin for shortfalls accepted by the existing tolerance.

**Pending**
- RF-17 test, implementation and review stages.

**Next step:** Resolve the shortfall margin rule at G1 before starting RF-17 tests.

## 2026-11-15 - RP-11 lossy-master reporting implemented and checked

**Done**
- G1 correction #141 merged as `3d52977`. On `feat/RP-11`, tests `75050d7` and implementation `22dd47a` cover the `confirmed.txt` kind `lossy-master` and its dedicated §10 report. Errors appear there and in `tracks.csv`; the shared §8 list, acceptance targets and grid results exclude the new class.
- Code review found no issues. `just check`, `just ci`, `just trace` and `just privacy-check --base origin/main` passed. All 13 new integration tests passed. A before/after run on the unmodified v2 synthetic corpus produced byte-identical CSVs and Markdown identical after normalizing run metadata.
- L8 passed on the real corpus with config v5 and detection v8. Release executables from `3d52977` and PR #142 used the same scratch cache and current lists. CSVs were byte-identical and Markdown differed only in run ID and date. Adding the unlabelled verified release left the existing CSV rows and evaluation reports unchanged (including T1) and reported 10 Fake, 3 Real and 2 Suspicious tracks. A second verified release overlapped `trusted.txt` (13 reference tracks). A controlled comparison removed its entries in the scratch baseline before adding both releases and found the same invariance, with 12 Fake, 15 Real and 4 Suspicious tracks across 2 albums. All 31 PCM CRCs matched the database-verified rip logs. The real lists and audio were untouched.

**Pending**
- Implementation PR #142 is open for the owner's G2 review and merge.

**Next step:** Review PR #142 at G2.

## 2026-11-15 - RP-11 tests committed; error reporting needs G1 clarification

**Done**
- Spec PR #140 merged at `c3f1fa5`; baseline `just check` and `just trace` pass. On `feat/RP-11`, `75050d7` adds synthetic tests for the approved scenarios and updates the two approved unknown-kind expectations. The new tests are red as expected except the passing legacy compatibility case.

**Pending**
- Implementation, review, `just ci`, and the L8 real-corpus comparison. The implementation stage stopped before changing production code.

**Blocker / question for the human**
- `specs/calibration/evaluation.md` section 2 requires `lossy_master` errors in the shared section 8 error list. `specs/calibration/sample-corpus.md` section 6 requires every list in sections 3–9 to equal the baseline, and the protected test asserts that those errors are absent from the shared list. Both expect the errors in the dedicated section 10 track list.

**Next step:** Resolve at G1 whether `lossy_master` errors also appear in the shared error list, then align the specs and tests before implementation resumes.

## 2026-11-14 - Library diagnosis; RF-17 and RP-11 lossy-master specs opened for G1

**Done**
- Read-only diagnosis of the F7 run from a copy of the app's result cache and a throwaway probe built on `fakeflac-core` and `fakeflac-audio`; the probe reproduces the cached lossless verdicts exactly (501 fake, 337 suspicious, 3,395 real). Aggregates only here; private lists are in ignored `target/review-2026-11-14/`.
- The 64 F7 track errors are real corruption (55 FLAC, 9 MP3): ffmpeg fails on all 64. The interface already reports them as a corrupt stream. No change needed.
- Lossless fakes: 218 by V3 and V4, 188 by V4 only, 95 by V3 only. 223 lie in 24 albums that are at least 75 % fake; 142 are isolated in 92 albums under 25 % fake. 54 of the isolated ones are within 3-4 dB of a threshold, against 7 of the whole-album ones.
- Threshold trade-off on the latest acceptance `tracks.csv` (run `20261114T153457Z`, offline, nothing written to the calibration directory): `step_fake_db` 12 would move 30 isolated and 1 whole-album fake to suspicious but costs 2.29 pp on `opus128`, above the 0.5 pp E6 gate. Only "12 dB below 17 kHz" stays inside the gate, and it moves 12 tracks. No rule change is proposed.
- Rip-log check: 6 of the 135 albums with a lossless fake have a rip log. 35 fake tracks in 5 albums match the log's copy CRC bit for bit, and 8 of them (2 albums) are database-verified. A fake verdict describes the audio, not the file's history.
- Independent labelling by a second agent (177 of 501 fakes): 3 real (database-verified rips, all V3 only), 0 fake, 174 undecided. The 133 suspicious M4A are AAC 320 from one streaming platform with a uniform 17 kHz edge. Of the 277 `streamingSampleRate` MP3, 149 are probable rips, 83 plausible direct 48 kHz downloads and 45 unresolved.
- Opened spec [PR #139](https://github.com/salvador-arance/FakeFlac/pull/139) (RF-17, explain a Fake verdict: `evidence` field with margins, album note, audio note, contract version 9, no detection version change) and spec [PR #140](https://github.com/salvador-arance/FakeFlac/pull/140) (RP-11 follow-up: kind `lossy-master` in `confirmed.txt`, own class outside T1, report only). The owner decided the separate class on 2026-11-14. `just check`, `just trace` and `just privacy-check --base main` pass on both branches.

**Pending**
- G1 on #139 (Q1-Q16) and #140 (OQ-32 to OQ-39). After each: `/feature RF-17`, `/feature RP-11`. The owner lists the verified releases in `confirmed.txt` only after the tool supports the kind.
- Not specified yet: a repeatable library census recipe, the F7 spectrogram check (lifetime peaks cannot show a 32 MiB increase below the analysis peak) and a fixed performance reference (the last `just perf` was 5-8 % slower on all four medians).
- Encoder-specific AAC cutoffs (lossy-source.md section 10) for the 17 kHz platform edge, and the share of legitimate 48 kHz files under L6a, are measured but not proposed.
- Carried over: the owner's agent-configuration edits from memory.md section 13, then `just agents-sync`. Local branch `docs/RNF-02-g2-handoff` is merged as #138 and can be pruned.

**Blockers / questions for the human**
- None beyond G1. `just trace` reports one requirement pending a scenario on `main` as well; not caused by these PRs.
- This session wrote throwaway scripts and a cache copy to the session's temporary folder outside the repository before the write guard flagged it; later temporaries are under ignored `target/`.

**Next step:** the owner reviews #139 and #140 at G1.

## 2026-11-14 - RNF-02 merged at G2

**Done**
- The owner merged [PR #137](https://github.com/salvador-arance/FakeFlac/pull/137) as `accc3ee`. Updated local main by fast-forward and ran `just prune-branches`, deleting the merged `feat/RNF-02` and `spec/RNF-02-memory-adr` branches.
- RNF-02 is integrated with the two-pass Opus reader, chunk seam, normal memory gate and opt-in `just memory` suite. G1 spec #134 and accepted ADR-0050 (#136) are closed. Independent review, local `just ci`, F6, fresh performance, unchanged cached corpus acceptance and F7 are recorded in #137 and the earlier blocks.
- F7 met both indicative targets on 11,396 tracks. Analysis private/working-set peak increases were 221.457/246.477 MiB against 544 MiB targets. The 60 min 32 s spectrogram raised neither peak. All 13,203 files checked retained sizes and last-write timestamps. The run recorded 64 track errors.
- Updated RNF-02 traceability and architecture wording to record G2. Documentation follow-up branch is `docs/RNF-02-g2-handoff`.
- Fresh `just check` passes on the merged code and this handoff, including all Rust suites and 83 frontend files / 1,430 tests (`target/rnf02-g2-check.txt`).

**Pending**
- Remote CI for the final F7 documentation head and the merge commit was still running at the G2 audit.
- The owner maintains the two agent-configuration edits assigned by memory.md section 13, adding `just memory` to the canonical `CLAUDE.md` command list and the memory-gate/long-report requirements to `.claude/agents/code-reviewer.md`, then running `just agents-sync`.

**Blockers / questions for the human**
- None for RNF-02 implementation or F7. The human-owned configuration follow-up remains assigned as approved at G1.

**Next step:** the owner completes the memory-related agent-configuration edits from memory.md section 13 and runs `just agents-sync`.

## 2026-11-14 - F7 whole-app memory check measured

**Done**
- Desktop control recovered after the owner requested another attempt. Built the current release with `just build` from `2f800f2`, started it fresh, captured P0, cleared its results cache and analysed the complete music folder through the normal UI. Selected the longest track after sorting by descending duration and verified its rendered spectrogram before P2.
- F7 covers 11,396 tracks / 902 albums, including 66 folders with exactly 15 supported tracks, in a folder of 150,714.379 MB including hidden files. Machine has 16 logical processors / 15 workers. Longest analysed track is 60 min 32 s (3,632.639978 s from read-only metadata).
- P0 private/working-set peaks were 8,110,080 / 33,443,840 B. P1 and P2 were both 240,324,608 / 291,893,248 B. Analysis growth is 221.457 / 246.477 MiB against 544 MiB indicative targets. Spectrogram peak growth is 0 / 0 MiB against 32 MiB targets. Both targets are met.
- Run completed with 9,003 real, 1,173 suspicious, 1,156 fake and 64 track errors. These errors were recorded, without a separate diagnosis in this memory check. All 13,203 files including hidden files retained exact paths, sizes and last-write timestamps before/after.
- Aggregate report is `target/f7-report.md`. Private selection details and snapshots remain only in ignored `target/f7-*` files. PR #137 CI on `2f800f2` passed all jobs including `ci-ok` before this documentation update.
- Added the complete aggregate F7 report to [PR #137](https://github.com/salvador-arance/FakeFlac/pull/137). Fresh `just check` passes after the measurement, including every Rust suite and 83 frontend files / 1,430 tests (`target/f7-check.txt`).

**Pending**
- Final-head CI after this F7 documentation update, human G2 review/merge and the human-owned agent-configuration edits from memory.md section 13 remain pending.

**Blockers / questions for the human**
- None for F7. The desktop-control blocker is resolved and both indicative targets are met.

**Next step:** inspect final-head CI on #137, then the owner reviews G2.

## 2026-11-14 - RNF-02 feature PR #137 opened

**Done**
- Confirmed PR #136 is merged with all final-head CI checks green; ADR-0050 is accepted. Created `feat/RNF-02` from up-to-date main. Baseline full `just check` passed (all Rust suites; 83 frontend files / 1,430 tests).
- Fresh test author committed `404dbd7`: serial gate/long drivers, synthetic length-parametrized fixtures, child peak protocol, portable chunk checks, pure checks, report and recipe wiring tests. The test-stage preflight verified all fixture metadata, file growth and outcomes in 48.497 s; only Opus peak growth failed (+2,224,128 B private / +2,166,784 B working set).
- Implementation stopped when full workspace compilation exposed a shared fixture helper referencing a module absent from calibration tests. The owner explicitly authorized a mechanical test-stage repair; a fresh test author committed `5060a3e`, passing unchanged Gaussian noise through a callback. No assertions/bounds changed; the 30 s Opus fixture remains byte-identical (SHA256 `4DAF103B7FEBDAB62CD57758A3C19EC225A2CA6436BD4A42E084603736CEA235`).
- A fresh implementer verified and committed `bbc98ea`: the two-pass Opus reader, one-pass chunk seam and `just memory` recipe. Full `just check` passes (all Rust suites, memory gate 7 passed / 2 ignored in 60.06 s, 291 hook/script tests, 83 frontend files / 1,430 tests). The first implemented run reduced Opus growth to 36,864 B private / 16,384 B working set. All 21 existing Opus tests pass unchanged. Reports are under `target/rnf02-*.txt`.
- Before/after probes, spectrograms, errors and raw Opus sample bits match over 234 cached synthetic fixtures; both fingerprint reports have SHA256 `354CC44D90C200440D8E40A09FB6347B1E7AFDCA106D892A60E2A7B4803C729B` (`target/rnf02-equivalence*`).
- F6 `just memory` passes in 166.18 s excluding compilation: one-hour FLAC/Opus and 24-bit/96 kHz FLAC meet all bounds. Hi-res peaks over baseline are 7,327,744 B private / 7,086,080 B working set, confirming the provisional 64 MiB ceiling. Full report: `target/rnf02-memory-long-report.md`.
- Fresh `just perf` passes: T1 off/cold medians 170/198 ms and T2 off/cold 563/594 ms; differences from the current section 10.1 reference are +4.9%/+6.5%/+8.5%/+5.3%, below the 20% regression threshold. Full report: `target/rnf02-perf-report.md`.
- Independent read-only review found no blocking or improvement findings, verified report/hash evidence and independently passed `just guard-tests`, `just lint-arch` and `git diff --check`. No implementation changes followed review.
- Final `just ci` passes, including all workspace/frontend tests, traceability, protected-test/version checks, generated bindings, Tauri clippy and Tauri tests (`target/rnf02-ci.txt`). Stage is clear.
- With the owner's explicit authorization for the external calibration outputs, ran `just accept` on `701336f` (run `20261114T153457Z`). Every summary line matches baseline `20261114T130952Z` after excluding run id/date/tool commit. Primary 0/78 Fake and 0/6 albums; T1/T3 pass and the previously accepted T2 misses remain 3 all / 1 holdout. Cache: 49,443 hits, zero decoded/transcoded/failures/skipped/errors. Decoder equivalence is covered separately by the synthetic comparison. All 12,411 corpus files checked retain size/mtime; no additions/removals during the run.
- With the owner's explicit permission, published `feat/RNF-02` and opened [PR #137](https://github.com/salvador-arance/FakeFlac/pull/137), with complete F6 and fresh performance reports, independent review and acceptance evidence. The owner retains G2. Privacy check before publication: 11 changed files, zero matches.

**Pending**
- Final-head remote CI on #137, then human G2 review/merge. F7 remains the owner's manual whole-app check; record its indicative aggregates in the PR or later. Local reports and PR text remain under `target/rnf02-*`.
- Architecture wording is updated per memory.md section 13. Human-owned command-list and reviewer-checklist edits remain due, followed by `just agents-sync`. No spec, threshold, dependency or detection/extraction version change is proposed.

**Blockers / questions for the human**
- None for implementation. The protected-helper and external-output authorization questions are resolved. Remote CI and human G2 remain pending; the branch has not been merged.

**Next step:** inspect final-head CI on #137, then the owner performs G2 review/merge.

## 2026-11-14 - RNF-02 ADR approved at G1

**Done**
- The owner approved ADR-0050 in chat and explicitly authorized the agent to merge [PR #136](https://github.com/salvador-arance/FakeFlac/pull/136) after CI passes. This authorization applies to #136; the human merge rule remains the default for future PRs.
- Marked ADR-0050 `accepted` in the ADR and its index, and recorded G1 in the architecture overview. The approved RNF-02 spec (#134) and its bounds are unchanged.
- Full `just check` passed before this documentation-only G1 record (83 frontend files / 1,430 tests and all Rust suites).

**Pending**
- Integrate #136 after its final-head CI passes, then `/feature RNF-02`: tests, the two-pass Opus reader, the chunk seam, the memory harness and recipe, independent review, and `just ci`.
- F6 long-suite and fresh `just perf` reports for the feature PR; F7 manual whole-app check.
- Architecture wording and human-owned agent configuration edits at the times assigned by memory.md section 13. Earlier throwaway measurement build folders remain.

**Blockers / questions for the human**
- None. G1 is closed; integration waits for CI.

**Next step:** integrate #136 when CI is green, then `/feature RNF-02`.

## 2026-11-14 - RNF-02 memory infrastructure ADR proposed for G1

**Done**
- `86c5e6e` proposes [ADR-0050](../adr/0050-memory-budget-os-peaks-std-only-gated-in-ci.md), as required by memory.md Q9: OS peaks per child through PowerShell, a std-only CI gate, an opt-in `just memory` long suite, the memory-only extension of ADR-0046 and ADR-0019, rejected alternatives and explicit shared-runner failure/flake policy.
- Linked the proposal from the ADR index and architecture overview. ADR status remains `proposed`; RNF-02 remains `specified` by #134. No spec, test, production code, recipe or agent configuration changed.
- Clarified the order from memory.md section 13: the human's `CLAUDE.md` recipe entry follows implementation of the recipe; the reviewer checklist is due with or after the feature PR. Neither is an ADR-preparation prerequisite.
- `just check` passed (all Rust suites; 83 frontend files / 1,430 tests), as did `just trace`, `git diff --check` and `just privacy-check --base main` (0 matches). This handoff is a documentation-only follow-up.

**Pending**
- Human G1 review of ADR-0050 on `spec/RNF-02-memory-adr`, then the RNF-02 feature cycle: fresh test-author, fresh Rust implementer, independent review, `just ci` and feature PR.
- The feature PR needs the F6 long-suite report (including the provisional 64 MiB hi-res ceiling) and a fresh `just perf` report. F7 remains a manual whole-app check.
- With or after the feature PR: architecture RNF-02 wording and the human-owned agent configuration edits from memory.md section 13, followed by `just agents-sync`.
- Earlier throwaway measurement build folders under `target/rnf02-measure/` remain; no cleanup attempted in this session.

**Blockers / questions for the human**
- ADR-0050 awaits G1. No new dependency or change to the approved memory bounds is proposed.

**Next step:** review and approve ADR-0050 at G1, then `/feature RNF-02`.

## 2026-11-14 - RNF-02 spec approved at G1 (PR #134)

**Done**
- Spec PR [#134](https://github.com/salvador-arance/FakeFlac/pull/134) is merged as `f0b9124`. It adds `specs/contracts/memory.md` and `specs/features/memory.feature`, and sets RNF-02 to `specified`. It also holds the traceability row.
- Criterion: a 300 s file may use at most 1 MiB more peak memory than a 30 s copy, per format, and stays within 32 MiB of an idle process. Measured per child process through PowerShell, with no new dependency.
- F1-F5 are measured on today's code. Opus fails the growth bound (+2.1 MiB, one index entry per packet in `opus.rs`). Every other format grows by at most 92 KiB, noise is at most about 190 KiB per pair, and the gate takes about 55 s against 120 s.
- Throwaway measurement harness lives under `target/rnf02-measure/` (untracked).

**Pending**
- ADR extending ADR-0046 and ADR-0019 (CI memory gate, PowerShell measurement and its flake risk, `just memory`). It must exist before the `feat/RNF-02` PR opens (memory.md Q9). No ADR was merged at this G1, so none moves to `accepted`.
- Human edits after the spec (memory.md section 13): `CLAUDE.md` `memory` recipe line and `code-reviewer.md`, then `just agents-sync`.
- With or after the feature PR: `overview.md` RNF-02 row and `backend.md` section 5 wording ("reused window buffers").
- F6 (hi-res and 1-hour `just memory` report, 64 MiB provisional) and F7 (manual whole-app check).
- Delete `target/rnf02-measure/harness/target` and `harness2/target` (a guard hook blocked the agent's `rm -rf`).

**Blockers / questions for the human**
- None.

**Next step:** `/feature RNF-02` (tests, then the Opus two-pass reader and the `memory` recipe), after the ADR and the `CLAUDE.md` edits above.

## 2026-11-14 - RP-11 acceptance documentation PR #133 opened

**Done**
- With the owner's authorization, published `docs/RP-11-acceptance-v5` and opened PR [#133](https://github.com/salvador-arance/FakeFlac/pull/133) from acceptance record `9b0cded`.
- The PR records the v5 corpus acceptance and closes #132's G2 status in traceability; numerical evidence and the unchanged-corpus verification are in the next block.
- `just check` passed on `9b0cded`; this publication handoff changes documentation only. `just trace`, `git diff --check` and `just privacy-check --base main` verify the documentation follow-up. No additional acceptance or grid-search run.

**Pending**
- Owner's review and merge of documentation PR #133.

**Blockers / questions for the human**
- None.

**Next step:** owner reviews and merges #133, then chooses the next requirement.

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

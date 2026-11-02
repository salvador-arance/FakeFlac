# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

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

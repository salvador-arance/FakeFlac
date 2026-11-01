# Lessons

> Agent mistakes that repeat (>= 2 times) are fixed in the harness (rule, hook, lint or skill), not only in the code (RP-10). One entry per mistake: what happened, what the harness now does about it.

## ADR status left as `proposed` after G1 (2026-10-18)

- **What happened:** ADRs 0033–0038 were approved at G1 (their spec PRs were merged), but their status stayed `proposed` in each ADR and in `docs/adr/README.md`. Docs then said things were "pending until ADR-xxxx is accepted" when they already were. The traceability matrix also kept "UI pending" for RF-01, RF-05 and RF-06 after their UI and tests shipped in #27.
- **Harness fix:** `/handoff` now sets ADRs merged at G1 to `accepted` and updates traceability for tests added in feature PRs.
## Compile-only stubs in the `test(` commit break `guard-tests` (2026-10-19, RF-14)

- **What happened:** `test-author.md` allows "signatures with `todo!()` so the code compiles". The RF-14 test author put those stubs (`src/` files, `Cargo.toml`, `Cargo.lock`) in the `test(RF-14)` commit. `guard-tests` protects every file a `test(` commit touches, so the implementer's edits to those files failed CI. The orchestrator split the commit by hand (stubs in a `build(RF-14)` commit placed before the tests).
- **Harness fix:** `guard-tests.mjs` protects only the test files of a `test(` commit. It uses the same definition as the `guard-protected` hook (`scripts/test-paths.mjs`), tested in `scripts/guard-tests.test.mjs`. Stubs in production code stay editable.

## Unit tests that pass without the fix they cover (2026-10-19, RF-14)

- **What happened (twice in one feature):**
  - The Layer I test used a file under the 1 MiB scan window, so it passed with a 4× frame-length bug.
  - The ID3v2 tests used Info-tagged streams, so they passed while the reader still started at byte 0.
  - In both cases the implementer reported the defect fixed, and only a later review with mutation checks and the corpus probe found it.
- **Harness fix:** `code-reviewer.md` asks for a mutation check when a diff fixes earlier findings: revert each fix in a copy outside the repo and confirm its tests fail.

## Resuming an agent that is already overloaded (2026-10-19, RF-14)

- **What happened:** after its stage, the RF-14 test author had 283k tokens of context. Sending it another round would have cost the most and given the worst work. The owner asked that no agent with that much context ever be resumed. RF-14 already used a fresh agent per iteration, but only because the owner asked; the harness did not enforce it.
- **Harness fix:** the `guard-agents` hook (PreToolUse `SendMessage`) reads the subagent's transcript and blocks resuming any subagent of any type over 150k tokens of context (`FAKEFLAC_AGENT_CONTEXT_LIMIT`). `/feature` and `workflow.md` say every stage rerun gets a fresh agent with a short brief.

## Claude attribution in a commit merged to `main` (2026-10-25, #66)

- **What happened:** the harness asked for a `Co-Authored-By` trailer and the "Generated with Claude Code" line. The orchestrator had not read `docs/agents/workflow.md`, which forbids both, and added them to #66. The squash merge put the trailer in `main`. The owner had to rewrite `main` with a force-push.
- **Harness fix:** the rule is now in the non-negotiable rules of `CLAUDE.md` (#67). That file is loaded in every session; `workflow.md` is not.

## Indicative thresholds that were wrong on both sides (2026-10-25, RF-16)

- **What happened:** the spec-writer proposed the L6a window `[19500, 21000)` as "indicative, not measured". A measurement run before G1 used a throwaway harness on FakeFlac's own code, ffmpeg cascades and the corpus. It showed that the window would flag 96–100 % of honest 48 kHz LAME 320 encodes and 88–91 % of LAME 224/256 encodes. The measured window `[19800, 20200)` cut those to 3–5 % and 0 %, and recall stayed at 99.5 %. Without the run, the bad values would have reached G1 and the implement stage.
- **Harness fix:** `/new-spec` step 3 asks for indicative detection thresholds to be measured before G1 when they can be measured locally.

## No handoff after a spec merge (2026-10-25, RF-16)

- **What happened:** `/new-spec` ended at "open a PR and stop". After the owner merged #78, ADR-0043 was still `proposed`, `progress.md` had no entry and `backend.md` named a stale default. The owner had to point it out.
- **Harness fix:** `/new-spec` step 5 runs `/handoff` on a `docs/...` branch after the merge.

## A fixed delay in a spec that the measured run made impossible (2026-10-31, RNF-01)

- **What happened:** the RNF-01 spec draft cancelled the timed album 1000 ms after the start to measure cancel latency (T9), and required `cancelled: true`. The baseline run before G1 showed that the album finishes in about 0.9 s, so the cancel would never land and T9 would fail on any fast host. This is the second time (after the RF-16 thresholds) that an unmeasured figure in a spec was wrong, and only a measurement before G1 caught it.
- **Harness fix:** `/new-spec` step 3 now asks for every locally measurable figure (timings, delays and anything a test waits on, not only detection thresholds) to be measured before G1. T9 now cancels on the first `trackDone`.

## Old-format assertions surviving an approved format addition (2026-11-01, Opus)

- **What happened (two test iterations):** the Opus test stage updated primary scenarios but left older calibration guard and duplicate-label assertions behind. One test both included and excluded the same Opus row. Implementation then stopped twice on protected tests whose expectations contradicted already-approved C14/C15.
- **Test-stage check:** audit every existing scenario affected by a format addition, including error extraction, implicit folder selection and duplicate labels. Run the complete affected crate test suite before protecting the updated tests; classify every failure as missing production or a contradiction. When Tauri tests change, run `just check-app` as well: `just check` excludes that adapter. Respect a fixture's approved bitrate tolerance and compare a real track's estimate to its actual declared bitrate, rather than assuming a VBR target is exact. For synthetic container variants, check the in-memory layout and a cold concurrent fixture write, so cached files cannot hide a broken recipe or a race.

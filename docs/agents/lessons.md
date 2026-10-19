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

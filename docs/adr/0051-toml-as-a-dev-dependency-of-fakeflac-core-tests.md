# ADR-0051: `toml` as a dev-dependency of `fakeflac-core` tests

- Status: accepted
- Date: 2026-11-19

## Context and rationale

ADR-0033 approved `toml` as a dev-dependency of the `fakeflac-audio` tests, to check that the default `DetectionParams` equal the shipped params files, and wrote "the core stays free of parsers".

Issue #146 (RF-17) asks for a guard: a `Fake` verdict must always carry evidence, or the result cache would treat the row as a miss on every run and nothing would fail. Criterion X8 of `specs/detection/fake-evidence.md` requires a test in `fakeflac-core` that runs the verdict and evidence functions with the default params and with each params file v2 to v8. Those files are TOML, and `fakeflac-core` has no way to read them: the core builds `DetectionParams` from values and never reads files (hexagonal architecture, ADR-0011).

The guard needs the real files, not copies of their values: a copy in the test would drift from the files, which is the failure the guard exists to catch.

## Decision

Add `toml` 1 as a **dev-dependency** of `fakeflac-core`, used only by its integration tests (`tests/fake_evidence.rs`). The runtime dependencies of the core stay `serde` and `thiserror`. "The core stays free of parsers" of ADR-0033 keeps its meaning for the code the core ships; it does not cover test code.

It is the same crate and the same major version that `fakeflac-audio` already uses as a dev-dependency (ADR-0033) and `fakeflac-calibrate` as a normal dependency (ADR-0036), so `Cargo.lock` gains one edge and no new package.

## Alternatives considered

- **Move X8 to `fakeflac-audio/tests/`**, which already parses the files and depends on the core. It keeps ADR-0033 as written, but criterion X8 says the test is in the core, the verdict and evidence functions are core code, and the spec would need another round at G1 for a change of place only.
- **Give the core a params loader** (`DetectionParams::from_toml`). It puts a parser in the shipped core and changes the architecture for a test.
- **Copy the values of v2 to v8 into the test.** No new dependency, but the copies drift from the files and the guard stops guarding.
- **Write a small TOML reader in the test.** It adds code to maintain and to get wrong for a format that `toml` already reads.

## Consequences

- `crates/fakeflac-core/Cargo.toml` has one more dev-dependency; `Cargo.lock` has one more edge; the built core, the app and the installers do not change.
- `scripts/lint-arch.mjs` only checks dependencies between workspace crates, so it is not affected.
- The next core test that needs a params file can use it without a new ADR.
- ADR-0033 is amended (status line and one note in its table row).

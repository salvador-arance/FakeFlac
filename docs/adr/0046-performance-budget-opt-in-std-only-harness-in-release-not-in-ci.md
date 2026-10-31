# ADR-0046: Performance budget: an opt-in, std-only harness in a release build, not timed in CI

- Status: accepted
- Date: 2026-10-31

## Context and rationale

RNF-01 asks FakeFlac to "analyse a typical track (40 MB) in under 3 s and a 15-track album in under 20 s on an average machine". The planning document (`docs/planning/fakeflac-phases-1-3.es.md`, sections 3.9 and 3.11) planned `criterion` benchmarks with a regression threshold in a nightly CI workflow (`ci-nightly.yml`). That workflow was never written. The spec (`specs/contracts/performance.md`) has to settle three things:

1. **Where the gate runs.** GitHub's shared Windows runners have shared vCPUs and variable disks. A timing assertion there either fails at random or is so loose that it means nothing.
2. **What "average machine" means**, in a form a test can use.
3. **Which tool.** `criterion` is a new dependency with its own statistics and output, and it cannot call the app's wiring (`src-tauri/src/run.rs`) or the SQLite cache the way the app does.

The baseline measured before G1 (performance.md section 10) meets both budgets by a wide margin: 270 ms for the typical track and 861 ms for the album with 7 workers, on a Ryzen 7 5800X. So the harness is a regression guard, not an optimisation driver.

## Decision

1. **The gate is an opt-in recipe, `just perf [FOLDER]`.** It runs ignored tests of `src-tauri/tests/perf.rs` in a release build (`cargo test --release … -- --ignored --test-threads=1`) and times them with `std::time::Instant` only. Each figure is the median of 5 runs after 1 warm-up. It is not part of `check`, `check-fast`, `test`, `check-app` or `ci`, and no workflow calls it. Its report goes into the feature PR, and into any later PR that touches the analysis paths listed in `specs/contracts/performance.md` Q13 (code-reviewer checklist).
2. **CI times nothing.** It runs a smoke test of the harness inside `just check-app`, on tiny fixtures with no time assertion, so the harness keeps compiling and its report logic stays tested.
3. **The reference machine** is a Windows 10/11 x64 PC with 4 cores and 8 threads (2019 or later), 8 GB of RAM and an SSD. The gated album uses `min(worker_count(), 7)` workers, so a bigger host does not pass only through more parallelism.
4. **`flacenc` 0.5 becomes a dev-dependency of `src-tauri`**, the same version that `fakeflac-audio` and `tools/fakeflac-calibrate` already use for test fixtures (ADR-0033). No non-dev dependency is added.
5. This extends ADR-0019 with the `perf` recipe, and it replaces the planning document's plan for `criterion` in nightly CI.

## Alternatives considered

- **`criterion` benchmarks in a nightly CI workflow** (the planning document's plan). Rejected: it is a new dependency, the timings on shared runners are noisy, and it could not time the app's own wiring and cache.
- **A timing assertion in `just ci` with a generous bound** (for example 5× the budget). Rejected: a bound loose enough never to fail at random would only catch a collapse, and a red CI caused by a slow runner wastes G2 time.
- **A self-hosted runner** with fixed hardware. Rejected for now: it is infrastructure the project does not have, for a budget that is already met by more than 10×.
- **The harness in `tools/fakeflac-calibrate`**, which already has `flacenc`. Rejected: it cannot call `run.rs` or the cache as the app does, so it would not time what the user waits for.
- **Measuring on the real corpus only.** Rejected as a gate: the corpus is local and private (ADR-0022), and the figures depend on its content. It stays a manual check (`just perf FOLDER`, aggregates only).

## Consequences

- The budget has a repeatable, documented measurement, and a regression shows up as a failed `just perf` on the developer's machine, not as a random CI failure.
- Enforcement depends on people: the code-reviewer asks for the report when the analysis path changes. No tool can enforce timing without CI.
- The figures depend on the host. The report names the CPU so that G2 can judge them, and the album figure caps the workers at 7.
- `src-tauri` gains one dev-dependency (`flacenc`), which is already in the lock file.

# ADR-0034: The analysis runs on std scoped threads in the core

- Status: proposed
- Date: 2026-10-15
- Partially supersedes: ADR-0006 (the thread pool; cooperative cancellation stays)

## Context and rationale

ADR-0006 chose rayon for the per-track parallelism and ADR-0033 placed `rayon` in `src-tauri` as the "use case runner". When the `AnalyzeFolder` use case was implemented (RF-02, RF-05), two rules pulled the other way:

- The use case lives in `fakeflac-core`, and its tests (event order, cancel, single run, RNF-04) run there with fakes of the ports. The core may only depend on `serde` and `thiserror` (`crates/fakeflac-core/CLAUDE.md`, `docs/architecture/dependencies.md`).
- Leaving the loop in `src-tauri` would put the run logic where `just test` does not reach (the Tauri crate is excluded) and where domain logic is not allowed.

What ADR-0006 wanted from rayon is a bounded number of threads and load balancing between short and long tracks. The workload is coarse: one task per track, tens to thousands of tasks, each taking from tens to hundreds of milliseconds.

## Decision

`analyze_folder` runs `workers` threads with `std::thread::scope`. Each thread takes the next track index from a shared counter (dynamic balancing, no work stealing needed at this granularity). The composition root chooses `workers` (cores − 1). Cancellation stays cooperative through a `CancelToken` (atomic flag), checked before each track.

## Alternatives considered

- **rayon in `fakeflac-core`**: same behaviour with a pool, but it breaks the "serde and thiserror only" rule of the core for a gain that does not show at one task per track.
- **An executor port injected from `src-tauri` (rayon behind it)**: keeps rayon but adds a port and indirection whose only purpose is to hide a loop (D3, over-engineering).
- **The loop in `src-tauri`**: untested by `just test`, and logic outside the core.

## Consequences

- No new dependency; `rayon` is not used in H1 (ADR-0033 note).
- Threads are created per run (cheap compared with a run) instead of reusing a global pool.
- A panic inside one track is caught per track and becomes an error track (ADR-0013), so it cannot stop the run.
- If finer-grained parallelism is ever needed (for example, windows of one track in parallel), revisit with a new ADR.

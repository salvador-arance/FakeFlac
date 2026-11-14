# ADR-0050: Memory budget: OS peak memory of a child process, std-only, gated in CI

- Status: accepted
- Date: 2026-11-14

G1: approved by the owner in chat on 2026-11-14, with authorization for the agent to merge PR #136 after CI passes.

## Context and rationale

RNF-02 requires streaming decode without loading the whole track into memory. The approved [memory contract](../../specs/contracts/memory.md) (#134) makes this measurable: compare a 30 s and a 300 s synthetic track in each supported format, with at most 1 MiB of peak growth and at most 32 MiB over an idle baseline, for both private bytes and working set. Measurements use one analysis or spectrogram call per fresh child process because an operating system peak cannot be reset.

The pre-G1 figures in the contract, section 12, justify a gate rather than a manual observation. Every non-Opus pair grew by at most 94 208 bytes; the Opus packet index grew by about 2.1 MiB and failed the bound. The largest measured run-to-run spread was 94 208 bytes per child on the development machine, about 190 KiB for a pair, below the 1 MiB bound. The gate was estimated at about 55 s against a 120 s budget. These observations do not guarantee the same noise or runtime on a shared CI runner.

[ADR-0046](0046-performance-budget-opt-in-std-only-harness-in-release-not-in-ci.md) keeps timing gates out of CI because runner speed varies. A peak of a deterministic, single-threaded allocation sequence is a different measurement: it is the child process's memory, not the time taken by a shared CPU or disk. RNF-02 therefore needs an explicit memory-only exception to ADR-0046's opt-in policy, while retaining an opt-in suite for expensive fixtures. The workspace forbids unsafe code, and this decision adds no dependency.

## Decision

1. **Measure the operating system's peaks in fresh child processes, using only `std` and Windows PowerShell.** Each child re-executes the audio integration-test binary, performs exactly one call with the shipped default parameters (or no call for a baseline), and, while still alive, starts `powershell.exe -NoProfile -NonInteractive` to query its own PID through `Get-Process`. `PeakPagedMemorySize64` supplies peak private bytes (peak commit charge, `PeakPagefileUsage`); `PeakWorkingSet64` supplies peak working set (`PeakWorkingSetSize`). Both are bytes, not sampled instantaneous values. The parent process generates fixtures and is never the process being measured. The protocol and pure validation functions are defined by the memory contract, sections 3 and 7.
2. **Gate memory in CI.** Normal tests in `crates/fakeflac-audio/tests/memory.rs`, including the serial `memory_gate` driver, run through `just test`, `just check` and CI's Windows `test-rust` job. The driver starts one child at a time, under both libtest and nextest. Windows runs the OS measurements; fixture, chunk, pure-function and wiring checks run on every platform as specified. This extends ADR-0046 and explicitly inverts its title's "not timed in CI" policy and decision 2 ("CI times nothing") **for memory only**: CI asserts memory bounds, never elapsed time. The RNF-01 timing gate stays opt-in.
3. **Extend [ADR-0019](0019-justfile-as-the-single-command-entry-point.md) with `just memory`.** This recipe runs the ignored long suite in a release build, serially (`--ignored --test-threads=1`), with its report visible. It covers one-hour FLAC and Opus analyses and spectrograms, plus 24-bit/96 kHz FLAC, per the memory contract. Like `just perf`, it is excluded from `check`, `check-fast`, `test`, `check-app`, `ci` and every workflow. Its report is required in the RNF-02 feature PR; the 64 MiB hi-res ceiling remains provisional until F6 is measured, as the contract requires.
4. **Measurement failures fail the suite.** Failure to start PowerShell, a timeout, a non-zero exit, malformed output, or a child panic, non-zero exit or missing measurement is never skipped or retried. The contract's inclusive bounds and exact validation errors apply without adjustment. A recurring flake sends the bounds or the metric back to G1 with evidence; it does not justify a retry or an automatic relaxation.
5. **Keep fixtures local, synthetic and bounded on disk.** A single serial driver creates one case group at a time under `CARGO_TARGET_TMPDIR`, in a unique Unicode path, and cleans it before the next group, including on failure. No measurement uses or writes the private corpus (ADR-0022). Reports contain synthetic case names and aggregate figures. Whole-app memory remains the manual, indicative check M1, outside the per-track CI gate.

## Alternatives considered

- **A counting global allocator, through `peak_alloc` or `dhat`.** Rejected: implementing `GlobalAlloc` here needs an unsafe implementation, forbidden by workspace policy; using a crate adds a dev-dependency. Although deterministic and cross-platform, it measures the Rust heap only and can miss a memory-mapped file and other process allocations.
- **Calling `GetProcessMemoryInfo` through the `windows` crate.** Rejected: the API requires an unsafe call, also forbidden. PowerShell exposes the required OS peak counters without unsafe Rust or a new dependency.
- **Task Manager only.** Rejected for the per-track criterion: it is manual, has no automated regression gate, and does not provide the specified child protocol. It remains suitable for the separate whole-app check.
- **An opt-in-only memory gate, following ADR-0046 unchanged.** Rejected: peaks of a deterministic single-threaded call are not timings, and the measured noise leaves margin within the growth bound. Running the small suite in CI catches reintroducing a whole-track buffer or per-packet index. Expensive long fixtures stay opt-in.
- **A looser bound, skipped measurements or automatic retries on CI.** Rejected: these can hide a real regression or a missing metric. Repeated instability requires a reviewed change at G1.

## Consequences

- The memory criterion becomes an automated regression guard across the supported formats. The required two-pass Opus reader is already specified in memory.md section 8; this ADR changes no detection rule, parameter version, IPC contract or error mapping.
- The small gate adds fixture generation and PowerShell start-up cost to Windows tests. It records duration against the contract's indicative 120 s budget without asserting elapsed time. If exceeded, section 5.4 permits the test stage to shorten pairs to about 20 s and 200 s while preserving the 10× ratio, whole blocks and at least 4 MiB of file growth, and to report the revised frame counts.
- Shared-runner risks remain explicit: PowerShell may fail to start, time out, start slowly, or be absent from a runner image; peak noise may exceed the 94 208 bytes measured per child on the development machine. Slow start-up increases suite duration, and measurement failures fail the suite. Repeated flakes require evidence and G1 reconsideration of the bounds or metric.
- OS peak enforcement is Windows-only. Other platforms retain the contract's portable checks, rather than reporting an unmeasured OS peak as passing.
- Long-suite figures F6 must be supplied for review, and a contradiction of the provisional bounds returns the spec to G1. Whole-app figures F7 remain manual. Neither is silently substituted for the per-track gate.
- The human maintains the later `CLAUDE.md` command entry and reviewer checklist, followed by `just agents-sync`, at the times assigned in memory.md section 13. The implement stage adds the recipe; it does not edit agent configuration.

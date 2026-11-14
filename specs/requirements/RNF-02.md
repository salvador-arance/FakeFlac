---
id: RNF-02
title: Memory
priority: high
milestone: H1
status: specified
---

**Memory.** Decode by streaming; never load the whole track in memory.

Measurable acceptance (pending G1). The bounds are backed by memory.md F1–F5, measured before G1 against today's code (one 16-core Windows 11 machine, dev/test profile, synthetic fixtures, a throwaway harness, one fresh child per analysis). F6 (the long suite) and F7 (the whole app) are measured later, in the feature PR or opt-in runs:
- **What is measured.** Each analysis (`SymphoniaAnalyzer::analyze`, and `spectrogram` for RF-07) runs alone in a fresh child process of the test binary. The child reads its own OS peaks after the call: peak private bytes (`PeakPagefileUsage`) and peak working set (`PeakWorkingSetSize`), through `powershell.exe` `Get-Process`. This uses std only, with no new dependency.
- **Growth.** For each pair of synthetic fixtures that differ only in length (about 30 s and 300 s of WAV, FLAC, FLAC of undeclared length, MP3 320 kbps, M4A AAC 256 kbps, M4A ALAC and Opus 256 kbps), each peak grows by **at most 1 MiB**. Holding the decoded track would add about 45 MiB, and the fixtures make holding the compressed file add at least 4 MiB (measured: at least 8.69 MB, F4). Measured today (F1, F2): every non-Opus growth is at most 94 208 bytes (9 % of the bound), and one child's run-to-run noise is at most 94 208 bytes, so a pair's worst noise is about 190 KiB; a bound under about 512 KiB would risk flakes.
- **Ceiling.** Each call, on the short file and on the long file alike, stays **at most 32 MiB** over a baseline child that makes no call. Measured today (F1): the worst child, the long Opus file, is 7.67 MB (working set) and 6.12 MB (private) over the baseline, a margin of about 4×.
- **Real work.** Every intact analysis returns `ok` with a duration within 1 ms of the fixture's length. MP3 and M4A AAC are allowed one codec frame (27 ms and 24 ms).
- **Chunks.** Every decoding pass hands the analysis chunks of **at most 65 535 frames**, checked through a hidden seam `decode_chunks_for_test`.
- **RNF-04 and RNF-09.** A truncated long FLAC and a FLAC that declares 2^36 − 1 frames fail as `corruptStream` within the same bounds. Unicode names are used throughout.
- **Empty state.** A WAV with an empty `data` chunk returns `ok` with a duration of 0 ms from the adapter, within the same ceiling.
- **Where it runs.** The gate runs in `just test` and in CI's Windows `test-rust` job.
  - One serial driver test writes every fixture and starts every child, with one case group on disk at a time, in a fixture folder unique per driver and process. It is safe under `cargo test` threads and under nextest processes.
  - Its time budget is 120 s. F3 measured about 55 s on the development machine (dev profile), a margin of about 2× for a slower CI runner.
- **ADR.** An ADR extending ADR-0046 must exist before the feature PR opens (memory.md Q9). It inverts ADR-0046's "not in CI" stance for memory, and it names the flake risk of measuring through PowerShell.
- **Opt-in long suite.** `just memory` checks 1-hour FLAC and Opus files the same way, and a 24-bit/96 kHz FLAC with a 64 MiB ceiling. These figures are not measured yet (F6), so the 64 MiB ceiling is provisional: the first `just memory` report in the feature PR confirms it, or the spec returns to G1.
- **Manual check.** The whole app's peak on a real folder is a manual, indicative check (M1; F7, not measured yet).

Today's Opus reader keeps a per-packet index, and it fails the growth gate: from 30 s to 300 s its peaks grow by 2 142 208 bytes (working set) and 2 248 704 bytes (private), about twice the bound (F1). It must be made two-pass with identical results (memory.md Q4).

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4).

Spec: `specs/contracts/memory.md`, which covers:
- the criterion, the metric and the measurement protocol;
- the fixtures, the gate suite, the long suite and `just memory`;
- the report and the pure functions with their exact messages;
- the levers allowed if a gate fails, and the layers;
- acceptance A1–A10 and manual checks M1–M2;
- G1 questions Q1–Q9 and decided defaults D1–D8;
- what the code does today, the figures measured before G1 (F1–F5) and the figures measured later (F6, F7).

Scenarios: `specs/features/memory.feature` (tag `@RNF-02`; `@RNF-04` and `@RNF-09` where they apply).

Decisions: ADR-0002, ADR-0010, ADR-0019, ADR-0022, ADR-0046, and a new ADR extending ADR-0046 (memory.md Q9; required before the feature PR, not yet written).

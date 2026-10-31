---
id: RNF-01
title: Performance
priority: high
milestone: H2
status: specified
---

**Performance.** Analyse a typical track (40 MB) in under 3 s and a 15-track album in under 20 s on an average machine.

Measurable acceptance: in a release build, `just perf` generates the typical track (FLAC, 16-bit, 44.1 kHz, stereo, 15 872 000 frames = 359.909 s of full-band noise, 38 000 000 to 42 000 000 bytes) and the album (15 copies of it, with Unicode names). The median of 5 runs, after 1 warm-up run, of the whole analysis (from the call to its return: scan, every track, last event; IPC and UI excluded) is **under 3000 ms** for the typical track (app path, `run::run_analysis`) and **under 20000 ms** for the album (with `min(worker_count(), 7)` workers, the reference machine's 7), both with the result cache off and with a new empty cache file per run (T1, T2). Every timed run must give the fixtures' truth (`real`, no error), so a fast failure cannot pass (T3). An album with one truncated track stays under 20000 ms and reports that track as `corruptStream` (T4, RNF-04). The reference ("average") machine is a Windows 10/11 x64 PC with a 4-core/8-thread CPU from 2019 or later, 8 GB of RAM and an SSD. The gate runs locally and opt-in (ignored tests in `src-tauri/tests/perf.rs`, `std::time::Instant` only); CI runs only a smoke test of the harness, with no assertion on any time (T12–T14). Warm cache, the app's own worker count, FLAC 24/96, WAV, cancellation latency (500 ms, RF-05), spectrogram time (2 s, RF-07) and a sequential album are reported with indicative targets; real-folder figures per format (MP3, M4A: lossy-source.md B11) and a cold OS-cache read are manual (M1–M3). Memory is RNF-02.

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4). Spec: `specs/contracts/performance.md` (fixtures, what is timed, cache and OS-cache modes, reference machine, harness and `just perf [FOLDER]`, report format, levers allowed if the budget is missed, acceptance T0–T14, manual checks M1–M3, G1 questions Q1–Q14, decided defaults D1–D10, baseline). Scenarios: `specs/features/performance.feature` (tag `@RNF-01`). Decisions: ADR-0006, ADR-0034, ADR-0010, ADR-0007, ADR-0008, ADR-0044, ADR-0026, ADR-0019, ADR-0022, ADR-0033, and a new ADR (performance.md Q14, proposed).

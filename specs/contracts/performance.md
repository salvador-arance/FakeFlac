---
id: SPEC-PERFORMANCE
requirements: [RNF-01, RNF-04, RNF-05, RNF-09]
status: proposed
---

# Performance budget: what is timed, on what, and where the gate runs (RNF-01)

> RNF-01 asks FakeFlac to "analyse a typical track (40 MB) in under 3 s and a 15-track album in under 20 s on an average machine". This spec turns each word of that sentence into something a test can measure: the fixtures (section 2), what is timed (section 3), the reference machine (section 4), the harness and where its gate runs (section 5), its report (section 6), what the implement stage may change if the budget is missed (section 7) and the acceptance criteria (section 9). It is not a backend ↔ frontend contract: no command, event or type changes (`commands.md` stays at version 7), and no UI changes. Scenarios: `specs/features/performance.feature` (tag `@RNF-01`, with `@RNF-04`, `@RNF-05` and `@RNF-09` where they apply). Decisions cited: ADR-0006 and ADR-0034 (parallel tracks, cooperative cancel), ADR-0010 (window sampling), ADR-0007, ADR-0008 and ADR-0044 (result cache), ADR-0026 (real FFT), ADR-0019 (justfile), ADR-0022 (corpus outside the repository), ADR-0033 (test dependencies). ADR-0046 (`docs/adr/0046-performance-budget-opt-in-std-only-harness-in-release-not-in-ci.md`, accepted) records Q7, Q8 and Q9 (Q14). Planning source: `docs/planning/fakeflac-phases-1-3.es.md` sections 2.4 (RNF-01), 3.9 ("Rendimiento") and 3.11 (`ci-nightly.yml`).

## 0. Open questions for G1

Each question has a default, and the rest of this spec is written with it. On 2026-10-31 the owner delegated the G1 answers to an agent and confirmed them. The defaults of Q1–Q3 and Q5–Q14 are approved. Q9 (the `flacenc` dev-dependency) was confirmed explicitly by the owner. Q4 is changed: T7 is a single-run report row like T11 (section 6.1). Changing a default changes only the sections, criteria and scenarios cited in its row (scenarios are in `features/performance.feature`, named by their title).

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| Q1 | **The typical track.** A 40 MB FLAC of CD audio (16-bit, 44.1 kHz, stereo) holds about 6 minutes of music (≈ 890 kbps). Define the typical track as that: a FLAC 16/44.1 stereo of **360 s** (rounded down to whole 4096-frame blocks: 15 872 000 frames, 359.909 s) whose file is **40 MB ± 5 %** (38 000 000 to 42 000 000 bytes), its signal full-band white noise at the level that gives that size (indicative: about −42 dBFS RMS, about 10 bits per sample after FLAC)? The alternative, full-scale noise until the file reaches 40 MB, gives only about 227 s: fewer samples to decode for the same bytes, so an optimistic figure | Yes: 360 s **and** 40 MB ± 5 % | 2.1; T1, T3; scenarios "The typical track is analysed in under 3 seconds", "The fixtures are what the budget means" |
| Q2 | **The album.** Read "15-track album" as **15 typical tracks** (600 MB, about 90 minutes)? A real 15-track CD album is nearer 60 minutes and 400 MB, so this is the conservative reading and matches "typical track" in the same sentence | Yes: 15 × the typical track | 2.2; T2; scenario "A 15-track album is analysed in under 20 seconds" |
| Q3 | **Which formats are gated.** Gate only FLAC 16/44.1 (Q1). Report, with the same 3 s as an indicative target, a **FLAC 24-bit/96 kHz** track and a **WAV 16/44.1** track of 40 MB each. MP3 and M4A are timed only on a real folder (section 5.3): the synthetic lossy writers of `lossy-fixtures.md` write mono streams with count1 coding only, which decode much faster than real joint-stereo MP3 or AAC, so a figure on them would mean nothing | FLAC 16/44.1 gated; FLAC 24/96 and WAV reported; MP3/M4A from a real folder | 2.3, 5.3; T8, M2 |
| Q4 | **What is timed.** The wall-clock time, with `std::time::Instant`, of one whole run: from the call that starts the analysis to its return, which covers the scan, every track and the last event, with a sink that converts every event to its contract DTO as the app does and then drops it. Not timed: Tauri's IPC, the WebView and the table. Per-track `analyze` times are a diagnostic, not the criterion. Each gated figure is the **median of 5 measured runs after 1 unmeasured warm-up run**; the first and the maximum are reported too. The report rows T7 and T11 are one measured run each (section 6.1) | Yes, with T7 and T11 as single runs (G1) | 3; T1, T2 |
| Q5 | **The result cache (RF-11).** The gates hold in two modes: cache **off** (`ResultCache::Off`, nothing looked up) and **cold cache** (a new empty cache file for each run, so the time includes opening the store, a miss per track and a `put` per track: the app's first run). A **warm** run (every track a hit) is reported with an indicative target only | Off and cold gated; warm indicative | 3.2; T1, T2, T6; scenarios under "Result cache" |
| Q6 | **The OS file cache.** Windows keeps recently read files in memory, and emptying that cache needs administrator tools. The gates are measured with the fixtures in the OS cache (the warm-up run reads them), which isolates the CPU work and is repeatable. A cold read is a manual check (M1): the first `just perf FOLDER` run on a real album after a reboot. Reading 600 MB from a SATA SSD (≈ 500 MB/s) costs about 1.2 s; the album budget leaves room for that | Gates OS-cached; cold read manual (M1) | 3.3; M1 |
| Q7 | **"Average machine".** A Windows 10/11 x64 PC with a CPU of **4 cores and 8 threads** released in 2019 or later, 8 GB of RAM or more, an SSD, on mains power with the default power plan. On it `worker_count()` (`src-tauri/src/run.rs`, cores − 1, ADR-0034) gives **7**. The gated album runs use `workers = min(worker_count(), 7)`, so a host with more threads does not pass only through more parallelism. A host's faster single core is not normalised; the report names the CPU so G2 can judge. The album with the app's own worker count is reported (T7) | Yes | 4; T2, T7 |
| Q8 | **Where the gate lives.** Timing on shared CI runners is noisy (shared vCPUs, variable disks), so a timing assertion there would fail at random or be too loose to mean anything. The gate is an **opt-in recipe `just perf`** that runs **ignored** Rust tests in a release build, with `std::time::Instant` only (no `criterion`, no new dependency). It is not part of `just check`, `just ci` or any workflow. CI runs only a **smoke test** of the harness (a tiny fixture set, no assertion on any time), so the harness keeps compiling and working. The figures of `just perf` go into the feature PR and into section 10. The planning document planned `criterion` benchmarks with a regression threshold in a nightly CI workflow; this replaces that plan (ADR-0046, Q14) | Yes | 5; T0, T12–T14; scenarios under "Harness" |
| Q9 | **Test dependency.** The harness lives in `src-tauri/tests/perf.rs`, because only there can it call the app's own wiring (`run::run_analysis`, `run::worker_count`, `run::get_spectrogram`) and the SQLite store. `src-tauri` has `hound`, `rand` and `rand_chacha` as dev-dependencies but not `flacenc`, which writes the FLAC fixtures in `fakeflac-audio` and `tools/fakeflac-calibrate`. Add **`flacenc = "0.5.1"` as a dev-dependency of `src-tauri`** (same version as the other crates; ADR-0033 already approved it for tests)? Alternative: the harness in `tools/fakeflac-calibrate` (which has `flacenc`), at the cost of not timing `run.rs` and the cache | Yes, owner approval needed | 5.1, 8 |
| Q10 | **Cancellation latency** (`backend.md` section 5: checking the token between windows "is pending until per-track times are measured (RNF-01)"; the RF-05 scenario "Cancel a running analysis" says "stops within 500 ms"). RNF-01 **measures and reports** it (T9, indicative 500 ms) and does **not** implement cancellation inside the analyzer: that changes the `SpectrumAnalyzer` port (`analyze(path, params)` has no cancel argument today) and every fake of it in the protected tests, so it needs its own spec change (RF-05) at G1, with T9's figures as input | Report only; a miss goes to G1 as an RF-05 change | 3.5; T9; scenario "Cancellation latency is reported" |
| Q11 | **Spectrogram time** (RF-07: "within 2 seconds", manual today). Report `run::get_spectrogram` on the typical track with RF-07's 2 s as an indicative target (T10)? It decodes the whole file, so it grows with the track, and the harness already has the fixture. IPC (the `u8` matrix sent as a JSON array) and drawing are not timed, so RF-07's own manual check stays | Yes, reported | 3.6; T10; scenario "Spectrogram time is reported" |
| Q12 | **Levers if the budget is missed** (section 7). The implement stage may only make changes that leave every result identical: the same `AudioProbe` bit for bit, so no new `EXTRACTION_VERSION`, no `DetectionParams` change (window count, segments and FFT size are data) and no new `algorithm_version`. Anything else (seeking instead of decoding the whole stream, more workers, parallel windows inside a track, fewer segments) goes back to G1 with the figures | Yes | 7 |
| Q13 | **Keeping it a regression guard.** No tool can enforce timing without CI. Add to the code-reviewer's checklist (`.claude/agents/code-reviewer.md`, outside `specs/`, done by the orchestrator) a rule: a diff that touches `crates/fakeflac-audio/src/`, `crates/fakeflac-fs/src/`, `crates/fakeflac-store/src/`, `crates/fakeflac-core/src/analyze_folder.rs` or `src-tauri/src/run.rs` needs the `just perf` report in its PR, and a gated median more than 20 % above the reference figures of section 10 (10.1 since #94; an indicative bound) is flagged for G2? | Yes | 5.4 |
| Q14 | **ADR.** Q7, Q8 and Q9 are recorded in ADR-0046, `docs/adr/0046-performance-budget-opt-in-std-only-harness-in-release-not-in-ci.md` (proposed): "Performance budget: an opt-in, std-only harness in a release build, not timed in CI". It replaces the planning document's `criterion`-in-nightly-CI plan and extends ADR-0019 with the `perf` recipe. Accept it at G1? | Yes | 5; ADR-0046 |

**Decided defaults (not questions).** They follow from approved decisions or from the scope, and are listed so that G1 sees them:
- **D1. 1 MB = 1 000 000 bytes**, as RNF-06 was read (`release.md` Q7). "Under 3 s" and "under 20 s" are strict: a median of 3000 ms fails.
- **D2. Release build only.** The figures come from `cargo test --release`. `perf.rs` has a pure function `check_release_build(debug_assertions: bool) -> Result<(), String>`: it returns `Err("perf: figures need a release build (run: just perf)")` when `debug_assertions` is true and `Ok(())` otherwise. The timed suite calls it first, with `cfg!(debug_assertions)`; on `Err` it prints that line, times nothing and fails. The smoke test calls it with `true` and `false` (T0).
- **D3. One harness test at a time** (`--test-threads=1`), so two timed runs never compete for the CPU.
- **D4. Fixtures are generated inside the test** (`specs/detection/fixtures.md`, ADR-0022), in a new folder under `std::env::temp_dir()`, never in the repository or the corpus. The album is the typical track written once and copied 14 times (`std::fs::copy`): equal content costs the same to decode, and generation stays short. Every generated file and cache file is deleted when the test ends, also when it fails (a drop guard). The synthetic suite needs about 1.3 GB free in the temporary folder (album 0.6 GB, `album_corrupt` 0.6 GB, the rest 0.1 GB); `album_corrupt`'s 14 intact files may be hard links (`std::fs::hard_link`) to the album's instead of copies, and where `hard_link` fails the harness falls back to `std::fs::copy`. There is no free-space check (std has no portable call for it): a full disk makes a write fail, which fails the test with that I/O error, and the drop guard removes what was written. Audio files are written once, before the first run, and never rewritten between runs; between runs only the cache file is deleted (3.2). The warm-up run (3.3) is mandatory in every cache mode (`off`, `cold`, `warm`) and in the smoke test, except for the single-run report rows T7 and T11 (section 6.1).
- **D5. Unicode names by default (RNF-09).** Every fixture lives in a folder named `Álbum ″Ñandú″` and is named `NN – Canción ‘x’.flac` (or `.wav`), with `NN` from `01`; so every timed run also checks that Unicode names cost nothing extra.
- **D6. No contract or UI change.** `commands.md` stays at version 7. `AnalysisSummary.elapsedMs` is reported next to the harness's own figure, not changed.
- **D7. Memory is RNF-02**, out of scope; the harness reports no memory figure.
- **D8. A real folder is reported in aggregates only** (ADR-0022): no path, no file name, no folder name, not even the root. `just privacy-check` applies to the feature PR as to any other.
- **D9. Indicative targets never fail.** Only T0–T5 and T12–T14 can fail the harness or CI. An indicative target is `met` when the median is strictly below it, like the gates (D1), and `missed` otherwise; a missed one is discussed at G2. T6's target is rounded to the nearest whole ms (so it can be 0 on the smoke test's tiny set, where it is then `missed`).
- **D10. lossy-source.md B11** (median and maximum time per MP3 and M4A track in a release build) is satisfied by the per-format table of `just perf FOLDER` (section 6.2) on a folder that holds MP3 and M4A files.

## 1. Scope

- **In:**
  - the definition of the typical track and the album as generated fixtures;
  - what is timed, in which build, with which cache state and OS-cache state;
  - the reference machine;
  - the harness (`src-tauri/tests/perf.rs`), the recipe `just perf [FOLDER]` and its report;
  - the CI smoke test of the harness;
  - the levers the implement stage may use if the budget is missed;
  - the baseline figures (section 10), measured before G1.
- **Reported, not gated:** a warm-cache run, the album with the app's own worker count, a FLAC 24/96 track, a WAV track, a sequential album, the cancellation latency, the spectrogram time, and a real folder's per-format times.
- **Out:**
  - memory (RNF-02, D7);
  - implementing cancellation inside the analyzer or the scan (Q10);
  - any change to `DetectionParams`, `algorithm_version`, `EXTRACTION_VERSION`, the worker count rule (ADR-0034) or the decoding strategy (Q12, section 7);
  - frontend performance: the table (`ui/sort-filter.md` P1–P2) and the CSV export (`ui/export-csv.md` M7) keep their own criteria;
  - timing in CI (Q8);
  - Opus (no decoder yet, ADR-0030).

## 2. Fixtures

All are generated by the harness with a fixed seed (`rand_chacha`), quantised with plain rounding, with lengths rounded down to whole 4096-frame blocks (`fixtures.md` "Levels" and "Length"; `flacenc` needs it, ADR-0033 notes). "Size" is the file size in bytes. Before any run the harness checks every fixture against its row (size window, format, sample rate, bit depth, channels, frames) and stops with `perf: fixture <id> is <bytes> bytes, outside [<min>, <max>]` (or `perf: fixture <id> has <what> <value>, expected <value>`) if one does not match, without timing anything. The size check is the pure function `check_fixture(id: &str, bytes: u64, min: u64, max: u64) -> Result<(), String>` of `perf.rs`: `Ok(())` when `min <= bytes <= max`, otherwise `Err` with the first message, numbers in plain digits (for example `perf: fixture typical is 43000000 bytes, outside [38000000, 42000000]`). The timed suite calls it with each fixture's real size; the smoke test calls it with given values (T5).

### 2.1 Gated: the typical track (`typical`, Q1)

| Field | Value |
|---|---|
| Container, codec | FLAC (`flacenc`) |
| Sample rate, depth, channels | 44 100 Hz, 16-bit, 2 channels |
| Length | 15 872 000 frames (3875 blocks of 4096; 359.909 s) |
| Signal | Independent white Gaussian noise per channel, full band (no filter) |
| Level | The one RMS level, the same for both channels, that puts the size in the window below. Indicative: about −42 dBFS (a Gaussian of σ ≈ 245 LSB has an entropy of about 10 bits per sample, and 31.7 M samples × 10 bits ≈ 40 MB). It must be at or above −45 dBFS, well above `silence_dbfs` (−60), so that no frame is skipped. The test stage picks it once, records it in the harness and confirms the size |
| Size | 38 000 000 to 42 000 000 bytes (40 MB ± 5 %, D1) |
| Truth | `Real` (full band, like `full_band` of `fixtures.md`); the test stage confirms it. If it is not `Real`, the test stage stops and the spec returns to G1 |

Known limit: noise is not music. `flacenc` will pick a low LPC order for white noise where real music uses orders of 8 to 12, so the FLAC decoder's prediction loop does a little less work per sample. M1 and M2 time real music to cover it.

### 2.2 Gated: the album (`album`, Q2)

**Layout.** Each timed folder is its own subfolder of the test's temporary folder, and each is named `Álbum ″Ñandú″` inside a parent of its own (`track/`, `album/`, `album_corrupt/`, `hires/`, `wav/`), so a run of one never scans another. The `track` folder holds `typical` alone, named `01 – Canción ‘x’.flac`; the `hires` and `wav` folders likewise hold their fixture alone.

The album folder holds 15 files `01 – Canción ‘x’.flac` to `15 – Canción ‘x’.flac`, each a byte copy of `typical` (D4): 15 × 38–42 MB.

**`album_corrupt`** (RNF-04): a copy of the album folder with file `08` replaced by the first half of the bytes of `typical` (`floor(size / 2)` bytes), so its STREAMINFO declares 15 872 000 frames and the stream ends about half way. Truth: `08` is `Error(corruptStream)` (the stream ends before the declared length, `decode.rs`); the 14 others are `Real`. This relies on `flacenc` writing a non-zero total-samples field in STREAMINFO (a zero there means "unknown length", and a truncated stream would not be `corruptStream`), so the harness reads that field from `typical`'s bytes and checks it is 15 872 000 before any run, stopping with `perf: fixture typical has STREAMINFO total samples <value>, expected 15872000` otherwise (part of T5; the smoke test checks the same on its tiny files with 438 272).

### 2.3 Reported (Q3)

| Id | Format | Length | Size | Truth |
|---|---|---|---|---|
| `hires` | FLAC 24-bit, 96 000 Hz, stereo (`flacenc`); independent white Gaussian noise per channel, at the one level that puts the size in the window, at or above −45 dBFS (indicative: about −39 dBFS, about 18.5 bits per sample) | 8 638 464 frames (2109 blocks of 4096; 89.984 s) | 38 000 000 to 42 000 000 bytes | not `Error` (expected `Real`; any non-error verdict is accepted, the fixture exists for its time) |
| `wav` | WAV PCM 16-bit, 44 100 Hz, stereo (`hound`); independent white Gaussian noise per channel at −12 dBFS RMS, samples clamped to the 16-bit range | 10 000 000 frames (226.757 s) | 40 000 000 bytes of samples plus `hound`'s header, inside 38 000 000 to 42 000 000 | not `Error` |

The CI smoke test (T12) uses its own tiny set instead: two FLAC 16/44.1 stereo files of 10 s (107 blocks, 438 272 frames) in `Álbum ″Ñandú″`, and no size window. In the dev profile the workspace crates run at opt-level 0 (only third-party packages get opt-level 3, root `Cargo.toml`), so the smoke test has a time budget: **the whole smoke test under 60 s in `just check-app` on the development machine**, measured and recorded in the feature PR (not asserted by the test, which asserts no time). If it is exceeded, the test stage may shorten the tiny files toward 5 s, keeping them a whole number of 4096-frame blocks and above `min_duration_s` (5.0 s): no fewer than 54 blocks (221 184 frames, 5.016 s). The frame counts in T12 and T13 then follow the new length.

## 3. What is timed

### 3.1 One run

A **run** is timed with `std::time::Instant` from just before the call to just after it returns:
- **App path:** `run::run_analysis(&slot, root, &send)` (cache off) or `run::run_analysis_cached(&slot, &export, Some(cache_file), root, &send)` (cold or warm cache). Its workers are `run::worker_count()`.
- **Reference path** (Q7): `fakeflac_core::analyze_folder_cached` with the same adapters and params as `run.rs` (`WalkDirScanner`, `SymphoniaAnalyzer`, `DetectionParams::default()`), `workers = min(run::worker_count(), 7)`, the `ResultCache` of the mode, and a sink that converts each event to its `dto::ProgressEvent` (as `run::analyze` does) and drops it. In cold-cache mode the timed span also includes `SqliteResultStore::open` with the `EngineTag` of `run.rs`, because the app opens the store after taking the slot and before the scan (`result-cache.md` section 7).

Wiring details for the test stage:
- `run_analysis_cached` needs an `&ExportState`: the harness passes `ExportState::new()`.
- One `AnalysisSlot` (`AnalysisSlot::new()`) may serve every run, because a run releases it when it returns (its `ActiveRun` is dropped); a fresh `AnalysisSlot` per run is equally valid.
- `run::engine_tag()` is private, so the reference path's cold mode builds the same value in the test: `EngineTag { app_version: env!("CARGO_PKG_VERSION").into(), extraction_version: fakeflac_audio::EXTRACTION_VERSION }` (inside the `fakeflac` package `CARGO_PKG_VERSION` is the app's version, so the values are equal).

In both, `send` converts and drops the event and returns `true`; it also records, with `Instant`, when each `trackStarted` and `trackDone` arrives (for the diagnostics of section 6). Each run checks its own result (T3) before its time counts.

### 3.2 Cache modes (Q5)

| Mode | How |
|---|---|
| `off` | App path `run_analysis`, or reference path with `ResultCache::Off` |
| `cold` | A cache file `perf-cache.sqlite3` in the test's temporary folder; before every run (the warm-up included) the harness deletes it and its `-wal` and `-shm` files with `fakeflac_store::clear_cache_files`, so each run creates an empty cache. The run must end with `cache: 'ok'`, `fromCache` 0 |
| `warm` | The same file after one `cold` run of the same folder, not deleted between runs. Each measured run must end with `cache: 'ok'` and `fromCache` equal to `total` |

The cache file is never in `%LOCALAPPDATA%` (`result-cache.md` section 5).

### 3.3 Repeats and the OS cache (Q4, Q6)

Each figure except T7 and T11 (one measured run each, section 6.1) is: 1 warm-up run (not measured; it also brings the files into the OS cache), then 5 measured runs. The harness reports the first measured run, the median (the 3rd of the 5 sorted values) and the maximum, in whole milliseconds (rounded to the nearest). **The gate is the median.**

### 3.4 Per-track figures

A track's time is from its `trackStarted` to its `trackDone` as `send` sees them. In a parallel run they include contention between workers, so they are diagnostics only. The sequential album (T11) and the folder mode's per-format table (section 6.2) use one worker, so their per-track times are clean.

### 3.5 Cancellation latency (Q10)

Reference path, `album`, cache off. The harness owns the `CancelToken`; `send` calls `cancel()` when it receives the run's first `trackDone`. A fixed delay is not used: the baseline album finishes in about 0.9 s (section 10), so a cancel at 1000 ms would arrive after the run had finished, and `cancelled: true` would never hold on a fast host. The trigger needs fewer workers than tracks, which `min(worker_count(), 7)` < 15 guarantees: when the first track finishes, at least 8 tracks are not yet started. Because the gated workers start their tracks together, the measured latency is close to the shortest remaining work and is a lower figure; the upper bound is the slowest track still running, which T11's per-track maximum shows. The latency is the time from `cancel()` to the return of `analyze_folder_cached`. The run must end `cancelled: true` with every `trackStarted` matched by a `trackDone` (the RF-05 rule). The core sets `cancelled` to `done < total` (`analyze_folder.rs`), so the `cancelled: true` check holds only when, at the first `trackDone`, some track is still unstarted, which requires **workers < tracks**: the gated album guarantees it (at most 7 workers, 15 tracks); the smoke test (T12) runs T9 with `workers = 1` on its two tiny files, so the first `trackDone` leaves 1 track unstarted. 1 warm-up and 5 measured runs; median reported.

### 3.6 Spectrogram (Q11)

`run::get_spectrogram(path)` on `typical`, timed from the call to its return (the DTO built). 1 warm-up and 5 measured calls; median reported. The result must have `width` > 0 and `height` > 0.

## 4. The reference machine (Q7)

| Part | Reference |
|---|---|
| OS | Windows 10 or 11, 64-bit (RNF-07) |
| CPU | x86-64, 4 cores and 8 threads, released 2019 or later |
| RAM | 8 GB or more |
| Disk | SSD (SATA or NVMe) |
| Power | Mains power, default power plan; no other heavy work running |
| `worker_count()` | 7 |

The gated album figures use `min(worker_count(), 7)` workers. On a host with fewer than 8 logical CPUs they use fewer, and the report says `host below reference`; a pass there is still a pass. The single-track figure uses one worker on any host, so it depends on the speed of one core, which the harness does not normalise: the machine line that `just perf` prints before the report (section 6.1) lets G2 judge it.

## 5. The harness

### 5.1 Where it is

- **`src-tauri/tests/perf.rs`** (an integration test of `src-tauri`, so it gets the Windows app manifest from `build.rs`, as `src-tauri/CLAUDE.md` requires).
- The timed suite is one or more tests marked `#[ignore]`, so `cargo test` (and so `just test`, `just check`, `just check-app` and `just ci`) never runs them.
- The smoke test (T12) is a normal test in the same file, so `just check-app` runs it in CI's Windows job.
- `perf.rs` compiles in both the dev and the release profile, and passes `check-app`'s `cargo clippy -p fakeflac --all-targets -- -D warnings` (no item used only under one profile may raise a dead-code or unused warning in the other).
- Dev-dependencies: `flacenc` added to `src-tauri` (Q9); `hound`, `rand` and `rand_chacha` are already there. Nothing is added to any non-dev dependency list. The `Cargo.toml` change is not a test file; section 8 gives it to the implement stage, so the compile is one of the reds of the `test(RNF-01)` commit.

### 5.2 The recipe (`justfile`, ADR-0019)

```
# RNF-01 performance harness (specs/contracts/performance.md). Release build, opt-in: not part of
# check, ci or any workflow. With FOLDER, also times that folder and reports aggregates only.
perf *args   (positional arguments, a Node script like `accept`)
```

What it does, in order:
1. More than one argument: prints `usage: just perf [FOLDER]` and exits 2.
2. Prints the machine line (section 6.1) from Node's `os` module: `os.cpus()[0].model`, `os.cpus().length`, `os.totalmem()`, `os.version()` and `os.release()`. Only the recipe prints it, before cargo starts; it is not part of the harness output, so a direct `cargo test … --ignored` run (and the smoke test) has no `machine:` line.
3. With a FOLDER argument, sets the environment variable `FAKEFLAC_PERF_FOLDER` to it, unchanged (so paths with spaces, `&`, `'` or any Unicode character pass, as `accept` passes its arguments).
4. Spawns `cargo test --release -p fakeflac --test perf -- --ignored --nocapture --test-threads=1` with inherited stdio, and exits 0 if cargo exited 0, otherwise 1.

It is not added to `ci`, `check`, `check-fast`, `test` or `check-app`, and no workflow calls it.

Step 4 builds the `fakeflac` package and its dependencies with the workspace release profile (`lto = true`, `codegen-units = 1`), which takes minutes on the first run. That is accepted: the recipe is local-only and opt-in.

### 5.3 Folder mode (`just perf FOLDER`)

When `FAKEFLAC_PERF_FOLDER` is set, the timed suite times only that folder (no synthetic fixtures), read-only, cache off:
1. **Total time:** app path `run_analysis`, 1 first run (reported as `first`: after a reboot it is the cold-read figure of M1) and 3 more runs (median of 3).
2. **Per track:** one sequential pass on the reference path with `workers = 1`, cache off; per-track times grouped by `TrackInfo.format`.
A folder the run refuses ends the folder mode with `perf: <CODE>` (the `AppError` code only, for example `perf: NO_AUDIO_FOUND`; the message is not printed because it may hold the path, D8) and a failure. The harness writes nothing into the folder (RNF-05): no cache, no export.

### 5.4 Regression guard (Q13)

Once the baseline is recorded (section 10), the gates T1 and T2 are the guard. The code-reviewer asks for a fresh `just perf` report in every PR whose diff touches the paths listed in Q13, and flags a gated median more than 20 % above the reference figures of section 10.1 (indicative). The gate itself stays absolute: a median at or above the budget fails.

## 6. The report

Printed to stdout so it can be pasted into a PR. Lines and columns in this order. Times are whole milliseconds.

### 6.1 Synthetic suite (`just perf`)

Printed by the recipe only (5.2 step 2), before cargo starts; **not part of the harness output** (a direct `cargo test … --ignored` run and the smoke test have no such line):

```
machine: <cpu model> | <n> logical CPUs | <RAM in GB, one decimal> GB RAM | Windows <os.release()> (<os.version()>)
```

The harness output starts here:

```
RNF-01 performance report (specs/contracts/performance.md)
build: <release | debug>
workers: app <worker_count()>, reference 7, gated album <min(worker_count(), 7)>[, host below reference]
fixtures: typical <bytes> B <frames> frames, hires <bytes> B <frames> frames, wav <bytes> B <frames> frames
| id | case | cache | workers | runs | first ms | median ms | max ms | target ms | kind | result |
|---|---|---|---|---|---|---|---|---|---|---|
```

Then one row per line of this table, in this order (`kind` is `gate`, `indicative` or `report`; `result` is `pass` or `FAIL` for a gate, `met` or `missed` for an indicative target, `-` for a report):

| id | case | cache | workers | target ms | kind |
|---|---|---|---|---|---|
| T1 | typical track | off | app | 3000 | gate |
| T1 | typical track | cold | app | 3000 | gate |
| T2 | album | off | gated album | 20000 | gate |
| T2 | album | cold | gated album | 20000 | gate |
| T4 | album, one corrupt track | off | gated album | 20000 | gate |
| T6 | album | warm | gated album | the smaller of 1000 and 10 % of the T2 `off` median | indicative |
| T7 | album | off | app | 20000 | indicative |
| T8 | hires track | off | app | 3000 | indicative |
| T8 | wav track | off | app | 3000 | indicative |
| T9 | cancel latency, album | off | gated album | 500 | indicative |
| T10 | spectrogram, typical track | - | - | 2000 | indicative |
| T11 | album, sequential | off | 1 | - | report |

`workers` shows the number used. For the `app` rows (T1, T7, T8) it shows `worker_count()`, the number passed to the run, even though a folder of one track keeps only one thread busy. T7 and T11 have `runs` 1, with no warm-up of their own (the album's files are already in the OS cache from the rows before them), and show their single time in all three time columns. After the table:

```
elapsedMs (median run): T1 off <ms>, T2 off <ms>
per track (T11, sequential): median <ms>, max <ms>
result: pass
```

or `result: FAIL (<id> <cache>, …)` listing every failed gate row in table order (for example `result: FAIL (T1 cold, T2 off)`). A failed gate makes the ignored test fail after the whole report is printed, so every figure is still shown.

### 6.2 Folder mode (`just perf FOLDER`)

The recipe first prints its `machine:` line (as in 6.1, recipe only, not part of the harness output). The harness output:

```
RNF-01 folder report (aggregates only, ADR-0022)
build: <release | debug>
tracks <total> | errors <errors> | <total size in MB, one decimal> MB | flac <n>, wav <n>, mp3 <n>, m4a <n>, opus <n>
| run | workers | runs | first ms | median ms | max ms |
|---|---|---|---|---|---|
| folder | <worker_count()> | 3 | <first run> | <median of the next 3> | <max of the next 3> |
| format | tracks | errors | MB | median ms per track | max ms per track |
|---|---|---|---|---|---|
| flac | … | … | … | … | … |
```

One rule for every count: **every `trackDone` of the sequential pass (5.3 step 2) counts, error tracks included**, grouped by its `info.format`. So `tracks` is the summary's `total`, `errors` is how many of those tracks have an error, the header's per-format counts add up to `tracks`, and each format row's `tracks`, `MB` and per-track times include its error tracks. A format row's `errors` is how many of its tracks have an error, so the rows' `errors` add up to the header's `errors` (this tells which format an error came from without naming the file; the error codes are not printed). Sizes are `info.sizeBytes`, which is the scanned file's size (set from the scan for every track, error tracks included), not a value decoded from the stream; the harness reads no file metadata of its own. The header always lists all five formats, with 0 where absent (today `opus` is always 0: the scanner skips `.opus`, ADR-0030). One format row per format with at least one track, in the order flac, wav, mp3, m4a, opus. No path or name appears anywhere (D8). When the run refuses the folder (5.3), the harness output is the two lines `RNF-01 folder report (aggregates only, ADR-0022)` and `build: …` followed by the single line `perf: <CODE>` and nothing else.

### 6.3 The `build:` line and the result logic

`build:` says `release` when `cfg!(debug_assertions)` is false and `debug` otherwise. The timed suite never gets past D2's check in a debug build, so its reports always say `release`; the CI smoke test (T12, T13), which runs in the dev profile, says `debug`. Building the report (rows, results, the `result:` line) is one function of the harness that takes the figures as input, so the smoke test can check the result logic with given figures, independently of any real time.

## 7. If the budget is missed (Q12)

**If the baseline (section 10) meets T1 and T2 in both modes**, the feature PR adds the harness and the recipe only, and the gates become the regression guard (5.4).

**If it does not**, the implement stage may optimise, but only with changes that keep every result identical:
- for every fixture of `fixtures.md`, `lossy-fixtures.md` and section 2, the `AudioProbe` is equal bit for bit (so `EXTRACTION_VERSION` stays 2) and so is the `TrackAnalysis`;
- `DetectionParams` (segments, frames per segment, FFT size, window, overlap and every threshold), `algorithm_version` and the event order of `commands.md` do not change;
- the full existing test suite stays green unchanged, and `just accept` on the corpus gives the same `summary.md` figures as before the change (recorded in the PR, ADR-0022).

Allowed levers (examples, all inside `fakeflac-audio` or the use case's plumbing): plan the FFT and allocate its scratch buffers once per track or per thread instead of per frame; skip windowing and FFT work for decoded samples outside the sampled blocks; reuse decode buffers across packets; a larger read buffer for the `MediaSourceStream`; remove copies between the decoder's buffer and the mono mix **provided the mix keeps its summation order** (a different order changes the `f32` bits).

**Not allowed in RNF-01** (each needs G1, and most an ADR): seeking to the sampled blocks instead of decoding the whole stream (ADR-0010 says "with seek", but the adapter decodes the whole stream today, `MonoStream::for_each_chunk`; seeking would change the spectra of MP3 and AAC through decoder priming, and would no longer find a corrupt packet outside the sampled blocks, RNF-04); more workers than cores − 1, or parallel windows inside a track (ADR-0034); fewer segments or a smaller FFT (`DetectionParams`, new `algorithm_version`); counting the frames of an undeclared-length FLAC from packet durations instead of decoding (`rules.md` section 1 step 2; `progress.md` notes it as an idea); any change to the scan's rules. If the allowed levers do not reach the budget, the implement stage stops and reports the figures, and the spec returns to G1.

## 8. Layers

- **`src-tauri/tests/perf.rs`** (test stage; protected test code from then on): the harness itself (fixtures, timing, report, `check_release_build`, `check_fixture`) **and** its smoke test, both written by the test stage, because the harness is test code. It uses only the public items of `run.rs`, `fakeflac_core`, `fakeflac_audio`, `fakeflac_fs` and `fakeflac_store` that already exist.
- **Reds at the `test(RNF-01)` commit:** the compile (`flacenc` is not yet a dev-dependency of `src-tauri`) and T14 (the `perf` recipe is missing from `justfile`). Nothing else may be red.
- **Implement stage:** `src-tauri/Cargo.toml` (the `flacenc` dev-dependency, Q9) and the `justfile` recipe `perf` (5.2), both unprotected. It does not edit `perf.rs`.
- **Production code:** none. The baseline (section 10) meets the budget, so section 7's levers do not apply; if a later `just perf` misses it, section 7 governs.
- **Frontend, bindings, `commands.md`**: unchanged (D6).

## 9. Acceptance (measurable)

**Gates of `just perf`** (release build, local, reference configuration of section 4; the report of the run goes in the feature PR):
- **T0 (build, D2).** `check_release_build(true)` returns exactly `Err("perf: figures need a release build (run: just perf)")` and `check_release_build(false)` returns `Ok(())` (automated: the smoke test, T12). The timed suite calls it first with `cfg!(debug_assertions)`, so in a build with debug assertions it prints that line, times nothing and fails.
- **T1 (typical track).** Median of 5 runs of `typical`, app path, **< 3000 ms** in mode `off` **and** in mode `cold`.
- **T2 (album).** Median of 5 runs of `album`, reference path with `min(worker_count(), 7)` workers, **< 20000 ms** in mode `off` **and** in mode `cold`.
- **T3 (real work, checked on every timed run, the warm-up included).** The run finished (`cancelled: false`); `done` = `total` = the fixture's file count; every `trackDone` of `typical` and `album` has verdict `real`, no error, `info.durationMs` within 1 ms of the fixture's length (359 909 ms) and `info.sampleRate` 44100; `errors` 0. A run that fails this check fails the suite, whatever its time. (So a regression that turns every track into a fast error cannot pass as fast.)
- **T4 (a corrupt track, RNF-04).** Median of 5 runs of `album_corrupt`, reference path, mode `off`, **< 20000 ms**; every run has `errors` 1, the error is `corruptStream` on `08 – Canción ‘x’.flac`, and the 14 others are `real`.
- **T5 (fixtures).** Every fixture matches its row of section 2 before anything is timed (section 2, first paragraph): the size through `check_fixture(id, bytes, min, max)`, and `typical`'s STREAMINFO total samples is 15 872 000 (section 2.2). The function's message is checked with given values by the smoke test (T12).

**Indicative and reported figures** (shown, never failing, D9): T6 (warm album: `fromCache` 15, `cache: 'ok'` on every run; target the smaller of 1000 ms and 10 % of the T2 `off` median, consistent with `result-cache.md` M1), T7 (album on the app path), T8 (`hires`, `wav`), T9 (cancel latency, target 500 ms), T10 (spectrogram, target 2000 ms), T11 (sequential album). Their correctness checks (T6's `fromCache`, T8's non-error verdict, T9's `cancelled: true` and matched events, T10's non-empty size) do fail the suite: an indicative figure from a wrong run is not reported.

**Automated in CI** (`just check-app`, no assertion on any time):
- **T12 (smoke).**
  - The non-ignored smoke test runs the whole synthetic suite's code path on the tiny set of section 2.3 with 1 warm-up and 1 measured run per row (rows T2, T4, T6, T7 and T9 use the two tiny files as the album, T4 with the second one truncated to half its bytes; T1, T8 and T10 use the first one), with T3's and the indicative rows' correctness checks adapted to the tiny set (verdict not `error` except the truncated file, durations of 438 272 frames, STREAMINFO total samples 438 272). Rows T8 `hires` and `wav` use the first tiny FLAC too: the smoke test does not write a WAV, which is accepted, because WAV decoding is covered by the existing `fakeflac-audio` tests. Row T9 runs with `workers = 1`, so its first `trackDone` leaves 1 track unstarted and the run ends `cancelled: true` (3.5) with `done` 1, 1 `trackStarted` and 1 `trackDone` (matched); its `workers` cell shows 1, and its latency may be close to 0 ms, which is not a failure. It checks: the harness header lines of 6.1, that is `RNF-01 performance report (specs/contracts/performance.md)`, `build: debug`, a `workers:` line and a `fixtures:` line, and no `machine:` line (only the recipe prints it, 5.2); exactly the 12 rows of section 6.1 in order, each with its id, case, cache and kind; every cell of the `first ms`, `median ms` and `max ms` columns is an integer, and every `target ms` cell is an integer or `-` (T11 only); the `cache` and `workers` cells of T10 are `-`; a `result:` line last. It accepts `pass` or `FAIL` (times on CI mean nothing) and does not check the size windows.
  - With given figures (6.3): a T1 `cold` median of 3000 ms gives `FAIL` on that row and the last line `result: FAIL (T1 cold)`; 2999 ms gives `pass`; T1 `cold` 3000 ms and T2 `off` 20000 ms give `result: FAIL (T1 cold, T2 off)`; a T9 median of 501 ms gives `missed` on row T9 and leaves `result: pass`.
  - With given values (D2, T0): `check_release_build(true)` returns exactly `Err("perf: figures need a release build (run: just perf)")` and `check_release_build(false)` returns `Ok(())`.
  - With given values (section 2, T5): `check_fixture("typical", 43000000, 38000000, 42000000)` returns exactly `Err("perf: fixture typical is 43000000 bytes, outside [38000000, 42000000]")`; `check_fixture("typical", 38000000, 38000000, 42000000)` and `check_fixture("typical", 42000000, 38000000, 42000000)` return `Ok(())` (the window is inclusive).
  - Time budget (section 2.3): the whole smoke test under 60 s in `just check-app` on the development machine, measured and recorded in the feature PR; not asserted by the test.
- **T13 (empty state and folder mode).** The smoke test runs the folder mode's code on (a) a temporary folder holding only `notes.txt`, whose harness output is exactly the 3 lines `RNF-01 folder report (aggregates only, ADR-0022)`, `build: debug` and `perf: NO_AUDIO_FOUND` (no `machine:` line, which only the recipe prints; no table), and a failure; and (b) a temporary folder named `Álbum ″Ñandú″ & co` holding the two tiny files, which gives the folder report of section 6.2 with `tracks 2`, `errors 0`, `flac 2, wav 0, mp3 0, m4a 0, opus 0`, one `flac` row whose `tracks` cell is 2 and whose `errors` cell is 0, and none of these appears as a substring of the output: the folder name, either file name, and every component of 4 or more characters of the folder's absolute path (split on `\` and `/`, the drive excluded). And (c) a temporary folder holding the first tiny file and the second one truncated to half its bytes (as in T4), which gives `tracks 2`, `errors 1` and one `flac` row whose `tracks` cell is 2 and whose `errors` cell is 1. All three leave the files' size, mtime and content hash unchanged and create nothing in the folder (RNF-05).
- **T14 (wiring).** The smoke test reads `justfile` as text and checks: a `perf` recipe exists; its body holds `--release`, `--ignored`, `--test-threads=1`, `--test perf`, `FAKEFLAC_PERF_FOLDER` and `usage: just perf [FOLDER]`; and the recipes `ci`, `check`, `check-fast`, `test` and `check-app` do not name `perf`, checked as a whole word (`\bperf\b`) on each of those recipes' dependency list and body, so that a word such as `performance` does not trip it. A recipe's dependency list is the rest of its header line after the `:`; its body is the indented lines that follow, up to the next non-empty line that is not indented. Comment lines (first non-blank character `#`) are not part of the body. It reads `.github/workflows/ci.yml` and `release.yml` and checks that neither contains `just perf`. Running the recipe with two arguments (exit 2) is checked by hand (M3).

**Manual** (owner, release build; aggregates only in the PR, ADR-0022):
- **M1 (cold read).** After a reboot, before anything else reads it, `just perf FOLDER` on a real 15-track lossless album (FLAC 16/44.1) from the corpus. Record the `first` figure and the album's size and track count. Indicative: `first` < 20000 ms.
- **M2 (real formats, B11).** `just perf FOLDER` on a folder of at least 200 tracks that holds FLAC, WAV, MP3 and M4A files. Record the folder report. Indicative: the median per track of every format < 3000 ms. This is lossy-source.md B11's report for MP3 and M4A (D10).
- **M3 (the gate on record).** The report of `just perf` on the owner's machine, with the machine line the recipe prints before it, is pasted into the feature PR, and section 10.1 is updated with it if the figures differ from the reference figures by more than 20 %. `just perf a b` prints `usage: just perf [FOLDER]` and exits 2.

## 10. Baseline and reference figures

### 10.1 Reference figures (`just perf`, #94)

The figures the regression guard compares against (5.4, M3). They replace 10.2 as the reference because T1 came out about 40 % faster than 10.2's (162 against 270 ms), above M3's 20 %: the harness writes its fixtures with `flacenc` and Gaussian noise, while 10.2 used `ffmpeg` and uniform noise, so FLAC does less decoding work per byte. Measured on 2026-10-31 on `feat/RNF-01` (merged as `c3a8389`), with the machine of 10.2:

```
machine: AMD Ryzen 7 5800X 8-Core Processor | 16 logical CPUs | 31.9 GB RAM | Windows 10.0.26300 (Windows 11 Pro)
workers: app 15, reference 7, gated album 7
fixtures: typical 39999729 B 15872000 frames, hires 41289467 B 8638464 frames, wav 40000044 B 10000000 frames
```

| id | case | cache | workers | first ms | median ms | max ms | target ms | result |
|---|---|---|---|---|---|---|---|---|
| T1 | typical track | off | 15 | 165 | 162 | 165 | 3000 | pass |
| T1 | typical track | cold | 15 | 186 | 186 | 193 | 3000 | pass |
| T2 | album | off | 7 | 516 | 519 | 527 | 20000 | pass |
| T2 | album | cold | 7 | 542 | 564 | 580 | 20000 | pass |
| T4 | album, one corrupt track | off | 7 | 477 | 477 | 509 | 20000 | pass |
| T6 | album | warm | 7 | 3 | 4 | 4 | 52 | met |
| T7 | album | off | 15 | 321 | 321 | 321 | 20000 | met |
| T8 | hires track | off | 15 | 135 | 134 | 135 | 3000 | met |
| T8 | wav track | off | 15 | 49 | 48 | 50 | 3000 | met |
| T9 | cancel latency, album | off | 7 | 23 | 27 | 45 | 500 | met |
| T10 | spectrogram, typical track | - | - | 358 | 358 | 392 | 2000 | met |
| T11 | album, sequential | off | 1 | 2546 | 2546 | 2546 | - | - |

T11 per track: median 170 ms, max 175 ms. `result: pass`.

**Manual checks (owner, 2026-10-31, `main` at `2a44206`, release build, aggregates only, ADR-0022):**
- **M1 (cold read):** a real 15-track FLAC 16/44.1 album of 198.4 MB from the corpus, after a reboot: `first` 381 ms, median of the next 3 383 ms (indicative < 20000 ms: met). On this NVMe SSD the cold read does not show: `first` is within the spread of the warm runs.
- **M2 (real formats, B11):** a folder of 210 tracks (3120.7 MB) outside the repository and the corpus; folder run with 15 workers: `first` 5421 ms, median 5362 ms. Per track, sequential (indicative < 3000 ms: met for every format):

  | format | tracks | MB | median ms per track | max ms per track |
  |---|---|---|---|---|
  | flac | 80 | 1696.6 | 171 | 790 |
  | wav | 20 | 620.8 | 36 | 62 |
  | mp3 | 80 | 576.5 | 288 | 527 |
  | m4a | 30 | 226.7 | 181 | 365 |

  `errors 1`: a FLAC whose stream is damaged (`ffmpeg` also fails to decode it: `invalid residual`), reported as `corruptStream`, with the run continuing (RNF-04). The report did not say which format it came from, which is why section 6.2 now has an `errors` column per format.

### 10.2 Baseline (measured before G1)

Measured on 2026-10-31 on `main` at `831786d` (detection v7, `EXTRACTION_VERSION` 2), release profile of the workspace (`lto`, `codegen-units = 1`, `opt-level = "s"`), with a throwaway harness under `target/` that follows sections 2 and 3, with these differences:
- the fixtures were written by `ffmpeg` (its FLAC encoder, uniform white noise from `anoisesrc`), not by `flacenc` with Gaussian noise; the sizes and lengths are inside section 2's windows;
- every run used the reference path (`analyze_folder_cached` with the adapters of `run.rs`); the "app" rows used `worker_count()` workers instead of calling `run::run_analysis`;
- the sink formatted each event with `Debug` instead of converting it to its DTO (more work, not less);
- T10 called `SymphoniaAnalyzer::spectrogram` without building the DTO.

| Figure | Value |
|---|---|
| Machine line | AMD Ryzen 7 5800X 8-Core Processor \| 16 logical CPUs \| 31.9 GB RAM \| Windows 10.0.26300 (Windows 11 Pro); NVMe SSD |
| `worker_count()` on that machine | 15 (gated album: 7) |
| `typical` size, frames, verdict | 40 048 151 B (uniform amplitude 0.012, about −43 dBFS RMS), 15 872 000 frames, `real` |
| `hires`, `wav` sizes | 41 764 776 B (8 638 464 frames, 24-bit/96 kHz); 40 000 078 B (10 000 000 frames) |
| T1 `off` median (first, max) | 270 ms (271, 274) |
| T1 `cold` median (first, max) | 281 ms (282, 286) |
| T2 `off` median (first, max), workers | 861 ms (862, 864), 7 workers; 15 `real` |
| T2 `cold` median (first, max), workers | 881 ms (881, 889), 7 workers; `cache: ok`, `fromCache` 0 |
| T4 median | 729 ms; 14 `real`, 1 error |
| T6 warm median | 3 ms; `fromCache` 15 (target: the smaller of 1000 and 86 ms → met) |
| T7 app-path album median | 434 ms, 15 workers |
| T8 `hires`, `wav` medians | 188 ms, 48 ms |
| T9 cancel latency median | 25 ms (max 28); `cancelled: true`, 7 of 15 done. A cancel 1000 ms after the start (the first draft of 3.5) arrived after the run had finished |
| T10 spectrogram median | 445 ms |
| T11 sequential album; per-track median and max | 3979 ms; 266 ms, 270 ms |
| Real folder (optional): per-format median and max per track | From the corpus, 1 worker, OS cache mostly cold (a 1-in-20 sample of 556 files): FLAC median 169 ms, max 1805 ms (a 407 MB file of about 40 minutes); FLAC files of 30–50 MB median 262 ms, max 438 ms; MP3 median 286 ms, max 636 ms; M4A median 192 ms, max 247 ms. Six real FLAC albums of 13–17 tracks (290–501 MB): 0.75–1.79 s on the first run with 3 workers, 0.31–0.78 s on a second run with 15 workers. Spectrogram of the 407 MB file: 3158 ms (above RF-07's 2 s; typical tracks 258–568 ms) |

Outcome: **meets T1 and T2 in both modes** (about 11× under 3 s, about 23× under 20 s), so the feature PR adds the harness and the recipe only (section 7, first paragraph). The time grows with the track's length, because the adapter decodes the whole stream (section 7). Not measured: a spinning disk, and a genuinely slower 4-core host.

## 11. Edits to approved specs and docs (proposed in #92)

- `requirements/RNF-01.md`: status `specified` and its measurable acceptance.
- `traceability.md`: the RNF-01 row.
- Outside `specs/`, for the orchestrator or `/handoff` (not part of this spec PR unless the owner asks):
  - `docs/architecture/backend.md` section 1: the pipeline says "decode with seek", but the adapter decodes the whole stream and samples blocks from it (`MonoStream::for_each_chunk`); section 3: the `SpectrumAnalyzer` row says `analyze(path, params, cancel)`, but the port is `analyze(path, params)`; section 5: the cancellation note points to T9 and Q10 instead of "pending until per-track times are measured";
  - `docs/architecture/overview.md` RNF-01 row: add "budget and harness: `specs/contracts/performance.md`, `just perf`";
  - `docs/architecture/glossary.md` / `CLAUDE.md` command list: the `perf` recipe;
  - `.claude/agents/code-reviewer.md`: the Q13 rule;
  - ADR-0046, `docs/adr/0046-performance-budget-opt-in-std-only-harness-in-release-not-in-ci.md` (Q14; already written, status proposed, with its row in `docs/adr/README.md`); its status becomes accepted if G1 accepts Q14.
- ADR-0010 is not edited: its decision ("analysis by sampling representative windows") holds; the "with seek" detail is noted in section 7 and in `backend.md`.

## 12. Revision after #94 (this PR)

- **Section 10:** the `just perf` figures of #94 become the reference (10.1), with the results of M1 and M2; the pre-G1 baseline stays as 10.2. The machine line of 10.2 follows 6.1's order (`Windows <os.release()> (<os.version()>)`), as the recipe prints it. 5.4, M3 and Q13's bound point to 10.1.
- **Section 6.2:** the format table of the folder report gets an `errors` column after `tracks`. T13 (b) checks it is 0 and a new T13 (c) checks it is 1 on a folder with one truncated track; scenarios "A folder whose path has spaces, an ampersand and Unicode characters is timed" (changed) and "The folder report says which format an error came from" (new).
- **Layers (after G1):** a feature PR on `feat/RNF-01-errors`. The harness is test code (section 8), so the test stage makes the whole change in `src-tauri/tests/perf.rs`: first the checks of T13 (b) and (c), red against today's format rows, then the `errors` column, which turns them green. The implement stage has nothing to change and only confirms `just check-app` is green. No production code, bindings or `commands.md` change.

---
id: SPEC-MEMORY
requirements: [RNF-02, RNF-04, RNF-09]
status: proposed
---

# Memory: streaming decode with a bounded working set per track (RNF-02)

> RNF-02 asks FakeFlac to "decode by streaming; never load the whole track in memory". This spec makes that measurable. A track counts as loaded whole when the memory used to analyse it grows with its length. The criterion (section 2) is that the **peak memory of one analysis does not grow with the track's length** beyond a small bound, and that it stays **under a fixed ceiling**. Both are measured as the operating system's own peak figures for a process that does nothing but that analysis (section 3). The spec also defines the fixtures (section 4), the suites and where they run (section 5), the report (section 6), the pure functions and their exact messages (section 7), what the implement stage may change (section 8), the layers (section 9) and the acceptance criteria (section 10). Section 11 lists what the code does today. Section 12 records the figures measured before G1 (F1–F5), which back the bounds, and the two figures measured later (F6, F7).
>
> This is not a backend ↔ frontend contract. No command, event, type or error changes (`commands.md` stays at version 8), and there is no UI change. Scenarios: `specs/features/memory.feature` (tag `@RNF-02`, plus `@RNF-04` and `@RNF-09` where they apply). Related: `specs/contracts/performance.md` (RNF-01; its D7 leaves memory to this spec, and its harness conventions are reused here), ADR-0002 (all computation in Rust, partly for RNF-02), ADR-0010 (window sampling; the adapter decodes the whole stream and keeps the sampled blocks), ADR-0019 (justfile), ADR-0022 (no corpus in the repository), ADR-0030, ADR-0040 and ADR-0047 (decoders), ADR-0034 (workers), ADR-0046 (the opt-in, std-only harness of RNF-01). A new ADR that extends ADR-0046 is required before the feature PR opens (Q9). Planning source: `docs/planning/fakeflac-phases-1-3.es.md` section 2.4 (RNF-02), risk R2 and section 3.4.5 ("one track per thread with a reusable window buffer → bounded memory").

## 0. Open questions for G1

Each question has a default, and the rest of this spec is written with that default. Changing a default changes only the sections, criteria and scenarios named in its row. Scenarios are in `features/memory.feature` and are named by their title.

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| Q1 | **The metric and the tool.** Measure each analysis as the operating system's **peak private bytes** (`PeakPagefileUsage`, the peak commit charge) and **peak working set** (`PeakWorkingSetSize`) of a child process that does only that analysis. The child reads its own peaks by running `powershell.exe` with `Get-Process -Id <own pid>` (`PeakPagedMemorySize64`, `PeakWorkingSet64`) after the analysis returns, while it is still alive. This needs only `std`, no new dependency and no `unsafe`. The workspace sets `unsafe_code = "forbid"`, so the alternatives cost more: (a) a counting `#[global_allocator]` needs `unsafe impl GlobalAlloc`, so it would have to come from a new dev-dependency (`peak_alloc` or `dhat`); it would be deterministic and cross-platform, but it counts only the Rust heap and would miss a memory-mapped file; (b) calling `GetProcessMemoryInfo` through the `windows` crate is an `unsafe fn`, so it is forbidden too; (c) Task Manager only, which is manual and has no gate | PowerShell self-query of the OS peaks, std only | 3; A3, A4, A8; scenarios under "How memory is measured" |
| Q2 | **Where the gate runs.** Memory is not timing: the peak commit of a single-threaded, deterministic allocation sequence barely changes between runs or machines. So, unlike RNF-01 (ADR-0046), the gate runs **in CI**, as normal tests of `crates/fakeflac-audio/tests/memory.rs`. `just test` runs them (`cargo test`, tests on threads of one process), and so does CI's `test-rust` job on `windows-latest` (`cargo nextest run`, every test in a process of its own). Everything that writes a fixture or starts a child runs in one serial driver test (5.1.1). The OS measurement is `#[cfg(windows)]`, while the pure functions and the chunk checks run on every platform. Alternative: opt-in only, like `just perf` | In CI (`just test`, `test-rust`), with an opt-in long suite (Q6) | 5.1; A1–A9 |
| Q3 | **Lengths and bounds.** The gate compares a **30 s** and a **300 s** version of each fixture (10× the length; 270 s more audio). **Growth bound: 1 MiB** (1 048 576 bytes) for each metric. **Ceiling: 32 MiB** (33 554 432 bytes) over the baseline child for each metric (64 MiB for the 24-bit/96 kHz fixture of the long suite). Holding the decoded mono `f32` signal of the extra 270 s at 44.1 kHz would add 47 628 000 bytes (45 × the bound). Holding the compressed file would add at least 4 MiB, because the fixtures guarantee it (A1). **Measured (section 12):** every non-Opus growth today is at most 94 208 bytes (9 % of the bound), and the run-to-run noise of one child is at most 94 208 bytes (F2), so the worst noise on a pair is about 190 KiB; a bound under about 512 KiB would risk flakes, and 1 MiB still catches the Opus index growth of today (2 142 208 bytes of working set, F1) and the 45 MiB of a decoded track. The worst child today is 7.67 MB (working set) over the baseline, so 32 MiB leaves a margin of about 4×. The **64 MiB** hi-res ceiling stays **provisional** until F6 is measured | Yes; 1 MiB and 32 MiB backed by F1 and F2; 64 MiB provisional (F6) | 2, 4, 5; A1, A3, A4, A10 |
| Q4 | **The Opus reader's per-packet index** (section 11). `OpusStream::open` keeps one `Packet` entry (its byte spans and TOC) for every 20 ms packet of the file. It never holds PCM or payload bytes, but by estimate it grows by 7.5–10 KB per second of audio: 2–2.7 MB from 30 s to 300 s, and 27–36 MB per hour. That is about the size of the compressed track itself at 64 kbps. **F1 confirms it:** from 30 s to 300 s the `opus` peaks grow by 2 142 208 bytes (working set) and 2 248 704 bytes (private), about 2 × the bound, so A3 fails today for `opus` and only for `opus`. Does the growth bound apply to Opus? Default: **yes**. The implement stage makes the reader two-pass: the first pass validates every page and packet as today (so every R1–R22 error of `lossy-source.md` section 2.4 is reported as today) and keeps only counters and header fields; the second pass reads the pages again in order and decodes each packet as it completes. The results are identical (section 8). Alternatives: (a) exempt Opus with its own bound (for example 16 KB per second of audio); (b) a compact index (about 16 bytes per packet: within the 1 MiB bound at 300 s, but about 2.9 MB at 1 hour, so it would fail L2) | The bound applies; two-pass reader | 8, 11; A3, A10; scenario "An Opus file's memory does not grow with its packet count" |
| Q5 | **A chunk seam.** To check the decoding loop directly and on every platform, add a hidden test seam to `fakeflac-audio`: `decode_chunks_for_test(path) -> Result<ChunkStats, TrackError>`, which runs one decoding pass and returns the number of chunks handed to the analysis, the frames decoded and the largest chunk (section 3.5). It is production code (`#[doc(hidden)]`, like `decode_opus_samples_for_test`), added by the implement stage. Alternative: no seam, and rely on A3 and A4 alone | Yes | 3.5; A5 |
| Q6 | **The long suite and its recipe.** The suite checks 1-hour tracks (FLAC and Opus, analysis and spectrogram) and a 24-bit/96 kHz track. Their fixtures take hundreds of MB of disk and minutes to write, so they run in an **ignored** serial driver test of the same file (`memory_long`, 5.1.1), through a new opt-in recipe **`just memory`** (release build, `--ignored --test-threads=1`). The recipe is not part of `check`, `check-fast`, `test`, `check-app` or `ci`, and no workflow calls it, as with `just perf` (ADR-0046). Its report goes into the feature PR | Yes | 5.2, 5.3; A9, A10 |
| Q7 | **The whole app.** A run analyses up to `worker_count()` tracks at once (ADR-0034), so the app's peak is about `workers ×` the per-track working set, plus the run's results. RNF-02 is about one track. The app's peak while it analyses a real folder is a **manual check reported with an indicative target** (M1), not a gate | Manual, indicative | 1, 10; M1 |
| Q8 | **Metadata that grows with length.** Container metadata that the file itself stores in proportion to its length is allowed inside the growth bound. Today that means the MP4 `moov` box, which `mp4::read` reads whole (capped at 256 MiB by `MAX_MOOV_BYTES`) and whose sample tables take about 4 bytes per AAC frame (about 172 bytes per second at 44.1 kHz). F1 measured the `m4a_aac` growth from 30 s to 300 s at 69 632 bytes (working set) and 94 208 bytes (private): real, but within the bound. The cap is not changed. An hour-long M4A is not gated (it is a known limit, section 11) | Yes | 2, 11 |
| Q9 | **ADR.** Q1, Q2 and Q6 are a test-infrastructure decision of the same kind as ADR-0046: a memory gate in CI that uses the OS peaks of a child process read through PowerShell, std only, plus an opt-in `just memory` recipe that extends ADR-0019. **Requirement:** an ADR ("Memory budget: OS peak memory of a child process, std-only, gated in CI") exists in `docs/adr/` before the `feat/RNF-02` PR opens. It must: (a) state that it extends ADR-0046 and ADR-0019 (`just memory`); (b) explicitly invert, for memory only, ADR-0046's "not in CI" stance (its title and decision 2, "CI times nothing"), and give the reason: peaks of a deterministic, single-threaded call are not timings (Q2); (c) name the flake risk of measuring through `powershell.exe` on a shared CI runner, which is its start-up failing, timing out or being slow, a runner image without it, and run-to-run noise of the peaks above what F2 measured (at most 94 208 bytes per child on the development machine); and state that such a failure fails the suite and is never skipped or retried (3.3), so a recurring flake sends the bounds or the metric back to G1; (d) record the rejected alternatives of Q1. Not written by the spec stage and not part of this spec PR (section 13). If the ADR does not exist, the feature PR does not open | ADR required before the feature PR | 3, 5, 13 |

**Decided defaults (not questions).** They follow from approved decisions or from the scope. They are listed so that G1 sees them.
- **D1. Binary units.** 1 KiB = 1024 bytes, 1 MiB = 1 048 576 bytes. Windows reports memory in these units, unlike the decimal MB of performance.md D1. Every bound is inclusive: a growth of exactly 1 048 576 bytes passes.
- **D2. One analysis per process.** A peak cannot be reset, so every measured figure comes from a fresh child process that performs exactly one call (section 3.2). The calls are single-threaded (`analyze` and `spectrogram` use no worker threads), so the figure is the working set of **one track**.
- **D3. Fixtures are generated inside the test**, under `CARGO_TARGET_TMPDIR` like the other `fakeflac-audio` fixtures, never in the corpus (ADR-0022), in a folder of their own that is unique per driver and process (5.1.1). **One case group is on disk at a time.** A case group is one pair of 4.1 or 4.2 plus the files derived from it. The `flac` group holds the `flac` pair and, one at a time, the `flac_unknown_length` pair, `flac_huge_length` and `flac_truncated`. `wav_empty` is a group of its own.
  - The driver writes a group just before measuring it. It writes each derived file (or pair) from the pair's bytes while the pair is on disk, and deletes it once it has been measured.
  - It deletes the whole group before it writes the next one.
  - Before it writes a group, it checks that the fixture folder holds no file (A1).
  - A drop guard deletes everything, even when the test fails.
  - The gate suite needs about 75 MB of free disk at a time (the `flac` pair plus the `flac_unknown_length` pair). The long suite needs about 420 MB.
- **D4. Unicode names (RNF-09).** Every fixture lives in a folder named `Memoria ″Ñandú″` and is named `NN – Canción ‘x’.<ext>`, with `NN` from `01`. The child gets the path as an `OsString` (an environment variable or an argument), never as a lossy string.
- **D5. Default parameters.** Analyses use `DetectionParams::default()` (v8), and spectrograms use `SpectrogramParams::default()` (2048-point Hann, 75 % overlap, at most 1200 columns, 512 rows). The working set depends on the parameters (segments, frames per segment, FFT size), and the defaults are what the app runs.
- **D6. Builds.** The gate suite runs in the dev profile, as `just test` builds it, and the long suite in the release profile. The report prints the build. Memory hardly depends on the profile, and both figures are gated.
- **D7. No contract, UI or detection change.** `commands.md` stays at version 8, `DetectionParams` and `algorithm_version` (8) do not change, and `EXTRACTION_VERSION` stays 2.
- **D8. Time is RNF-01.** The suites assert no time. Their duration is recorded in the feature PR against the 120 s budget of section 5.4 (F3 measured about 55 s on the development machine).

## 1. Scope

- **In:**
  - the criterion (growth and ceiling), the metric and how it is read;
  - the fixtures;
  - the gate suite (CI) and the long suite (`just memory`);
  - the chunk seam;
  - the report;
  - the levers allowed if a gate fails, including the Opus reader change of Q4.
- **Reported, not gated:**
  - the noise of the baseline (two baseline children);
  - the largest chunk per format;
  - the app's peak on a real folder (M1).
- **Out:**
  - **time** (RNF-01, `performance.md`);
  - **the run as a whole**: how many tracks are analysed at once (ADR-0034), the results kept per run (`ExportState`), the result cache, the CSV export and the table of 10 000 rows;
  - **the WebView and the frontend** (drawing the spectrogram, the JSON matrix sent over IPC);
  - **the OS file cache**, which is not memory of the process;
  - **seeking instead of decoding the whole stream** (ADR-0010; performance.md section 7: it changes MP3 and AAC spectra and corrupt-packet detection). RNF-02 asks for streaming, not for decoding less;
  - an hour-long M4A, and `moov` boxes up to `MAX_MOOV_BYTES` (Q8);
  - changing `MAX_MOOV_BYTES` or the MP3 sync-search window (1 MiB + 8 KiB), both constant in the track's length.

## 2. The criterion

For one call (`SymphoniaAnalyzer::analyze` or `SymphoniaAnalyzer::spectrogram`) on one file, let **P(file)** be the peak of a metric (private bytes or working set) of a child process that makes only that call (section 3).

- **Growth (the streaming criterion).** For each pair of fixtures that differ only in length (30 s and 300 s; 30 s and 3600 s in the long suite): `P(long) − P(short) ≤ 1 MiB`, for both metrics. A negative difference counts as 0.
- **Ceiling (the bounded working set).** For every measured call: `P(file) − P(baseline) ≤ 32 MiB` for both metrics (64 MiB for the 24-bit/96 kHz fixtures of the long suite). The baseline is a child that makes no call (section 3.4).
- **Chunks (the shape of the decoding loop).** Every decoding pass hands the analysis chunks of at most **65 535 frames** (FLAC's largest block; MP3 has 1152, AAC 1024, ALAC 4096 and Opus at most 5760), and the largest chunk of the long fixture equals the short one's (section 3.5).

Why these figures show the whole track. From 30 s to 300 s at 44.1 kHz, the decoded mono `f32` signal grows by 270 × 44 100 × 4 = 47 628 000 bytes (45 × the growth bound, and above the ceiling on its own). The interleaved 16-bit PCM of a stereo track grows by the same amount. The compressed file grows by at least 4 MiB (A1), which is 4 × the bound. So keeping any of them in memory fails, and so does anything that grows in proportion to them, such as an index whose size is of the order of the compressed file (Q4).

## 3. How memory is measured

### 3.1 The metrics

| Metric | Windows counter (`PROCESS_MEMORY_COUNTERS`) | Read through (`System.Diagnostics.Process`) | What it catches |
|---|---|---|---|
| `private` | `PeakPagefileUsage`: peak commit charge (private bytes) | `PeakPagedMemorySize64` | Every private allocation: the heap, decoder buffers, large `Vec`s, committed stacks |
| `working-set` | `PeakWorkingSetSize` | `PeakWorkingSet64` | Pages actually touched, including a memory-mapped file and code pages |

Both are peaks since the process started, kept by the OS, in bytes.

### 3.2 One measurement, one child

- **Re-executing the test binary.** The parent test re-runs its own test binary (`std::env::current_exe()`) with the libtest arguments `--exact memory_child --ignored --nocapture --test-threads=1`. `memory_child` is an ignored test of `memory.rs`. The parent passes the operation (`baseline`, `analyze` or `spectrogram`) and the file path through environment variables or arguments of the test stage's choice. This works under `cargo test` and under `cargo nextest`, and `lossy-fixtures.md` 1.3 uses the same pattern for `RUSTY_OPUS_ISA`.
- **When `memory_child` has no operation.** It returns at once and passes. This happens when `just memory` runs every ignored test, or when someone runs it by hand.
- **What the child does.**
  1. It makes its one call with the parameters of D5. It does not write the fixture, and it measures nothing else first.
  2. It runs the command of 3.3 for its own PID.
  3. It prints, on one line, the call's result (`ok` or the `TrackError` code in camelCase: `corruptStream`, `tooShort`, `unreadable`, `unsupportedFormat`), its `duration_ms` (analysis) or `width` and `height` (spectrogram), and the two peaks.
  4. It exits 0.

  The format of that line between the child and the parent is the test stage's choice.
- **What fails the suite.** A child that panics, exits non-zero or prints no such line fails the suite, with its stderr shown. It is never skipped.
- **The parent** is the serial driver test (`memory_gate` or `memory_long`, 5.1.1). It runs the children one at a time and generates and deletes the fixtures (D3). The parent's own memory is never measured.

### 3.3 Reading the peaks

```
powershell.exe -NoProfile -NonInteractive -Command "$p = Get-Process -Id <pid>; '{0} {1}' -f $p.PeakWorkingSet64, $p.PeakPagedMemorySize64"
```

- **Output.** The command prints one line: the peak working set, one space, then the peak private bytes, both as decimal integers. `parse_peaks` (section 7) reads it.
- **When it cannot be read.** If `powershell.exe` cannot be started, exits non-zero or prints anything else, the child reports the `parse_peaks` error and the suite fails. A missing figure is never a pass.
- **Cost.** Spawning the command allocates a little in the child after the call. This is the same for every child, the baseline included.

### 3.4 Baseline

Two `baseline` children are run first. Each makes no call, opens no file and only reads its peaks (3.3). The first is the baseline of the ceiling (A4). The difference between the two is reported as the measurement noise (section 6), and it does not fail the suite.

### 3.5 The chunk seam (Q5)

Added by the implement stage to `crates/fakeflac-audio/src/lib.rs`:

```rust
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkStats {
    /// Calls of the decoding loop's sink (one per decoded packet handed to the analysis).
    pub chunks: u64,
    /// Frames decoded in this pass.
    pub frames: u64,
    /// Frames of the largest chunk.
    pub max_chunk_frames: u64,
}

/// One decoding pass over `path` through the same `MonoStream` that `analyze` and `spectrogram`
/// use (test access for RNF-02, `specs/contracts/memory.md` section 3.5).
#[doc(hidden)]
pub fn decode_chunks_for_test(path: &Path) -> Result<ChunkStats, TrackError>;
```

- **One pass.** It makes one `MonoStream::open` and one `for_each_chunk` pass, counting in the sink. It returns the error of `for_each_chunk` unchanged, for example `CorruptStream` for a stream that ends before its declared length.
- **Undeclared length.** For a stream that does not declare its length, the pass is the counting pass of `analyze` (no length check). It does not do the second pass.
- **Cost.** It allocates nothing that grows with the stream.
- **Where it runs.** The seam runs in the parent process on every platform, so A5 does not need Windows.

## 4. Fixtures

All fixtures are generated with a fixed seed (`rand_chacha`) by the test-support writers of `fixtures.md` and `lossy-fixtures.md`, with a length parameter added. Their content only needs to be real audio that the analysis decodes in full, and their truth is not checked here. A fixture's **length** is the frame count its writer wrote, as listed below (Opus: the samples per channel after pre-skip and end trim). **Short** is about 30 s and **long** about 300 s. For each pair the suite checks the format, the sample rate, the channels and the frames (or packets) against its row, and that the long file is at least **4 MiB** larger than the short one (A1). It starts no child on a group before that group has passed every check (D3: the groups are written one at a time).

**The length tolerance.** A result is **within the length tolerance** when it is within these bounds:

| Case | `analyze` `duration_ms` | Seam `frames` (3.5) |
|---|---|---|
| every case except `mp3` and `m4a_aac` | `abs(duration_ms − floor(length × 1000 / sample_rate)) ≤ 1` (1 ms) | `abs(frames − length) ≤ floor(sample_rate / 1000)` (44 frames at 44.1 kHz, 48 at 48 kHz, 96 at 96 kHz; 1 ms) |
| `mp3` | `≤ 27` ms (one codec frame, 1152 × 1000 / 44 100 = 26.1 ms, rounded up) | `≤ 1152` frames |
| `m4a_aac` | `≤ 24` ms (one codec frame, 1024 × 1000 / 44 100 = 23.2 ms, rounded up) | `≤ 1024` frames |

The `mp3` and `m4a_aac` rows allow the ± 1 codec frame that `lossy-fixtures.md` section 3 check 1 allows, because the decoder may trim a codec delay. Their millisecond bound is one frame rounded up, because `floor(a) − floor(a − b) ≤ ceil(b)`. Elsewhere in this spec and in `memory.feature`, "within 1 ms of the length" means this tolerance, and the two lossy exceptions are stated where they apply.

### 4.1 Gate suite (CI)

| Case | Format | Short | Long | Long file (about) |
|---|---|---|---|---|
| `wav` | WAV PCM 16-bit, 44 100 Hz, stereo (`hound`); independent white Gaussian noise per channel at −12 dBFS RMS, clamped to 16 bits | 1 323 000 frames (30.000 s) | 13 230 000 frames (300.000 s) | 52.9 MB |
| `flac` | FLAC 16-bit, 44 100 Hz, stereo (`flacenc`); independent white Gaussian noise per channel at about −42 dBFS RMS (the level of `typical`, performance.md 2.1; any level from −45 dBFS up is allowed); whole 4096-frame blocks | 322 blocks, 1 318 912 frames (29.907 s) | 3229 blocks, 13 225 984 frames (299.909 s) | 33.6 MB |
| `flac_unknown_length` | The `flac` pair with the STREAMINFO total-samples field set to 0 (as `lp16k_unknown_length`, `fixtures.md`); the audio frames are unchanged. `analyze` decodes it twice (rules.md section 1.2) | as `flac` | as `flac` | 33.6 MB |
| `mp3` | MP3 (writer of `lossy-fixtures.md` 1.1), MPEG-1 Layer III, 44 100 Hz, mono, **320 kbps** CBR, an `Info` tag frame that declares its audio frames, coded edge near 20 000 Hz | 1148 audio frames (1 322 496 samples, 29.989 s) | 11 484 audio frames (13 229 568 samples, 299.990 s) | 12.0 MB |
| `m4a_aac` | M4A, AAC-LC (writer of 1.2), 44 100 Hz, mono, **256 kbps**, edge near 20 000 Hz, `moov` before `mdat` | 1292 frames (`mdhd` duration 1292 · 1024) | 12 920 frames (`mdhd` duration 12 920 · 1024) | 9.7 MB |
| `m4a_alac` | M4A, ALAC (writer of 1.2), 44 100 Hz, 16-bit, mono, uncompressed frames of 4096 samples of `full_band`-style noise. The writer (`write_m4a_alac`) requires a whole number of 4096-sample frames, so both lengths are multiples of 4096 | 323 frames, 1 323 008 samples (30.000 s) | 3229 frames, 13 225 984 samples (299.909 s) | 26.5 MB |
| `opus` | Opus (encoder of 1.3, Ogg writer of 1.4), **stereo** (seeds `s`, `s + 1`), **256 kbps**, `Fullband`, 20 ms frames, pre-skip 120, 50 packets per page | 1 440 000 samples per channel, 1501 packets | 14 400 000 samples per channel, 15 001 packets | 9.7 MB |

The last column is rounded from F4 (section 12), which lists the exact sizes of both files of every pair.

**RNF-04 cases.**
- **`flac_truncated`:** the long `flac` file cut to its first `floor(size / 2)` bytes. Its STREAMINFO still declares 13 225 984 frames. Expected result: `corruptStream`. This was first read from the code, and F5 confirmed it: one decoding pass ends with `CorruptStream("stream ended after 6610944 of 13225984 frames")`. In `decode.rs`, a probe failure, a `next_packet` or `decode` error, and a stream that ends (EOF) before its declared length all map to `TrackError::CorruptStream`. `Unreadable` comes only from `File::open`. The existing test `corrupt_is_error` (`tests/detection_lossless.rs`, `fixtures.md` `corrupt`: a FLAC cut after 16 KiB) already pins a truncated FLAC to `CorruptStream`.
- **`flac_huge_length`:** the short `flac` file with the STREAMINFO total-samples field set to **68 719 476 735** (2^36 − 1, the field's largest value). Expected result: `corruptStream` (the stream ends before its declared length). It checks that nothing is allocated in proportion to a declared length: 2^36 − 1 mono `f32` frames would be 275 GB. F5 confirmed the error: `CorruptStream("stream ended after 1318912 of 68719476735 frames")`, and F1 measured its child at 1.5 MB (working set) and 2.9 MB (private) over the baseline.

**Empty state.** **`wav_empty`** is a WAV whose 16-bit, 44 100 Hz, stereo header is valid and whose `data` chunk has 0 bytes.
- **Expected result:** `ok`, with `duration_ms` 0 and no spectrum (`AudioProbe.spectrum` is `None`). The seam (3.5) returns `Ok` with 0 chunks, 0 frames and a largest chunk of 0, and the report's `max chunk frames` cell is `0`.
- **How it was determined.** By reading the code (symphonia 0.6.1 and the adapter), not by running it:
  1. `symphonia-format-riff` 0.6.1 accepts a 0-byte `data` chunk and sets the track's `num_frames` to `Some(0)`. Its `next_packet` returns `Ok(None)` at once.
  2. `SymphoniaAnalyzer::analyze` treats `Some(0)` as an undeclared length. Both passes decode 0 frames, and `ended_short(0, 0, 0)` is false.
  3. `BlockCollector::new` plans no block for a length of 0, so `finish` returns `None`.
  4. `duration_ms` is `0 × 1000 / 44 100 = 0`.
- **Measured.** F5 confirmed the seam's result (0 chunks, 0 frames), and F1 measured the child at 1.9 MB (working set) and 0.28 MB (private) over the baseline.
- **In the app.** The adapter does not return `tooShort`. `fakeflac-core`'s `track_guard` turns the 0 ms probe into `tooShort`, but that is outside the adapter call measured here.
- **If the test stage observes a different outcome**, it records it in the PR and the spec returns to G1. The expected result is not widened.

### 4.2 Long suite (`just memory`)

| Case | Format | Short | Long | Long file (about) |
|---|---|---|---|---|
| `flac_1h` | As `flac` | the `flac` short | 38 759 blocks, 158 756 864 frames (3599.929 s) | 400 MB |
| `opus_1h` | As `opus` | the `opus` short | 172 800 000 samples per channel, 180 001 packets | 115 MB |
| `flac_hires` | FLAC **24-bit, 96 000 Hz**, stereo (`flacenc`), independent white Gaussian noise per channel at −45 dBFS RMS (the level of performance.md's `hires`); whole 4096-frame blocks | 703 blocks, 2 879 488 frames (29.995 s) | 7031 blocks, 28 798 976 frames (299.989 s) | 138 MB |

The long suite writes one case group (a short and a long file) at a time and deletes it after its children have run (D3).

## 5. The suites

### 5.1 Gate suite (CI, Q2)

These are normal (non-ignored) tests in `crates/fakeflac-audio/tests/memory.rs`. `just test`, `just check` and CI's `test-rust` run them.
- **The driver.** One driver test, `memory_gate` (5.1.1), writes the fixtures, runs the checks of A1 and A5, and starts the children.
- **Platforms.** Starting children is `#[cfg(windows)]` inside the driver. On other platforms the driver writes each group, runs A1 and A5, and starts no child.
- **Other tests.** The pure-function tests (section 7) and the wiring test (A9) are separate tests. They run on every platform.

The measurements are listed below in the order of the report (section 6). The driver measures them group by group (D3), and the report lists them by row number, whatever the measurement order.

| Row | Case | Operation | Files |
|---|---|---|---|
| 1–2 | baseline | none | – |
| 3–16 | `wav`, `flac`, `flac_unknown_length`, `mp3`, `m4a_aac`, `m4a_alac`, `opus` | `analyze` | short, long |
| 17–18 | `flac` | `spectrogram` | short, long |
| 19 | `flac_truncated` | `analyze` | the truncated long file |
| 20 | `flac_huge_length` | `analyze` | – |
| 21 | `wav_empty` | `analyze` | – |

Growth pairs: each case of rows 3–18 (short against long), and `flac_truncated` against the `flac` short `analyze` (row 5). The ceiling applies to **every** measured child of rows 3–21, the short files included (A4).

#### 5.1.1 Serial execution and fixture folders

- **One serial driver.** Every step that writes a fixture or starts a child runs inside the single test function `memory_gate`. It runs the baselines and then the groups in sequence. It starts one child at a time and waits for it to exit before it starts the next. The long suite is likewise the single ignored test `memory_long` (5.2).
  - No other test of `memory.rs` writes a fixture or starts a child. `memory_child` is the child itself; it reads its fixture and writes nothing.
  - The pure-function and wiring tests write nothing, so they may run in parallel with the driver.
- **Why one driver, not a mutex.** A `static Mutex` serialises tests only inside one process. That is enough under `cargo test` (`just test`: test threads of one process), but not under `cargo nextest run` (CI's `test-rust`: every test in a process of its own). A single driver is serial under both.
- **Other test binaries.** They may run at the same time as the driver (nextest runs tests of every binary in parallel). That does not change a child's figures, because both metrics are peaks of the child's own process (3.1).
- **Unique fixture folders.** The driver's fixture root is `<CARGO_TARGET_TMPDIR>/memory/<driver>-<pid>/Memoria ″Ñandú″/`.
  - `<driver>` is `gate` or `long`, which makes the folder unique per test inside one process (`cargo test` threads).
  - `<pid>` is the parent's `std::process::id()`, which makes it unique per process (nextest processes, or two runs at once).
  - Before its first group, the driver deletes `<driver>-<pid>` if it already exists (a folder left by a crashed run whose PID was reused). A drop guard deletes it at the end, pass or fail.
  - The driver does not use the shared, cached fixture folder of `tests/common` (`fixture_dir`, kept between runs of the same build).
  - Children get the full path of their file (D4), and they only read it.
- **D3 holds.** Inside the one driver, the groups are written, measured and deleted one after the other, and A1 checks that the folder is empty before each group.

### 5.2 Long suite (opt-in, Q6)

The long suite is the single ignored driver test `memory_long` in the same file (5.1.1). It measures `flac_1h` (`analyze` and `spectrogram`), `opus_1h` (`analyze`) and `flac_hires` (`analyze`), each with its own short file and its own two baseline children, and prints its report (section 6) with `suite: long`.

### 5.3 The recipe `just memory` (ADR-0019)

```
# RNF-02 long memory suite (specs/contracts/memory.md). Release build, opt-in: not part of
# check, ci or any workflow. Prints the long-suite report.
memory:
    cargo test --release -p fakeflac-audio --test memory -- --ignored --nocapture --test-threads=1
```

The recipe takes no arguments. It is not added to `ci`, `check`, `check-fast`, `test` or `check-app`, and no workflow calls it.

### 5.4 Time budget (indicative, not asserted)

- **Gate suite:** a budget of **120 s** for its whole run in `just test`.
  - F3 (section 12) measured about 55 s for an equivalent pass in the dev profile on the development machine, so 120 s leaves a margin of about 2× for a slower `windows-latest` runner. The 30 s and 300 s pairs are kept; they need not be shortened.
  - The CI duration of `test-rust` with the suite is recorded in the feature PR against this budget.
  - If the budget is exceeded, the test stage may shorten the pairs to about 20 s and 200 s, keeping 10× the length, whole blocks (whole 4096-sample frames for `m4a_alac`) and A1's 4 MiB. It then updates the frame counts of 4.1 in its report.
- **Long suite:** no budget. It is opt-in.

## 6. The report

The report is printed to stdout so it can be pasted into a PR. One function builds it from the figures, so it can be tested with given figures (A8).

```
RNF-02 memory report (specs/contracts/memory.md)
build: <release | debug>
suite: <gate | long>
baseline: private <B> B, working-set <B> B | repeat: private <B> B, working-set <B> B
| case | op | length | file B | result | max chunk frames | private B | working-set B | private over baseline B | working-set over baseline B | ceiling B | ceiling |
|---|---|---|---|---|---|---|---|---|---|---|---|
```

- **First table.** One row for each measured child of 5.1 or 5.2, rows 3 onward, in that order:
  - `length` is the nominal length (`30 s`, `300 s`, `3600 s`, or `0 s` for `wav_empty`);
  - `result` is `ok` or the error code;
  - `max chunk frames` comes from the seam (A5), is `-` for error rows, and is `0` for `wav_empty`;
  - the last column is `pass` or `FAIL`. A row is `FAIL` when either metric is above its ceiling.
- **Second table:**

```
| case | op | metric | short B | long B | growth B | bound B | growth |
|---|---|---|---|---|---|---|---|
```

  It has one row for each growth pair and metric (`private` first, then `working-set`). `growth` is `pass` or `FAIL`.
- **Last line:** `result: pass`, or `result: FAIL (<case> <op> <check> <metric>, …)`. It lists every failed check in table order, with `<check>` being `ceiling` or `growth`. Example: `result: FAIL (opus analyze growth private, opus analyze growth working-set)`.
- **A failed suite.** A failed check makes the suite fail after the whole report is printed.

## 7. Pure functions of `memory.rs` (exact messages)

| Function | `Ok(…)` when | `Err` text |
|---|---|---|
| `check_growth(id: &str, metric: &str, short: u64, long: u64, bound: u64) -> Result<(), String>` | `long.saturating_sub(short) <= bound` | `memory: <id> <metric> grew by <long − short> bytes, bound <bound>` |
| `check_ceiling(id: &str, metric: &str, peak: u64, baseline: u64, ceiling: u64) -> Result<(), String>` | `peak.saturating_sub(baseline) <= ceiling` | `memory: <id> <metric> is <peak − baseline> bytes over the baseline, ceiling <ceiling>` |
| `check_fixture_growth(case: &str, short_bytes: u64, long_bytes: u64, min: u64) -> Result<(), String>` | `long_bytes.saturating_sub(short_bytes) >= min` | `memory: fixture <case> grows by <long_bytes − short_bytes (0 if negative)> bytes, need at least <min>` |
| `parse_peaks(text: &str) -> Result<(u64, u64), String>` | `text` with surrounding whitespace trimmed is two decimal `u64`s separated by exactly one space: `Ok((working_set, private))` | `memory: cannot read the peaks from "<text as given>"` |

- **`id` and `metric`.** `id` is `<case> <op>` (for example `opus analyze`), and `metric` is `private` or `working-set`.
- **Numbers in messages** are plain digits. Example: `memory: opus analyze private grew by 2000000 bytes, bound 1048576`.

## 8. If a gate fails

The implement stage may change the code, but only in ways that keep every result identical:
- For every fixture of `fixtures.md`, `lossy-fixtures.md` (Opus included) and section 4, the `AudioProbe` and the `TrackAnalysis` are equal bit for bit, so `EXTRACTION_VERSION` stays 2.
- The `Spectrogram` is equal byte for byte.
- `DetectionParams`, `algorithm_version` and the `TrackError` reported for every error fixture (including the `opus.feature` errors R1–R22) do not change.
- The existing test suite stays green unchanged.
- `just accept` on the corpus gives the same `summary.md` figures as before the change (recorded in the PR in aggregates only, ADR-0022).

**Allowed levers** (examples):
- the two-pass Opus reader of Q4;
- opening a symphonia source as non-seekable when a reader is found to build an index while reading, as `FromOffset` already does for MP3;
- releasing or reusing buffers;
- not reserving capacity from a declared length;
- reading the MP4 header without keeping the whole `moov` box.

**Not allowed in RNF-02.** Each of these needs G1, and most need an ADR:
- seeking to the sampled blocks instead of decoding the whole stream (section 1, Out);
- fewer segments or a smaller FFT (`DetectionParams`);
- fewer workers (ADR-0034);
- a new dependency.

If the allowed levers do not meet a gate, the implement stage stops and reports the figures, and the spec returns to G1.

## 9. Layers

- **Test stage** writes `crates/fakeflac-audio/tests/memory.rs`: the gate suite as the single serial driver `memory_gate`, `memory_child`, the long suite as the single ignored driver `memory_long` (5.1.1), the pure functions and their tests, and the wiring check A9. It may extend the writers in `crates/fakeflac-audio/tests/common/` with a length parameter. Existing fixtures must keep their bytes.
- **Reds at the `test(RNF-02)` commit:**
  - the compile, because `decode_chunks_for_test` and `ChunkStats` do not exist yet;
  - once the seam exists, A3 for `opus` (both metrics; measured in F1);
  - A9, because the `memory` recipe is missing.

  Nothing else may be red: F1 found every other growth and ceiling of 5.1 within its bound against today's code. If another gate is red on the test stage's run, the test stage records it in the PR and the implement stage applies section 8.
- **Implement stage:**
  - the seam (3.5) in `crates/fakeflac-audio/src/lib.rs`;
  - the Opus reader (`crates/fakeflac-audio/src/opus.rs`, Q4);
  - the `memory` recipe in `justfile` (5.3);
  - any other lever of section 8.

  It does not edit `memory.rs` or `tests/common/`.
- **Frontend, bindings, `commands.md`, `DetectionParams`:** unchanged (D7).

## 10. Acceptance (measurable)

**Automated in `just test` and CI** (`memory.rs`; A3, A4 and A6 measure on Windows):
- **A1 (fixtures, one group at a time).**
  - Every fixture matches its row of 4.1: format, sample rate, channels, and frames or packets.
  - For every pair, `check_fixture_growth(case, short, long, 4194304)` returns `Ok`.
  - No child is started on a group before that group has passed these checks.
  - Before each group is written, the driver's fixture folder (5.1.1) holds no file.
- **A2 (real work).**
  - Every intact `analyze` child returns `ok` with `duration_ms` within the length tolerance of section 4: within 1 ms of its fixture's length, except `mp3` (within 27 ms, one codec frame of 1152 samples) and `m4a_aac` (within 24 ms, one codec frame of 1024 samples).
  - Every `spectrogram` child returns `ok` with width 1200 and height 512.
  - `flac_truncated` and `flac_huge_length` return `corruptStream`.
  - `wav_empty` returns `ok` with `duration_ms` 0 (4.1, Empty state).
  - A child that panics, crashes or prints no result fails the suite.
- **A3 (growth).** For every growth pair of 5.1 and both metrics, `check_growth(id, metric, short, long, 1048576)` returns `Ok`.
- **A4 (ceiling).** For every measured child of rows 3–21 (the short and the long file of every case, `flac_truncated`, `flac_huge_length` and `wav_empty`) and both metrics, `check_ceiling(id, metric, peak, baseline, 33554432)` returns `Ok`.
- **A5 (chunks, every platform).** For every intact file of 4.1, `decode_chunks_for_test`:
  - returns `max_chunk_frames` ≤ 65 535;
  - returns, for the long file, the same `max_chunk_frames` as for the short one;
  - returns `frames` within the length tolerance of section 4: within 1 ms (`floor(sample_rate / 1000)` frames) of the fixture's length, except `mp3` (within 1152 frames) and `m4a_aac` (within 1024 frames).

  For `flac_truncated` it returns `Err(CorruptStream(_))`. For `wav_empty` it returns `Ok` with 0 chunks, 0 frames and `max_chunk_frames` 0.
- **A6 (a corrupt track, RNF-04).** `flac_truncated` meets A3 against the `flac` short `analyze` and meets A4. `flac_huge_length` meets A4.
- **A7 (Unicode, RNF-09).** Every fixture is written and measured under the names of D4. A child that cannot open its path fails A2.
- **A8 (pure functions and report, every platform), with given values:**
  - `check_growth("opus analyze", "private", 1000000, 3000000, 1048576)` returns exactly `Err("memory: opus analyze private grew by 2000000 bytes, bound 1048576")`;
  - `check_growth(…, 1000000, 2048576, 1048576)` returns `Ok(())` (inclusive);
  - `check_growth(…, 3000000, 1000000, 1048576)` returns `Ok(())` (a negative difference counts as 0);
  - `check_ceiling("flac analyze", "working-set", 50000000, 10000000, 33554432)` returns exactly `Err("memory: flac analyze working-set is 40000000 bytes over the baseline, ceiling 33554432")`;
  - `check_ceiling(…, 43554432, 10000000, 33554432)` returns `Ok(())`;
  - `check_fixture_growth("opus", 9000000, 12000000, 4194304)` returns exactly `Err("memory: fixture opus grows by 3000000 bytes, need at least 4194304")`;
  - `parse_peaks("123 456\r\n")` returns `Ok((123, 456))`;
  - `parse_peaks("123")`, `parse_peaks("")` and `parse_peaks("12 x")` return exactly `Err("memory: cannot read the peaks from \"123\"")`, `Err("memory: cannot read the peaks from \"\"")` and `Err("memory: cannot read the peaks from \"12 x\"")`;
  - the report function, given figures where only the `opus` `analyze` private growth is 2 000 000 bytes, prints both tables in full, a `FAIL` in that growth row, and last line `result: FAIL (opus analyze growth private)`;
  - with every figure inside its bound, the last line is `result: pass`;
  - with a `flac_huge_length` private peak 33 554 433 bytes over the baseline, the last line is `result: FAIL (flac_huge_length analyze ceiling private)`.
- **A9 (wiring).** `memory.rs` reads `justfile` as text and checks:
  - a `memory` recipe exists;
  - its body holds `--release`, `--ignored`, `--test memory` and `--test-threads=1`;
  - none of the recipes `ci`, `check`, `check-fast`, `test` and `check-app` names `memory` as a whole word (`\bmemory\b`) in its dependency list or body (the same parsing rules as performance.md T14);
  - neither `.github/workflows/ci.yml` nor `release.yml` contains `just memory`.

**Opt-in** (`just memory`, release, local; the report goes in the feature PR):
- **A10 (long tracks).** `flac_1h` (`analyze` and `spectrogram`) and `opus_1h` (`analyze`) each meet the growth bound of 1 MiB against their 30 s files and the ceiling of 32 MiB. `flac_hires` meets the growth bound against its own short file and the ceiling of **64 MiB** (67 108 864 bytes). The A10 figures are not measured yet (F6). The 64 MiB ceiling is provisional: the first `just memory` report (M2) confirms it, or the spec returns to G1 with that report.

**Manual** (owner, release build of the app; aggregates only, ADR-0022):
- **M1 (the app on a real folder, Q7).**
  1. Start the app and run `Get-Process -Name FakeFlac | Select-Object PeakWorkingSet64, PeakPagedMemorySize64` in PowerShell (P0).
  2. Clear the result cache, analyse a real folder that holds a 15-track album and the longest file available (at least 40 minutes), and run the command again (P1).
  3. Open the spectrogram of the longest file, and run the command again (P2).
  4. Record the worker count, the track count, the folder's total size in MB, and P1 − P0 and P2 − P1 for both metrics.

  Indicative targets: P1 − P0 ≤ `workers × 32 MiB + 64 MiB`, and P2 − P1 ≤ 32 MiB. They never fail.
- **M2 (the long suite on record).** The report of `just memory` on the owner's machine is pasted into the feature PR.

## 11. What the code does today (read from the code; the measured figures are in section 12)

- **Analysis** (`lib.rs`, `decode.rs`, `spectrum.rs`) streams.
  - `MonoStream::for_each_chunk` decodes packet by packet. It mixes to mono into two buffers that are reused across packets and hands each packet to `BlockCollector::push`.
  - The collector keeps only the 64 sampled blocks of `N + 3 · N/2` frames, reserved up front: 64 × 10 240 × 4 B = 2.6 MB at 44.1/48 kHz (N = 4096), 5.2 MB at 88.2/96 kHz (N = 8192) and 10.5 MB at 176.4/192 kHz (N = 16 384).
  - It also keeps the per-frame band levels of at most 256 frames (256 × 220 bands × 8 B = 450 KB at 44.1 kHz; 1.97 MB at 192 kHz).
  - None of this depends on the track's length. A stream of undeclared length is decoded twice, and both passes stream.
- **Spectrogram** (`spectrogram.rs`) streams. It keeps a sliding buffer of less than `fft_size` plus one packet, and its output matrix is bounded by `max_width × height` (1200 × 512 = 614 400 bytes).
- **MP3 header** (`mp3.rs`): a 1 MiB + 8 KiB search window and a frame walk through a `BufReader`. Both are constant in the track's length. The symphonia MP3 reader gets a non-seekable source (`FromOffset`).
- **MP4 header** (`mp4.rs`) reads the whole `moov` box into a `Vec` (up to `MAX_MOOV_BYTES`, 256 MiB) and frees it before decoding. Its sample tables grow with the length, at about 4 bytes per AAC frame (`stsz`). symphonia's `isomp4` reader keeps its own sample tables while decoding. This is allowed by Q8. F1 measured the `m4a_aac` growth from 30 s to 300 s at 69 632 bytes (working set) and 94 208 bytes (private), within the bound, and A3 gates it at 300 s.
- **Opus** (`opus.rs`). `OpusStream::open` reads every page and keeps a `Vec<Packet>`, one per audio packet (every 20 ms). Each `Packet` holds a `Vec<Span>` (at least a 64-byte allocation), a `Vec<u8>` TOC (at least 8 bytes) and a frame count: about 150–200 bytes per packet with the allocator's overhead. That is 7.5–10 KB per second of audio (about 2–2.7 MB from 30 s to 300 s). **F1 measured A3 failing for `opus`**: a growth of 2 142 208 bytes (working set) and 2 248 704 bytes (private). A10 is expected to fail for `opus_1h` too (27–36 MB, estimated; F6). Decoding itself streams: each packet's bytes are read from the file when it is decoded. Q4 decides the change.
- **symphonia's FLAC, WAV and ALAC readers.** The repository's code does not show whether they retain anything per packet (for example a seek index on a seekable `File` source). F1 found no such growth at 300 s: at most 65 536 bytes for `wav`, none for `flac` and `flac_unknown_length` (their long peaks are lower), and at most 94 208 bytes for `m4a_alac`. Section 8 still allows a non-seekable source if a longer track shows one (F6).
- **Docs.**
  - `backend.md` section 5 says "reused window buffers → bounded memory (RNF-02)". The block buffers are allocated once per track, not reused across tracks. The memory is still bounded, so this is only wording.
  - `overview.md`'s RNF-02 row names no spec.

  Section 13 lists the edits.

## 12. Figures

**Status.** F1–F5 were measured before G1, and they back the growth bound, the ceiling, the CI budget, A1's 4 MiB and A5 (Q3, 5.4). With them the spec is **Ready for G1**. F6 and F7 are not measured. They are measured later: F6 by the opt-in `just memory` run whose report goes in the feature PR (M2), F7 by hand (M1). Neither feeds a CI gate. Until F6 is measured, the A10 bounds, and the 64 MiB hi-res ceiling in particular, are provisional, and a figure of F6 that contradicts them sends the spec back to G1 with the report. Every figure is recorded as an aggregate of synthetic fixtures (no corpus names or paths, ADR-0022).

| # | Figure | Feeds | Status |
|---|---|---|---|
| F1 | Both peaks of every child of 5.1 (short and long) against today's code | Q3 bounds, Q4, A3, A4 | Measured (12.2) |
| F2 | Noise: the spread (largest minus smallest) of repeats of the same child, each in a fresh process | Q3 (bound 1 MiB) | Measured (12.3) |
| F3 | Wall time of the gate suite (dev profile), and of fixture generation | 5.4 (120 s) | Measured (12.4) |
| F4 | File sizes of every pair, and the growth that A1 checks | A1 (4 MiB) | Measured (12.5) |
| F5 | Chunks per format: `max_chunk_frames`, chunks and frames of one decoding pass | A5 (65 535), the length tolerance of section 4 | Measured (12.5) |
| F6 | The peaks of the long suite (A10) | A10 bounds, 64 MiB hi-res ceiling (provisional) | **Not measured.** Measured in the opt-in `just memory` run, whose report goes in the feature PR (M2). Expected: like F1; hi-res about 3 MB more than 44.1 kHz; `opus_1h` fails until Q4 is done (27–36 MB, estimated) |
| F7 | M1 on the owner's machine (the whole app) | M1 indicative targets | **Not measured.** Manual (M1), recorded in the feature PR or later |

### 12.1 How F1–F5 were measured

- **Machine and build.** One 16-core Windows 11 development machine, dev/test profile (as `just test` builds), today's code (no seam, no Opus change).
- **Harness.** A throwaway harness outside the repository's tests, following section 3: synthetic fixtures of section 4.1 (with the `m4a_alac` lengths of that table), **one fresh child process per analysis**, the child reading its own `PeakWorkingSet64` and `PeakPagedMemorySize64` through `Get-Process` (3.3). The chunk figures (F5) come from an in-process pass equivalent to the seam of 3.5.
- **Units.** Peaks over the baseline are in MB (10^6 bytes), rounded to 0.01 MB as measured. Growths and spreads are in bytes. "Growth" is the 300 s peak minus the 30 s peak, shown signed here; a negative growth counts as 0 in A3. Windows reports peaks in steps of 4 KiB.

### 12.2 F1: peaks against today's code

Baseline child (no call): working set 5.54 MB, private 0.78 MB.

| Case | Op | Working set over baseline, short / long (MB) | Private over baseline, short / long (MB) | Growth, working set (B) | Growth, private (B) | A3 |
|---|---|---|---|---|---|---|
| `wav` | analyze | 4.40 / 4.47 | 3.58 / 3.65 | +65 536 | +65 536 | pass |
| `flac` | analyze | 4.96 / 4.93 | 3.76 / 3.73 | −32 768 | −28 672 | pass |
| `flac_unknown_length` | analyze | 4.96 / 4.95 | 3.76 / 3.74 | −16 384 | −24 576 | pass |
| `mp3` | analyze | 5.24 / 5.29 | 4.20 / 4.20 | +49 152 | 0 | pass |
| `m4a_aac` | analyze | 5.36 / 5.43 | 3.79 / 3.89 | +69 632 | +94 208 | pass |
| `m4a_alac` | analyze | 5.08 / 5.12 | 3.66 / 3.76 | +40 960 | +94 208 | pass |
| `opus` | analyze | 5.53 / 7.67 | 3.87 / 6.12 | **+2 142 208** | **+2 248 704** | **FAIL** (Q4) |
| `flac` | spectrogram | 2.20 / 2.19 | 1.10 / 1.09 | −8 192 | −8 192 | pass |

| Case (rows 19–21) | Working set over baseline (MB) | Private over baseline (MB) |
|---|---|---|
| `flac_truncated` | 2.6 | 2.9 |
| `flac_huge_length` | 1.5 | 2.9 |
| `wav_empty` | 1.9 | 0.28 |

What F1 shows:
- **Growth (A3).** Every non-Opus growth is at most 94 208 bytes, 9 % of the 1 MiB bound. `flac_truncated` peaks below the `flac` short `analyze` child, so its growth (A6) is 0. The `m4a_aac` sample-table growth (Q8) is real but small. `opus` grows by about 2 × the bound on both metrics, which confirms Q4: the two-pass reader is justified, and the `opus` growth scenario is the expected red of section 9.
- **Ceiling (A4).** The worst child is `opus` long: 7.67 MB of working set and 6.12 MB of private over the baseline, so the 32 MiB ceiling (33 554 432 bytes) has a margin of about 4×. Every other child is at most 5.53 MB over the baseline (`opus` short).

### 12.3 F2: noise

Five repeats of each child, each in a fresh process. Spread (largest minus smallest), in bytes:

| Child | Working set | Private |
|---|---|---|
| baseline | ≤ 69 632 | ≤ 20 480 |
| `flac` analyze, long | 49 152 | 57 344 |
| `flac` analyze, short | 86 016 | 90 112 |
| `opus` analyze, long | 77 824 | 94 208 |
| `opus` analyze, short | 73 728 | 73 728 |
| `flac` spectrogram | ≤ 8 192 | ≤ 40 960 |

- The largest spread of any child is 94 208 bytes (92 KiB), under the 256 KiB that would have sent the bound back to G1.
- The growth of a pair adds the noise of two children, so its worst noise is about 190 KiB, far below 1 MiB. A bound under about 512 KiB would risk flakes; 1 MiB does not.

### 12.4 F3: time

- One whole pass of the gate suite's equivalent took **45.4 s** in the dev profile:
  - fixture generation about 16 s in total, of which each long file took 1.2–4.2 s (`opus` long 4.2 s, `m4a_alac` long 3.4 s);
  - about 21 children at about 1.4 s each, dominated by the start-up of `powershell.exe`.
- The in-process chunk passes (A5) add about 7 s, so the gate suite is estimated at **about 55 s** on this machine.
- Against the **120 s** budget of 5.4 that is a margin of about 2× for a slower `windows-latest` runner. The budget is kept, and the pairs are not shortened.

### 12.5 F4 and F5: fixture sizes and chunks

| Case | Short file (B) | Long file (B) | Growth (B) | `max_chunk_frames` | Chunks, 300 s | Frames, 300 s | Fixture length, 300 s |
|---|---|---|---|---|---|---|---|
| `wav` | 5 292 044 | 52 920 044 | 47 628 000 | 1152 | 11 485 | 13 230 000 | 13 230 000 |
| `flac` | 3 352 748 | 33 625 372 | 30 272 624 | 4096 | 3229 | 13 225 984 | 13 225 984 |
| `mp3` | 1 200 587 | 12 000 653 | 10 800 066 | 1152 | 11 484 | 13 229 039 | 13 229 568 |
| `m4a_aac` | 965 749 | 9 652 314 | 8 686 565 | 1024 | 12 920 | 13 230 080 | 13 230 080 |
| `m4a_alac` | 2 649 173 | 26 478 373 | 23 829 200 | 4096 | 3229 | 13 225 984 | 13 225 984 |
| `opus` | 968 223 | 9 669 513 | 8 701 290 | 960 | 15 001 | 14 400 000 | 14 400 000 |

- **F4 (A1).** `flac_unknown_length` has the sizes of `flac` (only a header field differs). The tightest growths are `m4a_aac` (8.69 MB) and `opus` (8.70 MB), about 2× A1's 4 MiB (4 194 304 bytes).
- **F5 (A5).** Every largest chunk is far below 65 535 frames. WAV chunks are 1152 frames (symphonia's packet size for this WAV). `mp3` decodes 1 321 967 frames for the short file (1 322 496 written) and 13 229 039 for the long one, 529 frames short of each, within the 1152-frame tolerance. At 300 s every other case decodes exactly its fixture's length.
- **Error and empty cases.** `wav_empty`: 0 chunks, 0 frames. `flac_truncated`: `CorruptStream("stream ended after 6610944 of 13225984 frames")`. `flac_huge_length`: `CorruptStream("stream ended after 1318912 of 68719476735 frames")`.

## 13. Edits to approved specs and docs

- **In this spec PR:**
  - `requirements/RNF-02.md`: status `specified` and its measurable acceptance;
  - `traceability.md`: the RNF-02 row.
- **Outside `specs/`.** These edits are not part of this spec PR. None of them is done by the spec, test or implement stage, which may not edit these files. Each row names who does it:

  | Edit | Done by | When |
  |---|---|---|
  | The ADR of Q9 (`/new-adr`), with the content Q9 requires | Orchestrator (or the human) | Before the `feat/RNF-02` PR opens (required) |
  | `docs/architecture/overview.md` RNF-02 row: "streaming decode, bounded working set per track; criterion and gate: `specs/contracts/memory.md`, `just memory`" | Orchestrator or `/handoff` | With or after the feature PR |
  | `docs/architecture/backend.md` section 5: point "bounded memory (RNF-02)" to this spec and drop "reused" | Orchestrator or `/handoff` | With or after the feature PR |
  | `CLAUDE.md` command list: the `memory` recipe | **Human** (agent configuration; then `just agents-sync`, ADR-0048) | After the implement stage adds the recipe |
  | `.claude/agents/code-reviewer.md`: a diff that touches `crates/fakeflac-audio/src/` must keep `just test` green, including the memory gate; a change to a decoder or a reader also needs a `just memory` report in the PR | **Human** (agent configuration; then `just agents-sync`, ADR-0048) | With or after the feature PR |

  The `memory` recipe in `justfile` is not in this table. The implement stage adds it (section 9).
- **What already guards the risky changes:**
  - **The two-pass Opus reader (Q4)** changes no contract. It is guarded by the existing `opus.feature` scenarios, which stay green unchanged (section 8). These include "Stream information of Opus fixtures", "Pre-skip and end trim are applied", "The output gain of OpusHead is applied to every sample", "A packet split across pages is reassembled", "Verdict of Opus fixtures", "Broken Ogg or Opus structure is corruptStream" (R1–R22), "Recognised but unsupported Opus streams are unsupportedFormat", "A short Opus file is too short" and "The spectrogram of an Opus file". Memory adds only the growth scenario.
  - **M4A per-sample table growth (Q8)** was measured by F1 at 69 632 bytes (working set) and 94 208 bytes (private) from 30 s to 300 s, and A3 gates it at 300 s. The hour-long M4A remains out of scope (section 1).
- **`performance.md` is not edited.** Its D7 ("Memory is RNF-02") stays true.

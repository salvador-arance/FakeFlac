# ADR-0036: Calibration tool: a dev-only workspace crate (`tools/fakeflac-calibrate`) that reuses the core and the adapters, with a JSON Lines spectra cache and ffmpeg as an external process

- Status: proposed
- Date: 2026-10-18

## Context and rationale

RP-11 (risk R3, phase 5) needs `just accept` and `/calibrate`: measure and tune `DetectionParams` against a labelled local corpus (ADR-0011, ADR-0022). ADR-0035 left two known limits: the v2 thresholds were fitted and evaluated on the same data, and its figures come from a Python prototype, not from the Rust pipeline. The tool must:

- use **the same Rust pipeline as the app** (`fakeflac-audio` + `fakeflac-core`);
- never ship in the installer, and respect the crate rules checked by `just lint-arch`;
- cache measurements, so that recalibrating re-applies rules without decoding (about 400 trusted tracks × 9 encoder settings ≈ 4,000 entries);
- call ffmpeg, a development tool that is never part of the product (planning section 1.5, RNF-06);
- write only inside `FAKEFLAC_CALIBRATION`, and never into the corpus or the repository.

Spec: `specs/calibration/` (main file `calibration.md`).

## Decision

1. **Location.** A new workspace member `tools/fakeflac-calibrate` with `publish = false`, a library and a binary `fakeflac-calibrate`. It depends on `fakeflac-core`, `fakeflac-audio` and `fakeflac-fs`, and on nothing else in the workspace. Nothing depends on it, and `src-tauri` does not, so the installer (which bundles only the `src-tauri` binary) never contains it. It runs only through `just accept`, `just calibrate` and `just privacy-check` (ADR-0019), none of which is part of `just ci`. Its tests (sample corpus, no ffmpeg) run in `just test`.
2. **How measurements are exposed: nothing new in the core.** Spectra come from `SpectrumAnalyzer::analyze` → `AudioProbe { spectrum: Option<BandSpectra> }`. Measurements and verdicts come from the public `fakeflac_core::detect::{measure, decide, cutoff_estimate, detect}`. The tool computes margins from `Measurements` and `VerdictParams` as pure functions of its own. The cache uses a mirror of `BandSpectra` in the tool, so the core gains no serde derive and no new port (dependencies.md rule 4). The only addition outside the tool is `pub const EXTRACTION_VERSION: u32 = 2` in `fakeflac-audio`. It is bumped in the same PR as any adapter change that alters the `AudioProbe` produced for the same file and parameters.
3. **Cache.** JSON Lines, one file per **extraction fingerprint** (`EXTRACTION_VERSION` plus the `[input]` and `[fft]` sections), keyed by (relative path, size, mtime) for corpus tracks and by recipe for generated ones. Rule parameters and `algorithm_version` are not part of the key, so v2 and its candidates share one extraction. This departs from ADR-0008's app cache key on purpose (spec OQ-3). Floats round-trip exactly (`serde_json` feature `float_roundtrip`).
4. **ffmpeg** runs as an external process through `std::process::Command`, with an argument list and no shell: `FAKEFLAC_FFMPEG`, or `ffmpeg` on `PATH`. A `Transcoder` trait internal to the tool's library (not a core port) has two implementations: the ffmpeg one, used by the binary, and a fixture one, used by the tests, which writes the synthetic signals of `specs/detection/fixtures.md`. So CI needs no ffmpeg.
5. **Composition root.** Like `src-tauri`, the tool wires adapters to the core. Proposed amendment to `docs/architecture/dependencies.md`, rule 3:
   > 3. `src-tauri` is the composition root of the app. Dev-only tools under `tools/` (ADR-0036) are composition roots too: they may depend on `fakeflac-core` and on the adapters they need. No crate under `crates/`, and not `src-tauri`, may depend on a tool. Tools never ship in the installer.

   Also add a `tools/fakeflac-calibrate` row to the workspace table of `docs/architecture/overview.md`. `scripts/lint-arch.mjs` gains two checks: crates under `tools/` may depend only on `fakeflac-core`, `fakeflac-audio` and `fakeflac-fs` (more adapters need an ADR); and no crate under `crates/`, nor `src-tauri`, may depend on a crate under `tools/`.

### Dependencies (each needs the owner's approval, R11)

| Crate | Kind in `fakeflac-calibrate` | Status in the workspace | Why |
|---|---|---|---|
| `fakeflac-core`, `fakeflac-audio`, `fakeflac-fs` | normal | workspace crates | The app's pipeline and scanner (decision 2) |
| `serde` (derive) | normal | already a workspace dependency | Config, grid and cache structures |
| `serde_json` with the feature **`float_roundtrip`** | normal | crate already in the workspace; the feature is new | Cache lines with bit-exact floats |
| `toml` 1 | normal (in a dev-only binary) | approved as a dev-dependency (ADR-0033) | Read params, config and grid files |
| `flacenc`, `hound`, `rand`, `rand_chacha` | dev-dependencies | approved as dev-dependencies (ADR-0033) | Build the sample corpus from the fixtures |

**No new crate enters `Cargo.lock`.** External tools: ffmpeg with `libmp3lame` and `libopus` (installed by the developer, never shipped) and git (already required).

Deliberately not added: `clap` (a few flags; a hand-written parser is enough); `csv` (the tool only writes CSV, never reads it back, and RFC 4180 quoting is a few lines); `chrono` (the run id comes from `SystemTime`); `sha2`/`twox-hash` (FNV-1a plus the MurmurHash3 finaliser are a few lines and, unlike std's `DefaultHasher`, stable across Rust versions); `tempfile` (tests create temporary folders as the `fakeflac-fs` tests do); `toml_edit` (the proposal is written by replacing lines of the baseline text, then checked by parsing it back).

## Alternatives considered

- **The planned `fakeflac-cli`.** It is meant to become a second end-user entry adapter (overview.md), and it might ship one day. Putting ffmpeg, corpus labels and grid search into it would mix dev-only code into a product binary whose commands are not specified yet. Rejected for now. A future CLI reuses the same core and adapters, not this tool.
- **`xtask`.** Technically equivalent to a crate under `tools/`, but by convention `xtask` is build automation run through a `cargo xtask` alias. This is a domain tool with its own data formats and tests, run through `just`. Rejected for clarity.
- **`examples/` in `fakeflac-audio`.** The example would need `fakeflac-fs` and `toml`/`serde_json` as dev-dependencies of an adapter, which goes against rule 2 (adapters do not depend on each other), and examples are awkward to test. Rejected.
- **Keep the Python scripts.** They do not run the Rust pipeline, which is the problem RP-11 solves. Rejected.
- **Cache in SQLite through `fakeflac-store`,** or a binary format (`bincode`, MessagePack). `fakeflac-store` serves the app's result cache with another key and other contents, and reusing it would couple the two. Binary formats add a crate and cannot be inspected. JSON Lines needs only `serde_json` and survives a crash: at worst the last line is lost and skipped.
- **Key the cache by `algorithm_version`** (as the issue first proposed). Every candidate file has a new `algorithm_version`, so nothing would be reused. Rejected (spec OQ-3).
- **Link libav (`ffmpeg-next`) or bundle ffmpeg.** That means heavy native build dependencies for a dev-only feature. An external process is simpler and keeps ffmpeg out of `Cargo.lock`. Rejected.
- **Re-implement v1 inside the tool** to compare v1 and v2 in one run. v1 used a different extraction (one averaged spectrum, N = 8192, Blackman-Harris), so this means a second pipeline kept only to confirm a result that ADR-0035 already measured with the Rust v1 code. Rejected (spec OQ-1). v1 figures are quoted from ADR-0035.

## Consequences

- `docs/architecture/dependencies.md`, `overview.md` and `scripts/lint-arch.mjs` change as in decision 5. `fakeflac-audio` gains `EXTRACTION_VERSION`, and its `CLAUDE.md` gains the bump rule. Reviewers check that rule, because a forgotten bump serves stale spectra. Deleting `FAKEFLAC_CALIBRATION/features/` is always safe.
- The tool's tests need the fixture signals, which today live in `crates/fakeflac-audio/tests/common/mod.rs`. The test author may include that module by path. Extracting a shared dev-only crate instead would need its own ADR.
- With resolver 2, `cargo test --workspace` unifies features, so `float_roundtrip` is also enabled for other users of `serde_json` in that build (only float parsing precision and speed change). The app's release build (`src-tauri` alone) is not affected.
- `just accept` builds the tool with the release profile (LTO), which is slow to compile once. Accepted for a local, occasional command.
- An agent running `just accept` starts a binary that writes into `FAKEFLAC_CALIBRATION`, outside the repository. The binary checks its write locations at start-up, and agents still cannot use Write/Edit there. No hook changes (owner to confirm at G1, spec OQ-15).
- Only `summary.md` may go into PRs (ADR-0022). `just privacy-check` searches the branch diff for corpus names and the two root paths.

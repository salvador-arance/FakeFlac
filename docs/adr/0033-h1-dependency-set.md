# ADR-0033: H1 dependency set

- Status: proposed
- Date: 2026-10-15

## Context and rationale

H1 needs decoding (WAV, FLAC), a real FFT, parallelism, a recursive scan, a folder dialog, the generated contract and test fixtures. R11 requires every new dependency to be decided explicitly. This ADR lists the H1 set; versions are pinned by major in `Cargo.toml`/`package.json` and locked by `Cargo.lock`/`package-lock.json`. All licences are in the `deny.toml` allow-list.

## Decision

| Crate / package | Where | Why | Covered by |
|---|---|---|---|
| `symphonia` 0.5 (features `flac`, `wav`, `pcm`) | `fakeflac-audio` | Pure-Rust decoding, streaming, seek | ADR-0030 |
| `realfft` 3 (+ `rustfft` 6) | `fakeflac-audio` | Real FFT | ADR-0026 |
| `rayon` 1 | `src-tauri` (use case runner) | Parallel analysis | ADR-0006 |
| `walkdir` 2 | `fakeflac-fs` | Recursive scan without following links | this ADR |
| `tauri-specta` 2 + `specta` 2 (+ `specta-typescript`) | `src-tauri` | Generated TS bindings | ADR-0003 |
| `tauri-plugin-dialog` 2 / `@tauri-apps/plugin-dialog` 2 | `src-tauri`, frontend | Native folder picker (`pick_folder`) with the `dialog:allow-open` capability only | this ADR |
| `toml` (dev-dependency) | `fakeflac-audio` tests | Check that default `DetectionParams` equal `specs/detection/detection-params.v1.toml`. The core stays free of parsers | ADR-0011 |
| `hound` 3 (dev-dependency) | tests, `just fixtures` | Write synthetic WAV fixtures | ADR-0022 |
| `flacenc` (dev-dependency) | tests, `just fixtures` | Pure-Rust FLAC encoder for synthetic FLAC fixtures | ADR-0022 |
| `rand` + `rand_chacha` (dev-dependency) | tests | Seeded noise for reproducible fixtures | ADR-0022 |
| `@storybook/svelte-vite` + `@testing-library/svelte` (dev) | frontend | Component stories and tests (S6) | ADR-0014 |

`DetectionParams` is a plain struct in `fakeflac-core` with a `Default` that mirrors the TOML file; the app does not read the TOML at runtime in v1.

## Alternatives considered

- `ffmpeg` sidecar for decoding: bigger installer, external tool (ADR-0030).
- `claxon` for FLAC: decode-only and FLAC-only; symphonia covers every format we need except Opus.
- `rfd` for the dialog: works, but the official plugin integrates with Tauri capabilities.
- Committing fixture files: rejected by ADR-0022 (generated in code instead).

## Consequences

More crates in the lock file to audit (`cargo deny`, Dependabot). `tauri-specta` is a community crate (risk D1, fallback `ts-rs`). Adding anything not in this list still needs a new ADR.

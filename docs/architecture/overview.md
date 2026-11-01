# Architecture overview

> Hexagonal architecture (ports and adapters) on a Cargo workspace; Tauri v2 is the entry adapter; the frontend is Svelte 5 + TypeScript with atomic design. All computation is in Rust. Every decision is an ADR in `docs/adr/`.

```
Frontend (WebView2): Svelte 5 + TS, atomic design
   │  typed API client (generated from Rust)      ← src/lib/api/
   ▼
src-tauri: Tauri commands, progress Channel, composition root
   ▼
fakeflac-core: domain, use cases, ports (no I/O)
   ▲ implemented by
fakeflac-audio · fakeflac-fs · fakeflac-store · fakeflac-export  (adapters)
```

## Workspace

| Crate | Responsibility | Depends on |
|---|---|---|
| `fakeflac-core` | Domain model, use cases, ports (traits). No I/O, no Tauri | `serde`, `thiserror` only |
| `fakeflac-audio` | Decoding, windowed FFT, band spectra (mean and median), spectrogram | `core` |
| `fakeflac-fs` | Recursive scan (`walkdir`), technical metadata (`lofty`) | `core` |
| `fakeflac-store` | SQLite result cache (`rusqlite`) | `core` |
| `fakeflac-export` | CSV serialisation | `core` |
| `src-tauri` | Commands, channel, wiring | all |
| `fakeflac-cli` (future) | Second entry adapter reusing `core` | `core` + adapters |
| `tools/fakeflac-calibrate` (dev only, ADR-0036) | Calibration and acceptance on the local corpus (`just accept`, `/calibrate`); never shipped | `core`, `audio`, `fs` |

Why hexagonal: the detection core is the most valuable and most delicate asset (R1, R3). Isolating it allows testing with synthetic signals, calibrating without the UI, and a future CLI (ADR-0001).

## Use cases

`AnalyzeFolder` (with the optional result cache), `GetSpectrogram`, `ExportCsv`, `SummarizeByFolder`. See `backend.md`.

## Quality attributes → design

| Requirement | Design decision |
|---|---|
| RF-14 formats | symphonia for WAV, FLAC, MP3 and M4A (ADR-0030, ADR-0040); Opus with `rusty-opus` and an Ogg reader in the adapter (ADR-0047) |
| RF-03/RF-04 detection | Band spectra (power mean and per-frame median), relative cutoff step and 16 kHz gap step (ADR-0035); thresholds as versioned data (ADR-0011) |
| RNF-01 performance | parallel tracks on std scoped threads (ADR-0006, ADR-0034), window sampling (ADR-0010), cache (ADR-0007/0008), real FFT (ADR-0026); budget, fixtures and the opt-in harness `just perf`: `specs/contracts/performance.md`, ADR-0046 |
| RNF-02 memory | Reused window buffers, streaming decode |
| RNF-03 privacy | No network dependencies; Tauri CSP without external `connect-src`; minimal capabilities |
| RNF-04 reliability | Typed per-track errors (ADR-0013) |
| RNF-05 read-only | No write port over audio; cache stored outside the music folder |
| RNF-06 size | No bundled ffmpeg (ADR-0030); `lto` + `strip` in release; size gate `just release-assets` on every PR and release (ADR-0045) |
| RNF-07 platform | WebView2 on Windows |
| RNF-08 usability | Verdict = icon + text + colour (`frontend.md`) |
| RNF-09 Unicode | `PathBuf`/`OsString` in the core; CSV in UTF-8 (ADR-0012) |
| RNF-11 releases | Tagged releases built by `release.yml`, draft published by the human (ADR-0028, ADR-0032, ADR-0045) |

## CI and releases

`ci.yml` runs only `just` recipes: `lint`, `specs`, `test-rust`, `test-web`, `supply-chain` and `build-windows` (`just check-app`, `just build`, `just release-assets`), gated by the single required check `ci-ok`. `release.yml` runs on a `vX.Y.Z` tag pushed by the human (jobs `verify`, `package`, `publish`: draft release with NSIS, MSI and `SHA256SUMS.txt`) or by hand as a dry run (`verify` and `package` only). Details: `specs/contracts/release.md`, ADR-0045.

## Tauri security

Minimal capabilities (folder dialog, read of the chosen path, write only of the destination CSV), restrictive CSP, no `shell`, no open `fs` from the frontend. All disk access happens in Rust.

## Related

- [dependencies.md](dependencies.md) · [backend.md](backend.md) · [data-model.md](data-model.md) · [frontend.md](frontend.md) · [glossary.md](glossary.md)
- Full original design (Spanish): `docs/planning/fakeflac-phases-1-3.es.md`.

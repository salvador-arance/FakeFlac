# Traceability

> Requirement → spec → test → module. Validated by `just trace` (S1, S2). Updated by `docs-keeper` / `/handoff`.

| Requirement | Milestone | Spec | Tests | Module |
|---|---|---|---|---|
| RF-01 folder dialog / drop | H1 | `contracts/commands.md`, `features/analyze-folder.feature`, `ui/main-screen.md` | pending | `src-tauri`, `AnalysisPage` |
| RF-02 recursive scan | H1 | `contracts/commands.md`, `features/analyze-folder.feature` | `crates/fakeflac-fs/tests/scan.rs`, `crates/fakeflac-core/tests/analyze_folder.rs` | `fakeflac-fs`, `fakeflac-core::analyze_folder` |
| RF-03 average spectrum | H1 | `detection/rules.md`, `features/detection-lossless.feature` | `crates/fakeflac-audio/tests/{detection_lossless,window_and_fft}.rs` | `fakeflac-audio` |
| RF-04 cutoff + verdict | H1 | `detection/rules.md`, `detection/detection-params.v1.toml`, `features/detection-lossless.feature` | `crates/fakeflac-audio/tests/detection_lossless.rs`, `crates/fakeflac-core/tests/detect_rules.rs` | `fakeflac-core::{detect,analysis}`, `fakeflac-audio` |
| RF-05 progress + cancel | H1 | `contracts/commands.md`, `features/analyze-folder.feature`, `ui/main-screen.md` | `crates/fakeflac-core/tests/analyze_folder.rs` (use case; UI pending) | `fakeflac-core` (use case), `src-tauri`, `StatusBar` |
| RF-06 results table | H1 | `features/analyze-folder.feature`, `ui/components.md` | pending | `ResultsTable`, `VerdictCell` |
| RF-07 spectrogram | H1 | `contracts/commands.md`, `features/inspect-track.feature`, `ui/components.md` | pending | `fakeflac-audio`, `SpectrogramView` |
| RF-10 album summary | H0 smoke test (H2 feature) | `detection/album-summary.md`, `features/album-summary.feature` | `crates/fakeflac-core/tests/album_summary.rs` (11) | `fakeflac-core::summary` |
| RF-14 formats (WAV, FLAC in H1) | H1 / H2 | `contracts/commands.md`, `features/detection-lossless.feature` | `crates/fakeflac-audio/tests/detection_lossless.rs`, `crates/fakeflac-fs/tests/scan.rs` | `fakeflac-fs`, `fakeflac-audio` |
| other RF | see `specs/requirements/` | pending (`/new-spec`) | — | — |
| RNF-04, RNF-05, RNF-09 | H1 | scenarios in `features/analyze-folder.feature`, `features/detection-lossless.feature` | `crates/fakeflac-core/tests/analyze_folder.rs` (RNF-04), `crates/fakeflac-fs/tests/scan.rs` (RNF-05 scan, RNF-09); app-level pending | all |

Requirements are in `specs/requirements/`. Every `@RF-xx` / `@RNF-xx` tag in `specs/features/*.feature` must exist there.
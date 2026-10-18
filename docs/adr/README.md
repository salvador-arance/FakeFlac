# Architecture Decision Records

Format: MADR-style. New ADRs are created with `/new-adr` and approved at gate G1. ADRs 0001-0032 were migrated from the planning document (`docs/planning/fakeflac-phases-1-3.es.md`, section 3.3).

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-hexagonal-architecture-on-a-multi-crate-workspace.md) | Hexagonal architecture on a multi-crate workspace | accepted |
| [0002](0002-all-computation-in-rust-the-frontend-only-presents.md) | All computation in Rust; the frontend only presents | accepted |
| [0003](0003-single-contract-generated-from-rust-tauri-specta.md) | Single contract generated from Rust (tauri-specta) | accepted |
| [0004](0004-frontend-svelte-5-typescript-vite.md) | Frontend: Svelte 5 + TypeScript + Vite | accepted |
| [0005](0005-progress-through-tauri-ipc-channel-not-global-events.md) | Progress through tauri::ipc::Channel, not global events | accepted |
| [0006](0006-parallelism-with-rayon-and-cooperative-cancellation-atomic-f.md) | Parallelism with rayon and cooperative cancellation (atomic flag/token) | accepted |
| [0007](0007-result-cache-in-sqlite-rusqlite-wal-mode.md) | Result cache in SQLite (rusqlite, WAL mode) | accepted |
| [0008](0008-cache-key-canonical-path-size-mtime-algorithm-version.md) | Cache key = (canonical path, size, mtime, algorithm_version) | accepted |
| [0009](0009-spectrogram-computed-in-rust-sent-as-a-u8-db-matrix-colour-a.md) | Spectrogram computed in Rust, sent as a u8 dB matrix; colour applied in the frontend canvas | accepted |
| [0010](0010-analysis-by-sampling-representative-windows-with-seek-full-d.md) | Analysis by sampling representative windows (with seek); full decode only for the spectrogram | accepted |
| [0011](0011-thresholds-and-rules-are-versioned-data-detectionparams.md) | Thresholds and rules are versioned data (DetectionParams) | accepted |
| [0012](0012-csv-rfc-4180-utf-8-without-bom-comma-separator.md) | CSV: RFC 4180, UTF-8 without BOM, comma separator | accepted |
| [0013](0013-typed-per-track-errors-trackerror-no-panics.md) | Typed per-track errors (TrackError), no panics | accepted |
| [0014](0014-atomic-design-to-organise-the-frontend.md) | Atomic Design to organise the frontend | accepted |
| [0015](0015-fully-agentic-development-with-claude-code-as-the-harness.md) | Fully agentic development with Claude Code as the harness | accepted |
| [0016](0016-human-gates-only-at-spec-adr-g1-and-at-merge-g2.md) | Human gates only at spec/ADR (G1) and at merge (G2) | accepted |
| [0017](0017-separate-roles-for-test-author-and-implementer-protected-tes.md) | Separate roles for test author and implementer; protected tests | accepted |
| [0018](0018-progressive-disclosure-context-short-root-claude-md-nested-c.md) | Progressive-disclosure context: short root CLAUDE.md + nested CLAUDE.md + linked docs/architecture | accepted |
| [0019](0019-justfile-as-the-single-command-entry-point.md) | justfile as the single command entry point | accepted |
| [0020](0020-hooks-written-in-node-mjs.md) | Hooks written in Node (.mjs) | accepted |
| [0021](0021-parallelism-with-git-worktree-one-agent-per-branch-and-workt.md) | Parallelism with git worktree: one agent per branch and worktree | accepted |
| [0022](0022-real-audio-corpus-outside-the-repository-fakeflac-corpus-rea.md) | Real audio corpus outside the repository (FAKEFLAC_CORPUS), read-only; only generated synthetic fixtures in the repo | accepted |
| [0023](0023-agents-run-only-in-local-claude-code-sessions-ci-runs-no-age.md) | Agents run only in local Claude Code sessions; CI runs no agents. Pre-PR review is done locally by the `code-reviewer` subagent | accepted |
| [0024](0024-model-per-role-the-most-capable-for-specs-dsp-architecture-a.md) | Model per role: the most capable for specs, DSP, architecture and review; an intermediate one for implementation and tests; a light one for mechanical tasks | accepted |
| [0025](0025-the-fft-window-function-is-a-detectionparams-parameter-chose.md) | The FFT window function is a DetectionParams parameter, chosen by experiment. Initial hypothesis: 4-term Blackman-Harris with 75 % overlap; reference: Hann with 50 % | accepted |
| [0026](0026-real-fft-with-realfft-on-top-of-rustfft.md) | Real FFT with realfft (on top of rustfft) | accepted |
| [0027](0027-classic-desktop-ui-inspired-by-the-reference-tool-single-win.md) | Classic desktop UI inspired by the reference tool: single window, folder tree, dense table and detail panel (docs/architecture/frontend.md) | accepted |
| [0028](0028-ci-and-delivery-on-github-actions-one-required-aggregator-ch.md) | CI and delivery on GitHub Actions, one required aggregator check and releases generated from tags | accepted |
| [0029](0029-semver-conventional-commits-and-automatic-changelog.md) | SemVer, Conventional Commits and automatic changelog | accepted |
| [0030](0030-formats-in-stages-and-rust-decoders-wav-and-flac-h1-mp3-m4a.md) | Formats in stages and Rust decoders: WAV and FLAC (H1); MP3, M4A (AAC) and Opus (H2). symphonia for WAV/FLAC/MP3/M4A; Opus with a separate decoder chosen in a spike (opus crate/libopus bundled vs a pure-Rust alternative) | accepted |
| [0031](0031-common-verdict-model-for-all-formats-real-suspicious-fake-re.md) | Common verdict model for all formats: Real / Suspicious / Fake relative to the declared quality, plus estimated_source_bitrate_kbps for lossy formats | accepted |
| [0032](0032-the-installer-is-not-code-signed-in-v1.md) | The installer is not code-signed in v1 | accepted |
| [0033](0033-h1-dependency-set.md) | H1 dependency set | proposed |
| [0034](0034-analysis-runs-on-std-scoped-threads-in-the-core.md) | The analysis runs on std scoped threads in the core (partially supersedes 0006) | proposed |
| [0035](0035-detection-v2-relative-cutoff-step-and-16-khz-gap-step-on-the-m.md) | Detection v2: relative cutoff step on the mean spectrum and 16 kHz gap step on the per-frame median spectrum | proposed |
| [0036](0036-calibration-tool-dev-only-workspace-crate-reusing-core-and-adapters.md) | Calibration tool: a dev-only workspace crate (`tools/fakeflac-calibrate`) that reuses the core and the adapters, with a JSON Lines spectra cache and ffmpeg as an external process | proposed |
| [0037](0037-frontend-component-test-stack-jsdom-testing-library-storybook.md) | Frontend component test stack: jsdom, Testing Library and Storybook 10 | proposed |
| [0038](0038-detection-v3-full-band-guard-on-the-gap-and-step-rules.md) | Detection v3: a full-band guard (level and burstiness above 20.7 kHz) on the gap and step rules; v3 becomes the default | proposed |

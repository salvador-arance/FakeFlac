# FakeFlac

Desktop app for Windows that tells you whether your audio files are what they claim to be.

- **Fake lossless:** FLAC or WAV files that were really made from an MP3/AAC.
- **Fake bitrate:** an MP3 at 320 kbps that actually came from a 128 or 192 kbps source, and the same check for M4A and Opus.
- A verdict per track (real / suspicious / fake), the estimated cutoff frequency, the estimated real bitrate and a spectrogram that backs the verdict.

Supported formats: **WAV, FLAC, MP3, M4A, Opus**. Everything is processed locally; nothing is sent to the Internet. Files are never modified.

> **Status: pre-alpha.** Milestone H0 (harness) is done. Milestone H1 (MVP, WAV and FLAC) is nearly done: the detection engine (v2), folder analysis with progress and cancellation, the main screen and results table, and the calibration tool are built; detection v3 and the spectrogram (RF-07) are pending. MP3, M4A and Opus come in H2. See [`docs/agents/progress.md`](docs/agents/progress.md).

## How this project is built

FakeFlac is developed entirely with AI agents (Claude Code) following **Spec-Driven Development**: a requirement becomes an approved spec, the spec becomes failing tests, and only then is code written. Humans approve specs and decisions (gate G1) and merge changes (gate G2). See [`CLAUDE.md`](CLAUDE.md) and [`docs/agents/workflow.md`](docs/agents/workflow.md).

- Architecture: [`docs/architecture/overview.md`](docs/architecture/overview.md)
- Decisions: [`docs/adr/`](docs/adr/README.md)
- Requirements and specs: [`specs/`](specs/)
- UI reference screenshots: [`docs/reference/`](docs/reference/)
- Original planning document (Spanish, phases 1-3): [`docs/planning/`](docs/planning/fakeflac-phases-1-3.es.md)

## Tech stack

Tauri v2 · Rust (hexagonal architecture, Cargo workspace) · Svelte 5 + TypeScript · SQLite cache.

## Development

Requirements (Windows 10/11 x64): Rust (MSVC toolchain) with **Visual Studio Build Tools (C++ workload)**, Node.js 24+, [`just`](https://github.com/casey/just), WebView2 (preinstalled on Windows 11).

```
npm install
just check-fast   # fmt, clippy, svelte-check, architecture lint, quick tests
just check        # check-fast plus all tests
just test         # all tests
npm run tauri dev # run the app in development
```

## Downloads

Releases will be published on GitHub. Installers are **not code-signed**: Windows SmartScreen may show a warning on first run ("More info" → "Run anyway"). Each release includes SHA-256 checksums.

## License

[MIT](LICENSE).
# FakeFlac

Desktop app for Windows that tells you whether your audio files are what they claim to be.

- **Fake lossless:** FLAC or WAV files that were really made from an MP3/AAC.
- **Fake bitrate:** an MP3 at 320 kbps that actually came from a 128 or 192 kbps source, and the same check for M4A.
- A verdict per track (real / suspicious / fake), the estimated cutoff frequency, the estimated real bitrate and a spectrogram that backs the verdict.

Supported formats: **WAV, FLAC, MP3, M4A and Opus** (Ogg Opus in `.opus` files, mono/stereo). Everything is processed locally; nothing is sent to the Internet. Files are never modified.

> **Status: pre-alpha.** The first pre-release, **v0.1.0**, was published on 2026-10-31. Milestones H0 (harness) and H1 (MVP, WAV and FLAC) are built: the lossless detection rules, folder analysis with progress and cancellation, the main screen, results table and spectrogram, and the calibration tool. H2 is under way. Merged so far: MP3 and M4A decoding and source-bitrate detection for lossy files (detection v5); detection v6 (clears a known kind of lossless false positive) and v7 (flags 48 kHz streaming rips as suspicious); sorting and filtering, CSV export and per-album summaries; the result cache in SQLite (re-analysing an unchanged folder reuses stored results); the performance harness; and the release pipeline. Opus files (detection v8) were merged after v0.1.0 and ship in the next release. Next: a Spanish/English interface with a saved language preference (RF-13; its spec is approved, not built yet). See [`docs/agents/progress.md`](docs/agents/progress.md).

## How this project is built

FakeFlac is developed entirely with AI agents (Claude Code) following **Spec-Driven Development**: a requirement becomes an approved spec, the spec becomes failing tests, and only then is code written. Humans approve specs and decisions (gate G1) and merge changes (gate G2). See [`CLAUDE.md`](CLAUDE.md) and [`docs/agents/workflow.md`](docs/agents/workflow.md).

- Architecture: [`docs/architecture/overview.md`](docs/architecture/overview.md)
- Decisions: [`docs/adr/`](docs/adr/README.md)
- Requirements and specs: [`specs/`](specs/)
- UI reference screenshots: [`docs/reference/`](docs/reference/)
- Original planning document (Spanish, phases 1-3): [`docs/planning/`](docs/planning/fakeflac-phases-1-3.es.md)

## Tech stack

Tauri v2 · Rust (hexagonal architecture, Cargo workspace) · Svelte 5 + TypeScript · SQLite cache (`rusqlite`).

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

When a release is published, download the installer (`.exe`, NSIS, recommended, or `.msi`) and `SHA256SUMS.txt` from the repository's GitHub Releases page. Check the download in PowerShell with `(Get-FileHash .\<installer> -Algorithm SHA256).Hash.ToLower()` and compare it with the matching line of `SHA256SUMS.txt`.

Installers are **not code-signed** (ADR-0032): Windows SmartScreen may show "Windows protected your PC" on first run. After checking the hash, click **More info**, then **Run anyway**. The text of each release's notes is in [`specs/contracts/release.md`](specs/contracts/release.md) section 3.5.

Maintainers: the release procedure is in [`specs/contracts/release.md`](specs/contracts/release.md) section 6. Only the human tags and publishes (ADR-0045).

## License

[MIT](LICENSE).
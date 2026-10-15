# Progress

> Newest block first. Read the first block before starting. Updated with `/handoff`.

## 2026-10-15 - Repository bootstrap (H0, in progress)

**Done**
- Repository created at `C:\Workspaces\FakeFlac` (git, MIT, English docs, public-repo ready).
- Design migrated from the Spanish planning document: `docs/architecture/*`, `docs/adr/0001-0032`, `specs/requirements/` (RF/RNF/RP), reference UI screenshots in `docs/reference/`.
- New requirements captured: formats WAV/FLAC/MP3/M4A/Opus (RF-14) and real-bitrate detection for lossy files (RF-15, e.g. 320 kbps MP3 from 128 kbps).
- Cargo workspace skeleton (`core`, `audio`, `fs`, `store`, `export`, `src-tauri`) with the domain model in `fakeflac-core`.
- Frontend scaffold (Svelte 5 + Vite + TS), atomic design folders and dark design tokens. `npm run check` and `npm run build` pass.
- Harness: `CLAUDE.md` (root and nested), `justfile`, `.claude/settings.json` (permissions, hooks), hooks with 24 passing tests, 4 subagents, 4 skills, `.mcp.json`.
- `scripts/lint-arch.mjs`, `scripts/trace.mjs`, `scripts/guard-tests.mjs`.
- CI: `ci.yml`, `ci-nightly.yml`, `release.yml`, Dependabot, CODEOWNERS, PR/issue templates, `deny.toml`.

**Verified on this machine (after installing VS Build Tools and just):** `just check` green; `cargo check -p fakeflac` (Tauri) compiles; `npm run tauri build` produces NSIS (1.44 MB) and MSI (2.12 MB), well under RNF-06's 20 MB. CI workflows are still unrun.

**Pending**
- Verify `.mcp.json` package names, the GitHub Actions versions/inputs and pin actions by commit SHA (Dependabot can maintain them).
- Create the GitHub repository (public), push, protect `main`, require the `ci-ok` check.
- Step 0.13: smoke test of the full cycle with a minimal pure feature (RF-10 worst-verdict rule).
- Remaining harness pieces from the design: `spec-reviewer`, `architecture-guardian`, `docs-keeper`, `/new-component`, `/calibrate`, `/release`, `notify` hook, `settings.local.json` with `FAKEFLAC_CORPUS`.

**Blockers / questions for the human**
- The first push to `main` must be done by the human (`guard-bash` blocks agents pushing to `main`). The GitHub repo exists but is currently PRIVATE; make it public when ready.
- Keep the reference tool's player out of v1? (default: yes, see docs/architecture/frontend.md)

**Next step:** after the first push and branch protection (require `ci-ok`), start H1 specs (`/new-spec RF-01` … `RF-07`, `RF-14` for WAV/FLAC) following `docs/planning/fakeflac-phases-1-3.es.md` section 3.12, stage 1.
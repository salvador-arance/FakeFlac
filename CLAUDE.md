# FakeFlac

Detects fake lossless audio (FLAC/WAV made from a lossy source) and lossy files made from a lower bitrate
(e.g. a 320 kbps MP3 that came from 128 kbps). Formats: WAV, FLAC, MP3, M4A, Opus.
Tauri v2 · Rust (hexagonal, Cargo workspace) · Svelte 5 + TypeScript. Windows desktop app, MIT, public repo.
Process: Spec-Driven Development executed by agents. Read `docs/agents/workflow.md` before your first task.
Repo docs and code are in English. `docs/planning/` is the historical planning document (Spanish).

## Non-negotiable rules
- Nothing is implemented without an approved spec in `specs/`. If behaviour changes, change the spec first (human gate G1).
- In the `implement` stage, never modify `specs/` or the tests derived from a spec. If a spec or test looks wrong, stop and report it.
- `fakeflac-core` depends on no other workspace crate and does no I/O (`docs/architecture/dependencies.md`).
- Audio files are read-only. Never write outside this repository. The audio corpus lives outside the repo (`FAKEFLAC_CORPUS`, ADR-022).
- Svelte 5 runes only (`$state`, `$derived`, `$props`). No Svelte 4 syntax (`export let`, `$:`).
- No component calls `invoke()`: everything goes through `src/lib/api/`.
- Thresholds and the FFT window are data in `DetectionParams`, never constants in code (ADR-011, ADR-025).
- New dependency or architecture decision → ADR (`/new-adr`) before the PR. Ask before adding dependencies.
- Never merge to `main`, never push to `main`, never force-push. The human merges (gate G2).
- Do not mark a task done without `just check` green.
- Commits: Conventional Commits (`feat(RF-04): …`, `test(RF-04): …`, `docs: …`).

## Commands (single entry point: `justfile`, ADR-019)
```
just check        # fmt, clippy, svelte-check, arch lint, fast tests
just test         # all tests
just gen          # regenerate TS bindings from Rust (tauri-specta)
just trace        # validate the traceability matrix
just lint-arch    # crate dependency rules + atomic design import rules
just fixtures     # generate synthetic audio fixtures
just ci           # what CI runs
just prune-branches  # delete merged branches and clean agent worktrees (after G2); --dry-run lists
```

## Where things are
- Architecture: `docs/architecture/overview.md` · Decisions: `docs/adr/`
- Requirements and specs: `specs/` · Traceability: `specs/traceability.md`
- Reference UI screenshots: `docs/reference/` (UI style: `docs/architecture/frontend.md`)
- Current state and next step: `docs/agents/progress.md`

## End of session
Run `/handoff`: update `docs/agents/progress.md` (done · pending · blockers).

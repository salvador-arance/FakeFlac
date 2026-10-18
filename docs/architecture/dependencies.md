# Dependency rules

> Arrows point toward `fakeflac-core`; `core` knows nobody. Enforced by `just lint-arch` (`scripts/lint-arch.mjs`) and CI.

## Rust crates

1. `fakeflac-core` has **no** dependency on other workspace crates and performs no I/O.
2. Adapter crates (`audio`, `fs`, `store`, `export`) depend **only** on `fakeflac-core`; adapters never depend on each other.
3. `src-tauri` is the composition root of the app. Dev-only tools under `tools/` (ADR-0036) are composition roots too: they may depend on `fakeflac-core` and on the adapters they need. No crate under `crates/`, and not `src-tauri`, may depend on a tool. Tools never ship in the installer.
4. A new port in `core` requires an ADR (D3: no over-engineering).

## Frontend (atomic design, ADR-0014)

Levels, low to high: `tokens` < `atoms` < `molecules` < `organisms` < `templates` < `pages`.

1. A level imports only from the same or a lower level.
2. Atoms, molecules, organisms and templates are presentational: data in via props, events out. They never import `src/lib/stores` or `src/lib/api`, nor `@tauri-apps/*`.
3. Only pages connect to stores and the API client.
4. No `invoke()` outside `src/lib/api/`.
5. Svelte 5 runes only (no `export let`, no `$:`).
6. Colours and sizes come from tokens.

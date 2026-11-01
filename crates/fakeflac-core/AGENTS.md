<!-- Generated from crates/fakeflac-core/CLAUDE.md; run just agents-sync. Do not edit. -->

# fakeflac-core

Pure domain, use cases and ports (ADR-001).
- No I/O, no Tauri, no dependency on other workspace crates. Only `serde` and `thiserror`.
- No `unwrap` (denied by lint). Errors with `thiserror`; per-track errors are data (`TrackError`), never panics (ADR-013).
- Tests use synthetic signals and fakes of the ports; no real audio.
- Only the ports listed in `docs/architecture/backend.md` exist. A new port requires an ADR.

# src-tauri

Adapter and composition root only: commands, progress `Channel`, wiring of ports to adapters.
- No domain logic here. If you are writing an `if` about verdicts, it belongs in `fakeflac-core`.
- Minimal capabilities (`capabilities/default.json`); no `shell`, no open `fs` from the frontend. All disk access happens in Rust.
- The command contract is generated with `tauri-specta` (S3); never hand-write the TS types. Contract types are the DTOs in `src/dto.rs`; after changing them run `just gen`.
- `just check` does not build this crate. The done-gate for adapter work is `just check-app` (bindings up to date, clippy, tests; Windows).
- Tests go in `tests/` (integration tests get the Windows app manifest from `build.rs`; `#[cfg(test)]` modules in `src/` would crash at start).

# src-tauri

Adapter and composition root only: commands, progress `Channel`, wiring of ports to adapters.
- No domain logic here. If you are writing an `if` about verdicts, it belongs in `fakeflac-core`.
- Minimal capabilities (`capabilities/default.json`); no `shell`, no open `fs` from the frontend. All disk access happens in Rust.
- The command contract is generated with `tauri-specta` (S3); never hand-write the TS types.

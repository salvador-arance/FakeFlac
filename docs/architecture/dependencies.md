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

## Network (RNF-03)

The installed app has no path to the Internet. The rules and their exact lists are in `specs/contracts/privacy.md` section 3. `just lint-arch` checks them through `scripts/lint-network.mjs`, and `cargo deny check` refuses the network crates in the Windows graph (`deny.toml` `[bans] deny`).

1. **Tauri surface.** The CSP uses only `'self'`, `'unsafe-inline'`, `data:` (in `img-src`), `ipc:` and `http://ipc.localhost` (in `connect-src`), and `default-src` is `'self'`. `tauri.conf.json` has no `plugins`, no updater artifacts, no remote window `url` and no inline capabilities. Capabilities grant only `core:default` and the four `core:window` permissions of the title bar, with no `remote` key.
2. **Rust.** No workspace manifest depends on a crate of the NET list, and the only Tauri plugin is `tauri-plugin-dialog`. `tauri` has no TLS feature, and `windows` has no `Networking`, `Web` or `Win32_Network` feature. `Cargo.lock` holds no NET crate, except `reqwest`, `hyper` and `hyper-util`: those three are mobile-only dependencies of `tauri` and are not in the Windows graph. Shipped Rust sources use no `std::net` or socket type.
3. **Frontend.** Shipped sources (`index.html` and `src/` without tests and stories) contain no remote URL except the SVG namespace, and no `fetch(`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`, `RTCPeerConnection` or `window.open(`. There is no `@tauri-apps/plugin-*` package.
4. Changing any of these lists is a spec change (gate G1).

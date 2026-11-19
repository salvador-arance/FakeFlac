---
id: SPEC-PRIVACY
requirements: [RNF-03, RNF-04]
status: approved
---

# Privacy: the app has no path to the Internet (RNF-03)

> RNF-03: all processing is local; the app sends no data to the Internet. This spec turns that into checks that need no network test: the Tauri surface (CSP, configuration, capabilities), the Rust dependency graph and sources, and the frontend sources are held to written allow-lists and denylists by `just lint-arch` and by the existing `cargo deny check`. One owner-run manual check (M1) watches the installed app, as an indication only. This is not a backend ↔ frontend contract change: no command, event, DTO or error changes, and `commands.md` stays at version 9. Scenarios: `../features/privacy.feature` (tag `@RNF-03`; `@RNF-04` for unreadable files and the corrupt file of M1). Sources: `docs/planning/fakeflac-phases-1-3.es.md` lines 197, 812, 1043 and 1282; `docs/architecture/overview.md` line 43.

## 0. Questions for G1 (answered by the owner on 2026-11-20)

Each question had a default, and the rest of this spec is written with it. On 2026-11-20 the owner accepted every default, after a read-only check of each one against the repository (the facts that check added are in section 2). Changing an answer changes only the sections and scenarios in its row.

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| Q1 | **Scope of "the app".** RNF-03 covers the installed app at run time: the Rust binary, the webview content and their configuration. Out of scope: the installer's WebView2 bootstrapper download (`release.md` D3), the development environment, CI and agents (planning line 1043), OS components the app does not control (WebView2 runtime updates and diagnostics, Windows Error Reporting, the Windows shell dialog), and folders the user picks on a network share (reads the user starts from their own storage). **Resolved (owner, 2026-11-20): as stated.** | 1, 7 |
| Q2 | **Where the static checks live.** A new module `scripts/lint-network.mjs` (Node built-ins only, pure functions over file texts), called from `scripts/lint-arch.mjs`, so `just lint-arch` (CI `specs` job, `just check-fast`) runs it with no `justfile` or workflow change. Tests: `scripts/lint-network.test.mjs`, picked up by `node --test "scripts/*.test.mjs"` in `check-fast`. `lint-arch.mjs` has no tests today and exits on import, so its new rules go in the testable module. **Resolved (owner, 2026-11-20): as stated.** | 3, 4 |
| Q3 | **The transitive Rust graph.** A text check cannot tell which `Cargo.lock` packages are compiled for Windows. Default: (a) the offline lint refuses lock entries for crates that are absent today (R2), and (b) `deny.toml` gains a `[bans] deny` list of the network crates (R3), checked by the existing CI step `cargo deny check` on the Windows graph (`deny.toml` line 5). Precondition: the feature PR records that `reqwest`, `hyper` and `hyper-util` are not in the Windows graph (section 2). If they are, the feature stage stops and this returns to G1 (alternative: `wrappers = ["tauri"]`). **Resolved (owner, 2026-11-20): (a) and (b).** | 3.2, 4, P4 |
| Q4 | **Capabilities: exact allow-list.** Every permission in `src-tauri/capabilities/` must be one of the five granted today; any addition fails the lint and needs a spec change. This matches the file's own rule "Add permissions only with a spec and an ADR" (`default.json` line 4). The alternative is a denylist of network plugin prefixes. **Resolved (owner, 2026-11-20): exact allow-list.** | 3.1 T3 |
| Q5 | **CSP: token allow-list.** Every source in every directive must belong to a fixed set. Removing a source is allowed; adding one fails. The alternative is an exact string match, which fails on harmless tightening too. **Resolved (owner, 2026-11-20): token allow-list.** `data:` in `img-src` is not used by any shipped source today; T1 allows removing a source, so it can be dropped later without a spec change. | 3.1 T1 |
| Q6 | **Frontend scan scope.** Shipped sources only: `index.html`, plus `src/` without test files, stories and test helpers. One URL literal is allowed: `http://www.w3.org/2000/svg`. It is an SVG namespace, not a request (`Icon.svelte` lines 15 and 57). Third-party code in the bundle (`@tauri-apps/api`, the Svelte runtime) is not scanned; at run time the CSP covers it. **Resolved (owner, 2026-11-20): as stated.** | 3.3 F1, 7 |
| Q7 | **M1.** Owner-run once on a release candidate to close RNF-03; it is indicative. A remote connection by `fakeflac.exe` fails M1. A connection by its WebView2 child processes is recorded with its remote host and brought to the owner as a finding. It neither passes nor fails M1 automatically (Q1: runtime services of an OS component). **Resolved (owner, 2026-11-20): as stated.** | 6 |
| Q8 | **Breadth of the Rust rules.** R1–R3 apply to every workspace member, including `tools/fakeflac-calibrate` and dev-dependencies. None of them uses a network crate today, and a test that needs the network is not wanted either. R4 (source text) applies to shipped crates only. **Resolved (owner, 2026-11-20): as stated.** | 3.2 |
| Q9 | **ADR.** No ADR. There is no new dependency, port or architecture change; the rules live here and in `docs/architecture/dependencies.md` (new "Network (RNF-03)" section, docs-keeper). **Resolved (owner, 2026-11-20): no ADR.** | 4 |
| Q10 | **Documentation drift found.** `docs/architecture/overview.md` line 58 says the capabilities grant "folder dialog, read of the chosen path, write only of the destination CSV". `default.json` grants none of these: the dialog is opened from Rust (ADR-0033 note, line 48). Should the feature PR correct that sentence (documentation only)? **Resolved (owner, 2026-11-20): yes.** The feature PR replaces the sentence with: "Capabilities: `core:default` and four `core:window` permissions for the custom title bar; no plugin permission. Dialogs, reads and the CSV write happen in Rust." | 4 |

**Decided defaults (not questions).**
- **D1.** No production code change is expected: today's repository meets every rule (section 2). If a rule fails on today's code, the feature stage stops and returns to G1.
- **D2.** Fonts are system fonts only: `--font-ui: 'Segoe UI', system-ui, sans-serif` (`src/lib/tokens/tokens.css` line 51). There are no font files, no Google Fonts and no `public/` folder. F1 keeps it so.
- **D3.** `build.devUrl` (`http://localhost:1420`, `tauri.conf.json` line 8) and any `devCsp` are dev-only. The rules ignore them.
- **D4.** `just privacy-check` (`justfile` lines 154–159) is the ADR-0022 corpus-name check and is unrelated. The new module is named `lint-network` to keep the two apart.

## 1. Scope

- **In:** what the installed app can reach at run time, through its configuration (`src-tauri/tauri.conf.json`, `src-tauri/capabilities/`), its Rust dependencies and sources, and its frontend sources (`index.html`, `src/`).
- **Out:** see Q1. Updates are downloaded manually in v1 (planning line 1282; `release.md` D4: no in-app updates).

## 2. What is true today (2026-11-19, branch base `bfb938c`)

| Area | Fact | Evidence |
|---|---|---|
| Capabilities | One file grants `core:default` and four `core:window:*` permissions. There is no `remote` key and no plugin permission. `core:default` contains only `core:{path,event,window,webview,app,image,resources,menu,tray}:default`. `core:webview:default` does not create webviews. | `src-tauri/capabilities/default.json` lines 4–6; `src-tauri/gen/schemas/desktop-schema.json` lines 180, 1044 (a generated file, ignored by git, so it exists only after a build) |
| CSP | `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost`. `http://ipc.localhost` is Tauri's local IPC, not the Internet. | `src-tauri/tauri.conf.json` line 24 |
| Tauri config | No `plugins` key, no updater artifacts, no window `url`, no inline capabilities, no `additionalBrowserArgs`. The WebView2 bootstrapper is used by the installer only. | `tauri.conf.json` lines 1–35, line 32; `release.md` D3 |
| Plugins | Only `tauri-plugin-dialog` 2.8.4 is a dependency and registered. It brings `tauri-plugin-fs` transitively, and no capability grants either to the frontend. | `src-tauri/Cargo.toml` line 28; `src-tauri/src/lib.rs` line 43; `Cargo.lock` lines 3857 and 3875 (checked on 2026-11-20 at `a435a35`) |
| Direct Rust deps | No network crate in any workspace manifest. `tauri` has no features. `windows` uses `System_UserProfile` and `Foundation_Collections` only. | `src-tauri/Cargo.toml` lines 17, 33; `crates/*/Cargo.toml`; `tools/fakeflac-calibrate/Cargo.toml` |
| `Cargo.lock` | It contains `reqwest` 0.13.5 (a dependency of `tauri`), `hyper` 1.12.0, `hyper-util` 0.1.21, `tower-http`, and `tokio` 1.53.3 with `mio` and `socket2`. It contains no TLS crate (`rustls`, `native-tls`, `openssl`, `hyper-rustls`, `hyper-tls`), no `h2` and no network plugin. | `Cargo.lock` lines 2852 (`reqwest`), 1554 (`hyper`), 1573 (`hyper-util`), 4262 (`tower-http`), 4124 (`tokio`), 2107 (`mio`), 3289 (`socket2`), checked on 2026-11-20 at `a435a35`. In the lock, `reqwest` is a dependency of `tauri` only, `hyper` of `hyper-util` and `reqwest`, and `hyper-util` of `reqwest`. The lock has 482 packages and none of the other NET names |
| `reqwest` on Windows | tauri 2.12.6 (the lock's version since #171; 2.12.5 at the check below) declares `reqwest` only for Android and iOS, in the published manifest of both versions. Its own `tokio` features are `rt`, `rt-multi-thread`, `sync`, `fs` and `io-util` (no `net`). So `reqwest`, `hyper` and `hyper-util` should not be compiled for `x86_64-pc-windows-msvc`. **Verified on 2026-11-19 at `997fa59`** with `cargo tree --offline --target x86_64-pc-windows-msvc -i <name>` for each of the three: no package (`nothing to print`). `tokio` is present through `tauri`, `tauri-plugin-dialog` and `tauri-plugin-fs`, with the features `bytes`, `default`, `fs`, `io-util`, `rt`, `rt-multi-thread` and `sync`: no `net`. The precondition of R3 is met for the default feature set. **That `cargo tree` ran at `997fa59`, before tauri moved to 2.12.6 (#171). It could not be repeated on 2026-11-20: the offline cache lacks tauri 2.12.6.** The feature PR repeats it with `-e normal,build,dev` and `--workspace` and records the output (P4). | tauri-2.12.5 `Cargo.toml` (registry source) lines 244–252, 325 |
| Rust sources | No `std::net`, `TcpStream`, `TcpListener` or `UdpSocket` in `crates/` or `src-tauri/`. The only URLs are comments in a test file. | `crates/fakeflac-audio/tests/common/aac_tables.rs` lines 12, 14 |
| WebView2 | wry passes `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection` by default (SmartScreen off in the webview). FakeFlac does not override it. | wry-0.57.0 `src/webview2/mod.rs` line 303 (registry source) |
| Frontend | The only runtime package is `@tauri-apps/api`; there is no `@tauri-apps/plugin-*`. `index.html` loads only `/src/main.ts`. `src/` has no `fetch`, `XMLHttpRequest`, `WebSocket` or `EventSource`. Its only URL literals are the SVG namespace. | `package.json` lines 36–38; `index.html` lines 1–12; `src/lib/components/atoms/Icon.svelte` lines 15, 57 |
| Dev tools | Storybook telemetry is off. Out of scope anyway (Q1). | `.storybook/main.ts` line 7 |
| Lint today | `lint-arch.mjs` checks crate edges, atomic imports and `invoke()`. It has no privacy rule and no test file. It runs in CI `specs` (Ubuntu, no `npm ci`) and in `check-fast`. | `scripts/lint-arch.mjs`; `.github/workflows/ci.yml` line 45; `justfile` line 14 |
| `deny.toml` | The graph targets Windows only. `[bans]` has no deny list. | `deny.toml` lines 5, 22–23 |

## 3. Rules

Violations join `lint-arch`'s existing output: the header `lint-arch: violations found`, one line per violation, exit 1. Each line has the form ` - <file>: RNF-03 <rule>: <detail>`. `<file>` is relative to the repository root with `/` separators, and `<detail>` names the offending token as written. A file that is missing or is not valid JSON gives one violation, `<file>: RNF-03 <rule>: missing` or `<file>: RNF-03 <rule>: invalid JSON`. The rule is T1 for `tauri.conf.json` (T2 is then skipped), T3 for a capability file, F2 for `package.json` and R2 for a missing `Cargo.lock`. A missing capabilities folder is the empty state and passes. The other rules still run, and nothing prints a stack trace (RNF-04). Node built-ins only (the `specs` job has no `npm ci`).

### 3.1 Tauri surface

- **T1 CSP.** `app.security.csp` in `tauri.conf.json` must be a string. Parsed into directives:
  - `default-src` is present and is exactly `'self'`;
  - every source is one of `'self'`, `'unsafe-inline'`, `data:`, `ipc:` and `http://ipc.localhost`;
  - `ipc:` and `http://ipc.localhost` appear only in `connect-src`;
  - `data:` appears only in `img-src`.

  Detail: `CSP source '<token>' not allowed in <directive>`, `CSP missing` (no `csp` key, or it is not a string: the object form is refused too) or `default-src must be 'self'`. A `default-src` that is not exactly `'self'` gives only the last of these. A directive with no sources (such as `upgrade-insecure-requests`) is accepted. `'unsafe-inline'` is accepted in any directive: it is not a privacy question, and narrowing it is out of this spec.
- **T2 Configuration.** `tauri.conf.json` has no `plugins` key, no `bundle.createUpdaterArtifacts`, no `app.windows[N].url` with a URL scheme (any `<scheme>:` prefix; a bare relative path is allowed), and no `app.security.capabilities`. The JSON path is written with the index, for example `app.windows[0].url`. Detail: `'<json path>' not allowed`. Other Tauri switches that bear on remote access (`app.security.dangerousRemoteDomainIpcAccess`, `app.withGlobalTauri`, the asset protocol) are not set today and are not checked: section 7.
- **T3 Capabilities.** Every file in `src-tauri/capabilities/` must be `.json`; anything else gets the detail `only JSON capability files are allowed`. No file has a `remote` key (`remote key not allowed`). Every permission, whether a string or an object's `identifier`, is one of `core:default`, `core:window:allow-minimize`, `core:window:allow-toggle-maximize`, `core:window:allow-close` and `core:window:allow-start-dragging` (`permission '<id>' not in the allow-list`). An empty folder or an empty `permissions` list passes.

### 3.2 Rust

**NET list** (normative; the lint module and `deny.toml` hold the same names, and a test checks they are equal): `reqwest`, `hyper`, `hyper-util`, `h2`, `h3`, `quinn`, `ureq`, `curl`, `curl-sys`, `isahc`, `attohttpc`, `surf`, `awc`, `tungstenite`, `tokio-tungstenite`, `async-tungstenite`, `rustls`, `native-tls`, `openssl`, `openssl-sys`, `hyper-rustls`, `hyper-tls`, `tokio-rustls`, `tokio-native-tls`, `tauri-plugin-updater`, `tauri-plugin-http`, `tauri-plugin-websocket`, `tauri-plugin-upload`, `tauri-plugin-shell`, `tauri-plugin-opener`.

- **R1 Direct dependencies** of every workspace member (`crates/*`, `src-tauri`, `tools/*`), in every dependency table (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[target.….dependencies]`, and `[….dependencies.<name>]` headers), and in the root `[workspace.dependencies]` table. Both the table form and the dotted form (`serde.workspace = true`, `name = { … }`) are parsed. The real name is the `package = "…"` value when present.
  - No name is in NET (`dependency '<name>' can reach the network`).
  - The only `tauri-plugin-*` is `tauri-plugin-dialog` (`tauri plugin '<name>' not in the allow-list`).
  - `tauri` has none of the features `native-tls`, `native-tls-vendored` or `rustls-tls`.
  - `windows` has no feature starting with `Networking`, `Web` or `Win32_Network` (`feature '<f>' of '<crate>' not allowed`).
- **R2 Lockfile (offline).** `Cargo.lock` has no `[[package]]` whose `name` is in NET minus `reqwest`, `hyper` and `hyper-util`. Those three are in the lock today as tauri's mobile-only dependencies (section 2). Detail: `Cargo.lock contains '<name>'`.
- **R3 Windows graph (transitive).** `deny.toml` `[bans] deny` lists every NET name. The existing CI step `cargo deny check` (supply-chain job) refuses any of them in the `x86_64-pc-windows-msvc` graph. Precondition, recorded in the feature PR: `cargo tree --workspace --target x86_64-pc-windows-msvc -e normal,build,dev -i <name>` for `reqwest`, `hyper` and `hyper-util` finds no package, and `cargo deny check bans` passes. Otherwise see Q3.
- **R4 Sources.** `crates/*/src/**/*.rs` and `src-tauri/src/**/*.rs` contain none of `std::net`, `TcpStream`, `TcpListener`, `UdpSocket` or `ToSocketAddrs` as whole words. Comments count too. Detail: `'<token>' in shipped source`.

### 3.3 Frontend

- **F1 Sources.** Scanned: `index.html` and the `.ts`, `.js`, `.svelte`, `.css` and `.html` files under `src/`. Not scanned: names containing `.test.` or `.stories.`, and files under a folder named `tests` or `testing`. Refused:
  - (a) any `http://`, `https://`, `ws://`, `wss://` or `ftp://` URL except exactly `http://www.w3.org/2000/svg` (`URL '<url>'`);
  - (b) a protocol-relative URL in `src=`, `href=`, `action=`, `url(` or `@import` (`URL '<url>'`);
  - (c) as whole words (`\bfetch\(` does not match `prefetch(`), the APIs `fetch(`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`, `RTCPeerConnection` and `window.open(` (`'<api>' not allowed`).
- **F2 Packages.** `package.json` has no `dependencies` or `devDependencies` entry named `@tauri-apps/plugin-*` (`package '<name>' not allowed`).

## 4. Layers after G1 (`/feature RNF-03`)

- **Test stage:** `scripts/lint-network.test.mjs`, written first. Fixture texts are written in each test, never committed as files. Every rule gets at least one violating case with its exact line and one passing case, plus the tightening and empty cases of T1 and T3, the SVG namespace, renamed packages, invalid JSON and missing files. There are three more checks: NET equals `deny.toml` `[bans] deny`; the real repository passes; `lint-arch.mjs` prints the RNF-03 lines with its header and exits 1.
- **Implementation stage:** add `scripts/lint-network.mjs` and a short call from `scripts/lint-arch.mjs`. Add the `[bans] deny` list to `deny.toml`. Record the R3 precondition output in the PR. There are no other script changes, and no `justfile`, workflow or production-code changes (D1).
- **Docs (docs-keeper):** a "Network (RNF-03)" section in `docs/architecture/dependencies.md`; `overview.md` line 43 points to this spec; Q10.

## 5. Acceptance

- **P1.** `just lint-arch` passes on the candidate with T1–T3, R1, R2, R4, F1 and F2 active.
- **P2.** `scripts/lint-network.test.mjs` passes in `just check-fast`, with the cases of section 4. Each rule has at least one failing fixture that yields exactly its documented line.
- **P3.** The NET names in `scripts/lint-network.mjs` equal the `deny.toml` `[bans] deny` names (checked by a test).
- **P4.** `cargo deny check` passes in CI with the bans, and the R3 precondition output is in the feature PR.
- **P5.** The feature PR changes no file under `src/`, `src-tauri/src/` or `crates/*/src/`, and `commands.md` stays at version 9.
- **P6.** M1 is recorded and passed (Q7).

RNF-03 is closed when P1–P5 are green and P6 is passed.

## 6. Manual check M1 (owner, once per closure, indicative)

Use an installed release candidate, with WebView2 already present. Record the app version, source commit and installer SHA-256. Use the synthetic RNF-07 fixture pack: a valid WAV or FLAC, a corrupt supported file, a valid file with a Unicode name, and an empty folder.

1. **Offline.** Turn on airplane mode, or disable every network adapter. Launch FakeFlac from Start and do the following:
   - analyze the fixture folder: the valid files get a non-error verdict, and the corrupt file gets an Error row;
   - open a spectrogram;
   - export the CSV;
   - analyze the empty folder (`NO_AUDIO_FOUND`);
   - change the language and save;
   - clear the cache;
   - close the app.

   Every step works.
2. **Online, watched.** Re-enable the network and open Resource Monitor (`resmon.exe`), Network tab. Tick `fakeflac.exe` and the `msedgewebview2.exe` processes it starts (Task Manager shows them under FakeFlac). Repeat step 1, then leave the app idle for 5 minutes.
   - **Pass:** `fakeflac.exe` shows no non-loopback address in "Network Activity" or "TCP Connections".
   - **WebView2 processes:** record any address of theirs with its host name, as a finding (Q7).
3. **Record** the date, OS build, WebView2 version, the result of each step and the count of remote addresses per process. Aggregates only: no audio names or paths (ADR-0022).

## 7. Limitations (what a static check cannot prove)

- **Names and patterns only.** The rules show that known network crates, plugins, permissions, CSP sources and API names are absent. They do not show that no socket is opened:
  - any crate can call `std::net`, Winsock or WinHTTP through `windows`/`windows-sys`, and the features that tauri, wry, webview2-com and rfd enable are not visible in `Cargo.lock`;
  - a network crate that is not in NET passes R1–R3;
  - R4 is a text match: `use std::{net::…}` and Winsock through `windows-sys` are not caught;
  - R2 refuses a new NET name in the lock even when it comes only from a mobile-target dependency of tauri (for example a bump that adds `h2` or `rustls` for Android). The answer is a change of this spec at G1, not a silent exception.
  - The Tauri switches `app.security.dangerousRemoteDomainIpcAccess`, `app.withGlobalTauri` and the asset protocol are not set today and T2 does not check them.
- **The webview.** The CSP blocks fetches and remote resources but does not govern navigation. F1 covers URL literals in our sources only; third-party JavaScript in the bundle is not scanned (Q6).
- **OS components.**
  - WebView2 is a Microsoft runtime: its updates (Edge Update) and diagnostic data follow Windows policy. SmartScreen in the webview is off by wry's default (section 2), which was read in source and not observed at run time.
  - Windows Error Reporting may upload crash data according to Windows settings. FakeFlac adds no crash reporter.
  - The folder dialog is the Windows shell (Q1).
- **M1 is indicative.** It covers one session on one machine, and Resource Monitor may miss very short connections.
- **Lists.** NET and the allow-lists need a spec change to evolve. A new network crate on crates.io is not caught until it is added to NET.
- **Unicode names (RNF-09).** They are not relevant to these rules: the lint reads repository files only. M1's fixture pack includes a Unicode name only as part of the normal session.

---
id: RNF-03
title: Privacy
priority: high
milestone: H1
status: specified
---

**Privacy.** All processing is local; the app sends no data to the Internet.

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4).

Measurable acceptance (pending G1). It needs no network test. Scope: the installed app at run time. The installer's WebView2 bootstrapper, the development environment and OS components are out of scope (privacy.md Q1).
- **Tauri surface (`just lint-arch`).** The CSP's sources all come from `'self'`, `'unsafe-inline'`, `data:` (images only) and the local IPC `ipc:` / `http://ipc.localhost` (`connect-src` only), with `default-src 'self'` (T1). `tauri.conf.json` has no `plugins`, no updater artifacts, no remote window URL and no inline capabilities (T2). The capabilities grant exactly the five permissions of today and have no `remote` key (T3).
- **Rust.** No workspace manifest declares a crate of the NET list (HTTP, WebSocket and TLS clients, and the updater, http, websocket, upload, shell and opener plugins). `tauri-plugin-dialog` is the only Tauri plugin. `tauri` has no TLS feature, and `windows` has no networking feature (R1). `Cargo.lock` holds no NET crate other than tauri's mobile-only `reqwest`, `hyper` and `hyper-util` (R2). `cargo deny check` bans every NET crate in the Windows graph (R3). Shipped Rust sources name no socket API (R4).
- **Frontend.** `index.html` and the shipped `src/` files contain no remote or protocol-relative URL other than the SVG namespace, and no `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource`, `sendBeacon`, `RTCPeerConnection` or `window.open` (F1). There is no `@tauri-apps/plugin-*` package (F2).
- **Pass/fail.** P1–P5: the rules pass on the candidate; each rule has a failing-fixture test with its exact message; the NET list equals the `deny.toml` bans; `cargo deny check` is green; there is no production-code change. A malformed or missing configuration file is one violation, never a crash (RNF-04).
- **Manual M1 (owner, once, indicative).** The installed app works fully offline on the synthetic fixture pack (valid, corrupt and Unicode-named files, and an empty folder). Online, Resource Monitor shows no non-loopback address for `fakeflac.exe`. Addresses of WebView2 processes are recorded as findings.
- **Limitations.** A static check cannot prove that a transitive crate never opens a socket (privacy.md section 7).

Spec: `specs/contracts/privacy.md` (rules T1–T3, R1–R4, F1–F2, acceptance P1–P6, M1, G1 questions Q1–Q10, decided defaults D1–D4, evidence of today's state). Scenarios: `specs/features/privacy.feature` (tag `@RNF-03`; `@RNF-04` for unreadable files and M1's corrupt file).

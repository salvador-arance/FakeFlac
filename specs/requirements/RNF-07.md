---
id: RNF-07
title: Platform
priority: high
milestone: H2
status: specified
---

**Platform.** Windows 10/11 (64-bit) in v1.

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4).

Approved acceptance (G1, 2026-11-02): `specs/contracts/windows-platform.md` (P1–P4 and G1 decision
P-Q1), with scenarios in `specs/features/windows-platform.feature`. It consolidates
the existing x64 NSIS/MSI release contract and requires recorded installed-app
checks on Windows 11 x64 (NSIS and MSI); by owner decision (2026-11-02) Windows 10 stays a stated target but is not part of the closure matrix. Windows 11 NSIS and MSI install/launch/analyse/uninstall passed on 2026-11-02; the synthetic fixture pack was used, WebView2 156.0.4314.58, installer hashes checked by the owner. RNF-07 is recorded as verified for this amended matrix;
existing Windows CI and the v0.1.0 Sandbox check do not establish both OS results.

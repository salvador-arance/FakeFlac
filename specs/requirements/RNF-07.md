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
checks for both Windows versions. The four-row runtime matrix is pending;
existing Windows CI and the v0.1.0 Sandbox check do not establish both OS results.

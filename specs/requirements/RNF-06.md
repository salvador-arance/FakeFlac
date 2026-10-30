---
id: RNF-06
title: Size
priority: high
milestone: H2
status: specified
---

**Size.** Installer under 20 MB.

Measurable acceptance: each installer file, the NSIS `.exe` and the MSI checked separately, is **strictly smaller than 20 MB = 20 000 000 bytes** (decimal; G1 question Q7 of the spec). The gate is `just release-assets` (`node scripts/release.mjs assets --max-bytes 20000000`). It runs in the `build-windows` job of every PR and every push to `main`, so the required check `ci-ok` fails when an installer is too big, and in the `package` job of every release, before anything is published. Its log prints both sizes in bytes and MB. Tested at the boundary by `node --test` (19 999 999 bytes pass, 20 000 000 bytes fail; acceptance R4 of the spec). The bundle keeps the WebView2 `downloadBootstrapper` mode. The current size is unknown: the only measurement on record is from H0 (NSIS 1.44 MB, MSI 2.12 MB), before the lossy decoders and SQLite were added. The owner records it from the CI log (manual check M2).

Source: docs/planning/fakeflac-phases-1-3.es.md (sections 2.4 and 3.11.11 c)). Spec: `specs/contracts/release.md` (section 3.3 step 3, section 4 `release-assets`, section 5, acceptance R4, manual check M2, G1 question Q7). Scenarios: `specs/features/release.feature` (tag `@RNF-06`). Decisions: ADR-0030 (no bundled ffmpeg), ADR-0045 (proposed).

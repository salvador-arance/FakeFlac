---
id: RNF-04
title: Reliability
priority: high
milestone: H1
status: specified
---

**Reliability.** A corrupt or unreadable file does not stop the analysis of the others; it is marked as an error.

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4). Scenarios: `specs/features/` (tag `@RNF-04`).

H3 extension (2026-11-20, draft): a file with a few damaged packets that otherwise decodes is analysed and its damage reported, under RNF-12. Until RNF-12's spec is approved, any packet that fails to decode keeps making the track an error.

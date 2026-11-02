---
id: RNF-08
title: Usability
priority: high
milestone: H2
status: specified
---

**Usability.** The verdict is understood without reading documentation (colour and clear text).

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4).

Approved acceptance (G1, 2026-11-02): `specs/ui/verdict-usability.md` (U1–U4 and G1 decisions
U-Q1/U-Q2), with scenarios in `specs/features/verdict-usability.feature`. Existing
label/icon/translation tests provide partial evidence. The current verdict text
fails the already approved 4.5:1 contrast criterion in some row states; correction
implemented in `235b07d`; U1-U4 recorded as passed on 2026-11-02 (see the spec's
recorded results).

---
id: RNF-10
title: Look and feel
priority: high
milestone: H1/H2
status: specified
---

**Look and feel.** Windows desktop application styled like the reference tool (docs/reference/): single window, icon toolbar, folder tree, dense table, detail panel. Own assets, no copied branding. For v1 the Albums panel takes the place of the folder tree (decided by the owner on 2026-11-20); the tree itself would be a future requirement.

Source: docs/planning/fakeflac-phases-1-3.es.md (section 2.4).

Acceptance (approved at G1, #167; questions Q1–Q7 answered by the owner on 2026-11-20): `specs/ui/look-and-feel.md` (element map L1–L7,
asset register, checks A1–A2, manual check M1), with scenarios in
`specs/features/look-and-feel.feature`. Measurable: (K1) the automated scenarios S1–S6 pass
in `just check`, among them that no shipped file is byte-identical to a reference screenshot
(S6); (K2) every shipped image, icon and font file and every `IconName` has a row with a
recorded origin in the asset register; (K3) M1 on a release build records each element E1–E7 as pass or as a
deliberate difference accepted at G1. Decided (Q1): the Albums panel takes the place
of the folder tree for v1, and the tree itself is recorded as a deliberate difference.

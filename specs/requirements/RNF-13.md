---
id: RNF-13
title: Project website
priority: medium
milestone: H3
status: draft
---

**Project website.** FakeFlac has a public website, in English and Spanish, that explains what it does, how it decides, how accurate it is and how to download and check it.

Measurable acceptance (to be fixed at G1 of its spec):
- **Content.** What FakeFlac detects; explained verdicts (RF-17) with an example; a plain-language summary of the detection; the measured accuracy with its known limitations; privacy by design (RNF-03, RNF-05); how the project is built; download with the `SHA256SUMS` check and the SmartScreen note (ADR-0032); FAQ, licence and links.
- **Privacy.** No analytics, trackers, third-party fonts or cookies by default; no corpus names, paths or real-library examples (ADR-0022); screenshots from the synthetic fixtures or `docs/reference/`.
- **Quality.** Both languages complete (RF-13); the contrast rules of RNF-10; links and the named release checked in CI, so the site never points to a missing release.
- **Hosting.** Static, built from this repository.

Source: owner's decision of 2026-11-20 (H3 extension, #184), with the site of a comparable tool (tool A) as the benchmark for quality. Issue #205.

Spec: pending. Owner decisions in #205 before the spec: when (after the accuracy figures of #195 are stable), hosting and domain, and the stack (plain HTML/CSS or a static generator that reuses the app's Svelte components and tokens); an ADR records them.

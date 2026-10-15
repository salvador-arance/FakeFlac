# specs/ — rules for writing specs

- One requirement per file in `requirements/` named `RF-xx.md` / `RNF-xx.md` / `RP-xx.md`, with front matter `id`, `priority`, `milestone`, `status`.
- Scenarios live in `features/*.feature` (Gherkin) and carry the tag of the requirement they cover (`@RF-04`). CI fails on unknown tags and on high-priority RF without a scenario (S1, S2).
- A spec is **Ready** when: requirement id cited, scenarios reviewed, affected contract defined, acceptance criterion measurable.
- Contracts (`contracts/`) describe the backend ↔ frontend surface; TS types are generated from Rust, never hand-written (S3).
- Detection rules and parameters (`detection/`) are versioned data (`algorithm_version`).
- Never change an approved spec silently. Propose the change in a `spec/…` PR for gate G1.

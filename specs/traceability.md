# Traceability

> Requirement → spec → test → module. Validated by `just trace` (S1, S2). Updated by `docs-keeper` / `/handoff`.

| Requirement | Milestone | Spec | Tests | Module |
|---|---|---|---|---|
| RF-10 | H0 smoke test (H2 feature) | `specs/detection/album-summary.md`, `specs/features/album-summary.feature` | pending | `fakeflac-core` |
| other RF | see `specs/requirements/` | pending (`/new-spec`) | — | — |
| RNF-01 … RNF-11 | see `specs/requirements/` | pending | — | — |

Requirements are in `specs/requirements/`. Every `@RF-xx` tag in `specs/features/*.feature` must exist there.
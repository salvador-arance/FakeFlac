# Lessons

> Agent mistakes that repeat (>= 2 times) are fixed in the harness (rule, hook, lint or skill), not only in the code (RP-10). One entry per mistake: what happened, what the harness now does about it.

## ADR status left as `proposed` after G1 (2026-10-18)

- **What happened:** ADRs 0033–0038 were approved at G1 (their spec PRs were merged), but their status stayed `proposed` in each ADR and in `docs/adr/README.md`. Docs then said things were "pending until ADR-xxxx is accepted" when they already were. The traceability matrix also kept "UI pending" for RF-01, RF-05 and RF-06 after their UI and tests shipped in #27.
- **Harness fix:** `/handoff` now sets ADRs merged at G1 to `accepted` and updates traceability for tests added in feature PRs.
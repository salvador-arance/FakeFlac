# ADR-0016: Human gates only at spec/ADR (G1) and at merge (G2)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

The human decides the what and accepts the result; the how is verified by tests and CI.

## Decision

Human gates only at spec/ADR (G1) and at merge (G2).

## Alternatives considered

Full autonomy with auto-merge; human review of every step

## Consequences

Result quality depends on spec quality: G1 is the review that matters most.

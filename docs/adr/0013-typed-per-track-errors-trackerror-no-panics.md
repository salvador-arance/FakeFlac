# ADR-0013: Typed per-track errors (TrackError), no panics

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Satisfies RNF-04: one broken track does not stop the batch.

## Decision

Typed per-track errors (TrackError), no panics.

## Alternatives considered

Result<_, String>; unwrap

## Consequences

More error variants to maintain.

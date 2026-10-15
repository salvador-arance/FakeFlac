# ADR-0006: Parallelism with rayon and cooperative cancellation (atomic flag/token)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

CPU-bound batch workload; rayon gives work stealing and a thread cap.

## Decision

Parallelism with rayon and cooperative cancellation (atomic flag/token).

## Alternatives considered

tokio::spawn_blocking per track; manual threads

## Consequences

Analysis runs in its own pool; the token is checked between windows.

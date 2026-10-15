# ADR-0006: Parallelism with rayon and cooperative cancellation (atomic flag/token)

- Status: accepted; thread pool partially superseded by ADR-0034 (cooperative cancellation stays)
- Date: 2026-10-15

## Context and rationale

CPU-bound batch workload; rayon gives work stealing and a thread cap.

## Decision

Parallelism with rayon and cooperative cancellation (atomic flag/token).

## Alternatives considered

tokio::spawn_blocking per track; manual threads

## Consequences

Analysis runs in its own pool; the token is checked between windows.

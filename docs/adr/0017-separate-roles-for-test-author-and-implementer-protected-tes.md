# ADR-0017: Separate roles for test author and implementer; protected tests

- Status: accepted
- Date: 2026-10-15

## Context and rationale

An agent that can change its own tests can approve itself (R6).

## Decision

Separate roles for test author and implementer; protected tests.

## Alternatives considered

A single agent doing the whole TDD loop

## Consequences

More hand-offs; the test author may create `todo!()` signatures so the code compiles.

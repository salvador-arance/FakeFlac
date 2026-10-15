# ADR-0001: Hexagonal architecture on a multi-crate workspace

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Isolates the detection engine (the most valuable and delicate asset), makes it testable with synthetic signals and enables a future CLI.

## Decision

Hexagonal architecture on a multi-crate workspace.

## Alternatives considered

A single monolithic crate; classic UI -> logic -> data layers

## Consequences

More files and trait boilerplate at the start.

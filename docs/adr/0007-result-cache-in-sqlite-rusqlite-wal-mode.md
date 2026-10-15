# ADR-0007: Result cache in SQLite (rusqlite, WAL mode)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Key lookups, indexes and transactions; one file that is easy to delete. Confirmed by the owner.

## Decision

Result cache in SQLite (rusqlite, WAL mode).

## Alternatives considered

JSON files per folder; sled

## Consequences

One more native binary (bundled); hand-written migrations.

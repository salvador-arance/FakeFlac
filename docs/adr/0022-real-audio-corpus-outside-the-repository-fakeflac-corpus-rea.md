# ADR-0022: Real audio corpus outside the repository (FAKEFLAC_CORPUS), read-only; only generated synthetic fixtures in the repo

- Status: accepted
- Date: 2026-10-15

## Context and rationale

The repo is public and music is copyrighted; synthetic signals are reproducible.

## Decision

Real audio corpus outside the repository (FAKEFLAC_CORPUS), read-only; only generated synthetic fixtures in the repo.

## Alternatives considered

Committing FLAC/MP3 to the repo; Git LFS

## Consequences

Acceptance tests with real music run only locally; CI uses synthetic fixtures.

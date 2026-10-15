# ADR-0008: Cache key = (canonical path, size, mtime, algorithm_version)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Hashing 40 MB per track costs almost as much as analysing it; size+mtime is enough (files are never modified, RNF-05).

## Decision

Cache key = (canonical path, size, mtime, algorithm_version).

## Alternatives considered

Content hash

## Consequences

A file edited keeping size and mtime would be served from cache (accepted, low risk).

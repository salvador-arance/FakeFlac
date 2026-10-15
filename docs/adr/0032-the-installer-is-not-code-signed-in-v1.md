# ADR-0032: The installer is not code-signed in v1

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Cost; the project is open source with no income. Decided by the owner.

## Decision

The installer is not code-signed in v1.

## Alternatives considered

Paid certificate (100-400 EUR/year)

## Consequences

Windows SmartScreen shows a warning on fresh installs; documented in the release notes and SHA-256 checksums are published.

# ADR-0019: justfile as the single command entry point

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Agents, hooks and CI run exactly the same; just works on Windows.

## Decision

justfile as the single command entry point.

## Alternatives considered

Scattered npm scripts and cargo aliases; Makefile

## Consequences

One more tool to install.

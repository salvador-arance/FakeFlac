# ADR-0021: Parallelism with git worktree: one agent per branch and worktree

- Status: accepted
- Date: 2026-10-15

## Context and rationale

No file conflicts; backend and frontend advance in parallel once the contract is fixed.

## Decision

Parallelism with git worktree: one agent per branch and worktree.

## Alternatives considered

A single sequential session; several agents on the same copy

## Consequences

More branches to integrate; at most 2-3 agents at once so human review stays viable.

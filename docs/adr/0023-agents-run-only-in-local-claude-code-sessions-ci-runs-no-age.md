# ADR-0023: Agents run only in local Claude Code sessions; CI runs no agents. Pre-PR review is done locally by the `code-reviewer` subagent

- Status: accepted
- Date: 2026-10-15

## Context and rationale

A single way of working, no Claude credentials in GitHub and no bot account; CI stays deterministic and reproducible with `just ci`. Decided by the owner.

## Decision

Agents run only in local Claude Code sessions; CI runs no agents. Pre-PR review is done locally by the `code-reviewer` subagent.

## Alternatives considered

Automatic PR review with claude-code-action on GitHub; a bot account

## Consequences

The second opinion depends on local session discipline; final authority is CI plus human review at G2.

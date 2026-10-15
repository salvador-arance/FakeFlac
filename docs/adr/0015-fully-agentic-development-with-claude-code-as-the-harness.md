# ADR-0015: Fully agentic development with Claude Code as the harness

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Subagents, skills, hooks, permissions and MCP are versioned files; it has a headless mode.

## Decision

Fully agentic development with Claude Code as the harness.

## Alternatives considered

Manual development with AI autocomplete; other agents (Codex CLI, Cursor, Aider); a custom orchestrator with the Agent SDK

## Consequences

Vendor dependency; mitigated with plain Markdown instructions and checks in `just`, reusable from another tool.

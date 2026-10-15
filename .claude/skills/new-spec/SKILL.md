---
name: new-spec
description: Creates the spec for a requirement - scenarios, contract and traceability entry. Usage - /new-spec RF-xx
---
# /new-spec <RF-xx>

1. Set stage `spec` (`.claude/stage`). Branch `spec/RF-xx`.
2. Delegate to the `spec-writer` subagent with the requirement id.
3. Run `just trace`. Delegate a read-only review to `code-reviewer` focused on spec readiness (Definition of Ready in `specs/CLAUDE.md`).
4. Open a PR `spec(RF-xx): ...` and stop. The human approves at gate G1 and merges. Only then may `/feature RF-xx` run.
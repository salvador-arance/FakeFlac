---
name: new-spec
description: Creates the spec for a requirement - scenarios, contract and traceability entry. Usage - /new-spec RF-xx
---
# /new-spec <RF-xx>

1. Set stage `spec` (`.Codex/stage`). Branch `spec/RF-xx`.
2. Delegate to the `spec-writer` subagent with the requirement id.
3. Run `just trace`. Delegate a read-only review to `code-reviewer` focused on spec readiness (Definition of Ready in `specs/AGENTS.md`).
   If the spec sets figures that can be measured locally (detection thresholds marked "indicative", timings, delays or anything a test will wait on), have them measured before G1 with ffmpeg, the corpus or a throwaway harness under `target/` (`docs/agents/lessons.md`, RF-16 and RNF-01).
4. Open a PR `spec(RF-xx): ...` and stop. The human approves at gate G1 and merges. Only then may `/feature RF-xx` run.
5. After the merge: run `/handoff` on a `docs/...` branch (ADR accepted, `progress.md`, stale architecture docs) and open its PR.
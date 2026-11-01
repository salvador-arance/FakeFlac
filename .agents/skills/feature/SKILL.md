---
name: feature
description: Orchestrates the build cycle of one requirement after its spec is approved (gate G1) - tests, implementation, review, PR. Usage - /feature RF-xx
---
# /feature <RF-xx>

Precondition: `specs/` contains an approved spec for the requirement (merged `spec/RF-xx` PR). If not, stop and tell the human to run `/new-spec` and pass gate G1.

Procedure (each step sets the stage in `.claude/stage`, which the hooks enforce):
1. `git switch -c feat/RF-xx` (or a worktree). Run `just check` to confirm the starting point is green; if red, fixing it is the first task.
2. **Stage `test`**: write `test` to `.claude/stage`; delegate to the `test-author` subagent. Result: failing tests for the expected reason and a commit `test(RF-xx): ...`.
3. **Stage `implement`**: write `implement` to `.claude/stage`; delegate to `rust-implementer` for `crates/` and `src-tauri/`, and to `svelte-implementer` for `src/`. A change that spans both gets one agent per side, Rust first. Iterate until `just check` is green. Commit `feat(RF-xx): ...`.
4. **Stage `review`**: write `review` to `.claude/stage`; delegate to `code-reviewer`. Blocking findings go back to step 3 (set the stage back to `implement`).
5. Write the stage file empty (or delete it). Run `just ci`.
6. Open the PR (ask first; it uses the PR template: cite RF/RNF, spec, issue, plan, checks run).
7. Run `/handoff`. Never merge: gate G2 is the human's.

If at any point the spec or a test looks wrong: stop, explain, and return to gate G1. Never adjust them silently.

**One agent per iteration.** Every time a stage runs again (back to `test`, back to `implement`, each review round), launch a fresh agent of that type: test author, implementer, reviewer or any other subagent. Do not resume a finished one with `SendMessage`. Give the new agent a short brief instead: scope agreed with the owner, what earlier rounds decided and changed (commit hashes), and exactly what to do now. The `guard-agents` hook blocks resuming any subagent whose context is over 150k tokens (`FAKEFLAC_AGENT_CONTEXT_LIMIT`).
<!-- Generated from .claude/skills/feature/SKILL.md; run just agents-sync. Do not edit. -->

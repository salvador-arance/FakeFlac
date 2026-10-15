# Agent workflow

> Spec-Driven Development executed by agents. Two human gates: **G1** approves specs and ADRs, **G2** merges to `main`. Every rule that matters is enforced by a tool (hook, lint or CI check), not only by text.

## Cycle of one requirement

```
requirement RF-xx
  → /new-spec RF-xx   (stage spec: spec-writer → PR spec/RF-xx)        → G1: human approves and merges
  → /feature RF-xx    (branch feat/RF-xx)
      stage test       : test-author writes failing tests, commit test(RF-xx)
      stage implement  : implementer makes them pass; specs and tests are protected
      stage review     : code-reviewer (read-only); blocking findings go back to implement
      just ci → PR (feat/RF-xx)                                         → G2: human reviews and merges
  → /handoff          (update docs/agents/progress.md)
```

- Two PRs per feature: `spec/RF-xx` (specs, contract, ADR) and `feat/RF-xx` (tests and implementation). Specs of one milestone may be grouped in a single spec PR.
- The stage lives in `.claude/stage` (ignored by git). Hooks read it, so protection does not depend on which agent is writing.
- If a spec or test looks wrong: stop, report, return to G1. Never adjust it silently.
- At most 2–3 agents in parallel (git worktrees, ADR-0021).
- Commit messages are plain Conventional Commits. **Never add a `Co-Authored-By` trailer or any Claude attribution** to commits or PR descriptions (owner's rule).

## Rule → mechanism

| Rule | Mechanism |
|---|---|
| Nothing without an approved spec (S5) | `/feature` precondition, PR template, CODEOWNERS on `specs/` |
| Specs and tests are immutable while implementing (RP-04) | `guard-protected` hook (stage `implement`) + `just guard-tests` in CI |
| Agents never write outside the repo (RP-06) | `guard-protected` hook + `deny` permissions |
| Humans merge (RP-02) | `deny` for `gh pr merge` and `git push --force`; `guard-bash` blocks pushes to `main` |
| Same checks everywhere (RP-03) | CI only runs `just` recipes |
| Crate and atomic design boundaries (S8) | `just lint-arch` |
| Requirement ↔ scenario traceability (S1, S2) | `just trace` |
| Contract generated, never hand-written (S3) | `just gen` + diff check (when the first contract exists) |
| Audio corpus is read-only (ADR-0022) | `guard-bash` + external `FAKEFLAC_CORPUS` |

## Local, personal settings

`.claude/settings.local.json` (not versioned) holds `FAKEFLAC_CORPUS`, the deny rule for the user's music library and personal preferences. No personal path or secret goes in versioned files.

## Hooks

`SessionStart` context · `PreToolUse` guards (`guard-protected`, `guard-bash`) · `PostToolUse` format · `Stop`/`SubagentStop` `just check-fast` (skipped in stages `spec` and `test`). Hooks have their own tests: `node --test ".claude/hooks/*.test.mjs"`.
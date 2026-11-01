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
  → after G2: just prune-branches (merged local/remote branches, clean agent worktrees)
```

- Two PRs per feature: `spec/RF-xx` (specs, contract, ADR) and `feat/RF-xx` (tests and implementation). Specs of one milestone may be grouped in a single spec PR.
- The stage lives in `.claude/stage` (ignored by git). Hooks read it, so protection does not depend on which agent is writing.
- If a spec or test looks wrong: stop, report, return to G1. Never adjust it silently.
- At most 2–3 agents in parallel (git worktrees, ADR-0021).
- One agent per iteration: each rerun of a stage (test, implement, review) gets a fresh agent with a short brief, never a resumed one (`guard-agents` hook).
- Commit messages are plain Conventional Commits. **Never add a `Co-Authored-By` trailer or any Claude attribution** to commits or PR descriptions (owner's rule).

## Rule → mechanism

| Rule | Mechanism |
|---|---|
| Nothing without an approved spec (S5) | `/feature` precondition, PR template, CODEOWNERS on `specs/` |
| Specs and tests are immutable while implementing (RP-04) | `guard-protected` hook (stage `implement`) + `just guard-tests` in CI |
| Agents never write outside the repo (RP-06) | `guard-protected` hook + `deny` permissions |
| Humans merge (RP-02) | `deny` for `gh pr merge` and `git push --force`; `guard-bash` blocks pushes to `main` |
| No overloaded agents: one fresh agent per iteration | `guard-agents` blocks `SendMessage` to a subagent over 150k tokens of context (`FAKEFLAC_AGENT_CONTEXT_LIMIT`) |
| Same checks everywhere (RP-03) | CI only runs `just` recipes |
| Crate and atomic design boundaries (S8) | `just lint-arch` |
| Requirement ↔ scenario traceability (S1, S2) | `just trace` |
| Contract generated, never hand-written (S3) | `just gen-check` (regenerates and diffs the bindings; part of `check-app`, so it runs in CI) |
| Audio corpus is read-only (ADR-0022) | `guard-bash` + external `FAKEFLAC_CORPUS` |
| Only the human releases (ADR-0045) | `guard-bash` blocks tag pushes and `gh release create\|edit\|upload`; `release-draft` runs only in GitHub Actions |

## CI

`.github/workflows/ci.yml` runs jobs `lint`, `specs`, `test-rust`, `test-web`, `supply-chain` and `build-windows`; the aggregator `ci-ok` is the one required check. `.github/workflows/release.yml` has `verify` (`just release-check` on a tag, `just version-check` on a dry run, then `just ci`), `package` (`just build`, `just release-assets`) and `publish` (`just release-draft`, tag pushes only). A manual run is a dry run that never creates a release.

## Local, personal settings

`.claude/settings.local.json` (not versioned) holds `FAKEFLAC_CORPUS`, the deny rule for the user's music library and personal preferences. No personal path or secret goes in versioned files. When loading this JSON from Windows PowerShell 5.1, use `Get-Content -Raw -Encoding UTF8`; the default ANSI decoding can corrupt non-ASCII paths.

## Hooks

### Single source of truth (ADR-0048)

Maintain instructions in root/scoped `CLAUDE.md` files and roles, skills and hooks under `.claude/`. Run `just agents-sync` after changing them. It derives sibling `AGENTS.md` files, `.agents/skills/`, `.codex/agents/`, hook entry points and `.codex/hooks.json`; never edit these outputs by hand. `just agents-check` is read-only and runs in `check-fast` to catch drift locally and in CI. The `.claude/stage` file is shared by both harnesses. Claude models, workflow, checks and gates remain unchanged.

Codex model and reasoning-effort assignments are maintained only in `.claude/codex-models.json` and emitted into the generated TOML files. The owner-approved defaults are Astra/high for specs and review, Sol/high for tests and Rust, and Sol/medium for Svelte. An incomplete or invalid assignment fails before any output changes. Editing Claude's `model` aliases does not implicitly substitute an OpenAI model; the explicit provider assignments remain in this canonical source directory.

The Codex dispatcher translates native payloads and invokes the canonical Claude hooks; it contains no duplicate policy. Claude model/tool metadata is recorded in generated comments, not interpreted as Codex configuration or enforcement. Claude permissions are not translated into automatic Codex approvals. Native Codex transcript-based context limits and host permissions still require separate live-host verification. Contract and acceptance: `specs/contracts/agent-harness.md`.

Provider format references: [OpenAI hooks](https://learn.chatgpt.com/docs/hooks), [custom agents](https://learn.chatgpt.com/docs/agent-configuration/subagents) and [scoped instructions](https://learn.chatgpt.com/docs/agent-configuration/agents-md).

`SessionStart` context · `PreToolUse` guards (`guard-protected`, `guard-bash`, `guard-agents`) · `PostToolUse` format · `Stop`/`SubagentStop` `just check-fast` (skipped in stages `spec` and `test`). Codex hook commands resolve their scripts from `git rev-parse --show-toplevel`, so they work across checkouts and when the session starts in a subdirectory (no machine-specific paths). Hooks have their own tests: `node --test ".claude/hooks/*.test.mjs"`. `guard-bash` matches the command text, so a commit message or `echo` that quotes a blocked command (for example `gh release create`) is blocked too: reword it or use `git commit -F <file>`.

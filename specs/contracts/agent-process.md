---
id: SPEC-AGENT-PROCESS
requirements: [RP-02, RP-05, RP-07, RP-08, RP-09]
status: proposed
---

# Agent process: what enforces RP-02, RP-05, RP-07, RP-08 and RP-09 today

This spec gives each of the five agent-process requirements a criterion that exists today. It adds no hook, script, recipe, workflow, skill or dependency, and it changes no approved spec. Each mechanism below was read in the repository at the time of writing (line numbers refer to that revision). Where nothing enforces a requirement, the spec says so and either names the owner check that replaces it or raises a G1 question. Owner decision for issue #158 step 5, option A: one short spec for the five requirements without scenarios.

The other five process requirements (RP-01, RP-03, RP-04, RP-06, RP-10) are specified by `agent-harness.md` and `../features/agent-harness.feature`. RP-11 and RP-12 have their own specs. RNF-04 and RNF-09 do not apply here, because this spec reads no audio file and no file name.

Scenarios: `../features/agent-process.feature`. Each scenario covers only behavior that an existing test or CI command checks today. Owner checks are written in this contract, not as scenarios.

**Owner check** means a manual step by the owner. It has a cadence and leaves a record in the `/handoff` block of `docs/agents/progress.md` that closes the milestone. **Milestone close** means the handoff that declares a milestone (H0 to H3) finished. Issue #158 is the H2 close.

## 1. RP-02: human gates

**Statement.** Approving specs and ADRs (G1) and merging to `main` (G2) are exclusively human actions.

**What enforces it today**

| Mechanism | Where | What it does |
|---|---|---|
| `guard-bash` hook (PreToolUse, matcher `Bash\|PowerShell`) | `.claude/hooks/guard-bash.mjs` lines 9–11; registered in `.claude/settings.json` lines 48–51 | Blocks (exit 2) `git push` with `--force`, `--force-with-lease` or `-f`, any `git push` whose arguments name `main`, and `gh pr merge` |
| `deny` permissions | `.claude/settings.json` lines 31–33 | `git push --force`, `git push -f` and `gh pr merge` are denied to the Claude `Bash` tool |
| `ask` permissions | `.claude/settings.json` lines 23–24 | Every `git push` and `gh pr create` waits for the human to confirm it, unless a local setting or a permission mode skips the prompt |
| Instructions | `CLAUDE.md` line 18; `.claude/skills/feature/SKILL.md` line 16; `.claude/skills/new-spec/SKILL.md` line 11; `docs/agents/workflow.md` lines 3 and 33 | Never merge or push to `main`; open the PR and stop; the human approves at G1 and merges at G2 |
| `CODEOWNERS` | `.github/CODEOWNERS` lines 1–6 | Requests the owner's review on `specs/`, `docs/adr/`, `.claude/`, `.github/` and `CLAUDE.md` |
| Required check | `.github/workflows/ci.yml` lines 116–124 (`ci-ok`, comment "Single required check for branch protection") | Aggregates every CI job; the owner merges only when it is green |

Tests: `.claude/hooks/hooks.test.mjs` lines 72–108 (`guard-bash: "<command>" -> exit <code>`), for the rows `git push --force origin feat/x`, `git push -f`, `git push origin main` and `gh pr merge 12 --squash` (exit 2) and `git push origin feat/RF-04` and `gh pr view 12` (exit 0). The Codex parity test `scripts/codex-hooks.test.mjs` lines 87–98 (`@RP-03 Preserve canonical shell and lifecycle hook behavior: exec_command <command>`) checks `git push origin main` and `gh pr merge 7` (exit 2) through both payload shapes. Both files run in `just check-fast` (`justfile` line 15), which runs in the CI job `lint` (`ci.yml` line 33).

G1 has no hook of its own. A spec or ADR is approved when the owner merges its `spec/…` PR (`/new-spec` step 4) or writes the approval in chat (`agent-harness.md` line 3 records one). The merge itself is protected as described above.

**Measurable acceptance**

- A1 (automated): `node --test ".claude/hooks/*.test.mjs"` exits 0, and so does `just check-fast`, which includes it. Every row listed above is part of that run.
- A2 (owner check, at every milestone close): in the repository settings on GitHub, the protection of `main` still requires a pull request and the `ci-ok` check, and it blocks force-pushes and branch deletion. The owner records one line: `RP-02 review <milestone>: branch protection checked`.
- A3 (owner check, at every G1 and G2): the owner, not an agent, performs the merge.

**Not enforced**

- Branch protection of `main` is a GitHub setting and cannot be seen in the repository. The only trace in the repository is the comment on `ci-ok`. Whether administrators can bypass it, and whether "Require review from Code Owners" is on, is also not visible. Planning section 3.11.11 (d) says that formal PR approval is not used and that G2 is the merge itself, so `CODEOWNERS` probably only requests a review.
- `guard-bash` matches command text, so it is a guard rail, not a security boundary. Several forms are not matched: `git -C dir push origin main`, `git -c k=v push …`, git aliases, scripts that push, and `gh api` calls to the merge endpoint. `release.md` line 511 records the same limit for tags and releases. A bare `git push` while `main` is checked out does not name `main` either. The `ask` permission on `git push` and branch protection remain the boundary.
- The `deny` rules apply to the Claude `Bash` tool only. The Codex harness does not translate them (`agent-harness.md` section 4).

## 2. RP-05: resumability

**Statement.** A fresh session can continue the work from `CLAUDE.md`, `docs/agents/progress.md` and the issue.

**What enforces it today**

| Mechanism | Where | What it does |
|---|---|---|
| `session-context` hook (SessionStart) | `.claude/hooks/session-context.mjs` lines 18–28; registered in `.claude/settings.json` lines 38–42 | Injects the branch, the stage, `git status --short` and the newest `## ` block of `docs/agents/progress.md` |
| `/handoff` skill | `.claude/skills/handoff/SKILL.md` lines 7–14 | Adds a block at the top of `progress.md` with **Done**, **Pending**, **Blockers / questions for the human** and **Next step** |
| Instructions | `CLAUDE.md` lines 54 and 59; `docs/agents/workflow.md` line 15; `docs/agents/progress.md` line 3 | Read the newest block before starting; run `/handoff` at the end of a session |

Test: `scripts/codex-hooks.test.mjs` lines 110–120 (`@RP-03 Preserve canonical shell and lifecycle hook behavior: same newest progress context`). It runs the canonical hook and the Codex adapter with stage `implement` and a two-block `progress.md`. It asserts that both outputs are equal, that the output names the stage and contains the newest block, and that it does not contain the older block. It runs in `just check-fast`.

**Measurable acceptance**

- A1 (automated): the test above passes in `just check-fast`.
- A2 (owner check, at every milestone close; this is the "control test" of planning section 2.9): start a fresh session with no history in a clean worktree of `main`, and ask for the current state and the next step without giving any other input. The check passes when the answer names the **Next step** of the newest `progress.md` block and the issue it belongs to. The owner records one line: `RP-05 review <milestone>: pass | fail (<what was missing>)`.

**Not enforced**

- Nothing checks that `/handoff` ran at the end of a session. The Stop hook runs only `just check-fast`, and it is skipped in stages `spec` and `test`.
- The format of the `/handoff` block is not validated, and the skill does not require the block to cite the issue.
- No test asserts the branch line or the `git status` line of `session-context`. `.claude/hooks/hooks.test.mjs` has no `session-context` test; only the Codex parity test above covers the hook.
- The H0 criterion "a fresh session explains the architecture, the rules and the next step" (planning section 3.12, H0 acceptance) has no record in the repository. Whether it was ever run cannot be verified.

## 3. RP-07: work traceability

**Statement.** Every PR cites the RF/RNF, the spec and the issue, and includes the plan and the checks run.

**What enforces it today**

| Mechanism | Where | What it does |
|---|---|---|
| PR template | `.github/pull_request_template.md` lines 3–7 (References (RP-07): requirement, spec, issue, ADR), lines 9–10 (Plan followed), lines 12–18 (Checks run) | GitHub pre-fills the PR body with these sections when the PR is opened in the web UI or with an interactive `gh pr create` |
| `/feature` | `.claude/skills/feature/SKILL.md` line 15 | The PR "uses the PR template: cite RF/RNF, spec, issue, plan, checks run" |
| Commit scopes | `CLAUDE.md` line 20 | Conventional Commits with the requirement as scope (`feat(RF-04): …`) |
| Traceability matrix | `specs/traceability.md`; `.claude/skills/handoff/SKILL.md` line 14 | `/handoff` updates the matrix when requirements move |
| `just trace` | `scripts/trace.mjs` lines 24–31 (every `@RF-`, `@RNF-` and `@RP-` tag in `specs/features` must name a file in `specs/requirements`) and lines 34–40 (a high-priority RF marked `specified` must have a scenario); `justfile` lines 46–48; CI job `specs`, `ci.yml` line 44; `just ci`, `justfile` line 69 | Fails CI on an unknown tag or on a high-priority RF without a scenario |

Test: `just trace` itself, run on the repository in every PR (CI job `specs`) and in `just ci`. `scripts/trace.mjs` has no test file.

**Measurable acceptance**

- A1 (automated): `just trace` exits 0.
- A2 (owner check, at every G1 and G2): the PR body has the References section filled with the requirement ids, the spec path and the issue number, plus Plan followed and Checks run. A PR that lacks them goes back before it is merged.

**Not enforced**

- An agent that opens a PR with `gh pr create --body` or `--body-file` writes the body itself, so the template is not applied; the agent copies its sections by instruction only.
- No CI step reads the PR body. Planning section 3.11.11 (c) planned a "PR template validation (RP-07)" in the `specs` job. That job (`ci.yml` lines 35–47) runs `trace`, `lint-arch`, `guard-tests` and `version-check` only.
- The template's requirement line offers `RF-xx / RNF-xx`; RP ids are written in the same line by hand.
- Commit scopes and PR titles are not checked. `guard-tests` reads only the `test(` prefix.
- `just trace` never reads `specs/traceability.md`, although that file's line 3 says the matrix is "Validated by `just trace`". Missing or stale rows go unnoticed.
- The failure paths of `just trace` (an unknown tag, a high-priority RF without a scenario) have no test.

## 4. RP-08: fast feedback

**Statement.** Per-edit hooks run in under 5 s; `just check-fast` runs in under 60 s.

**Scope.** The per-edit hooks are the PreToolUse and PostToolUse hooks of `.claude/settings.json` lines 43–62: `guard-protected` (`Edit|Write|NotebookEdit`), `guard-bash` (`Bash|PowerShell`), `guard-agents` (`SendMessage`) and `format` (`Edit|Write`). The Stop and SubagentStop hooks run `just check-fast` with a 300 s timeout (lines 63–72); they run once per stop, not per edit.

**What keeps it fast today (design, not measurement)**

- The guards read stdin and match paths or regular expressions, and spawn no process. `guard-agents` reads one subagent transcript (`.claude/hooks/guard-agents.mjs` lines 31–35).
- `format` formats only the touched file, and only a `.rs` file, with `rustfmt` (`.claude/hooks/format.mjs` lines 10–12); it never blocks. The heavy checks are in Stop and in CI (planning section 3.13, D8).
- `check-fast` holds fmt, clippy, `svelte-check`, `lint-arch`, `agents-check` and the Node tests (`justfile` lines 8–15; line 8 comment: "Target < 60 s (RP-08)"). The Rust and Vitest suites are in `test`, not `check-fast`. `release.md` line 486 gives `release.test.mjs` a 10 s budget so that `check-fast` stays under 60 s.

**Measured**

- `just check-fast`: 24 s, exit 0, on 2026-11-19 on the development machine, warm build caches, branch based on `85dad04`. This is one warm-cache run and only indicative.
- Per-edit hooks: **not measured.** No figure exists in the repository.

**Measurable acceptance**

- A1 (owner check, at every milestone close and whenever a step is added to `check-fast`): on the development machine with warm caches, `Measure-Command { just check-fast }` reports under 60 s and the recipe exits 0.
- A2 (owner check, same cadence): for each tool call that triggers per-edit hooks, the hooks it triggers finish in under 5 s in total. For an `Edit` or `Write` that is `guard-protected` plus `format`; for a shell command it is `guard-bash`; for `SendMessage` it is `guard-agents`. Measure from the repository root with a sample payload. Use a `.rs` file that is already formatted, so `format` changes nothing. For example:
  `Measure-Command { '{"tool_input":{"file_path":"crates/fakeflac-core/src/lib.rs"}}' | node .claude/hooks/format.mjs }`
  For `guard-agents`, use the largest subagent transcript of a recent session.
- Remedy (planning section 2.9, "whatever exceeds it moves to CI"): a step that pushes `check-fast` over 60 s, or work that pushes a hook over 5 s, moves out of the fast path to CI. Where exactly it moves (the `test` recipe, the Stop hook, a CI job) is decided in the change that moves it.
- The owner records one line: `RP-08 review <milestone>: check-fast <s> s (warm); hooks max <s> s`.

**Not enforced**

- No test or CI step asserts a time. `hooks.test.mjs` runs every hook but measures nothing.
- `.claude/settings.json` sets no `timeout` on the PreToolUse or PostToolUse hooks, so the host's default applies; the 5 s bound is not configured anywhere.
- CI job times are visible in GitHub but are not compared with any bound.

## 5. RP-09: bounded cost

**Statement.** A monthly agent usage budget is defined and reviewed per milestone.

**What exists today.** There is no budget figure anywhere in the repository; no file in `docs/` or `specs/` states a monthly amount or plan for agent usage. Planning section 1.7 gives only an indicative range for Claude, "20–200 $/month depending on plan and usage", and section 1.10 says the usage plan is "a personal decision outside the document". No record of a per-milestone review was found. These mechanisms limit cost, but none of them is a budget:

- Model per role: ADR-0024, applied as the `model:` field on line 5 of each `.claude/agents/*.md`.
- Never resume a large subagent: the `guard-agents` hook, with a 150k-token limit (`.claude/hooks/guard-agents.mjs` lines 8 and 34–36, overridable with `FAKEFLAC_AGENT_CONTEXT_LIMIT`). Tested in `.claude/hooks/hooks.test.mjs` lines 141–161.
- At most 2–3 agents in parallel, and one fresh agent per iteration (`docs/agents/workflow.md` lines 22–23).
- No agents run in CI (ADR-0023), so CI incurs no agent cost.

**What "defined and reviewed per milestone" means (owner checklist).** At every milestone close:

1. **Defined.** A monthly budget exists, recorded where Q1 decides, with the figure given by the owner (Q2). This spec proposes no figure.
2. **Reviewed.** The owner compares the agent usage of the months the milestone spanned with the budget, using the provider's usage or billing page. Agents cannot read that page.
3. **Recorded.** The milestone-closing `/handoff` block has one line: `RP-09 review <milestone>: within | over budget; action: <none | …>`. If Q1 keeps the figure private, the line carries no amount.
4. **Acted on.** When usage is over budget, the action is named. Examples: change the model assignment (ADR-0024), lower `FAKEFLAC_AGENT_CONTEXT_LIMIT`, or run fewer agents in parallel.

**Measurable acceptance.** For every milestone closed after this spec's G1, `Select-String -Path docs/agents/progress.md -Pattern 'RP-09 review'` finds a line for that milestone.

**Not enforced.** Everything in this section is an owner check. No tool knows the budget or the usage. Until Q2 is answered, the "defined" part of RP-09 is unmet.

## 6. G1 questions

Each question has a default, which applies if the owner accepts it at G1.

| # | Requirement | Question | Default |
|---|---|---|---|
| Q1 | RP-09 | Where is the monthly budget recorded? (a) As a figure in the repository, in `docs/agents/workflow.md`. (b) Outside the repository (the repository is public), with only the review result recorded in `progress.md`. | (b), consistent with planning section 1.10. |
| Q2 | RP-09 | What is the monthly budget? Give an amount or a plan tier. | None proposed: the owner gives it. Under Q1 (b), the owner confirms only that it is defined. |
| Q3 | RP-09, RP-02, RP-05, RP-08 | When are the owner checks first done? | At the H2 close (#158), with no retroactive reviews of H0 and H1. |
| Q4 | RP-09 | Is "cost per feature" (planning section 3.11.12) required? | No. It is an optional note in the review line. |
| Q5 | RP-08 | Is the 5 s bound per hook, or the total of the hooks one tool call triggers? | The total per tool call, as in section 4 A2. |
| Q6 | RP-08 | Should the bound be enforced, with a timing assertion in the hook tests or a `timeout` on the PreToolUse and PostToolUse hooks in `.claude/settings.json`? | No. It stays an owner measurement. A timing assertion on shared runners would fail at random (the reasoning of ADR-0046). |
| Q7 | RP-08 | Is `just check-fast` measured with warm or cold caches? | Warm. A cold run may be reported but is not held to the bound. |
| Q8 | RP-07 | Should CI check the PR body (References filled: requirement id, spec path, issue number)? This needs a new script and a workflow change. | No, not now. It stays an owner check at G1 and G2 and is recorded as a gap. |
| Q9 | RP-07 | Should the PR template's requirement line offer `RP-xx`? | No change. RP ids are written in the same line. |
| Q10 | RP-07 | `specs/traceability.md` line 3 says "Validated by `just trace`", but `scripts/trace.mjs` never reads that file. Correct the sentence, or extend `trace.mjs`? And add a `scripts/trace.test.mjs` for its failure paths? | Correct the sentence in a later `spec/` PR to "Requirement ↔ scenario tags are validated by `just trace`; rows are kept by `/handoff`". No new test now. |
| Q11 | RP-02 | Should `guard-bash` also block `gh api` calls to a pull request's merge endpoint (and `git -C … push … main`), with new `hooks.test.mjs` rows? | Yes, in a separate harness PR after this spec. It is listed here because this spec changes no hook. |
| Q12 | RP-02 | Should a `just` recipe read the branch protection of `main` (`gh api …/branches/main/protection`) instead of the owner looking at the settings page? | No. It needs administrator scope on the token. It stays owner check A2. |
| Q13 | RP-05 | Should `/handoff` require the issue number in each block, and should anything check that a block was written? | No change. The issue is cited when there is one, and the fresh-session check (A2) is the measure. |
| Q14 | RP-01, RP-03, RP-04, RP-06, RP-10 | These five are set to `specified` on the strength of `agent-harness.md`. That spec specifies harness parity (Claude to Codex), not each requirement's own criterion: for example, nothing checks that "the human does not edit code" (RP-01) or that "no secrets are read" beyond `Read(./.env*)` (RP-06). Is that enough for `specified`? | Yes, as is. A gap found later goes through G1 like any other. |

## 7. Changes made by this spec

- Adds this contract and `../features/agent-process.feature`.
- Changes `status: draft` to `status: specified` in `requirements/RP-01.md` to `RP-10.md`, keeping each statement and appending a pointer to its spec.
- Adds one row for RP-02, RP-05, RP-07, RP-08 and RP-09 to `traceability.md`.
- Changes no code, test, hook, script, recipe, workflow, skill, `CLAUDE.md` or approved spec. G1 approves this contract and the feature file. G2 remains human-only.

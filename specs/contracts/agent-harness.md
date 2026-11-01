# Local agent harness: one source of truth

Status: approved at G1 by the owner in chat on 2026-11-01 ("apruebo todo"). Requirements: RP-01, RP-03, RP-04, RP-06, RP-10. Decision: ADR-0048.

## 1. Owner requirement and scope

The owner requires a single maintained source of truth and Codex to follow the existing Claude workflow. Claude's configuration is canonical. This change adapts Codex to that configuration; it does not redesign the workflow, change Claude's model assignments, reduce checks or change human gates.

Sources are the root and scoped `CLAUDE.md` files, `.claude/agents/`, `.claude/skills/`, `.claude/hooks/` and the shared hook registrations in `.claude/settings.json`. Personal settings remain local and are never copied into tracked outputs.

## 2. Derived Codex files

One dependency-free Node generator, `scripts/sync-agents.mjs`, produces these versioned compatibility files:

| Canonical source | Codex output | Conversion |
|---|---|---|
| Each tracked `CLAUDE.md`, including scoped files | Sibling `AGENTS.md` | Same instructions, with a generated-file notice |
| `.claude/skills/<name>/` | `.agents/skills/<name>/` | Same skill and supporting files; no provider-name or stage-path substitutions |
| `.claude/agents/<name>.md` | `.codex/agents/<name>.toml` | Name and description from front matter; full instruction body preserved |
| `.claude/settings.json` hook registrations | `.codex/hooks.json` | Same events, matchers, order and timeouts; portable commands invoke canonical hook logic through the compatibility adapter where necessary |
| `.claude/hooks/*.mjs` | Direct canonical execution or thin entry points | No copied policy implementations or copied hook tests |

The `.claude/stage` file remains the only stage file. References such as `src/CLAUDE.md` remain valid in Codex instructions; no manual second ruleset is introduced. Generated TOML must escape arbitrary quotes, backslashes, Unicode and multiline instruction text correctly.

Claude-specific model aliases and tool names must not be emitted as unsupported Codex configuration. Keep Claude defaults unchanged. Record unsupported metadata as compatibility information, not as a claim that Codex has enforced it. Personal settings are not modified.

### Codex model assignments (owner-approved amendment, 2026-11-01)

The owner approved this assignment in chat ("me parece bien ese reparto"). Maintain it only in `.claude/codex-models.json`, with schema `{ "version": 1, "agents": { "<canonical role name>": { "model": "<OpenAI model id>", "model_reasoning_effort": "<effort>" } } }`. The generator emits the supported `model` and `model_reasoning_effort` fields in each derived agent TOML, alongside the unchanged canonical instruction body. Claude's `model` fields remain unchanged.

| Role | Codex model | Reasoning effort |
|---|---|---|
| `spec-writer` | `gpt-6-astra` | `high` |
| `test-author` | `gpt-6.1-sol` | `high` |
| `rust-implementer` | `gpt-6.1-sol` | `high` |
| `svelte-implementer` | `gpt-6.1-sol` | `medium` |
| `code-reviewer` | `gpt-6-astra` | `high` |

When this optional source file is present, every canonical role must have exactly one assignment and no unknown role is allowed. Require schema version 1, an OpenAI `gpt-` model id, and a supported reasoning-effort value (`low`, `medium`, `high`, `xhigh` or `max`); reject incomplete, malformed or unknown fields before changing any output. When the file is absent, retain the existing inherited-model behavior. Changing this source must cause read-only drift checking to fail until regeneration; no manual Codex edit is required. These settings request models from the host; account availability and actual live-host selection still need verification.

Generated files have a notice identifying their source and regeneration command, except formats without comments: their generation ownership is recorded by the generator. New and removed canonical roles, skills and scoped instruction files are reflected automatically. Remove obsolete outputs only when they are known generated files; never delete unowned files, local configuration or personal state. Reject symlink escapes and output paths outside the repository.

## 3. Synchronization and verification

- `just agents-sync`: generate the Codex compatibility files. Run twice with unchanged sources: the second run changes no file. Output contains relative paths and counts, never personal settings or corpus data.
- `just agents-check`: compare expected and existing generated outputs without writing. Exit 0 when synchronized; exit nonzero with the relative paths and `just agents-sync` when stale, missing or obsolete. Invalid canonical input fails clearly before output mutation.
- `just check-fast` includes `agents-check`, so `just check`, `just ci` and existing CI catch drift without separate owner maintenance.
- The canonical workflow tells agents to edit Claude sources and run `just agents-sync` before final checks. The owner never edits derived files. Fresh clones contain usable generated files; no symlink privilege, global configuration, package installation or bootstrap step is needed.

## 4. Hook compatibility and parity

Canonical Claude hook logic remains the sole policy implementation. A Codex adapter translates payload shape and forwards stdin, stdout, stderr, environment and exit status. Existing Claude entry points and behavior remain unchanged.

- Shell commands reach `guard-bash` with the canonical `tool_input.command`, including Codex's documented `Bash` payload and supported native `exec_command` payloads.
- An `apply_patch` payload is expanded into all affected paths for `guard-protected`. Include add, update, delete, and both source and destination of a move. Every path is checked before any patch executes. One rejected path rejects the entire call with the canonical reason and blocking exit status.
- Formatting receives every applicable surviving edited file after a patch, without formatting deleted paths. It continues to use the canonical formatter.
- Session context uses the same branch, status, stage and newest progress block. Stop/SubagentStop continue to execute the existing `just check-fast` logic with its existing skip conditions.
- Fresh-agent rules remain unchanged. Do not claim that Claude's JSONL context guard understands native Codex transcripts. Verify supported message payloads explicitly and document any unavailable transcript enforcement rather than silently asserting parity.
- Portable hook commands must work from the root and a subdirectory in supported Windows PowerShell and Git Bash sessions, without machine-specific paths.

Claude's `permissions` settings are not a Codex permissions format. Do not copy allow/ask/deny lists into arbitrary Codex keys, broaden sandbox permissions or translate them into automatic approvals. Existing host permissions still apply. Report provider-specific enforcement gaps explicitly; instruction parity alone is not evidence of enforcement parity.

## 5. Acceptance and tests after G1

Automated tests in `scripts/sync-agents.test.mjs` and `scripts/codex-hooks.test.mjs` must cover:

1. Existing sources generate the expected scoped instructions, five roles and five skills; source bodies retain their stage paths and local instruction links.
2. Changing one canonical instruction, role, skill or hook registration makes check mode fail without writes; synchronization updates only the expected derived files, then check mode passes.
3. Added and removed sources update the managed set; unrelated files, local settings and files outside the repository remain untouched. Invalid input or an escaping path fails before mutation.
4. Repeated generation is byte-stable, including Windows line endings, Unicode and instruction text containing TOML delimiters. Generated agent TOML parses successfully.
5. Claude and Codex-shaped calls produce the same allow/block result for protected specs/tests in `implement`, read-only review, allowed implementation edits, outside-repository paths, blocked shell commands and allowed read-only commands.
6. Multi-file patches cannot hide a protected path in a later operation or move destination; rejected calls run no formatter or mutation.
7. Stop skip conditions and session-context output remain equivalent. Missing optional transcript data is identified as unverified enforcement, not a successful context-limit test.
8. Source-to-output drift checking is part of `check-fast` and therefore existing local and hosted checks.
9. The optional canonical model source emits the approved per-role assignments, preserves Claude defaults and role instruction bodies, participates in drift checks and idempotence, and rejects incomplete/unknown roles, invalid model/effort pairs and malformed input without partial output. Tests may live in `scripts/codex-models.test.mjs`.

Manual verification: in fresh Claude and Codex sessions, confirm role and skill discovery, scoped instruction loading, and the execution of SessionStart, PreToolUse, PostToolUse and Stop hooks. Exercise a blocked edit against a disposable repository-contained fixture, not an approved spec. Record host versions and any host-level difference. Completion requires `just check` green and explicit reporting of parity limits; no live-host parity claim may be based solely on unit tests.

G1 approves this contract, `features/agent-harness.feature` and ADR-0048 before tests or implementation. G2 remains human-only.

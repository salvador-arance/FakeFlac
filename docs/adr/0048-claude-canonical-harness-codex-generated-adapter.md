# ADR-0048: Claude is the canonical harness; Codex uses a generated adapter

- Status: accepted (owner G1 approval in chat, 2026-11-01)
- Date: 2026-11-01

## Context and rationale

The owner requires a single maintained source of truth and Codex to follow the workflow already used by Claude. Current root instructions, hooks, agent definitions and skills have independent copies that can drift. Codex's copied skills already name `.Codex/stage`, while both hook implementations read `.claude/stage`.

ADR-0015 anticipated reuse through plain Markdown and `just`. This decision extends that reuse to local Codex sessions and clarifies the Claude-only wording of ADR-0023. CI continues to run deterministic checks and no agents. Requirements: RP-01, RP-03, RP-04, RP-06, RP-10.

## Decision

Keep `CLAUDE.md` and `.claude/` as the canonical instructions and harness. Generate Codex discovery/configuration files with a dependency-free Node tool, reuse canonical hook logic through a payload adapter, and reject generated drift in `just check-fast`. The contract is `specs/contracts/agent-harness.md`.

Preserve Claude behavior, model defaults, stage ownership, checks and human gates. Provider-specific models, permissions and transcript formats are compatibility boundaries, not reasons to fork policy or claim identical runtime enforcement without evidence.

Owner-approved amendment (2026-11-01): keep Codex model/effort assignments in `.claude/codex-models.json` and generate the supported TOML fields from that source. Astra/high handles specs and review; Sol/high handles tests and Rust; Sol/medium handles Svelte. Claude model defaults remain unchanged. The owner approved the concrete table in chat; details and validation are in the harness contract.

## Alternatives considered

- Maintain two copies: rejected because every rule change requires duplicate maintenance.
- Replace Claude's working harness with a new shared directory: rejected because the owner requires preserving the existing Claude setup.
- Symlinks or directory junctions: rejected as the only mechanism because Windows privileges and Git checkout behavior vary, and Markdown roles still need conversion to Codex TOML.
- Thin instruction pointers alone: insufficient for role/skill discovery and detecting stale hook registrations.
- Silently copy Claude model aliases and permissions: rejected because they are not valid portable Codex configuration.

## Consequences

Agents edit canonical Claude sources and regenerate derived files. The owner maintains one ruleset. Existing CI detects drift, fresh clones work without global settings, and canonical hooks have one policy implementation. A small tested format adapter remains necessary; host-specific enforcement limits must be recorded. No dependency is added. The owner approved the contract, scenarios and this ADR at G1 in chat on 2026-11-01.

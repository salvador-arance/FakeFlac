---
name: handoff
description: Updates docs/agents/progress.md at the end of a session so a fresh session can continue. Usage - /handoff
---
# /handoff

Add a new block at the TOP of `docs/agents/progress.md` (newest first), with:
- `## <date> - <short title>`
- **Done:** what was completed, with commit or PR references.
- **Pending:** what is left, concretely.
- **Blockers / questions for the human.**
- **Next step:** the single next action, so a fresh session can start with it.

Also update `specs/traceability.md` if requirements moved (including tests added outside a spec PR). Set every ADR merged at G1 since the last handoff from `proposed` to `accepted`, in the ADR file and in `docs/adr/README.md`. Record any repeated agent mistake in `docs/agents/lessons.md` (RP-10). Keep it short.
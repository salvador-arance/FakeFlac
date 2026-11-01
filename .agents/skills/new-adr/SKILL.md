---
name: new-adr
description: Creates an Architecture Decision Record with the next number. Usage - /new-adr <title>
---
# /new-adr <title>

1. Find the highest number in `docs/adr/` and use the next one (4 digits).
2. Copy `docs/adr/template.md` to `docs/adr/NNNN-kebab-title.md` and fill it: context, options considered, decision, consequences. List the rejected alternatives with the reason.
3. Link it from `docs/architecture/overview.md` if it changes the architecture.
4. The ADR goes in the same PR as the change, and the human approves it at gate G1. Status stays `proposed` until then.
<!-- Generated from .claude/skills/new-adr/SKILL.md; run just agents-sync. Do not edit. -->

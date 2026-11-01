<!-- Generated from src/CLAUDE.md; run just agents-sync. Do not edit. -->

# src (frontend)

Svelte 5 + TypeScript + Vite. Atomic design (ADR-014); full rules in `docs/architecture/frontend.md`.
- Levels: `tokens` < `atoms` < `molecules` < `organisms` < `templates` < `pages`. A level imports only from lower levels.
- Atoms, molecules, organisms and templates are presentational: data in via props, events out. They never import stores or `src/lib/api/`.
- Only `pages` connect to stores and to the API client.
- Colours and sizes come from `tokens/tokens.css`. The verdict is never conveyed by colour alone (icon + text).
- Svelte 5 runes only. No `export let`, no `$:`.
- Every atom/molecule has a UI spec in `specs/ui/` and a Storybook story before it is built (S6).

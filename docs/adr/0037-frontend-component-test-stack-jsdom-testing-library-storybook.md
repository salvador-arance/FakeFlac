# ADR-0037: Frontend component test stack: jsdom, Testing Library and Storybook 10

- Status: accepted
- Date: 2026-10-18

## Context and rationale

The main-screen UI (RF-01, RF-05, RF-06) is the first Svelte code beyond the API client. S6 asks for a Storybook story per state and a component test before a component is used in a page (ADR-0014, `specs/ui/components.md`). ADR-0033 approved `@storybook/svelte-vite` and `@testing-library/svelte` but not the DOM that the tests render into, nor the exact packages. R11 requires every dependency to be decided explicitly. The owner chose jsdom and `user-event` on 2026-10-18.

## Decision

Dev dependencies only. None of them reaches the installer.

| Package | Why |
|---|---|
| `jsdom` 30 | DOM for vitest (`test.environment = 'jsdom'`). It is the most complete DOM emulation, including focus and keyboard events, which the table's ↑/↓/Home/End navigation needs |
| `@testing-library/svelte` 5 | Render Svelte 5 components and query them by role and text. Its `svelteTesting()` Vite plugin adds browser export conditions and cleanup after each test |
| `@testing-library/user-event` 14 | Realistic clicks and keyboard input |
| `storybook` 10 + `@storybook/svelte-vite` 10 | Stories in CSF (`*.stories.ts`) next to each component. `.storybook/main.ts` turns telemetry off |

Scripts: `npm run storybook` (dev server) and `npm run build-storybook`. Stories are protected test files (`guard-protected`), like component tests.

## Alternatives considered

- **happy-dom.** It is faster, but focus handling and keyboard events are less complete.
- **Vitest browser mode with Playwright.** It runs in a real browser, but it downloads browsers and makes CI much slower.
- **Svelte CSF stories (`@storybook/addon-svelte-csf`).** This adds one more package. TypeScript CSF is enough for prop-driven presentational components.
- **No Storybook, tests only.** This would go against S6 and ADR-0014.

## Consequences

- `package-lock.json` grows by about 2,800 lines (Storybook's tree). All licences are permissive.
- `esbuild`'s postinstall script is blocked by npm's `allowScripts`. Storybook builds without it, because the platform binary comes as an optional dependency.
- In Git Bash on this machine, `npx vitest` fails ("failed to find the runner", drive-letter case). Run frontend tests through `just` or PowerShell, as before.

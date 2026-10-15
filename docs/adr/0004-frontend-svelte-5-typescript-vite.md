# ADR-0004: Frontend: Svelte 5 + TypeScript + Vite

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Less code per component, runes reactivity, no virtual DOM; atomic design is framework-agnostic. Installer size is set by Tauri, not the framework. Confirmed by the owner.

## Decision

Frontend: Svelte 5 + TypeScript + Vite.

## Alternatives considered

React + TanStack

## Consequences

The virtualised-table ecosystem is smaller than React's; solved with our own row virtualisation.

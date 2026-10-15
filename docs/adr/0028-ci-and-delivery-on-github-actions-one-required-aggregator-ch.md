# ADR-0028: CI and delivery on GitHub Actions, one required aggregator check and releases generated from tags

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Public GitHub repository: free runners, native gh integration and the Tauri action.

## Decision

CI and delivery on GitHub Actions, one required aggregator check and releases generated from tags.

## Alternatives considered

Another CI (GitLab, Azure); manual publishing

## Consequences

Platform dependency; recipes live in `just` so they can migrate.

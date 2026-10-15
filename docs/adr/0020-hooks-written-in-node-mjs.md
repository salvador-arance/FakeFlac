# ADR-0020: Hooks written in Node (.mjs)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Node is already required by the frontend and behaves the same on Windows and Linux CI.

## Decision

Hooks written in Node (.mjs).

## Alternatives considered

Bash (Git Bash on Windows); PowerShell

## Consequences

Node startup on every hook (tens of ms); hooks are kept light.

# ADR-0005: Progress through tauri::ipc::Channel, not global events

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Typed, ordered per invocation, no orphan listeners.

## Decision

Progress through tauri::ipc::Channel, not global events.

## Alternatives considered

Global emit/listen; polling

## Consequences

The channel lives as long as the analysis command.

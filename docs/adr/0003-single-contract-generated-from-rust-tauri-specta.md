# ADR-0003: Single contract generated from Rust (tauri-specta)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Satisfies S3: one source of truth, no drift.

## Decision

Single contract generated from Rust (tauri-specta).

## Alternatives considered

Hand-written TS types; a separate OpenAPI/JSON Schema

## Consequences

Depends on a community library; its version is pinned (D1 has a fallback).

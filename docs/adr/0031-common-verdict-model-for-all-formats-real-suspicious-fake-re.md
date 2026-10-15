# ADR-0031: Common verdict model for all formats: Real / Suspicious / Fake relative to the declared quality, plus estimated_source_bitrate_kbps for lossy formats

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Table and CSV are the same for every format; the user reads one verdict.

## Decision

Common verdict model for all formats: Real / Suspicious / Fake relative to the declared quality, plus estimated_source_bitrate_kbps for lossy formats.

## Alternatives considered

A different model per format

## Consequences

Per-format and per-encoder rules live in DetectionParams.

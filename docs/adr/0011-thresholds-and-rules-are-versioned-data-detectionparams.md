# ADR-0011: Thresholds and rules are versioned data (DetectionParams)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Satisfies S4; allows calibration in phase 5 without touching the architecture; changing algorithm_version invalidates the cache.

## Decision

Thresholds and rules are versioned data (DetectionParams).

## Alternatives considered

Constants in code

## Consequences

The version must be kept coherent.

# ADR-0026: Real FFT with realfft (on top of rustfft)

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Half the compute and memory for real signals (RNF-01); small dependency by the same author.

## Decision

Real FFT with realfft (on top of rustfft).

## Alternatives considered

Complex rustfft with a zero imaginary part

## Consequences

One more dependency (requires confirmation, R11).

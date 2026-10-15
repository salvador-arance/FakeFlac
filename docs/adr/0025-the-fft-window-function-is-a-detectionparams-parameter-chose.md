# ADR-0025: The FFT window function is a DetectionParams parameter, chosen by experiment. Initial hypothesis: 4-term Blackman-Harris with 75 % overlap; reference: Hann with 50 %

- Status: accepted
- Date: 2026-10-15

## Context and rationale

The analysis looks for a drop of tens of dB in a band with large dynamic range, so spectral leakage from loud low frequencies must be minimised.

## Decision

The FFT window function is a DetectionParams parameter, chosen by experiment. Initial hypothesis: 4-term Blackman-Harris with 75 % overlap; reference: Hann with 50 %.

## Alternatives considered

Hann by default without measuring; rectangular window; Hamming

## Consequences

A window benchmark is needed (phase 5) and changing the window invalidates the cache. See docs/architecture/backend.md.

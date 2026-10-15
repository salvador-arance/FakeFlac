# ADR-0010: Analysis by sampling representative windows (with seek); full decode only for the spectrogram

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Meets RNF-01 and R2.

## Decision

Analysis by sampling representative windows (with seek); full decode only for the spectrogram.

## Alternatives considered

Always decode the whole track

## Consequences

The average spectrum is an estimate; the number of windows is a calibratable parameter.

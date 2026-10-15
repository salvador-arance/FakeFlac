# ADR-0009: Spectrogram computed in Rust, sent as a u8 dB matrix; colour applied in the frontend canvas

- Status: accepted
- Date: 2026-10-15

## Context and rationale

u8 is 4x smaller; the colour palette can change without recomputing.

## Decision

Spectrogram computed in Rust, sent as a u8 dB matrix; colour applied in the frontend canvas.

## Alternatives considered

Send a PNG; send f32

## Consequences

Dynamic range is reduced (enough for visual inspection).

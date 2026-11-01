# ADR-0030: Formats in stages and Rust decoders: WAV and FLAC (H1); MP3, M4A (AAC) and Opus (H2). symphonia for WAV/FLAC/MP3/M4A; Opus with a separate decoder chosen in a spike (opus crate/libopus bundled vs a pure-Rust alternative)

- Status: accepted; amended by ADR-0047
- Date: 2026-10-15

## Context and rationale

symphonia does not decode Opus; ffmpeg would bloat the installer (RNF-06) and add an external tool.

## Decision

Formats in stages and Rust decoders: WAV and FLAC (H1); MP3, M4A (AAC) and Opus (H2). symphonia for WAV/FLAC/MP3/M4A; Opus with a separate decoder chosen in a spike (opus crate/libopus bundled vs a pure-Rust alternative).

## Alternatives considered

ffmpeg as a sidecar; a single decoder

## Consequences

A possible native dependency for Opus; needs confirmation (R11) and measuring installer size.

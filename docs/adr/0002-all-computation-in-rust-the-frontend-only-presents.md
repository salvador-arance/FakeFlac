# ADR-0002: All computation in Rust; the frontend only presents

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Performance (RNF-01), memory (RNF-02) and a single implementation of the algorithm.

## Decision

All computation in Rust; the frontend only presents.

## Alternatives considered

Processing audio/FFT in the WebView (WebAudio/WASM)

## Consequences

The frontend depends on the contract; heavy data crosses IPC.

# ADR-0012: CSV: RFC 4180, UTF-8 without BOM, comma separator

- Status: accepted
- Date: 2026-10-15

## Context and rationale

Most portable and simplest format; not optimised for any particular program. Keeps accents and symbols such as the double prime (RNF-09). Decided by the owner.

## Decision

CSV: RFC 4180, UTF-8 without BOM, comma separator.

## Alternatives considered

UTF-8 with BOM and semicolon (tuned for Excel in Spanish)

## Consequences

Excel may mis-display accents on double click; accepted (import choosing UTF-8).

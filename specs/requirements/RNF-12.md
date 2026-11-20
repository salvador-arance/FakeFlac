---
id: RNF-12
title: Damaged streams
priority: medium
milestone: H3
status: draft
---

**Damaged streams.** A file with a few damaged packets that otherwise decodes is analysed, and the damage is reported; it is not turned into an error. Today any packet that fails to decode makes the whole track `corruptStream` (RNF-04; `MonoStream::for_each_chunk` in `fakeflac-audio`).

Measurable acceptance (to be fixed at G1 of its spec):
- **Tolerance.** A packet whose decode fails with a recoverable decoder error is skipped, up to a bound held in `DetectionParams` (a count or a fraction of the packets). Beyond the bound, or on an I/O or container error, the track stays `corruptStream` (RNF-04 unchanged).
- **Reporting.** The number of skipped packets is shown in the detail panel and the CSV, so a damaged file never looks clean.
- **Determinism.** The same file gives the same result everywhere (`EXTRACTION_VERSION` bump).

Source: owner's decision of 2026-11-20 (H3 extension, #184), from a 320 kbps MP3 in the owner's library that plays to the end (ffmpeg logs 4 damaged frames) and that FakeFlac reports as `corruptStream`. A comparable tool (tool A) is not the model: it hides decode damage (its managed FLAC reader ignores frame CRCs and skips failed frames without counting them). Issue #198.

Spec: pending. Open questions in #198: whether the tolerance applies to FLAC (frame CRC errors), AAC and Opus too, and whether a tolerated file keeps its normal verdict with a note or gets `Suspicious` with a new reason.

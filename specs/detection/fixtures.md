---
id: SPEC-FIXTURES
requirements: [RF-03, RF-04, RF-14]
status: proposed
---

# Synthetic test signals (ADR-0022)

> All automated tests use generated signals with a known truth. No music is ever committed. Signals are generated in code (a test-support module) with a fixed seed so they are reproducible; `just fixtures` can also write them as WAV/FLAC files under `fixtures/` (git-ignored) for manual inspection.

| Id | Signal | Truth |
|---|---|---|
| `full_band` | White noise, 44.1 kHz, 16-bit, 30 s, −12 dBFS RMS (see levels below), no low-pass | Real |
| `lp16k` | Same noise through a brick-wall low-pass at 16 kHz (≥ 100 dB/oct) | Fake, cutoff ≈ 16 kHz |
| `lp19k` | Same, low-pass at 19 kHz | Fake, cutoff ≈ 19 kHz |
| `lp20k` | Same, low-pass at 20 kHz | Fake, cutoff ≈ 20 kHz |
| `lp16k_96k` | `lp16k` generated at 96 kHz / 24-bit | Fake, cutoff ≈ 16 kHz |
| `gradual` | Noise through a gentle low-pass (60 dB/oct from 8 kHz: −60 dB at 16 kHz, ≈ 5 dB/kHz there) | Suspicious, `GradualRolloff` |
| `low_hf` | Pink noise low-passed at 6 kHz (little treble) | Suspicious, `LowHighFrequencyContent` |
| `bass_tone_lp16k` | `lp16k` plus a 60 Hz sine at −3 dBFS | Fake, cutoff ≈ 16 kHz (leakage must not hide the cliff) |
| `silence` | Digital silence, 30 s | Suspicious, `LowHighFrequencyContent` |
| `short` | `full_band`, 2 s | Error, `TooShort` |
| `corrupt` | A FLAC file with its bytes truncated after the header | Error, `CorruptStream` |

**Levels.** Every signal is set to −12 dBFS RMS and then scaled down, if needed, so that its peak does not exceed −1 dBFS: clipping would create content above the low-pass and invalidate the fixture. Relative levels inside a fixture (noise vs. tone) are preserved. **Length.** Signal lengths are rounded down to a whole number of 4096-frame FLAC blocks, so encoders write equal minimum and maximum block sizes.

Tolerance for cutoff assertions: ± `2 · smoothing_hz` (± 200 Hz with v1 parameters).

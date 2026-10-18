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
| `lp16k_unknown_length` | `lp16k` saved as FLAC with `total_samples` set to 0 in STREAMINFO (length not declared, as streaming encoders write it); the audio frames are unchanged | Fake, cutoff ≈ 16 kHz (the length is counted by decoding, rules.md section 1.2) |

## Real-world cases (added with algorithm_version 2)

These fixtures reproduce what a FLAC made from a decoded MP3 really looks like: noise above the low-pass instead of digital silence, and intermittent gaps above 16 kHz. **v1 returns `Real` for `lp16k_dither`, `lp20k_floor` and `sfb21_gaps`**, so they capture its failure. *Pink noise* means noise with amplitude `1/√f` (as in `low_hf`). All are 44.1 kHz, 16-bit, 30 s.

| Id | Signal | Truth |
|---|---|---|
| `lp16k_dither` | Pink noise through a brick-wall low-pass at 16 kHz, set to **−40 dBFS RMS** (not −12) and quantised to 16 bits **with TPDF dither** (sum of two independent uniform values in `[−½, ½)` LSB added before rounding). At this level the dither floor above 16 kHz is only ~50 dB below the signal | Fake, cutoff ≈ 16 kHz |
| `lp20k_floor` | Pink noise with gain `1/√f` below 20 kHz and a constant (white) gain from 20 kHz up to Nyquist, 15 dB below the pink gain at 20 kHz. Models the codec-noise floor above the low-pass of a ~320 kbps MP3 | Fake, cutoff ≈ 20 kHz |
| `sfb21_gaps` | Pink noise, full band. It is split exactly in the frequency domain at 16 kHz. The part from 16 kHz to Nyquist is multiplied by a gate: gain 1 during the last 0.5 s of every 2 s and −20 dB the rest of the time (75 % of the time), with 5 ms raised-cosine ramps. Models LAME's empty sfb21 band. The mean spectrum drops only ~6 dB above 16 kHz, so there is no cliff. The median spectrum drops ~19 dB | v2: Fake (from `G`). **v3: Suspicious, `HighBandGaps`** (the gated content reaches the upper window, so `FullBand`; a LAME transcode with its default low-pass has nothing in the upper window, see `sfb21_gaps_lp20k5`; a V0 transcode without a low-pass looks like this fixture and is a known limitation, rules.md section 6). Cutoff = Nyquist in both |
| `sfb21_gaps_weak` | `sfb21_gaps` with −6 dB instead of −20 dB in the gate (`G` ≈ 5 dB) | Suspicious, `HighBandGaps` |
| `hf_bursts` | Pink noise, full band, with the same gate (−20 dB, 75 %) applied from **8 kHz** to Nyquist instead of from 16 kHz: treble that comes and goes without an edge at 16 kHz, like hi-hats. Negative control for `G` | Real (v3: `FullBand` is true, but neither V3 nor V4 would fire anyway) |

The gate is the same in all three gated fixtures, so it is the edge frequency (16 vs 8 kHz) that separates a transcode from intermittent treble.

## Full-band guard (added with algorithm_version 3)

Pairs that differ only in what lies in the upper window (20.7–21.0 kHz, rules.md section 2): a stationary floor or nothing (the signature of a lossy low-pass) against intermittent content (music). "The gate" is the gate of `sfb21_gaps` (gain 1 during the last 0.5 s of every 2 s, −20 dB the rest, 5 ms raised-cosine ramps). All are 44.1 kHz, 16-bit, 30 s, plain rounding. FP1–FP4 are the confirmed false positives of rules.md ("Names used here").

**Exact low-pass at `F`** (used by the first two rows, not the ≥ 100 dB/oct brick wall of `lp16k`): take the whole signal **after gating**, compute its DFT over the full length, set to zero every bin with frequency above `F`, and take the inverse DFT; then apply the levels and the rounding below. Two reasons: a 100 dB/oct filter at 20.5 kHz attenuates only about 1.75 dB at 20.75 kHz (`100 · log2(20750 / 20500)`), the first band of the window; and gating after filtering would spread the 5 ms ramps' spectrum about ±200 Hz above `F`, into the window. Done this way, the only thing above `F` is the 16-bit rounding error.

| Id | Signal | Truth (v3) | Models |
|---|---|---|---|
| `sfb21_gaps_lp20k5` | `sfb21_gaps` (gated), then the exact low-pass at **F = 20 500 Hz** | Fake (from `G`); cutoff ≈ 20.5 kHz. Also Fake with v2 | A 320 kbps LAME transcode: sfb21 gaps and nothing but the rounding floor above 20.5 kHz (`U` far below −70 dB, `B` < 0.5 dB) |
| `sfb21_gaps_aa21k` | `sfb21_gaps` (gated), then the exact low-pass at **F = 21 000 Hz** | Suspicious, `HighBandGaps`; cutoff ≈ 21.0 kHz (v2: Fake) | FP1–FP3: a gap step at 16 kHz in a recording whose intermittent treble goes up to an anti-aliasing filter at ~21 kHz (every bin of the window is below `F`: `U` ≈ −12 dB, `B` ≈ 14 dB) |
| `shelf19k5` | Pink noise, full band, split exactly in the frequency domain at **19.5 kHz** (as `sfb21_gaps` is at 16 kHz); the part from 19.5 kHz to Nyquist is attenuated by **15 dB** (no gate) | Fake (from `D`); cutoff ≈ 19.5 kHz. Also Fake with v2 | A transcode whose stationary floor above a 19.5 kHz low-pass sits 15 dB down (`B` < 0.5 dB) |
| `shelf19k5_bursts` | `shelf19k5`, with the attenuated part (19.5 kHz to Nyquist) also multiplied by the gate before the two parts are added | Suspicious, `AmbiguousCutoff`; cutoff ≈ 19.5 kHz (v2: Fake) | FP4: a step in the lossy range with intermittent content above it up to Nyquist (`B` ≈ 14 dB) |

`G` ≈ 0 for both `shelf19k5` fixtures (no edge at 16 kHz). For `shelf19k5_bursts` the mean spectrum drops ≈ 21 dB at 19.5 kHz (15 dB of shelf plus ≈ 6 dB of gate), well above `step_fake_db`, so only the guard keeps it from `Fake`.

**Levels.** Every signal is set to −12 dBFS RMS (unless its row says otherwise) and then scaled down, if needed, so that its peak does not exceed −1 dBFS: clipping would create content above the low-pass and invalidate the fixture. Scaling is linear, so it adds nothing above an exact low-pass. Relative levels inside a fixture (noise vs. tone, signal vs. floor) are preserved. **Quantisation** is plain rounding unless the row asks for dither. **Length.** Signal lengths are rounded down to a whole number of 4096-frame FLAC blocks, so encoders write equal minimum and maximum block sizes.

Tolerance for cutoff assertions: ± `2 · band_hz` (± 200 Hz with v2 and v3 parameters).

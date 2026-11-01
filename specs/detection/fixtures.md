---
id: SPEC-FIXTURES
requirements: [RF-03, RF-04, RF-14]
status: approved
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
| `sfb21_gaps` | Pink noise, full band. It is split exactly in the frequency domain at 16 kHz. The part from 16 kHz to Nyquist is multiplied by a gate: gain 1 during the last 0.5 s of every 2 s and −20 dB the rest of the time (75 % of the time), with 5 ms raised-cosine ramps. Models LAME's empty sfb21 band. The mean spectrum drops only ~6 dB above 16 kHz, so there is no cliff. The median spectrum drops ~19 dB | v2: Fake (from `G`). v3: Suspicious, `HighBandGaps` (the gated content reaches the upper window, so `U` and `B` pass). **v4: Fake (from `G`)**: its ~6 dB mean drop at 16 kHz is its cutoff step `D` at `fs` ≈ 16 kHz, inside the lossy range and above `max_lossy_step_db`, so it is not `FullBand` (derivation and what the test stage confirms: section "Verdicts with v4" below). Cutoff = Nyquist in all three (`D` < `step_suspicious_db`) |
| `sfb21_gaps_weak` | `sfb21_gaps` with −6 dB instead of −20 dB in the gate (`G` ≈ 5 dB) | Suspicious, `HighBandGaps` (v2, v3 and v4) |
| `hf_bursts` | Pink noise, full band, with the same gate (−20 dB, 75 %) applied from **8 kHz** to Nyquist instead of from 16 kHz: treble that comes and goes without an edge at 16 kHz, like hi-hats. Negative control for `G` | Real (v3 and v4: `FullBand` is true, but neither V3 nor V4 would fire anyway) |

The gate is the same in all three gated fixtures, so it is the edge frequency (16 vs 8 kHz) that separates a transcode from intermittent treble.

## Full-band guard (added with algorithm_version 3)

Pairs that differ only in what lies in the upper window (20.7–21.0 kHz, rules.md section 2): a stationary floor or nothing (the signature of a lossy low-pass) against intermittent content (music). "The gate" is the gate of `sfb21_gaps` (gain 1 during the last 0.5 s of every 2 s, −20 dB the rest, 5 ms raised-cosine ramps). All are 44.1 kHz, 16-bit, 30 s, plain rounding. FP1–FP4 are the confirmed false positives of rules.md ("Names used here").

**Exact low-pass at `F`** (used by the first two rows, not the ≥ 100 dB/oct brick wall of `lp16k`): take the whole signal **after gating**, compute its DFT over the full length, set to zero every bin with frequency above `F`, and take the inverse DFT; then apply the levels and the rounding below. Two reasons: a 100 dB/oct filter at 20.5 kHz attenuates only about 1.75 dB at 20.75 kHz (`100 · log2(20750 / 20500)`), the first band of the window; and gating after filtering would spread the 5 ms ramps' spectrum about ±200 Hz above `F`, into the window. Done this way, the only thing above `F` is the 16-bit rounding error.

| Id | Signal | Truth (v3) | Truth (v4) | Models |
|---|---|---|---|---|
| `sfb21_gaps_lp20k5` | `sfb21_gaps` (gated), then the exact low-pass at **F = 20 500 Hz** | Fake (from `G`); cutoff ≈ 20.5 kHz. Also Fake with v2 | Fake, as v3 | A 320 kbps LAME transcode: sfb21 gaps and nothing but the rounding floor above 20.5 kHz (`U` far below −70 dB, `B` < 0.5 dB) |
| `sfb21_gaps_aa21k` | `sfb21_gaps` (gated), then the exact low-pass at **F = 21 000 Hz** | Suspicious, `HighBandGaps`; cutoff ≈ 21.0 kHz (v2: Fake) | Suspicious, `HighBandGaps`, as v3 (`FullBand`) | FP1–FP3: a gap step at 16 kHz in a recording whose intermittent treble goes up to an anti-aliasing filter at ~21 kHz, its deepest step above the lossy range (every bin of the window is below `F`: `U` ≈ −12 dB, `B` ≈ 14 dB) |
| `shelf19k5` | Pink noise, full band, split exactly in the frequency domain at **19.5 kHz** (as `sfb21_gaps` is at 16 kHz); the part from 19.5 kHz to Nyquist is attenuated by **15 dB** (no gate) | Fake (from `D`); cutoff ≈ 19.5 kHz. Also Fake with v2 | Fake, as v3 | A transcode whose stationary floor above a 19.5 kHz low-pass sits 15 dB down (`B` < 0.5 dB) |
| `shelf19k5_bursts` | `shelf19k5`, with the attenuated part (19.5 kHz to Nyquist) also multiplied by the gate before the two parts are added | Suspicious, `AmbiguousCutoff`; cutoff ≈ 19.5 kHz (v2: Fake) | **Fake** (from `D`); cutoff ≈ 19.5 kHz | FP4: a step in the lossy range with intermittent content above it up to Nyquist (`B` ≈ 14 dB) |

`G` ≈ 0 for both `shelf19k5` fixtures (no edge at 16 kHz). For `shelf19k5_bursts` the mean spectrum drops ≈ 21 dB at 19.5 kHz (15 dB of shelf plus ≈ 6 dB of gate), well above `step_fake_db`, so with v3 only the guard keeps it from `Fake`; with v4 that same step removes the guard.

## Verdicts with v4 (added with algorithm_version 4)

v4 adds one condition to `FullBand` (rules.md section 2): not `LossyStep`, that is, not (`InRange(fs)` and `D ≥ max_lossy_step_db` = 3 dB), where `D` at `fs` is the deepest cutoff step and the lossy range is [14500, 20300) Hz. It can only turn a v3 `FullBand` track into a non-`FullBand` one, so only the fixtures that v3 guards can change, and only from `Suspicious` back to `Fake` (rules.md section 3). The v3-guarded fixtures are `sfb21_gaps`, `sfb21_gaps_aa21k` and `shelf19k5_bursts` (and `hf_bursts`, which no rule makes `Fake`). Each verdict below is derived from the fixture's definition, and the test stage confirms the value named. **Measured once before G1** (2026-10-19, FLAC fixtures, default parameters, the v3 implementation on `feat/RF-04`, same extraction as v4): `sfb21_gaps` `D` = 6.15 dB at `fs` = 16050 Hz; `sfb21_gaps_aa21k` `D` = 70.9 dB at 21050 Hz; `shelf19k5_bursts` `D` = 20.8 dB at 19550 Hz; `hf_bursts` `D` = 0.48 dB at 20950 Hz (outside the lossy range). Every derivation below holds.

| Id | `D` at `fs` (derivation) | `LossyStep` | `FullBand` v4 | Verdict v4 | Confirmed by the test stage |
|---|---|---|---|---|---|
| `sfb21_gaps` | The gate leaves the mean power above 16 kHz at about `p + (1 − p) · 0.01` of the ungated level, `p` the share of analysed frames in the loud phase (0.25 by design, depending on where the 64 segments fall in the 2 s cycle): a drop of −10·log10(0.2575) ≈ 5.9 dB for `p` = 0.25, between 5.1 and 6.8 dB for `p` from 0.30 to 0.20. Pink noise changes by ~0.1 dB across the step windows and the `slope` term removes about that much, so `D` ≈ 5–7 dB at `fs` ≈ 16 kHz (15950 or 16050 Hz). No band above rebounds 6 dB, so it is a candidate. The v2/v3 tests already require `D` < 7 (cutoff reported at Nyquist) | true | false | **Fake**, no reasons (V3: `G` ≈ 19 dB); cutoff Nyquist | `D` ≥ 3 dB and `fs` in [14500, 20300). If the test stage measures otherwise, it stops and the spec returns to G1 |
| `sfb21_gaps_aa21k` | The exact low-pass at 21 kHz removes everything above it: the drop at 21 kHz is the deepest step, `fs` ≈ 20950 or 21050 Hz, above the lossy range (the 16 kHz drop of ~6 dB is smaller, so it is not `D`) | false | true (as v3) | Suspicious, `HighBandGaps` (V5); cutoff ≈ 21.0 kHz | none beyond the v3 assertions |
| `sfb21_gaps_lp20k5` | The cliff at 20.5 kHz is the deepest step, `fs` ≈ 20450 or 20550 Hz, at or above `fake_max_cutoff_hz` | false | false (`U`, `B` fail, as v3) | Fake (V3), as v3 | none |
| `shelf19k5` | ≈ 15 dB at `fs` ≈ 19.5 kHz | true | false (`B` fails, as v3) | Fake (V4), as v3 | none |
| `shelf19k5_bursts` | ≈ 21 dB at `fs` ≈ 19.5 kHz (fixture definition above) | true | false | **Fake**, no reasons (V4); cutoff ≈ 19.5 kHz | that the track is not `FullBand` with v4 although it is with v3 |
| `sfb21_gaps_weak` | ≈ 3.6 dB at ≈ 16 kHz (−10·log10(0.25 + 0.75 · 0.251)), close to `max_lossy_step_db`, but it does not matter: `B` ≈ 2.4 dB already fails, and `G` ≈ 5 dB is below `gap_fake_db` | either | false (as v3) | Suspicious, `HighBandGaps` (V5), as v3 | none |
| `hf_bursts` | The gate's edge is at 8 kHz, below `step_search_low_hz` (11 kHz), so the search range sees only smooth pink noise: `D` is the largest random fluctuation, expected below 1 dB | false (expected) | true (expected) | Real, as v3 | `FullBand` true with v4 (the sample corpus's watch `W1` relies on it, calibration.feature) |
| every other fixture | not `FullBand` with v3 (stationary floor, no upper window or `B` < 3) | — | false | as v3 | none |

The sample-corpus setting `fxv0` writes `sfb21_gaps`, so it follows that row: `Fake` with v4, as with v2 (calibration/sample-corpus.md section 4). Real V0 transcodes differ from this fixture: their sfb21 gating is milder in the mean, most have no lossy-range step of 3 dB, and the guard keeps them `Suspicious` (rules.md section 6). The fixture keeps its v2 purpose (a gap with content up to Nyquist must not be `Real`).

## Edge step (added with algorithm_version 6)

Signals for `EdgeStep` (rules.md section 2: `InRange(fs)`, `D_Q < 3`, `R ≥ 15`, `D < 16` with v6 values). All are 44.1 kHz, 16-bit, 30 s, plain rounding, built from pink noise as in `low_hf`. "The gate" is the gate of `sfb21_gaps` with gain 0 instead of −20 dB in its quiet phase (gain 1 during the last 0.5 s of every 2 s, 0 the rest, 5 ms raised-cosine ramps). **"The short gate"** is the same with gain 1 during the last **0.25 s** of every 2 s only. "Exact band-limit to `[a, b)`" sets to zero every DFT bin (over the full length) outside `[a, b)`, as the exact low-pass of the full-band section does. Each band-limit is applied **after** its gate, so the ramps spread nothing outside the band. Levels below are band levels (100 Hz bands) of the mean spectrum before the final scaling of "Levels", which keeps them relative to one another.

| Id | Signal | Truth (v5) | Truth (v6) | Models |
|---|---|---|---|---|
| `edge_bump19k3` | **Roll-off:** pink noise, full band, multiplied in the frequency domain by a gain of 0 dB below 17 kHz and `−12 · (f − 17 kHz)` dB per kHz above it, plus stationary white noise whose band level is **60 dB** below the roll-off's band level at 17 kHz. **Bump:** independent pink noise multiplied by **the short gate**, then exactly band-limited to `[19100, 19400)` Hz, scaled so that its band level while the gate is open is **15 dB** above the roll-off's band level at 19.25 kHz. The two are added | `Fake` (V4), cutoff ≈ 19.45 kHz | **`Suspicious`, `AmbiguousCutoff`** (V6, `EdgeStep`); cutoff ≈ 19.45 kHz | W4: a short narrow-band sound just below a natural roll-off |
| `edge_bump19k3_lp19k5` | `edge_bump19k3`, then the exact low-pass at **19 500 Hz** (full-band section) | `Fake` (V4), cutoff ≈ 19.5 kHz | `Fake`, as v5 (`EdgeStep` false: `D` far above the cap, `R` ≈ 0) | A lossy low-pass on a track with a content edge |
| `holes_lp19k5` | **Base:** pink noise, split exactly in the frequency domain into `[0, 18500)` Hz (kept as is) and `[18500, 19500)` Hz, which is multiplied by the gate and then exactly band-limited to `[18500, 19500)` Hz; nothing above 19.5 kHz. **Floor:** stationary white noise added over the full band, with a band level **19 dB** below the base's (ungated) band level at 19.25 kHz | `Fake` (V4), cutoff ≈ 19.5 kHz | `Fake`, as v5 (`EdgeStep` false **only** because of `R`) | A transcode whose quantiser leaves the top kilohertz below its 19.5 kHz low-pass empty in most frames: the median spectrum shows no step (`D_Q` ≈ 0), the step is moderate, and the floor starts right above the low-pass (`R` ≈ 0). A rule that left out `R` would clear it |

**Derivations** (with `w` = 3, so `below` and `above` are the three bands each side of the band at `fs`; the median of the log of a noise band's power has the same offset in every band, so it cancels in `D_Q`):

- `edge_bump19k3`. Relative to the roll-off's level at 17 kHz: roll-off bands at 18.85–19.05 kHz −22.2 to −24.6 dB, at 19.15–19.35 kHz −25.8 to −28.2 dB, at 19.55–19.75 kHz −30.6 to −33.0 dB. The open bump is at −12 dB; open 12.5 % of the time, its mean is −21 dB, so `M` ≈ −20 dB in the three bump bands. At `fs` = 19450 Hz: `below` ≈ −20.0, `above` ≈ −31.8, `slope` = max(0, −23.4 + 20.0) = 0, so **`D` ≈ 12 dB**. Elsewhere the linear roll-off gives `d` ≈ 1.2 dB (4.8 dB drop minus 3.6 dB slope) and the knee at 17 kHz ≈ 3 dB, and no band above 19.75 kHz rebounds, so 19450 Hz is `fs`. `Q` follows the roll-off, because the bump is closed in most frames: with the 93 ms frames, about 0.25 s + one frame of every 2 s, some 17 % of the frames, see it open. That share still raises the median of the bump bands a little, and the rise counts twice in `D_Q` (it raises `below_Q` and lowers the slope term from 3.6 dB), so **`D_Q` ≈ 2 dB** rather than the 1.2 dB of a perfectly smooth `Q`. The floor window's median is the band at 21.45 kHz, roll-off −53.4 dB plus the floor at −60 dB ≈ −52.5 dB, so **`R` ≈ 20.7 dB**. `G` ≈ 0 (no edge at 16 kHz); `B` < 0.5 dB (stationary in the upper window), so not `FullBand`. With v6: `EdgeStep`, then V5 does not match (`G` < 4) and V6 does.
- `edge_bump19k3_lp19k5`. Above 19.5 kHz only the 16-bit rounding is left, about 55 dB below the bump bands, so `D` ≈ 55 dB (≥ 16), `Q` steps as much as `M` (`D_Q` ≫ 3) and `R` ≈ 0.
- `holes_lp19k5`. With `P` the base's band level at 19.25 kHz and the floor at `P − 19` dB: in the gated bands `M` ≈ 10·log10(0.25 · 10^1.9 + 1) ≈ 13.2 dB above the floor, and right above 19.5 kHz `M` is the floor. At `fs` ≈ 19450 or 19550 Hz: **`D` ≈ 13 dB** (the `slope` windows are gated too, so `slope` ≈ 0); the drop of about 6 dB at 18.5 kHz, where the gate starts, is smaller. In 75 % of the frames the gated bands hold only the floor, so `Q` is the floor on both sides: **`D_Q` ≈ 0**. The floor window holds the floor too: **`R` ≈ 0**. `G` ≈ 0, `B` < 0.5 dB, `LossyStep` true. So `InRange(fs)`, `D_Q < 3` and `D < 16` hold and `EdgeStep` is false only because `R < 15`: V4 gives `Fake`.

**Measured once before G1** (2026-10-24, a numpy synthesis of these definitions with the spectra of rules.md section 1, 6 noise seeds; not the test stage's generator): `edge_bump19k3` `fs` = 19450 Hz, `D` 11.8–12.2 dB, `D_Q` 1.83–2.44 dB, `R` 20.6–20.9 dB, `B` ≤ 0.41 dB; `edge_bump19k3_lp19k5` `fs` = 19550 Hz, `D` 54.5–56.0 dB, `D_Q` ≈ 47 dB, `R` ≈ 0; `holes_lp19k5` `fs` 19450 or 19550 Hz, `D` 12.9–13.3 dB, `D_Q` 0.5–1.2 dB, `R` −0.1 to 0.3 dB. A first version of `edge_bump19k3` (the 0.5 s gate at +12 dB) gave `D_Q` 2.97–3.48 dB and failed `D_Q < 3` in 5 of 6 seeds; the short gate at +15 dB keeps `M` the same and lowers `Q`, and it is closer to W4's "few frames".

**Confirmed by the test stage** before relying on them: `edge_bump19k3` has `fs` in [19350, 19550] Hz, 10 ≤ `D` < 16, `D_Q` < 3 and `R` ≥ 15; `holes_lp19k5` has `fs` in the lossy range, 10 ≤ `D` < 16, `D_Q` < 3 and `R` < 15. If the test stage measures otherwise, it stops and the spec returns to G1. `shelf19k5` (`Fake`) and every other fixture must keep their v5 verdicts with `EdgeStep` false: their steps either lie outside the lossy range, are below `step_fake_db`, or show in `Q` as much as in `M` (stationary content, `D_Q` ≈ `D`).

**Levels.** Every signal is set to −12 dBFS RMS (unless its row says otherwise) and then scaled down, if needed, so that its peak does not exceed −1 dBFS: clipping would create content above the low-pass and invalidate the fixture. Scaling is linear, so it adds nothing above an exact low-pass. Relative levels inside a fixture (noise vs. tone, signal vs. floor) are preserved. **Quantisation** is plain rounding unless the row asks for dither. **Length.** Signal lengths are rounded down to a whole number of 4096-frame FLAC blocks, so encoders write equal minimum and maximum block sizes.

Tolerance for cutoff assertions: ± `2 · band_hz` (± 200 Hz with v2 to v6 parameters).

---
id: SPEC-DETECTION-LOSSLESS-V6-PROPOSAL
requirements: [RF-03, RF-04]
status: pending (proposal for G1, not approved)
algorithm_version: 6 (proposed; the next free number, rules.md section 4)
params: specs/detection/detection-params.v6.toml (to be written after G1)
amends: specs/detection/rules.md (v4 lossless rules, used unchanged by v5)
adr: docs/adr/0042-detection-v6-edge-step-on-the-step-rule.md (proposed)
---

# Proposal: detection v6, an "edge step" exception to the step rule V4

> PENDING. This file lives in `specs/pending/`, which neither `just trace` nor the test stage reads, so no test is derived from it yet. If G1 approves the direction, a spec PR moves the rules into `specs/detection/rules.md`, adds `detection-params.v6.toml`, the fixtures and the scenarios, and the values marked *indicative* are decided by `just accept` on the real corpus (section 7).

**In one paragraph.** V4 makes a lossless track `Fake` when its deepest cutoff step `D` lies in the lossy range and reaches `step_fake_db`. The step is measured on the **mean** spectrum only. A short, loud, narrow-band sound just below a natural roll-off makes such a step without any low-pass: the mean spectrum drops, the per-frame **median** spectrum does not, and above the step the spectrum keeps falling for many dB before it reaches the floor. A real low-pass leaves the spectrum at the floor right above the step. v6 adds two measurements of the existing spectra, the median step `D_Q` and the post-step residual `R`, and does not apply V4 when both say "content edge, not low-pass" (`EdgeStep`). Such a track falls through to V6 and is `Suspicious` (`AmbiguousCutoff`). Nothing else changes: no new extraction, no contract change, and against v5 the only possible verdict change is `Fake → Suspicious`.

**Names used here.** `W4` (FP4) is the confirmed false positive of rules.md (the 4th `false-positive` line of `confirmed.txt`). `N1` and `N2` are two other `primary` tracks **of the same purchased album as W4**, not listed in `confirmed.txt`; N1 is very likely the `primary` track "0.12 dB below the `Fake` side's threshold" of rules.md section 6, and N2 the `primary` track "with a lossy-range step and `G ≥ 4`" of the same section. No path or name of the corpus appears here (ADR-0022).

## 1. Evidence

### 1.1 What the three tracks measure (v5 defaults)

Measured with the core's own `measure()` on the adapter's band spectra (64 × 4 frames, N = 4096, 100 Hz bands, 44.1 kHz). `D_Q` and `R` are the measurements proposed in section 2.

| id | v5 verdict (rule) | `D` (dB) | `fs` (Hz) | `G` (dB) | `U` (dB) | `B` (dB) | `D_Q` (dB) | `R` (dB) |
|---|---|---|---|---|---|---|---|---|
| W4 | `Fake` (V4) | 14.22 | 19450 | −0.25 | −47.06 | 2.86 | 0.32 | 17.48 |
| N1 | `Suspicious` `AmbiguousCutoff` (V6) | 9.88 | 20150 | −2.51 | −32.64 | 17.29 | 1.02 | 22.35 |
| N2 | `Suspicious` `HighBandGaps` (V5) | 7.39 | 16550 | 5.65 | −51.13 | 2.37 | −0.24 | 26.72 |

W4's `D`, `fs`, `G`, `U` and `B` equal the figures rules.md gives for FP4, so this is the same track measured the same way.

**What makes W4's step.** Around the step (`M` and `Q` relative to `L_ref`, dB):

| band centre (Hz) | 18950 | 19050 | **19150** | **19250** | **19350** | 19450 | 19550 | 19650 | 19750 | 20450 | 21450 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `M` (mean) | −25.8 | −26.8 | **−18.6** | **−19.3** | **−20.0** | −31.8 | −32.6 | −33.9 | −34.0 | −41.4 | −51.2 |
| `Q` (median) | −34.5 | −35.6 | −36.0 | −37.1 | −37.4 | −38.1 | −38.6 | −39.5 | −40.2 | −46.4 | −51.3 |

The three bands below `fs` stand 8–12 dB above their neighbours in `M` while `Q` is smooth: a sound that is present in a few frames only (`M − Q` ≈ 17 dB there). `d(i)` compares those three bands with the three above and finds 14.22 dB, but `Q` falls only 0.32 dB across the same bands, and above the step `M` keeps falling for another ~17 dB before it reaches the floor at ~21 kHz. N1 is the same pattern at 19.6–20.1 kHz (`M` ≈ −12 dB, `Q` ≈ −42 dB). N2 is a different case: its verdict comes from the gap rule (`G` = 5.65 dB, `Q` falls about 8 dB across 16 kHz), not from the step (section 6, Q4).

**Cross-check outside the detector.** The same album bought as 320 kbps MP3 from the same store was compared on six tracks (W4, N1, N2 and three `Real` ones; full-track Welch spectra, 1 kHz and 0.5 kHz bands). It matches the lossless files within 0.3 dB up to 19.5 kHz and lies 30–38 dB below them in the 20–20.5 kHz band: the encoder's low-pass at ~20.1 kHz. If the lossless files came from a lossy source with a 19.5 kHz (W4) or 16.5 kHz (N2) low-pass, the store's MP3 would show that cut too. It does not.

### 1.2 Transcodes of the same album

The 13 lossless tracks of the album were transcoded with seven settings of `calibration.v2.toml` (cbr128, cbr192, cbr256, cbr320, v2, aac128, aac256; same ffmpeg arguments, decoded back to 16-bit FLAC with `decode_args`), giving 91 transcodes. v0 and opus128 were not run. With v5: 78 of 78 `Fake` on the six settings with a low-pass. aac256 (ffmpeg's native encoder, no low-pass) gives 2 `Fake` and 1 `Suspicious`: they are W4's, N1's and N2's own transcodes, which keep the original's pattern, and 10 `Real`.

The proposal can only lose transcodes whose `Fake` rests on V4 alone (`G < gap_fake_db`): 50 of the 91.

| setting | V4-only transcodes | `D_Q` min / median (dB) | `R` median / max (dB) |
|---|---|---|---|
| cbr192 | 8 | −10.36 / −2.83 | 8.22 / 12.05 |
| cbr256 | 12 | −5.24 / 12.28 | 4.74 / 6.86 |
| cbr320 | 13 | −4.73 / 15.51 | 3.86 / 9.94 |
| v2 | 2 | −8.81 / 28.38 | 2.42 / 2.42 |
| aac128 | 13 | 29.29 / 42.63 | 3.71 / 14.24 |
| aac256 | 2 | 0.52 / 1.39 | 20.58 / 20.58 |

(cbr128's transcodes all have `G` ≥ 30 dB, so V3 catches them whatever V4 does.)

**`D_Q` alone does not work.** The first idea, "a low-pass must also show in the median spectrum", fails: many genuine transcodes have `D_Q` ≤ 0. Below an MP3 or AAC low-pass the quantiser already leaves holes, so the per-frame median is low on both sides of the cutoff (cbr192: median `D_Q` −2.83 dB).

**`R` alone does not work either.** Two V4-only transcodes have `R` ≥ 12 dB (cbr192 12.05, aac128 14.24); both have a large `D_Q` (23.61 and 40.12 dB). Only the conjunction separates W4 and N1 from every transcode with a low-pass.

### 1.3 Simulation of the proposed rule

Offline re-implementation of V1–V10 (validated against the core: 0 mismatches on the 104 rows), then `EdgeStep` with `max_median_step_db` = `x` and `min_residual_db` = `y`:

| variant | `x`, `y` (dB) | W4 | N1 | N2 | other 10 album tracks | transcodes `Fake → ¬Fake` (six low-pass settings) | aac256 |
|---|---|---|---|---|---|---|---|
| v5 | — | `Fake` | `Susp.` | `Susp.` | `Real` | — | 2 `Fake`, 1 `Susp.` |
| **A** (V4 only) | **3, 12** | **`Susp.`** | `Susp.` | `Susp.` | `Real` | **0 of 78** | the 2 `Fake` → `Susp.` |
| A | 2–5, 12–15 | `Susp.` | `Susp.` | `Susp.` | `Real` | 0 of 78 | same |
| A | 2–5, 10 | `Susp.` | `Susp.` | `Susp.` | `Real` | 1 (cbr192, `D_Q` −6.11, `R` 10.03) → `Susp.` | same |
| C (V4 and V6) | 3, 12 | `Real` | `Real` | `Susp.` | `Real` | 0 of 78 | 3 caught → `Real` |

Margins of variant A at (3, 12): W4 2.68 dB on `D_Q` and 5.48 dB on `R`. The nearest V4-only transcode with `D_Q` < 3 is a cbr192 track at `R` = 10.03 dB, 1.97 dB below `y`. **One album is not evidence enough for these values**; they are a starting point for the corpus run (section 7), which has about 1,400 detectable transcodes per setting.

## 2. Measurements (added to rules.md section 2)

Both are defined only when `fs` is defined; `i` is the band whose centre is `fs` and `w = step_window_bands`, as in `d(i)`.

| Measurement | Definition |
|---|---|
| **Median step `D_Q`** (v6) | `d(i)` of section 2 computed on the median spectrum `Q` instead of `M`, at the same band `i`: `below_Q − above_Q − max(0, mean(Q[i−2w … i−w−1]) − below_Q)`. `L_ref` cancels in the differences. No candidate search: it is evaluated only at the `fs` chosen on `M` |
| **Top floor `F_top`** (v6) | median of `M` over the bands with centre in `[edge.floor_low_hz, edge.floor_high_hz)` and at or below `top`. Undefined when that holds no band (e.g. Nyquist 16 kHz). With the indicative values at 44.1 kHz: the 11 bands centred 20950 … 21950 Hz |
| **Post-step residual `R`** (v6) | `above(i) − F_top`, where `above(i) = mean(M[i+1 … i+w])` is the `above` of `d(i)`. Undefined when `F_top` is |
| **`EdgeStep`** (v6) | the params have an `[edge]` section, `fs` is defined, `InRange(fs)`, `D_Q` and `R` are defined, `D_Q < edge.max_median_step_db` and `R ≥ edge.min_residual_db`. Otherwise `false`; always `false` without `[edge]` |

The floor window is **absolute**, like the upper window and `gap_edge_hz`. At 88.2/96 kHz a window near Nyquist would sit far above a lossy residue (an upsampled transcode would get a large `R` and be cleared); at 21–22 kHz it sits in that residue. For a genuine hi-res file with music at 21–22 kHz, `F_top` is higher and `R` smaller, so `EdgeStep` is less likely: the error goes the conservative way.

`D_Q`, `R` and `EdgeStep` are not part of `TrackAnalysis` (no contract change), but the calibration tool reports them, as it does `U`, `B` and `FullBand` (a change to `specs/calibration/evaluation.md` sections 4, 5 and 8 in the same spec PR).

## 3. Verdict (rules.md section 3)

One condition is added to V4; every other row is unchanged:

| # | Condition | Verdict | Reasons |
|---|---|---|---|
| V4 | `D ≥ step_fake_db` and `InRange(fs)` and not `FullBand` **and not `EdgeStep`** | `Fake` | — |

Consequences:

- An `EdgeStep` track that V4 would have made `Fake` falls through. V5 matches if `G ≥ gap_suspicious_db`; otherwise V6 does (`D ≥ step_fake_db ≥ step_suspicious_db` and `InRange(fs)`): `Suspicious` with `AmbiguousCutoff`. It can never become `Real`.
- **`EdgeStep` does not touch `LossyStep` or `FullBand`.** `LossyStep` still uses `D`, so an `EdgeStep` track still has no full-band guard and V3 still catches its gaps. This is why variant A loses no transcode with a large `G` (cbr128, v2), and why variant B, which ignored the step everywhere, was rejected: it re-enabled the guard on a v2 transcode with `G` = 29.42 dB.
- Against v5, for the same spectra, the only possible change is `Fake → Suspicious` on V4 tracks. The caught rate (`Fake` + `Suspicious`) of every setting is therefore unchanged by construction.
- The reported cutoff (`fs` and its steepness) does not change.
- Lossy files (lossy-source.md, rule L3) are not affected: see Q5.

## 4. Parameters (a new optional section, `detection-params.v6.toml`)

```toml
algorithm_version = 6
# ... every v5 section and value unchanged ...

[edge]                        # proposal, lossless files only (rule V4)
floor_low_hz = 20900.0        # F_top: bands with centre in [floor_low_hz, floor_high_hz) at or below top
floor_high_hz = 22000.0
max_median_step_db = 3.0      # INDICATIVE: EdgeStep needs D_Q below this
min_residual_db = 12.0        # INDICATIVE: ... and R at or above this
```

- **v2–v5 stay loadable.** `[edge]` is optional; without it `EdgeStep` is always `false` and every file gives exactly its own verdicts, reasons and cutoffs, so the calibration tool compares v5 and v6 in one run.
- **Shape:** inside `[edge]` all four fields are required; unknown fields are rejected.
- **Values** (`DetectionParams::validate`): `edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz` (20600 Hz with v5 values: the floor window never overlaps the `above` window of an in-range step), `edge.floor_low_hz < edge.floor_high_hz`, and `edge.min_residual_db > 0`. NaN fails every check. Names follow the existing style (`edge.floor_low_hz < edge.floor_high_hz does not hold (…)`).
- **No new extraction.** `D_Q`, `F_top` and `R` come from the existing `M` and `Q`: `[input]` and `[fft]` are unchanged, `EXTRACTION_VERSION` stays, cached spectra are reused. The app's result cache is invalidated by the new `algorithm_version` (ADR-0008).
- The next free `algorithm_version` after this file becomes **7**.

## 5. Test signals and scenarios (for the spec PR after G1)

New synthetic fixtures (fixtures.md), at 44.1 kHz, 16-bit:

- `edge_bump19k3`: full-band noise with a gradual roll-off (about −12 dB/kHz from 17 kHz, reaching the dither floor near 21 kHz), plus a 300 Hz band of noise centred at 19.25 kHz, gated on in 25 % of frames at +12 dB. Expected v5: `Fake` (V4); v6: `Suspicious` `AmbiguousCutoff`, `EdgeStep` true. The test stage confirms `D ≥ 10`, `D_Q < 3`, `R ≥ 12` before relying on it.
- `edge_bump19k3_lp19k5`: the same signal through a 19.5 kHz brick-wall low-pass and back to 16 bits. Expected v5 and v6: `Fake`, `EdgeStep` false (`R` near 0).
- The existing `shelf19k5` (`Fake`) must stay `Fake` with `EdgeStep` false.

Scenarios (`detection-full-band.feature` or a new `detection-edge-step.feature`, `@RF-04`): the two fixtures; the W4, N1 and N2 measurements as rule-level rows (W4 → `Suspicious` `AmbiguousCutoff`, N1 and N2 unchanged); the boundaries `D_Q = max_median_step_db` (not `EdgeStep`) and `R = min_residual_db` (`EdgeStep`); `fs` outside the lossy range (never `EdgeStep`); an empty floor window (32 kHz); a v5 file reproducing v5 verdicts on every fixture; and the parameter checks of section 4.

## 6. Open questions for G1

| # | Question | Default |
|---|---|---|
| Q1 | **V4 only (variant A) or also V6 (variant C)?** C would make W4 and N1 `Real`. On this album it costs no low-pass transcode, but it breaks "only `Fake → Suspicious`" (A4-style criteria) and lets a misread low-pass become `Real` | A: V4 only. W4 becomes `Suspicious`, N1 stays `Suspicious` |
| Q2 | A new reason (for example `contentEdgeStep`) instead of `AmbiguousCutoff` for a track that `EdgeStep` moved? It is a contract change (bindings, UI text) | No new reason |
| Q3 | Thresholds: are `x` = 3 dB and `y` = 12 dB acceptable as indicative starting values, with the corpus run deciding (section 7)? The transcode-side margin on `R` is 1.97 dB on one album | Yes; if E6 fails, the figures go back to G1, the implementer does not tune them |
| Q4 | **N2 is out of scope.** Its verdict comes from the gap rule V5 (`G` = 5.65 dB, between `gap_suspicious_db` 4 and `gap_fake_db` 6), not from the step. A `Suspicious` there is what the table means by weak evidence. Accept it? | Accept. Revisit with the per-frame statistics of rules.md section 6 |
| Q5 | Apply `EdgeStep` to the lossy rule L3 (`fake_min_step_db`) too? An MP3 made from a lossless track like W4 has its own low-pass above 19.45 kHz, so the case is different | Not in v6 |
| Q6 | Absolute floor window `[20900, 22000)` Hz (section 2) rather than one relative to Nyquist? | Absolute |
| Q7 | Should the calibration tool's watch list gain N1 and N2 (as `false-positive` lines of `confirmed.txt`, which renumbers nothing because they go after W4)? It is a local corpus file, not a repo change | Yes, the owner adds them |

## 7. Acceptance criteria (v6)

Measured locally with `just accept --params specs/detection/detection-params.v5.toml --params specs/detection/detection-params.v6.toml` (v5 is the baseline), split `all` unless stated. Comparisons are against the v5 column of the same run. Aggregates go into the implementation PR (ADR-0022).

| # | Criterion | Pass |
|---|---|---|
| E1 | W4 | Not `Fake` (the gate); its `D`, `fs`, `D_Q`, `R` and `EdgeStep` are reported. W1–W3 keep their v5 verdicts. N1 and N2 are reported if listed (Q7) |
| E2 | Confirmed fakes | 15 of 15 `Fake`, as with v5 |
| E3 | Negatives | `clean`: 0 `Fake`. `primary`: 0 `Fake` (expected v5: 1, W4, as in the v4 criteria), so T1 passes. `reference`: reported |
| E4 | Only `Fake → Suspicious` | Every off-diagonal count of the v5 × v6 table other than `Fake → Suspicious` is 0 (holds by construction; a failure is an implementation bug) |
| E5 | Caught rate | Equal to v5 per setting (by construction) |
| E6 | **`Fake`-rate drop per protected setting** (cbr128, cbr192, cbr256, cbr320, v2, aac128, opus128; detectable transcodes) | **Gate:** at most 0.5 percentage points per setting, and 0 of the 15 confirmed fakes `EdgeStep`. Report: the same for v0 and aac256, and per setting the distribution (median, 5th percentile) of `R` and `D_Q` over the V4-only transcodes |
| E7 | Synthetic | The scenarios of section 5 and every existing `@RF-04` scenario pass (`just test`) |

If E1, E2, E3 or E6 fails, the parameters are not tuned in the implement stage: the figures go back to G1. 0.5 pp is a proposal: v4's A8 allowed `FullBand` on up to 5 % of transcodes. A loss here is a `Fake` that becomes `Suspicious` (still caught), but the target is for `Fake` on a transcode to stay the rule.

## 8. Known limitations of the proposal

- **Small evidence base:** one album (one false positive and two weak positives) and 91 transcodes of it. The rule is physically motivated (a low-pass leaves the floor right above it; a content edge does not), but its margins come from one album.
- A lossy source whose low-pass happens to sit just above a W4-like bursty band could get `EdgeStep`; it would be `Suspicious`, not `Real`.
- N2 and N1 stay `Suspicious` (Q1, Q4).
- The 64 × 4-frame sampling (ADR-0010) decides how often a short sound is caught: W4's bump comes from a few frames. A different sampling could move `D` and `R`; the rule does not depend on the bump being caught, only on what it does to `M` when it is.

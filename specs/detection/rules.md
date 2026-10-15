---
id: SPEC-DETECTION-LOSSLESS
requirements: [RF-03, RF-04, RF-14]
status: proposed
algorithm_version: 1
params: specs/detection/detection-params.v1.toml
---

# Detection rules v1: lossless files (WAV, FLAC)

> For a WAV or FLAC file: compute the average power spectrum, find where real content ends (the cutoff), measure how sharply it ends, and decide **real / suspicious / fake**. Every number below is a named parameter in `detection-params.v1.toml`; the code never hard-codes them (S4, ADR-0011). Values are provisional and will be calibrated in phase 5 without changing these rules.

Lossy formats (MP3, M4A, Opus, RF-15) get their own rules in H2. Upsample detection (RF-12) is H3.

## 1. Average spectrum (RF-03)

1. Decode to `f32`, mix channels to mono (mean).
2. If the duration is shorter than `min_duration_s`, the track is `Error(TooShort)`.
3. Pick `segments` analysis positions evenly spread over the track, skipping the first and last `edge_skip_percent` of the duration. At each position take a block long enough for the FFT and its overlap.
4. FFT size `N` = power of two nearest to `sample_rate / target_bin_hz` (8192 at 44.1/48 kHz).
5. Apply the window `window` (periodic) with overlap `overlap`. Real FFT.
6. Accumulate **power** `|X|²`, normalised by `Σw²`. Average across all frames. Convert to dB at the end: `S(f)`.
7. Smooth `S(f)` with a running median of width `smoothing_hz`: `Ŝ(f)`.

Silent frames (RMS below `silence_dbfs`) are skipped. If no frame is left, the track is `Suspicious` with reason `LowHighFrequencyContent`: a silent track proves nothing either way.

## 2. Measurements

All in dB relative to `L_ref` = median of `Ŝ` over `[ref_band_low_hz, ref_band_high_hz]`.

| Measurement | Definition |
|---|---|
| Noise floor `F` (reported, not used to decide) | the `noise_floor_percentile` percentile of `Ŝ` over `[ref_band_low_hz, nyquist − nyquist_guard_hz]` |
| High-frequency content `H` | median of `Ŝ` over `[hf_band_low_hz, hf_band_high_hz]` |
| Cutoff `fc` | scanning down from `nyquist − nyquist_guard_hz`, the first frequency where `Ŝ(f) > −content_drop_db` holds continuously for at least `content_min_width_hz`. If it already holds at the start of the scan, `fc = nyquist` (full band) |
| Steepness `k` (dB/kHz) | `(Ŝ(fc − steepness_span_hz) − Ŝ(fc + steepness_span_hz)) / (2 · steepness_span_hz / 1000)` |
| Cliff | `k ≥ cliff_min_steepness_db_per_khz` **and** `fc < nyquist − full_band_margin_hz` |

`CutoffEstimate { frequency_hz: fc, steepness_db_per_khz: k, noise_floor_db: F }` is always reported, whatever the verdict.

## 3. Verdict (RF-04)

Evaluated in this order; the first match wins.

| # | Condition | Verdict | Reasons |
|---|---|---|---|
| V1 | Decoding failed or too short | `Error` | — (the `TrackError` explains it) |
| V2 | `H < low_hf_threshold_db` (little treble: a cappella, soft instrumental) | `Suspicious` | `LowHighFrequencyContent` (never `Fake` only because of the cutoff, 2.5 of the planning doc) |
| V3 | Cliff and `fc ≤ fake_max_cutoff_hz` | `Fake` | — |
| V4 | Cliff and `fc > fake_max_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V5 | No cliff and `fc < gradual_min_cutoff_hz` | `Suspicious` | `GradualRolloff` |
| V6 | Otherwise | `Real` | — |

Higher sample rates (88.2/96/192 kHz) use the same absolute thresholds: a lossy origin still shows a cliff at or below ~20.5 kHz.

## 4. Invariants

- Deterministic: the same file and parameters always give the same result.
- Read-only: the file is opened for reading only (RNF-05).
- `algorithm_version` in every `TrackAnalysis` equals the `algorithm_version` of the parameters used.
- The code's default parameters equal `detection-params.v1.toml` (a test checks it).

## 5. Test signals

Tests use synthetic signals only (`specs/detection/fixtures.md`). Acceptance against the real corpus (criteria 2.7) runs locally with `just accept`.

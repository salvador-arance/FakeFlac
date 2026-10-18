---
id: SPEC-DETECTION-LOSSLESS
requirements: [RF-03, RF-04, RF-14]
status: proposed
algorithm_version: 2
params: specs/detection/detection-params.v2.toml
supersedes: algorithm_version 1 (detection-params.v1.toml, kept as a record)
---

# Detection rules v2: lossless files (WAV, FLAC)

> For a WAV or FLAC file: compute two band spectra (the power **mean** and the per-frame **median**), measure the deepest relative **step** in the treble and the **gap step** at 16 kHz, and decide **real / suspicious / fake**. Every number below is a named parameter in `detection-params.v2.toml`; the code never hard-codes them (S4, ADR-0011). Design rationale and evidence: ADR-0035.

Lossy formats (MP3, M4A, Opus, RF-15) get their own rules in H2. Upsample detection (RF-12) is H3.

## Why v1 was replaced

v1 placed the cutoff where the smoothed mean spectrum fell more than `content_drop_db = 60` dB below the reference level. A FLAC made from a decoded MP3 still has noise above the MP3 low-pass: codec noise, plus dither or requantisation noise from writing 16 bits. In real files this floor sits only 40–55 dB below `L_ref`, so v1 counted it as content, put the cutoff at Nyquist and returned `Real`. On a local corpus with known truth (16 tracks: 7 transcodes, 9 lossless), the v1 code returned `Real` for all 16. v2 does not use an absolute level to find the cutoff. It looks for a **relative step** against the level just below it, and it adds a measurement that does not depend on a clean cutoff: the intermittent empty scale-factor band above 16 kHz (sfb21) that LAME leaves, which shows up in the **median** spectrum.

## 1. Spectra (RF-03)

1. Decode to `f32` and mix channels to mono (mean).
2. If the duration is shorter than `min_duration_s`, the track is `Error(TooShort)`.
3. Pick `segments` analysis positions spread evenly over the track, skipping the first and last `edge_skip_percent` of the duration. At each position take a block of `frames_per_segment` consecutive FFT frames with hop `N · (1 − overlap)` (block length `N + (frames_per_segment − 1) · hop`).
4. FFT size `N` = power of two nearest to `sample_rate / target_bin_hz` (4096 at 44.1/48 kHz, 8192 at 88.2/96 kHz).
5. Apply the window `window` (periodic). Real FFT. Power per FFT bin `|X|² / Σw²`.
6. Skip frames whose RMS is below `silence_dbfs`. If no frame is left, the track is `Suspicious` with reason `LowHighFrequencyContent`: a silent track proves nothing either way.
7. **Bands.** Group the FFT bins of each frame into bands of width `band_hz`: band `b` holds the bins with frequency in `[b · band_hz, (b + 1) · band_hz)`. Bins at or above the lower edge of the last full band (the one that ends at or below Nyquist) go into that last band. Band power = mean power of its bins. The band centre is `c(b) = (b + ½) · band_hz`.
8. **Mean spectrum** `M(b)` = `10·log10` of the band power averaged over all kept frames.
9. **Median spectrum** `Q(b)` = median over all kept frames of `10·log10` of the band power of each frame.

v1's running-median smoothing (`smoothing_hz`) is removed: band grouping replaces it.

## 2. Measurements

Levels are relative to `L_ref` = median of `M(b)` over the bands whose centre lies in `[ref_band_low_hz, ref_band_high_hz]`. Below, `M` means `M − L_ref`. `top` is the last band whose centre is at or below `nyquist − nyquist_guard_hz`.

| Measurement | Definition |
|---|---|
| Noise floor `F` (reported, not used to decide) | the `noise_floor_percentile` percentile of `M` over the bands with centre in `[ref_band_low_hz, nyquist − nyquist_guard_hz]` |
| High-frequency content `H` | median of `M` over the bands with centre in `[hf_band_low_hz, hf_band_high_hz]` |
| Content extent `fe` (only used for `GradualRolloff` and as the reported cutoff when there is no step) | scanning down from `top`, the lower edge of the first band of the first run of bands with `M > −content_drop_db` at least `content_min_width_hz` wide. If the run starts at `top`, `fe = nyquist` |
| Step `d(i)` at band `i` (mean spectrum) | with `w = step_window_bands`: `below = mean(M[i−w … i−1])`, `above = mean(M[i+1 … i+w])`, `slope = max(0, mean(M[i−2w … i−w−1]) − below)`; `d(i) = below − above − slope`. Band `i` itself is the transition band and is left out of both windows. `slope` discounts a roll-off that was already under way |
| Rebound | band `i` is **not** a candidate if any band from `i+w+1` up to `top` has `M > above + step_rebound_db`: content comes back higher up, so the drop is not where the content ends |
| Cutoff step `D` at `fs` | over the candidate bands `i` with `c(i) ∈ [step_search_low_hz, nyquist − step_search_top_margin_hz]`, `D` = the largest `d(i)` and `fs = c(i)` for that band (the lowest such band on ties). If there is no candidate or the largest `d(i)` is ≤ 0: `D = 0`, `fs` undefined |
| Gap step `G` at `gap_edge_hz` (median spectrum) | with `e = gap_edge_hz`, `g = gap_band_hz` and `q(lo, hi)` = median of `Q` over the bands with centre in `[lo, hi)`: `G = (q(e−g, e) − q(e, e+g)) − max(0, q(e−2g, e−g) − q(e−g, e))`. Taking the median across bands stops a single tone from counting as a step. If `nyquist < e + g`, `G = 0` |

`gap_edge_hz` is an absolute frequency, not relative to Nyquist: MP3 encodes at 44.1 and 48 kHz both start sfb21 at about 16 kHz, and a hi-res file made from an MP3 inherits that edge.

**Reported cutoff.** `CutoffEstimate { frequency_hz, steepness_db_per_khz, noise_floor_db: F }` is always reported, whatever the verdict:
- if `D ≥ step_suspicious_db`: `frequency_hz = fs`, `steepness_db_per_khz = D / ((step_window_bands + 1) · band_hz / 1000)` (the distance between the centres of the two windows);
- otherwise: `frequency_hz = fe`, `steepness_db_per_khz = 0`.

## 3. Verdict (RF-04)

`InRange(f)` means `lossy_min_cutoff_hz ≤ f < fake_max_cutoff_hz`. The rules are evaluated in order; the first match wins.

| # | Condition | Verdict | Reasons |
|---|---|---|---|
| V1 | Decoding failed or too short | `Error` | — (the `TrackError` explains it) |
| V2 | `H < low_hf_threshold_db` (little treble: a cappella, soft instrumental) | `Suspicious` | `LowHighFrequencyContent` (never `Fake` only because of the spectrum shape, 2.5 of the planning doc) |
| V3 | `G ≥ gap_fake_db` | `Fake` | — |
| V4 | `D ≥ step_fake_db` and `InRange(fs)` | `Fake` | — |
| V5 | `G ≥ gap_suspicious_db` | `Suspicious` | `HighBandGaps` |
| V6 | `D ≥ step_suspicious_db` and `InRange(fs)` | `Suspicious` | `AmbiguousCutoff` |
| V7 | `D ≥ step_low_suspicious_db` and `fs < lossy_min_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V8 | `D ≥ step_high_suspicious_db` and `fake_max_cutoff_hz ≤ fs < ambiguous_max_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V9 | `D < step_suspicious_db` and `fe < gradual_min_cutoff_hz` | `Suspicious` | `GradualRolloff` |
| V10 | Otherwise | `Real` | — |

Higher sample rates (88.2/96/192 kHz) use the same absolute frequencies: a lossy origin still shows its step at or below ~20.3 kHz and its gaps at 16 kHz.

## 4. Invariants

- Deterministic: the same file and parameters always give the same result.
- Read-only: the file is opened for reading only (RNF-05).
- `algorithm_version` in every `TrackAnalysis` equals the `algorithm_version` of the parameters used (2).
- The code's default parameters equal `detection-params.v2.toml` (a test checks it). `detection-params.v1.toml` stays in the repository as the record of version 1. The code does not have to load it.

## 5. Test signals

Tests use synthetic signals only (`specs/detection/fixtures.md`). The fixtures `lp16k_dither`, `lp20k_floor` and `sfb21_gaps` reproduce the real failure of v1 (v1 returns `Real` for all three). Acceptance against the real corpus (criteria 2.7) runs locally with `just accept`.

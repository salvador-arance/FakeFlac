---
id: SPEC-DETECTION-LOSSLESS
requirements: [RF-03, RF-04, RF-14]
status: proposed
algorithm_version: 3
params: specs/detection/detection-params.v3.toml
supersedes: algorithm_version 2 (detection-params.v2.toml, kept and still loadable, section 4); algorithm_version 1 (detection-params.v1.toml, kept as a record)
---

# Detection rules v3: lossless files (WAV, FLAC)

> For a WAV or FLAC file: compute two band spectra (the power **mean** and the per-frame **median**), measure the deepest relative **step** in the treble, the **gap step** at 16 kHz and whether there is **music above the lossy range** (the full-band guard), and decide **real / suspicious / fake**. Every number below is a named parameter in `detection-params.v3.toml`; the code never hard-codes them (S4, ADR-0011). Design rationale and evidence: ADR-0035 (v2) and ADR-0038 (v3).

Lossy formats (MP3, M4A, Opus, RF-15) get their own rules in H2. Upsample detection (RF-12) is H3.

**Names used here.** `FP1`–`FP4` are the four purchased tracks confirmed as false positives of v2. The 2026-10-18 report lists them as watch ids `W1`–`W4` (the tool numbers `false-positive` lines of `confirmed.txt` in line order, inputs.md section 2). The sample corpus of the calibration tool has its own, unrelated watch id `W1` (a synthetic track). **"Content up to `fe`"** always means the content extent of section 2: `fe` is the lower edge of the highest band with content, so that band reaches `fe + band_hz`.

## Open questions for G1 (v3)

| # | Question | Status |
|---|---|---|
| Q1 | The v3 values (`low_hz` 20.7 kHz, `min_level_db` −70, `min_burst_db` 3) are reasoned from the v2 figures and the fixtures, not measured. Pre-measure `U` and `B` before approving, or let acceptance decide? | **Resolved (owner, G1): approve without pre-measuring; A1 and A5 decide.** The acceptance run must report `U` and `B` for FP1–FP4 and, per setting, the distribution of `B` and the `FullBand` count of the transcodes (A8), with particular attention to `cbr320`: its low-pass (20.5 kHz) is only 200 Hz below the window, so LAME's transition band could leak bursty content into it |
| Q2 | Should A5 allow a tolerance on lost detections, in particular for V0, whose transcodes are full band? | **Resolved (owner, G1): A5 gates the caught rate** (`Fake` + `Suspicious`) per setting against the v2 caught rate of the same run. The drop of the `Fake` rate (v2 `Fake` → v3 `Suspicious`) per setting is reported as information, not a gate |
| Q3 | Should a guarded track get a new reason (for example `fullBandGaps`) instead of `highBandGaps` / `ambiguousCutoff`? It is a contract change (bindings, UI text). | Open. Default: no new reason |
| Q4 | Changes to approved scenarios and calibration behaviour: (a) `sfb21_gaps` (gated content up to Nyquist) goes from `Fake` to `Suspicious` with v3, and so does the sample-corpus setting `fxv0` that uses it; (b) the "Deterministic result" scenario of `detection-lossless.feature` now uses `shelf19k5_bursts` instead of `lp19k` (it exercises the guard); (c) the calibration tool's defaults (`--params`, grid v1 baseline) stay on v2 until v3 is accepted; (d) **version numbers:** a `/calibrate` proposal, and the sample corpus's `candidate.toml`, used to take `algorithm_version` = baseline + 1, which from a v2 baseline is 3 and collides with this v3 (and with the app's cache key, ADR-0008). From now on they take the next free version: 1 + the highest `algorithm_version` among `specs/detection/detection-params.v*.toml` and the baseline (4 today; calibration/evaluation.md section 7); (e) the protected tests listed in ADR-0038 ("Consequences") change in the test stage. Agree? | Open. Default: yes to all |
| Q5 | Is `lowpass_hz = 22050` for `v0` (no low-pass with ffmpeg's `-q:a 0`) confirmed? The V0 limitation in section 6 relies on it. It may be answerable from data: the LAME tag that the tool parses since `0bef5ad` (first MPEG frame's Info header) also carries LAME's low-pass value (one byte, in units of 100 Hz). Today the tool records only the encoder string (generation.md section 2), so reading that byte needs a small spec addition, or a manual check of one generated V0 file during acceptance | Open. Default: taken as confirmed until the tag says otherwise |
| Q6 | The caught rates of `v0` and `aac256` with v2 are not in the recorded figures. The acceptance run reports them (A5); add them to section 6 then? | Open. Default: yes, from the acceptance run |
| Q7 | FP4 is cleared only if its content above 20.7 kHz comes and goes (`B ≥ 3` dB). If it is stationary (for example tape hiss above an analogue low-pass), v3 leaves FP4 `Fake`. Accept that, or look for another discriminator in a later version? | Open. Default: accept; A1 reports it |

## Why v1 was replaced

v1 placed the cutoff where the smoothed mean spectrum fell more than `content_drop_db = 60` dB below the reference level. A FLAC made from a decoded MP3 still has noise above the MP3 low-pass: codec noise, plus dither or requantisation noise from writing 16 bits. In real files this floor sits only 40–55 dB below `L_ref`, so v1 counted it as content, put the cutoff at Nyquist and returned `Real`. On a local corpus with known truth (16 tracks: 7 transcodes, 9 lossless), the v1 code returned `Real` for all 16. v2 does not use an absolute level to find the cutoff. It looks for a **relative step** against the level just below it, and it adds a measurement that does not depend on a clean cutoff: the intermittent empty scale-factor band above 16 kHz (sfb21) that LAME leaves, which shows up in the **median** spectrum.

## Why v2 was replaced

The first acceptance run of v2 with the Rust pipeline on the real corpus (RP-11, 2026-10-18) gave 0/546 `Fake` on `clean` tracks and 15/15 confirmed fakes `Fake`, but 4 of the 67 purchased tracks (`primary`) were `Fake`. The owner confirmed all 4 as false positives:

| id | watch id (2026-10-18) | `G` (dB) | `D` (dB) | `fs` (Hz) | `fe` (Hz) | v2 rule |
|---|---|---|---|---|---|---|
| FP1 | W1 | 18.39 | 8.93 | 20450 | 20900 | V3 |
| FP2 | W2 | 13.18 | 14.79 | 21450 | 21400 | V3 |
| FP3 | W3 | 10.26 | 14.70 | 21250 | 21100 | V3 |
| FP4 | W4 | −0.25 | 14.22 | 19450 | 22050 | V4 |

FP1–FP3 have a 16 kHz gap step as large as a 320 kbps transcode's, so no value of `gap_fake_db` separates them (calibration/evaluation.md section 7). FP4 has a cutoff step in the lossy range. What the four share is **content above the highest lossy low-pass** (the 320 kbps LAME low-pass, 20.5 kHz): `fe` = 20.9–21.4 kHz for FP1–FP3 and `fe` = Nyquist for FP4.

`fe` cannot be the guard: a decoded transcode's floor above the low-pass is usually less than 60 dB below `L_ref`, so most transcodes also have `fe` = Nyquist (the v1 failure). The fixtures `lp16k_dither` and `lp20k_floor` are exactly that case. v3 instead uses a property the floor does not have: **above a lossy low-pass there is no coded audio, only a floor (dither, rounding noise of the decoder output) whose level hardly changes from frame to frame, so its per-frame median `Q` is within a fraction of a dB of its mean `M`. Music up there comes and goes with transients, so `M` is several dB above `Q`.** v3 measures the level `U` and this burstiness `B` in a window just above 20.5 kHz. When both show content (`FullBand`), the gap rule V3 and the step rule V4 give `Suspicious` instead of `Fake`.

## 1. Spectra (RF-03)

Unchanged from v2.

1. Decode to `f32` and mix channels to mono (mean).
2. If the duration is shorter than `min_duration_s`, the track is `Error(TooShort)`. If the stream does not declare its length (a FLAC whose STREAMINFO has `total_samples = 0`, which streaming encoders write), decode it once to count its frames, then continue as usual. A missing length is never reported as silence.
3. Pick `segments` analysis positions spread evenly over the track, skipping the first and last `edge_skip_percent` of the duration. At each position take a block of `frames_per_segment` consecutive FFT frames with hop `N · (1 − overlap)` (block length `N + (frames_per_segment − 1) · hop`).
4. FFT size `N` = power of two nearest to `sample_rate / target_bin_hz` (4096 at 44.1/48 kHz, 8192 at 88.2/96 kHz).
5. Apply the window `window` (periodic). Real FFT. Power per FFT bin `|X|² / Σw²`.
6. Skip frames whose RMS is below `silence_dbfs`. If no frame is left, the track is `Suspicious` with reason `LowHighFrequencyContent` and no cutoff (`cutoff: null`): a silent track proves nothing either way, and there are no measurements to report.
7. **Bands.** Group the FFT bins of each frame into bands of width `band_hz`: band `b` holds the bins with frequency in `[b · band_hz, (b + 1) · band_hz)`. Bins at or above the lower edge of the last full band (the one that ends at or below Nyquist) go into that last band. Band power = mean power of its bins. The band centre is `c(b) = (b + ½) · band_hz`.
8. **Mean spectrum** `M(b)` = `10·log10` of the band power averaged over all kept frames.
9. **Median spectrum** `Q(b)` = median over all kept frames of `10·log10` of the band power of each frame.

v1's running-median smoothing (`smoothing_hz`) is removed: band grouping replaces it.

## 2. Measurements

Levels are relative to `L_ref` = median of `M(b)` over the bands whose centre lies in `[ref_band_low_hz, ref_band_high_hz]`. Below, `M` means `M − L_ref` unless a row says otherwise. `top` is the last band whose centre is at or below `nyquist − nyquist_guard_hz`.

| Measurement | Definition |
|---|---|
| Noise floor `F` (reported, not used to decide) | the `noise_floor_percentile` percentile of `M` over the bands with centre in `[ref_band_low_hz, nyquist − nyquist_guard_hz]` |
| High-frequency content `H` | median of `M` over the bands with centre in `[hf_band_low_hz, hf_band_high_hz]` |
| Content extent `fe` (only used for `GradualRolloff` and as the reported cutoff when there is no step) | scanning down from `top`, the lower edge of the first band of the first run of bands with `M > −content_drop_db` at least `content_min_width_hz` wide. If the run starts at `top`, `fe = nyquist` |
| Step `d(i)` at band `i` (mean spectrum) | with `w = step_window_bands`: `below = mean(M[i−w … i−1])`, `above = mean(M[i+1 … i+w])`, `slope = max(0, mean(M[i−2w … i−w−1]) − below)`; `d(i) = below − above − slope`. Band `i` itself is the transition band and is left out of both windows. `slope` discounts a roll-off that was already under way |
| Rebound | band `i` is **not** a candidate if any band from `i+w+1` up to `top` has `M > above + step_rebound_db`: content comes back higher up, so the drop is not where the content ends |
| Cutoff step `D` at `fs` | over the candidate bands `i` with `c(i) ∈ [step_search_low_hz, nyquist − step_search_top_margin_hz]`, `D` = the largest `d(i)` and `fs = c(i)` for that band (the lowest such band on ties). If there is no candidate or the largest `d(i)` is ≤ 0: `D = 0`, `fs` undefined |
| Gap step `G` at `gap_edge_hz` (median spectrum) | with `e = gap_edge_hz`, `g = gap_band_hz` and `q(lo, hi)` = median of `Q` over the bands with centre in `[lo, hi)`: `G = (q(e−g, e) − q(e, e+g)) − max(0, q(e−2g, e−g) − q(e−g, e))`. Taking the median across bands stops a single tone from counting as a step. If `nyquist < e + g`, `G = 0` |
| **Upper window** (v3) | the bands with centre in `[full_band.low_hz, full_band.high_hz)` and at or below `top`. With v3 values at 44.1 kHz: the 3 bands centred at 20750, 20850 and 20950 Hz. If the params have no `[full_band]` section, or the window holds no band (for example at 32 kHz, Nyquist 16 kHz), `U` and `B` are **undefined** |
| **Upper level `U`** (v3) | median of `M` (relative to `L_ref`, as everywhere in this table) over the upper window |
| **Upper burstiness `B`** (v3) | median over the upper window of `M(b) − Q(b)`, both in absolute dB (the difference does not depend on `L_ref`). A stationary floor (dither, rounding noise) gives under 0.5 dB with 100 Hz bands at N = 4096; intermittent content gives several dB (≈ 14 dB for the −20 dB, 75 % gate of fixtures.md; ≈ 2.4 dB for the −6 dB gate of `sfb21_gaps_weak`) |
| **`FullBand`** (v3) | the params have a `[full_band]` section, `U` and `B` are defined, `U ≥ full_band.min_level_db` and `B ≥ full_band.min_burst_db`. Otherwise `false`. It is evaluated by the verdict step from `U`, `B` and the params, so without a `[full_band]` section it is `false` whatever values of `U` and `B` it is given |

`gap_edge_hz` is an absolute frequency, not relative to Nyquist: MP3 encodes at 44.1 and 48 kHz both start sfb21 at about 16 kHz, and a hi-res file made from an MP3 inherits that edge. The upper window is absolute for the same reason: a lossy origin keeps its low-pass at or below 20.5 kHz whatever the sample rate.

**Medians** over an even number of values take the upper middle value (nearest-index percentile, rank `round(½ · (n − 1))` rounding halves up, as the v2 code already does): with 2 values, the larger; with 4, the third smallest. It matters for the upper window whenever a params file gives it an even number of bands: `full_band.low_hz = 20600` (4 bands) and `20800` (2 bands) in calibrate-grid.v2.toml.

**Headroom.** FP1's highest content band (centre 20950 Hz, `fe` = 20900 Hz) is the last band of the window, so `full_band.high_hz` cannot be raised for FP1 without adding bands where it has no content. Its `U` passes with v3 values because the run that defines `fe` is at least 200 Hz wide, so the bands centred at 20850 and 20950 Hz are above −60 dB (two of the three window bands). Its `B` is not known.

`U` alone would not do: like `fe`, it cannot tell a transcode's floor from music. Its threshold only rules out residues near digital silence (for example in a 24-bit transcode), where `B` would be meaningless. `B` alone would count those residues as content. The decision rests on `B`.

**Reported cutoff.** `CutoffEstimate { frequency_hz, steepness_db_per_khz, noise_floor_db: F }` is reported for every track that has spectra, whatever the verdict (it is `null` only for `Error` and for a silent track, section 1.6). Unchanged from v2:
- if `D ≥ step_suspicious_db`: `frequency_hz = fs`, `steepness_db_per_khz = D / ((step_window_bands + 1) · band_hz / 1000)` (the distance between the centres of the two windows);
- otherwise: `frequency_hz = fe`, `steepness_db_per_khz = 0`.

`U` and `B` are new optional fields of the core's `Measurements` (undefined = absent). They and `FullBand` are not part of `TrackAnalysis` (the contract does not change), but the calibration tool reports them (calibration/evaluation.md sections 4, 5 and 8).

## 3. Verdict (RF-04)

`InRange(f)` means `lossy_min_cutoff_hz ≤ f < fake_max_cutoff_hz`. The rules are evaluated in order; the first match wins. v3 changes only V3 and V4 (in bold).

| # | Condition | Verdict | Reasons |
|---|---|---|---|
| V1 | Decoding failed or too short | `Error` | — (the `TrackError` explains it) |
| V2 | `H < low_hf_threshold_db` (little treble: a cappella, soft instrumental) | `Suspicious` | `LowHighFrequencyContent` (never `Fake` only because of the spectrum shape, 2.5 of the planning doc) |
| V3 | `G ≥ gap_fake_db` **and not `FullBand`** | `Fake` | — |
| V4 | `D ≥ step_fake_db` and `InRange(fs)` **and not `FullBand`** | `Fake` | — |
| V5 | `G ≥ gap_suspicious_db` | `Suspicious` | `HighBandGaps` |
| V6 | `D ≥ step_suspicious_db` and `InRange(fs)` | `Suspicious` | `AmbiguousCutoff` |
| V7 | `D ≥ step_low_suspicious_db` and `fs < lossy_min_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V8 | `D ≥ step_high_suspicious_db` and `fake_max_cutoff_hz ≤ fs < ambiguous_max_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V9 | `D < step_suspicious_db` and `fe < gradual_min_cutoff_hz` | `Suspicious` | `GradualRolloff` |
| V10 | Otherwise | `Real` | — |

Consequences of the order (no new rule is needed; they rely on the load-time checks of section 4):
- A track that V3 would have made `Fake` but is `FullBand` falls through. `gap_suspicious_db ≤ gap_fake_db`, so V5 makes it `Suspicious` with `HighBandGaps` (FP1–FP3).
- A track that V4 would have made `Fake` but is `FullBand` falls through. `step_suspicious_db ≤ step_fake_db`, so V6 makes it `Suspicious` with `AmbiguousCutoff`, unless V5 matches first (FP4: `AmbiguousCutoff`).
- The guard only turns `Fake` into `Suspicious`. No verdict becomes `Real` or `Error`, and no `Real` or `Suspicious` verdict of v2 changes. So the caught rate (`Fake` + `Suspicious`) of every setting is the same as with v2.
- The upper window always lies above the lossy range (`fake_max_cutoff_hz < full_band.low_hz`, section 4), so a V4 step is always below the window.

Higher sample rates (88.2/96/192 kHz) use the same absolute frequencies: a lossy origin still shows its step at or below ~20.3 kHz, its gaps at 16 kHz and no music in the upper window.

## 4. Invariants and parameter validation

- Deterministic: the same file and parameters always give the same result.
- Read-only: the file is opened for reading only (RNF-05).
- `algorithm_version` in every `TrackAnalysis` equals the `algorithm_version` of the parameters used (3 with the defaults).
- The code's default parameters equal `detection-params.v3.toml` (a test checks it).
- **v2 stays loadable.** `[full_band]` is optional. A params file without it (such as `detection-params.v2.toml`) loads with `FullBand` always `false`, and then gives exactly the v2 verdicts, reasons and cutoffs (a test checks it on every fixture). This lets the calibration tool compare v2 and v3 in one run (calibration/evaluation.md section 9).
- **Who checks a params file.** The core has no file loader: `DetectionParams` is deserialised with serde by whoever reads a file (today the calibration tool and tests; the app uses `DetectionParams::default()`). Two layers:
  1. **Shape** (serde, at deserialisation): every section rejects unknown fields; `[full_band]` may be absent, but if present its four fields `low_hz`, `high_hz`, `min_level_db`, `min_burst_db` are all required.
  2. **Values:** a pure function `DetectionParams::validate(&self) -> Result<(), ParamsError>` in `fakeflac-core` (no I/O; `ParamsError` names the inequality that does not hold). It checks `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db` and, when `[full_band]` is present, `fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz`. The section 3 consequences depend on these. Every loader calls it after deserialising and refuses an invalid file (the calibration tool: exit 2, evaluation.md section 9). A test checks that the defaults are valid. When `DetectionParams` reaches the settings file (H2), the settings loader calls it too.
- `detection-params.v1.toml` stays in the repository as the record of version 1. The code does not have to load it.
- `[input]` and `[fft]` are the same as in v2, so v3 needs no new extraction: `fakeflac-audio::EXTRACTION_VERSION` does not change and cached spectra are reused. The app's result cache is invalidated by the new `algorithm_version` (ADR-0008).
- `algorithm_version` numbers are allocated by the files in `specs/detection/`. A parameter file produced elsewhere (a `/calibrate` proposal, a test candidate) takes the next free number, never one already used here (calibration/evaluation.md section 7).

## 5. Test signals

Tests use synthetic signals only (`specs/detection/fixtures.md`). The fixtures `lp16k_dither`, `lp20k_floor` and `sfb21_gaps` reproduce the real failure of v1 (v1 returns `Real` for all three). The v3 pairs separate a stationary floor from intermittent content above the lossy range: `sfb21_gaps_lp20k5` (`Fake`) against `sfb21_gaps_aa21k` and `sfb21_gaps` (`Suspicious`), and `shelf19k5` (`Fake`) against `shelf19k5_bursts` (`Suspicious`). Rule-level scenarios with the FP1–FP4 measurements and the parameter checks: `specs/features/detection-full-band.feature`. Acceptance against the real corpus runs locally with `just accept` (section 7). Tests derived from v2 that v3 changes are listed in ADR-0038 ("Consequences").

## 6. Known limitations (v3)

Detection rates per encoder setting with v2 on the real corpus (RP-11 run of 2026-10-18; detectable transcodes, split `all`): cbr128 99.5 %, cbr192 99.9 %, cbr256 98.0 %, cbr320 91.6 %, v0 18.2 %, v2 99.6 %, aac128 99.0 %, **aac256 0.6 %**, opus128 96.8 %. These rounded figures are context only: acceptance compares against the v2 column of its own run (section 7). **v3 adds no rule for LAME V0 or AAC 256.** They stay known limitations of the band-spectrum features, for these reasons:

- **LAME V0 (18.2 % detected).** The calibration config records V0 without a low-pass (`lowpass_hz = 22050`, calibration.v1.toml; Q5), and VBR codes sfb21, so a V0 transcode usually has neither a cutoff step nor a 16 kHz gap. Its band spectra look like those of a full-band lossless file. Any rule on `G`, `D`, `fs`, `fe`, `U` or `B` that caught it would also catch lossless tracks like FP1–FP3, which are what v3 protects. The V0 transcodes that v2 catches are caught through `G`. If they also carry intermittent content in the upper window, v3 turns them from `Fake` into `Suspicious`: that is the intended behaviour of the guard (with these measurements they are indistinguishable from FP1–FP3). They stay caught, so the caught rate that A5 gates does not change; the drop of V0's `Fake` rate is reported by A5 as information.
- **AAC 256, ffmpeg's native encoder (0.6 % detected).** It cuts at about 20 kHz and leaves no sfb21 gap (`G` ≈ 0), so only V4 could catch it. Doing so would mean lowering `step_fake_db` or moving `fake_max_cutoff_hz` near 20 kHz. The RP-11 run found that no v2 threshold reaches these transcodes (docs/agents/progress.md, "Detection v3 findings"). FP4, a genuine lossless track with a 14.2 dB step at 19.45 kHz, shows that a step near 20 kHz is not lossy evidence on its own. The full-band guard does not help either: a lossless master band-limited near 20 kHz has no content in the upper window, just like the AAC transcode. Steps of `step_suspicious_db` to `step_fake_db` in the lossy range are already `Suspicious` (V6).
- **Lossy sources without a low-pass** (V0 above; MP3 encoders run with the low-pass disabled) are `Suspicious` at best when they show a 16 kHz gap, for the same reason.
- **Lossless tracks without content above 20.7 kHz** (masters band-limited at or below 20.5 kHz) with a gap or a lossy-range step are still `Fake`, as in v2.

Catching V0 and AAC 256 needs new measurements, not new thresholds. Candidates recorded for a later version, not specified here: per-frame statistics beyond the median in the 16–20 kHz bands (how often a band falls far below its median: the holes a VBR or AAC quantiser leaves), and codec-frame structure in time (MP3 granules of 576 samples, AAC frames of 1024). Both need a new extraction (an `EXTRACTION_VERSION` bump) and fit better with the codec-domain analysis of RF-15 (H2).

## 7. Acceptance criteria (v3)

Measured locally on the real corpus with `just accept --params specs/detection/detection-params.v2.toml --params specs/detection/detection-params.v3.toml` (v2 is the baseline), split `all`. Every comparison is against the v2 column **of the same run**, not against the rounded figures of section 6. The aggregate figures from `summary.md` go into the implementation PR (ADR-0022).

| # | Criterion | Pass |
|---|---|---|
| A1 | FP1–FP4 in the watch list | None of the four is `Fake` with v3 (expected `Suspicious`: FP1–FP3 `HighBandGaps`, FP4 `AmbiguousCutoff`) |
| A2 | Confirmed fakes (`confirmed_fake` row) | 15 of 15 `Fake` with v3, as with v2 |
| A3 | Negatives | `clean`: 0 `Fake` (v2: 0/546). `primary`: 0 `Fake` (v2: 4, exactly FP1–FP4), so target T1 is `PASS` |
| A4 | No track becomes `Real` or leaves `Real` | In the verdict-change table v2 × v3, every off-diagonal count other than `Fake → Suspicious` is 0 (it holds by construction, section 3) |
| A5 | Caught rate per encoder setting | **Gate:** for every setting, the v3 caught rate (`Fake` + `Suspicious` over detectable transcodes) is at least the v2 caught rate of the same run (by construction it is equal; the run confirms it). **Information, not a gate:** per setting, the number of positives that change `Fake → Suspicious` and the resulting drop of the `Fake` (detection) rate, v2 minus v3, in percentage points |
| A6 | Targets | T2 and T3 `PASS` with v3 (as with v2) |
| A7 | Synthetic | Every scenario of `detection-lossless.feature` and `detection-full-band.feature` passes (`just test`) |
| A8 | Full-band measurements (report, not a gate) | `U`, `B` and `FullBand` of FP1–FP4 (watch list), and per setting the `FullBand` count and the median, 95th percentile and maximum of `B` of the transcodes, in particular `cbr320` (low-pass 200 Hz below the window) |

If A1 or A5 fails, or A8 shows the guard firing on many transcodes, the implementer does not tune the parameters: the run's figures (and, locally, the listed tracks with `U` and `B`) go back to G1, where the owner decides between new values (a new params version) and accepting the result.

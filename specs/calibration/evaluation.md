---
id: SPEC-CALIBRATION-EVALUATION
requirements: [RP-11, RF-04]
status: proposed
---

# Evaluation output, parameter comparison and grid search

> Each run writes one folder `FAKEFLAC_CALIBRATION/reports/<run>/`. The main metric is the false-positive rate on trusted negatives: marking a good disc as `Fake` is worse than letting a transcode through. Main spec: [calibration.md](calibration.md).

## 1. Outputs

| File | Content | May go into a PR |
|---|---|---|
| `summary.md` | Aggregate figures only (calibration.md section 6) | **yes, the only one** |
| `report.md` | Everything in `summary.md`, plus the per-track lists with paths relative to the roots | no |
| `tracks.csv` | One row per (params file, labelled track): measurements, verdict and margins (section 8). CSV per ADR-0012 | no |
| `detection-params.v<N>.toml` | `calibrate` only, when there is a proposal (section 7) | only through a separate spec PR (G1) |

Numbers: rates as percentages with 1 decimal, dB with 2 decimals, Hz with no decimals. Tables are produced for every params file, in `--params` order.

**Header** of `summary.md` and `report.md`: run id; tool commit (`git rev-parse HEAD`, or `unknown`); `ffmpeg_version` and the `encoder_version` of each setting; each params file (name, `algorithm_version`, content hash, fingerprint id); tracks and albums per class, with albums in `tune` and `holdout`; generated entries per setting; cache statistics (generation.md section 4.2); warnings (small sample, fewer than 2 holdout albums, generation failures, entries not generated, sources skipped).

## 2. Definitions

- **Verdict** of a labelled track under a params file: `fakeflac_core::detect` on its cached spectra (`Real`, `Suspicious`, `Fake`), or `Error` with its `TrackError`.
- **Negatives:** `truth = real`. **Positives:** `truth = fake`. **Evaluation set:** every labelled row except class `reference`. `reference` rows appear only in their own row of section 3.
- **False positive (FP):** negative with `Fake`. **Soft FP:** negative with `Suspicious`. **False negative (FN):** positive with `Real`. **Detected:** positive with `Fake`. **Caught:** positive with `Fake` or `Suspicious`.
- Rate denominators exclude `Error` tracks, which are counted in their own column.
- **Splits:** `all`, `tune`, `holdout` (inputs.md section 4).

## 3. False positives by origin (main metric; OQ-13)

One table per params file and split (`all`, `tune`, `holdout`):

| origin | class | tracks | albums | Fake | Fake rate | 95 % CI | albums with ≥ 1 Fake | Suspicious | Suspicious rate | Error |
|---|---|---|---|---|---|---|---|---|---|---|

- `tracks` counts the tracks whose verdict is not `Error`; `Error` tracks are counted only in the last column. `albums` counts albums with at least one such track.
- One row per origin present, grouped by class (`primary`, `reviewed`, `clean`, `reference`), with a subtotal row per class.
- **Primary figure** (shown first, in bold, in `summary.md` and `report.md`): the `primary` subtotal, in the form `k/n tracks (p %, 95 % CI a–b %), j/m albums`. Only `Fake` counts. The `Suspicious` rate is shown next to it.
- **Interval:** Wilson score interval with `z = report.z` (1.96): `centre = (p + z²/2n) / (1 + z²/n)`, `half = z / (1 + z²/n) · √(p(1 − p)/n + z²/4n²)`, where `p = k/n`. Bounds are clamped to [0 %, 100 %] (so `k = 0` gives a lower bound of `0.0`, never `-0.0`). With `n = 0`: `n/a`.
- **Small-sample warning** for any row with fewer than `report.small_sample_tracks` (100) tracks or fewer than `report.small_sample_albums` (10) albums: "small sample: n tracks from m albums; tracks of one album are correlated, so the interval is optimistic". With today's corpus it will always appear on the primary figure (5 albums).
- **Holdout warning** (OQ-7): for each class with fewer than 2 albums in `holdout`.

## 4. Confusion matrices

- **Global**, per params file and split (`all`, `tune`, `holdout`), over the evaluation set:

| truth \ verdict | Real | Suspicious | Fake | Error | total |
|---|---|---|---|---|---|
| real | | | | | |
| fake | | | | | |

  followed by the FP rate, the soft-FP rate, the FN rate, the detection rate and the caught rate.
- **Per encoder setting**, per params file (`all`; `tune` and `holdout` too in `calibrate`): one row per setting in config order, then a `confirmed_fake` row. Columns `Real`, `Suspicious`, `Fake`, `Error`, total, detection rate, FN rate.

## 5. Watch list and `clean` albums that turn Fake

**Watch list.** Every `false-positive` track of `confirmed.txt` (inputs.md section 2) is listed one by one in every run, whatever its split, with, for each params file: verdict, reasons, `G`, `D` at `fs`, `fe`, `H` and `nearest_fake_margin_db`. `report.md` shows its path; `summary.md` shows only its watch id.

The 4 confirmed false positives of the issue are tracks of discs the user bought. The values below were measured by the Python prototype of 11/10/2026, not by the Rust pipeline. They show what to expect: with v2, A to C should be `Fake` by rule V3 (`G ≥ 6`), and D sits at the `step_fake_db` limit.

| Disc, track | Step at 16 kHz (≈ `G`) | Cutoff step (≈ `D` at `fs`) |
|---|---|---|
| purchased disc A, 06 | 14.1 dB | 13.8 dB at 21.4 kHz |
| purchased disc B, 05 | 16.3 dB | 9.5 dB at 20.4 kHz |
| purchased disc C, 13 (interlude) | 10.8 dB | 14.6 dB at 21.2 kHz |
| purchased disc D, 03 | −1.0 dB | 10.0 dB at 19.4 kHz |

**`clean` albums that turn Fake.** For each params file, every album of class `clean` with at least one `Fake` track, marked `new` when the baseline has no `Fake` track in that album. It may be a new false positive or a mistake of the old analysis; the user reviews it. `report.md` gives the album id and the tracks; `summary.md` gives only the counts (albums, tracks, `new`).

## 6. Acceptance targets (OQ-12)

Data in `[[targets]]` of the config. Each target selects rows by `classes` or by `settings`, a `verdict`, and a `max_count`. Version 1:

| id | Selection | Verdict | Max | Source |
|---|---|---|---|---|
| T1 | class `primary` | Fake | 0 | planning 2.7: no real track marked fake |
| T2 | settings `cbr128`, `cbr192` | Real | 0 | H1 delivery criterion: 0 false negatives at 128 and 192 kbps |
| T3 | setting `cbr320` | Real | 0 | planning 2.7: transcodes from 320 kbps marked fake or suspicious |

Reported as `PASS`/`FAIL` with the count, per params file, for `all` and `holdout`. A `FAIL` does not change the exit code.

## 7. `/calibrate` grid search (OQ-11)

**Grid file** (`specs/calibration/calibrate-grid.v1.toml`): `grid_version`, `baseline` (params file), `must_catch` (setting ids), `max_points`, `[weights]` and `[values]`, where each key `"section.field"` maps to a list of values.

- Only rule parameters can be searched: the fields of `[bands]`, `[extent]`, `[step]`, `[gaps]` and `[verdict]`. A key in `[input]`, `[fft]` or `[spectrogram]`, or `algorithm_version`, is an error (exit 2: `extraction parameter; compare params files with accept instead`). Unknown keys and values of the wrong type (`step.window_bands` is an integer) are errors (exit 2).
- **Points:** the Cartesian product of the value lists, in file order (first key varies slowest). Points that break an invariant are skipped and counted: `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db`, `lossy_min_cutoff_hz < fake_max_cutoff_hz ≤ ambiguous_max_cutoff_hz`, `ref_band_low_hz < ref_band_high_hz`, `hf_band_low_hz < hf_band_high_hz`. More than `max_points` points: exit 2.
- **Data: `tune` only.** Tune negatives are classes `primary`, `reviewed` and `clean` in `tune`. Tune positives are class `positive` in `tune`. `Error` rows are left out. `holdout` rows are never read by the search.
- **Feasible:** (1) no `Real` among the tune positives of the `must_catch` settings; and (2) **hard constraint: no track in `confirmed.txt` of kind `fake` may be classified as `Real`** (verified fakes must never receive a `Real` verdict).
- **Cost:** `fp · #Fake(neg) + fn · #Real(pos) + suspicious_negative · #Suspicious(neg) + suspicious_positive · #Suspicious(pos)`. Weights are confirmed at G1: `fp = 10.0`, `fn = 1.0`, `suspicious_negative = 0.5`, `suspicious_positive = 0.1`.
- **Order:** feasible before infeasible; then lower cost; then the larger smallest `nearest_fake_margin_db` among the tune negatives that are not `Fake` (an empty set counts as +∞); then fewer values different from the baseline; then earlier in grid order.
- The baseline is ranked with the same order. **Proposal** = the best point, only if it ranks strictly above the baseline. Otherwise the output says "no change proposed" (the baseline is at least as good), and no file is written. If no point is feasible, nothing is proposed and the 10 best infeasible points are listed with their `must_catch` FN count.
- **Rule families and Detection v3.** The tool architecture must support evaluating **rule families** beyond numerical tuning of v2 thresholds. The 4 confirmed false positives of purchased discs have a 16 kHz step $G$ of 10–16 dB, identical to 320 kbps MP3 transcodes. No adjustment of threshold values alone can separate them without failing 320 kbps transcode detection. What distinguishes them is bandwidth: they contain audio energy up to 20.4–21.4 kHz, whereas an MP3 transcode has no content above ~20 kHz.
  Candidate rule recorded for **detection v3**:
  *Step at 16 kHz ($G \ge \text{gap\_fake\_db}$), but with full band (spectral content up to 20.4–21.4 kHz) $\rightarrow$ `Suspicious`, not `Fake`.*
- **Output:** the objective, the counts of points evaluated and skipped, the 10 best points (changed values, cost and tune counts; in `summary.md` too), and every table of sections 3 to 6 and 8 for the baseline and the proposal on `tune` and on `holdout`. The headline figures of the proposal are the **holdout** ones.
- **Proposal file** `detection-params.v<N>.toml`, where `N` = the baseline's `algorithm_version` + 1. It is the baseline file's text with: two comment lines prepended (`# PROPOSAL from /calibrate run <run>. Not approved: it needs a spec PR (G1).` and `# Changed: <key> <old> -> <new>, …`); `algorithm_version = N`; and each changed value replaced on its own line inside its section, keeping the rest of the line (comments included). The tool parses the result and checks that it equals the intended parameters (exit 1 otherwise). The tool never writes inside the repository and never commits.

## 8. Margins

For every labelled track with spectra and every params file, signed margins against each threshold of rules.md section 3, in `tracks.csv` and in the near-threshold list of `report.md`:

| Column | Formula | When defined | The rule fires when |
|---|---|---|---|
| `m_low_hf_db` | `H − low_hf_threshold_db` | always | `< 0` |
| `m_gap_fake_db` | `gap_fake_db − G` | always | `≤ 0` |
| `m_gap_suspicious_db` | `gap_suspicious_db − G` | always | `≤ 0` |
| `m_step_fake_db` | `step_fake_db − D` | `InRange(fs)` | `≤ 0` |
| `m_step_suspicious_db` | `step_suspicious_db − D` | `InRange(fs)` | `≤ 0` |
| `m_step_low_db` | `step_low_suspicious_db − D` | `fs < lossy_min_cutoff_hz` | `≤ 0` |
| `m_step_high_db` | `step_high_suspicious_db − D` | `fake_max_cutoff_hz ≤ fs < ambiguous_max_cutoff_hz` | `≤ 0` |
| `m_gradual_hz` | `fe − gradual_min_cutoff_hz` | `D < step_suspicious_db` | `< 0` |
| `fs_fake_range_hz` | below the range: `lossy_min_cutoff_hz − fs`; at or above it: `fs − fake_max_cutoff_hz`; inside: `−min(fs − lossy_min_cutoff_hz, fake_max_cutoff_hz − fs)` | a step exists (`D > 0`) | inside when negative |
| `nearest_fake_margin_db` | `min(m_gap_fake_db, m_step_fake_db)`, ignoring an undefined term | always | `≤ 0` |

Undefined margins are empty cells. `tracks.csv` also has: params file name, `algorithm_version`, the identity columns of `labels.csv` (`path`, `root`, `truth`, `class`, `trust_origin`, `setting`, `album_id`, `split`, `watch_id`), `verdict`, `reasons`, `error`, `H`, `F`, `fe`, `D`, `fs`, `G`.

- **Near threshold:** a negative with `0 < nearest_fake_margin_db ≤ report.near_db` (1.0 dB), or a positive with `−report.near_db < nearest_fake_margin_db ≤ 0`, or any step with `|fs_fake_range_hz| ≤ report.near_hz` (300 Hz). `report.md` lists them with path and measurements.
- **Margin summary** per params file and class (in `summary.md` and `report.md`): the smallest `nearest_fake_margin_db` among negatives that are not `Fake`, the number of near negatives, and the number of near positives. This is the check that would have shown v2's 0.1 dB fit.
- **Error list** (`report.md`): every FP, soft FP, FN and caught-but-not-detected positive, and every `Error` track with its `TrackError`, with path, truth, class or setting, verdict, reasons and all measurements. Sorted by kind, then class, then path.

## 9. Several parameter files (OQ-1)

- Each `--params` file is loaded into `DetectionParams` (unknown fields rejected). A file of another rule family fails with exit 2: `<file name>: not a DetectionParams file of the v2 rule family (v1 is not supported, ADR-0036)`. Other TOML errors give the parser's message with its line. The same content given twice: exit 2.
- **Side by side** (`summary.md` and `report.md`): one column per params file with its name, `algorithm_version`, content hash (16 hex digits of `fmix64(fnv1a64(bytes))`), fingerprint id, primary figure, FP and soft-FP rates per class, FN rate, detection rate per setting, smallest negative margin and targets.
- **Verdict changes against the baseline:** for each other file, a 4 × 4 count table (baseline verdict × verdict) over the evaluation set, in `summary.md` and `report.md`. `report.md` also lists the changed tracks with truth, both verdicts, `G`, `D` and `fs`.
- **v1 reference:** `summary.md` quotes, as fixed text, the v1 figures of ADR-0035 (known-truth album with the Rust v1 code: 0 of 7 transcodes found, all 16 tracks `Real`).

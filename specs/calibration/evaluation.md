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
- **Domain (RF-15).** Every table, figure, list and target of sections 3 to 9 counts **lossless-domain rows only**; lossy rows (MP3, AAC, Opus) have their own tables (`lossy.md` section 6). Wherever this document says "labelled track" or "row", it means a lossless-domain one.
- **Negatives:** `truth = real`. **Positives:** `truth = fake`. **Evaluation set:** every labelled row except class `reference`. `reference` rows appear only in their own row of section 3.
- **False positive (FP):** negative with `Fake`. **Soft FP:** negative with `Suspicious`. **False negative (FN):** positive with `Real`. **Detected:** positive with `Fake`. **Caught:** positive with `Fake` or `Suspicious`.
- **Undetectable transcodes:** Generated entries whose source has no content the codec would remove: either the source's measured bandwidth `source.fe <= setting.lowpass_hz`, or the maximum drop in high-frequency bands ($b \ge \text{lowpass\_hz}/100$) between source and transcode is below `generate.min_removed_hf_db`. They cannot exhibit an encoder-induced cutoff step and are classified as undetectable. They are excluded from the `must_catch` feasibility constraint and from the cost calculation, and reported in the `Undetectable` column of the confusion matrices and listed in `report.md`.
- Rate denominators exclude `Error` tracks and undetectable transcodes, which are counted in their own columns.
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

| truth \ verdict | Real | Suspicious | Fake | Undetectable | Error | total |
|---|---|---|---|---|---|---|
| real | | | | — | | |
| fake | | | | | | |

  followed by the FP rate, the soft-FP rate, the FN rate, the detection rate and the caught rate. Detection and caught rates on positives are computed over detectable tracks (`total − Undetectable − Error`).
- **Per encoder setting**, per params file (`all`; `tune` and `holdout` too in `calibrate`): one row per setting in config order, then a `confirmed_fake` row. Columns `Real`, `Suspicious`, `Fake`, `Undetectable`, `Error`, total, detection rate, caught rate (`Fake` + `Suspicious` over detectable tracks, = 100 % − FN rate), FN rate. For a params file with a `[full_band]` section, also: the number of detectable positives that are `FullBand`, and the median, 95th percentile and maximum of `B` over the detectable positives with `B` defined (empty if none). These columns serve rules.md section 7, A5 and A8.

## 5. Watch list and `clean` albums that turn Fake

**Watch list.** Every `false-positive` track of `confirmed.txt` (inputs.md section 2) is listed one by one in every run, whatever its split, with, for each params file: verdict, reasons, `G`, `D` at `fs`, `fe`, `H`, `U`, `B`, `full_band` and `nearest_fake_margin_db`. `U`, `B` and `full_band` are empty for a params file without a `[full_band]` section (rules.md section 4). `report.md` shows its path; `summary.md` shows only its watch id.

The 4 confirmed false positives of the issue are tracks of discs the user bought. The values below were measured by the Python prototype of 11/10/2026, not by the Rust pipeline. They show what to expect: with v2, A to C should be `Fake` by rule V3 (`G ≥ 6`), and D sits at the `step_fake_db` limit.

| Disc, track | Step at 16 kHz (≈ `G`) | Cutoff step (≈ `D` at `fs`) |
|---|---|---|
| purchased disc A, 06 | 14.1 dB | 13.8 dB at 21.4 kHz |
| purchased disc B, 05 | 16.3 dB | 9.5 dB at 20.4 kHz |
| purchased disc C, 13 (interlude) | 10.8 dB | 14.6 dB at 21.2 kHz |
| purchased disc D, 03 | −1.0 dB | 10.0 dB at 19.4 kHz |

The first run with the Rust pipeline (2026-10-18) confirmed it: the watch ids W1–W3 `Fake` by V3 and W4 `Fake` by V4 (values in rules.md, "Why v2 was replaced", where these four real tracks are called FP1–FP4 to keep them apart from the sample corpus's watch id W1). Detection v3 (ADR-0038) added the full-band guard for them; its acceptance run (2026-10-19) left W4 `Fake` and cost too many detections (rules.md, "Why v3 was replaced"). With detection v4 (ADR-0039) W1–W3 are expected `Suspicious` and W4 `Fake`, an accepted known limitation (rules.md section 7, A1).

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

- Only rule parameters can be searched: the fields of `[bands]`, `[extent]`, `[step]`, `[gaps]`, `[full_band]` and `[verdict]`. A key in `[input]`, `[fft]` or `[spectrogram]`, or `algorithm_version`, is an error (exit 2: `extraction parameter; compare params files with accept instead`). Unknown keys and values of the wrong type (`step.window_bands` is an integer) are errors (exit 2). A `full_band.*` key with a baseline that has no `[full_band]` section is an error (exit 2: `full_band.<field>: the baseline has no [full_band] section`). A `full_band.max_lossy_step_db` key with a baseline whose `[full_band]` section has no `max_lossy_step_db` (a v3 file) is an error too (exit 2: `full_band.max_lossy_step_db: the baseline's [full_band] section has no max_lossy_step_db`): the proposal replaces values on their own lines and never adds a line.
- **Points:** the Cartesian product of the value lists, in file order (first key varies slowest). Points that break an invariant are skipped and counted: `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db`, `lossy_min_cutoff_hz < fake_max_cutoff_hz ≤ ambiguous_max_cutoff_hz`, `ref_band_low_hz < ref_band_high_hz`, `hf_band_low_hz < hf_band_high_hz`, when the point has a `[full_band]` section, `fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz` and, when it has `max_lossy_step_db`, `0 ≤ full_band.max_lossy_step_db ≤ step_fake_db` (the checks of `DetectionParams::validate`, rules.md section 4, plus the band and range orders). More than `max_points` points: exit 2.
- **Lossy rows** never enter the grid search, and a `lossy.*` key is an error (exit 2; `lossy.md` section 7).
- **Data: `tune` only.** Tune negatives are classes `primary`, `reviewed` and `clean` in `tune`. Tune positives are class `positive` in `tune`. `Error` rows and undetectable transcodes are left out. `holdout` rows never enter the cost or the ranking; the only `holdout` rows the search reads are the confirmed fakes, for hard constraint (2).
- **Feasible:** (1) no `Real` among the detectable tune positives of the `must_catch` settings (undetectable transcodes whose source has no content above `lowpass_hz` are excluded); and (2) **hard constraint: no track in `confirmed.txt` of kind `fake` may be classified as `Real`** (verified fakes must never receive a `Real` verdict). Constraint (2) covers **every** confirmed fake, in `tune` and in `holdout`: it is a safety rule, not an objective, and the known-truth album must constrain the search wherever the split puts it. The report says how many of the confirmed fakes checked by (2) are in `holdout`.
- **Cost:** `fp · #Fake(neg) + fn · #Real(pos) + suspicious_negative · #Suspicious(neg) + suspicious_positive · #Suspicious(pos)`. Undetectable transcodes are excluded from the cost. Weights are confirmed at G1: `fp = 10.0`, `fn = 1.0`, `suspicious_negative = 0.5`, `suspicious_positive = 0.1`.
- **Order:** feasible before infeasible; then lower cost; then the larger smallest `nearest_fake_margin_db` among the tune negatives that are not `Fake` (an empty set counts as +∞); then fewer values different from the baseline; then earlier in grid order.
- The baseline is ranked with the same order. **Proposal** = the best point, only if it ranks strictly above the baseline. Otherwise the output says "no change proposed" (the baseline is at least as good), and no file is written. If no point is feasible, nothing is proposed and the 10 best infeasible points are listed with their `must_catch` FN count.
- **Rule families and Detection v3.** The tool architecture must support evaluating **rule families** beyond numerical tuning of v2 thresholds. The 4 confirmed false positives of purchased discs have a 16 kHz step $G$ of 10–16 dB, identical to 320 kbps MP3 transcodes. No adjustment of threshold values alone can separate them without failing 320 kbps transcode detection. What distinguishes them is bandwidth: they contain audio energy up to 20.4–21.4 kHz, whereas an MP3 transcode has no content above ~20 kHz.
  Candidate rule recorded for **detection v3**:
  *Step at 16 kHz ($G \ge \text{gap\_fake\_db}$), but with full band (spectral content up to 20.4–21.4 kHz) $\rightarrow$ `Suspicious`, not `Fake`.*
  It was specified in `specs/detection/rules.md` as detection v3 (algorithm_version 3, ADR-0038), extended to the cutoff-step rule V4 (FP4). "Full band" is measured by the level `U` and the burstiness `B` of the mean and median spectra in the upper window 20.7–21.0 kHz, not by `fe`: most transcodes also have `fe` = Nyquist because of their floor. Detection v4 (algorithm_version 4, ADR-0039) adds that a track whose cutoff step lies in the lossy range is never full band (`full_band.max_lossy_step_db`), so in practice the guard only acts on the gap rule. The v3 parameters are searched with `specs/calibration/calibrate-grid.v2.toml` (baseline v3); no grid file has a v4 baseline yet, because on the real corpus no point is feasible (rules.md, Q10).
- **Known limitations (LAME V0, AAC 256).** Their low detection rates under v2 (18.2 % and 0.6 % on 2026-10-18) are not a tuning problem: no measurement of rules.md separates them from full-band or band-limited lossless tracks (rules.md section 6). The reports keep them in every table; the `must_catch` settings stay `cbr128` and `cbr192`.
- **Output:** the objective, the counts of points evaluated and skipped, the 10 best points (changed values, cost and tune counts; in `summary.md` too), and every table of sections 3 to 6 and 8 for the baseline and the proposal on `tune` and on `holdout`. The headline figures of the proposal are the **holdout** ones.
- **Proposal file** `detection-params.v<N>.toml`, where `N` is the **next free version**: 1 + the largest of the baseline's `algorithm_version` and the `algorithm_version` of every `specs/detection/detection-params.v*.toml` in the repository (read-only; the repository is found as in calibration.md section 3; if it is not found, the tool writes no proposal and exits 2, because `N` cannot be allocated safely). Version numbers are allocated by `specs/detection/` (rules.md section 4), so a proposal never reuses one, which would collide with an existing params version and with the app's cache key (ADR-0008). Today, from the v2, v3, v4 or v5 baseline, `N` = 6 (`detection-params.v5.toml`, detection v5, takes 5). It is the baseline file's text with: two comment lines prepended (`# PROPOSAL from /calibrate run <run>. Not approved: it needs a spec PR (G1).` and `# Changed: <key> <old> -> <new>, …`); `algorithm_version = N`; and each changed value replaced on its own line inside its section, keeping the rest of the line (comments included). The tool parses the result and checks that it equals the intended parameters (exit 1 otherwise). The tool never writes inside the repository and never commits.

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
| `m_full_band_level_db` (v3) | `U − full_band.min_level_db` | `U` defined (rules.md section 2) | the guard needs `≥ 0` |
| `m_full_band_burst_db` (v3) | `B − full_band.min_burst_db` | `B` defined | the guard needs `≥ 0` |
| `full_band_margin_db` (v3) | `min(m_full_band_level_db, m_full_band_burst_db)` | `U` and `B` defined | `U` and `B` pass when `≥ 0` |
| `m_full_band_step_db` (v4) | `full_band.max_lossy_step_db − D` | the params have `max_lossy_step_db` and `InRange(fs)` (rules.md section 2) | `LossyStep` holds, and the guard is off, when `≤ 0` |
| `nearest_fake_margin_db` | with `a = min(m_gap_fake_db, m_step_fake_db)` and the guard margin `g = min(full_band_margin_db, m_full_band_step_db)`, each ignoring an undefined term: `max(a, g)` when `full_band` is `true`, otherwise `a` | always | V3 or V4 gives `Fake` (unless V2 matches first) when `a ≤ 0` and the track is not `FullBand`. Without `max_lossy_step_db` (v3 files) `g = full_band_margin_db`, and `full_band` is `true` exactly when `full_band_margin_db ≥ 0`, as in v3. Without a `[full_band]` section this is v2's `min(m_gap_fake_db, m_step_fake_db)` |

`full_band` is `true` exactly when `full_band_margin_db ≥ 0` and `m_full_band_step_db` is undefined or `> 0` (rules.md section 2: `FullBand` needs `D < max_lossy_step_db` when `fs` is in the lossy range).

Undefined margins are empty cells. `tracks.csv` also has: params file name, `algorithm_version`, the identity columns of `labels.csv` (`path`, `root`, `truth`, `class`, `trust_origin`, `setting`, `album_id`, `split`, `watch_id`), `verdict`, `reasons`, `error`, `H`, `F`, `fe`, `D`, `fs`, `G`, `U`, `B` and `full_band` (`true`, `false`, or empty when `U` or `B` is undefined).

- **Near threshold:** a negative with `0 < nearest_fake_margin_db ≤ report.near_db` (1.0 dB), or a positive with `−report.near_db < nearest_fake_margin_db ≤ 0`, or any step with `|fs_fake_range_hz| ≤ report.near_hz` (300 Hz). For a params file **with** a `[full_band]` section, also a negative kept from `Fake` only by the guard at its threshold: `a ≤ 0`, `full_band` is `true` and `g = 0` (its `nearest_fake_margin_db` is 0; with v4 values only `full_band_margin_db` can be 0 there, since `FullBand` needs `m_full_band_step_db > 0`). For a params file without the section the list is exactly v2's. `report.md` lists them with path and measurements.
- **Margin summary** per params file and class (in `summary.md` and `report.md`): the smallest `nearest_fake_margin_db` among negatives that are not `Fake`, the number of near negatives, and the number of near positives. This is the check that would have shown v2's 0.1 dB fit.
- **Error list** (`report.md`): every FP, soft FP, FN and caught-but-not-detected positive, and every `Error` track with its `TrackError`, with path, truth, class or setting, verdict, reasons and all measurements. Sorted by kind, then class, then path.

## 9. Several parameter files (OQ-1)

- Each `--params` file is loaded into `DetectionParams` (unknown fields rejected). The **v2 rule family** is the v2, v3, v4 and v5 files: a v3 file is a v2 file plus the optional `[full_band]` section, a v4 file is a v3 file plus the optional `full_band.max_lossy_step_db`, a v5 file is a v4 file plus the optional `[lossy]` section (lossless rows ignore it; lossy rows need it, `lossy.md` section 6.1), and a file without them is evaluated exactly as v2 or v3 (rules.md section 4). A file of another rule family fails with exit 2: `<file name>: not a DetectionParams file of the v2 rule family (v1 is not supported, ADR-0036)`. After deserialising, the loader calls `DetectionParams::validate()` (rules.md section 4): a file that breaks `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db`, with a `[full_band]` section `fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz`, or with `max_lossy_step_db` `full_band.max_lossy_step_db ≥ 0` or `full_band.max_lossy_step_db ≤ step_fake_db`, or with a `[lossy]` section any check of lossy-source.md section 6, fails with exit 2, naming the file and the inequality that does not hold. A `[full_band]` section with a missing or unknown field is a TOML error. Other TOML errors give the parser's message with its line. The same content given twice: exit 2.
- **Side by side** (`summary.md` and `report.md`): one column per params file with its name, `algorithm_version`, content hash (16 hex digits of `fmix64(fnv1a64(bytes))`), fingerprint id, primary figure, FP and soft-FP rates per class, FN rate, detection rate and caught rate per setting, smallest negative margin and targets.
- **Verdict changes against the baseline:** for each other file, a 4 × 4 count table (baseline verdict × verdict) over the evaluation set, in `summary.md` and `report.md`, followed by the same changes on positives broken down per setting (one row per setting with the count of each off-diagonal change and the change of the detection rate in percentage points). `report.md` also lists the changed tracks with truth, both verdicts, `G`, `D`, `fs`, `U` and `B`.
- **v1 reference:** `summary.md` quotes, as fixed text, the v1 figures of ADR-0035 (known-truth album with the Rust v1 code: 0 of 7 transcodes found, all 16 tracks `Real`).

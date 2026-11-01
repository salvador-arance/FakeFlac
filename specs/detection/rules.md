---
id: SPEC-DETECTION-LOSSLESS
requirements: [RF-03, RF-04, RF-14]
status: proposed
algorithm_version: 6
params: specs/detection/detection-params.v6.toml
supersedes: algorithm_version 5 and 4 (detection-params.v5.toml and detection-params.v4.toml, same lossless rules, kept and still loadable, section 4); algorithm_version 3 (detection-params.v3.toml, kept and still loadable, section 4); algorithm_version 2 (detection-params.v2.toml, kept and still loadable, section 4); algorithm_version 1 (detection-params.v1.toml, kept as a record)
---

# Detection rules v6: lossless files (WAV, FLAC)

> For a WAV or FLAC file: compute two band spectra (the power **mean** and the per-frame **median**), measure the deepest relative **step** in the treble, the **gap step** at 16 kHz, whether there is **music above the lossy range** (the full-band guard) and whether a lossy-range step is a **content edge** rather than a low-pass (the edge step), and decide **real / suspicious / fake**. Every number below is a named parameter in `detection-params.v6.toml`; the code never hard-codes them (S4, ADR-0011). Design rationale and evidence: ADR-0035 (v2), ADR-0038 (v3), ADR-0039 (v4) and ADR-0042 (v6).

Lossy formats (MP3, M4A with AAC, Opus; RF-15) get their own rules in H2: `specs/detection/lossy-source.md`, detection v5 (`detection-params.v5.toml`, ADR-0041). v5 keeps every section and value of v4 and adds `[lossy]`, so the v4 rules and values apply unchanged under v5 to WAV, FLAC and ALAC (in M4A). **v6 is the default.** It keeps every section and value of v5 and adds `[edge]`, which only acts on rule V4 of lossless files; lossy files get exactly their v5 verdicts. **Since detection v7** (lossy-source.md section 11, RF-16) the default is `detection-params.v7.toml`, which keeps every section and value of v6 and adds `[lossy.streaming]` (lossy files only): the lossless rules and values of this file do not change, and lossless results with v7 equal those with v6. **Since detection v8** (lossy-source.md section 12, RF-14/RF-15 Opus) the default is `detection-params.v8.toml`, which keeps every section and value of v7 and adds the `opus` rows of `[lossy.cutoffs]` (Opus files only): lossless results with v8 equal those with v7. Where sections 3–7 say "v4", they describe the rules shared by v4, v5 and v6; the v6 additions are marked "(v6)". Upsample detection (RF-12) is H3.

**Names used here.** `FP1`–`FP4` are the four purchased tracks confirmed as false positives of v2. The reports of 2026-10-18 and 2026-10-19 list them as watch ids `W1`–`W4` (the tool numbers `false-positive` lines of `confirmed.txt` in line order, inputs.md section 2); this spec uses both names for the same tracks. **`W5` and `W6`** are two other `primary` tracks of W4's purchased album, which the owner adds to `confirmed.txt` as the 5th and 6th `false-positive` lines (Q18); the v6 proposal called them N1 and N2. W5 is the `primary` track "0.12 dB below the `Fake` side's threshold" of section 6 and W6 the `primary` track "with a lossy-range step and `G ≥ 4`" of the same section. The sample corpus of the calibration tool has its own, unrelated watch id `W1` (a synthetic track). **"Content up to `fe`"** always means the content extent of section 2: `fe` is the lower edge of the highest band with content, so that band reaches `fe + band_hz`. **"The v3 run"** is the acceptance run `20261019T091918Z` (v2 baseline and v3, split `all`); **"the v4 run"** is `20261019T141953Z` (v2 baseline and v4, split `all`, `tune` and `holdout`). Only their aggregate figures and the measurements of W1–W6 appear here (ADR-0022).

**What changes in v4.** One parameter, `full_band.max_lossy_step_db`, and one more condition in `FullBand` (section 2): a track whose cutoff step lies in the lossy range is never full band. Every other value, measurement and rule is the one of v3. The verdict table (section 3) is unchanged; with v4 values the guard only ever acts on the gap rule V3.

**What changes in v6.** Three measurements of the existing spectra (the median step `D_Q`, the top floor `F_top` and the post-step residual `R`), one predicate (`EdgeStep`), one more condition in V4 ("and not `EdgeStep`") and an optional `[edge]` section with six parameters (sections 2–4). Against v5, for the same spectra, the only possible verdict change is `Fake → Suspicious` on V4 tracks. No new extraction, no contract change (ADR-0042).

## Open questions for G1 (v6)

Q12–Q18 are the questions Q1–Q7 of the v6 proposal (merged at G1 in #60 without explicit answers). The owner delegated them on 2026-10-23: each takes the proposal's default.

| # | Question | Status |
|---|---|---|
| Q12 | (proposal Q1) `EdgeStep` on V4 only, or on V4 and V6 (W4 and W5 would become `Real`)? | **Resolved (owner delegated, 2026-10-23): V4 only.** W4 becomes `Suspicious` (`AmbiguousCutoff`), W5 stays `Suspicious`; against v5 the only change is `Fake → Suspicious` |
| Q13 | (proposal Q2) A new reason (for example `contentEdgeStep`) for a track that `EdgeStep` moved? | **Resolved (owner delegated, 2026-10-23): no new reason.** Such a track gets `AmbiguousCutoff` from V6, or `HighBandGaps` from V5; no contract change |
| Q14 | (proposal Q3) Thresholds `x` = 3 dB and `y` = 12 dB as starting values, with the corpus deciding? | **Resolved (owner delegated, 2026-10-23): yes.** The corpus analysis then showed that (3, 12) fails E6 (cbr192 −1.7 pp, aac128 −1.2 pp; "Why v5's lossless rules were replaced"), and the owner decided to add a cap on `D`. The values of v6 are those of Q19 |
| Q15 | (proposal Q4) W6 (N2) is out of scope: its `Suspicious` comes from the gap rule V5 (`G` = 5.65 dB). Accept? | **Resolved (owner delegated, 2026-10-23): accept.** Revisit with the per-frame statistics of section 6 |
| Q16 | (proposal Q5) Apply `EdgeStep` to the lossy rule L3? | **Resolved (owner delegated, 2026-10-23): not in v6.** lossy-source.md is unchanged |
| Q17 | (proposal Q6) Absolute floor window `[20900, 22000)` Hz rather than one relative to Nyquist? | **Resolved (owner delegated, 2026-10-23): absolute** (section 2) |
| Q18 | (proposal Q7) Should the watch list gain N1 and N2? | **Resolved (owner delegated, 2026-10-23): yes, as W5 and W6.** The owner adds them to the local `confirmed.txt` after W4 (no repo change; nothing is renumbered) |
| Q19 | **Values.** `max_median_step_db` = 3 dB, `min_residual_db` = 15 dB and the cap `max_step_db` = 16 dB were chosen on split `tune` only and then checked on `holdout` ("Why v5's lossless rules were replaced"). On `holdout` the worst protected setting is cbr192 at 0.41 pp: one more lost transcode there (out of 484) would break the 0.5 pp gate. The run reuses the cache, so on the 16-bit settings it will reproduce these figures exactly; the uncertainty is about other corpora (section 8.2: an album-level bootstrap gives about a 30 % chance that a new corpus of this size fails E6 with nothing wrong). Points with a better `holdout` figure, such as (2, 15, 16), were found by looking at `holdout` and are not taken. Accept these values for the corpus run? | **Resolved (owner, 2026-10-25): yes, kept after the run.** The run of section 8.3 passes E6 on `all` (cbr192 0.28 pp, as predicted) and misses the original `holdout` gate by one transcode (cbr192 and cbr192_s24 at 0.51 pp). The values are not tuned; the `holdout` gate changes instead (Q23) |
| Q20 | **24-bit residual.** `F_top` is never taken below `edge.floor_min_db` (absolute), the floor that the same audio would have at 16 bits (section 2). Only above 16 bits, or always? At which level? | **Resolved (owner, 2026-10-24): always, at −104 dB.** On every measured file it is equivalent to applying it only above 16 bits, and the measurement then needs no bit depth (no API change). At 16 bits it moves `F_top` by at most 0.23 dB on 319 of the cached transcodes and changes no verdict. −104 dB is the floor of two channels averaged; dual-mono and mono files are a limitation (section 6) |
| Q21 | **Calibration at 24 bits.** Which settings does `calibration.v3.toml` decode to 24 bits (generation.md section 2), gated in E6 like the protected settings? | **Resolved (owner, 2026-10-24): `cbr128_s24`, `cbr192_s24`, `cbr256_s24` and `cbr320_s24`.** aac128 was dropped for cbr256: aac128's residue rarely falls below the 16-bit floor (31 % of the sampled pairs, `fs` moves in 3 %), while cbr256 has the most transcodes near the conjunction. About 5,700 more transcodes on the first run. Their losses are strongly correlated with those of the 16-bit settings they repeat, so they are not independent evidence |
| Q22 | **Changes to approved specs and protected tests** in this PR: (a) this file: defaults and `algorithmVersion` become v6/6, the next free `algorithm_version` becomes **7**; (b) `detection-lossless.feature`: "Parameters come from the versioned file" names `detection-params.v6.toml` and `algorithmVersion 6`, and the two scenarios that analyse every synthetic signal ("A v2 params file changes only the guarded verdicts", "A v3 params file differs only where a lossy-range step removes the guard") use `detection-params.v5.toml` instead of the defaults, because `edge_bump19k3` differs with v6; (c) `decode-lossy.feature`: "A lossy row has the same fields as a lossless row" expects `algorithmVersion 6`; `detection-lossy.feature` says that the defaults are v6 with v5's `[lossy]` values, and "v5 gives the v4 lossless verdicts" uses `detection-params.v5.toml`, not the defaults; (d) `calibration.feature`: the proposals of "The grid search proposes new parameters from tune data" and "The full-band guard can be searched from a v3 baseline" are `detection-params.v7.toml` / `algorithm_version 7`; `calibration/evaluation.md` section 7 and `calibration/sample-corpus.md` section 4: next free version 7; (e) `lossy-source.md`: defaults v6, its `[lossy]` section unchanged; (f) `contracts/commands.md`: `algorithmVersion` 6 with the defaults, no type change; (g) the calibration specs: `evaluation.md` (sections 4, 5, 7, 8, 9: `D_Q`, `R`, `EdgeStep` and the edge report), `generation.md` (a per-setting `decode_args`), `calibration.v3.toml`; (h) the protected tests listed in ADR-0042 ("Consequences") change in the test stage. Agree? | Open. Default: yes to all |
| Q23 | **The `holdout` gate of E6, decided after the run.** The corpus run (section 8.3) fails the original E6 gate on `holdout` only: cbr192 and cbr192_s24 each lose 2 of at most 392 detectable transcodes (0.51 pp, 95 % Wilson interval 0.14–1.84 pp), against a gate of 0.5 pp. The four losses are the same two source tracks at 16 and 24 bits. After the library was reorganised, the new `holdout` has 392 transcodes per setting instead of 484, so one track is 0.26 pp and a 0.5 pp gate allows one loss; it also mixes in albums of the old `tune`, where the values were chosen. Change the `holdout` gate to "the lower bound of the 95 % Wilson interval of the drop is at most 0.5 pp" (`all` keeps the 0.5 pp gate on the drop itself; section 8)? | **Owner's choice (2026-10-25): yes**, against keeping v5 (W4 stays a `Fake` false positive) or re-choosing the values (only possible by looking at this `holdout`). Approved at G1 with this change. This is a change made after seeing the result; it is recorded as such, and the new gate applies to every later E6 run |

## Open questions for G1 (v4)

| # | Question | Status |
|---|---|---|
| Q1 | (v3) Pre-measure `U` and `B` before approving the v3 values? | **Resolved (owner, G1 v3): approved without pre-measuring.** The v3 run then measured them, and v3 failed A1, A2 and A8 ("Why v3 was replaced"). v4 keeps the v3 window and values: four other windows were re-measured and none does better (same section) |
| Q2 | Should A5 allow a tolerance on lost detections? | **Resolved (owner, G1 v3), kept for v4:** A5 gates the caught rate (`Fake` + `Suspicious`) per setting against the v2 column of the same run; the drop of the `Fake` rate is reported as information |
| Q3 | Should a guarded track get a new reason (for example `fullBandGaps`) instead of `highBandGaps`? It is a contract change (bindings, UI text). With v4 values only V3 is ever guarded, so the new reason would only replace `highBandGaps` | Open. Default: no new reason |
| Q4 | (v3) Changes to approved scenarios, calibration defaults and version numbering | **Resolved (owner, G1 v3): defaults applied.** The v4 equivalents are Q8 |
| Q5 | Is `lowpass_hz = 22050` for `v0` (no low-pass with ffmpeg's `-q:a 0`) confirmed? Section 6 relies on it. The LAME tag carries LAME's low-pass value (one byte, in units of 100 Hz); reading it needs a small spec addition to generation.md, or a manual check of one generated V0 file | Open. Default: taken as confirmed until the tag says otherwise |
| Q6 | Add the v2 caught rates of `v0` and `aac256` to section 6? | Open. Default: yes, from the v4 acceptance run (A5) |
| Q7 | FP4 (W4) is not cleared by the guard. Accept, or look for another discriminator? | **Resolved (owner, G1 2026-10-19): accept.** The v3 run measured W4 at `U` = −47.06 dB and `B` = 2.86 dB (below `min_burst_db`, so v3 already left it `Fake`), and its step (14.22 dB at 19.45 kHz, in the lossy range) cannot be told from a cbr256 transcode's with these measurements. W4 stays `Fake` as a known limitation (section 6); A1 reports it |
| Q8 | Changes to approved specs, scenarios and calibration behaviour: (a) the fixtures `sfb21_gaps` and `shelf19k5_bursts` go back from `Suspicious` (v3) to `Fake` (v4), and so does the sample-corpus setting `fxv0` (`sfb21_gaps`): v2 and v4 give the same sample-corpus matrices; (b) in `detection-full-band.feature` the FP4 row and the "gap and lossy step" row become `Fake` with v4 and their v3 verdicts move to a new scenario that applies `detection-params.v3.toml`; (c) the calibration tool's defaults (`--params`, grid v1 baseline v2) and `calibrate-grid.v2.toml` (baseline v3) do not change, and no grid with a v4 baseline is added while Q10 stands; (d) the next free `algorithm_version` becomes 5: `/calibrate` proposals and the sample corpus's `candidate.toml` take 5, and the calibration scenarios that name `detection-params.v4.toml` / `algorithm_version 4` as a proposal now name v5 / 5; (e) the protected tests listed in ADR-0039 ("Consequences") change in the test stage. Agree? | Open. Default: yes to all |
| Q9 | Acceptance targets T2 and T3 count transcodes with verdict `Real` over **all** generated rows of the setting, undetectable transcodes included, while every rate excludes them (evaluation.md sections 2 and 6). On the v3 run v2 already fails both (T2 = 4, T3 = 8; 5 of those 12 rows are undetectable; all 12 are in `tune`). Should T2 and T3 exclude undetectable transcodes, like the rates do? Or be judged on `holdout`? It is a change to the calibration spec (evaluation.md section 6), not to detection v4, and it needs its own spec PR. A6 of v4 does not depend on it | Open. Default: no change in this PR; A6 compares against v2 |
| Q10 | **`/calibrate` has no feasible point on the real corpus.** In the grid run made for the v4 analysis, every point, the baseline included, breaks the `must_catch` constraint (evaluation.md section 7, constraint 1), because 2 detectable `cbr128` transcodes in `tune` are `Real` under every point: one has its step at 14450 Hz, 50 Hz below `lossy_min_cutoff_hz`, with `D` = 8.07 dB (below `step_low_suspicious_db`); the other has content up to Nyquist, no step (`D` = 5.49 dB) and no gap (`G` = −19.5 dB). Until this is dealt with (a tolerance in `must_catch`, a review of those two transcodes, or a refined undetectable rule), `/calibrate` proposes nothing on this corpus. Follow-up for RP-11 (calibration spec), not part of v4 | Open. Default: follow-up spec PR after v4 |
| Q11 | A8 is a gate in v4 with a bound of 5 % `FullBand` per protected setting (section 7). Tighten it (for example 2 %), or keep it a report as in v3? | Open. Default: gate at 5 % |

## Why v1 was replaced

v1 placed the cutoff where the smoothed mean spectrum fell more than `content_drop_db = 60` dB below the reference level. A FLAC made from a decoded MP3 still has noise above the MP3 low-pass: codec noise, plus dither or requantisation noise from writing 16 bits. In real files this floor sits only 40–55 dB below `L_ref`, so v1 counted it as content, put the cutoff at Nyquist and returned `Real`. On a local corpus with known truth (16 tracks: 7 transcodes, 9 lossless), the v1 code returned `Real` for all 16. v2 does not use an absolute level to find the cutoff. It looks for a **relative step** against the level just below it, and it adds a measurement that does not depend on a clean cutoff: the intermittent empty scale-factor band above 16 kHz (sfb21) that LAME leaves, which shows up in the **median** spectrum.

## Why v2 was replaced

The first acceptance run of v2 with the Rust pipeline on the real corpus (RP-11, 2026-10-18) gave 0/546 `Fake` on `clean` tracks and 15/15 confirmed fakes `Fake`, but 4 of the 67 purchased tracks (`primary`) were `Fake`. The owner confirmed all 4 as false positives:

| id | watch id | `G` (dB) | `D` (dB) | `fs` (Hz) | `fe` (Hz) | v2 rule |
|---|---|---|---|---|---|---|
| FP1 | W1 | 18.39 | 8.93 | 20450 | 20900 | V3 |
| FP2 | W2 | 13.18 | 14.79 | 21450 | 21400 | V3 |
| FP3 | W3 | 10.26 | 14.70 | 21250 | 21100 | V3 |
| FP4 | W4 | −0.25 | 14.22 | 19450 | 22050 | V4 |

FP1–FP3 have a 16 kHz gap step as large as a 320 kbps transcode's, so no value of `gap_fake_db` separates them (calibration/evaluation.md section 7). FP4 has a cutoff step in the lossy range. What the four share is **content above the highest lossy low-pass** (the 320 kbps LAME low-pass, 20.5 kHz): `fe` = 20.9–21.4 kHz for FP1–FP3 and `fe` = Nyquist for FP4.

`fe` cannot be the guard: a decoded transcode's floor above the low-pass is usually less than 60 dB below `L_ref`, so most transcodes also have `fe` = Nyquist (the v1 failure). The fixtures `lp16k_dither` and `lp20k_floor` are exactly that case. v3 used a property the floor does not have: **above a lossy low-pass there is no coded audio, only a floor (dither, rounding noise of the decoder output) whose level hardly changes from frame to frame, so its per-frame median `Q` is within a fraction of a dB of its mean `M`. Music up there comes and goes with transients, so `M` is several dB above `Q`.** v3 measures the level `U` and this burstiness `B` in a window just above 20.5 kHz. When both show content (`FullBand`), the gap rule V3 and the step rule V4 give `Suspicious` instead of `Fake`.

## Why v3 was replaced

The v3 run failed three of its acceptance criteria (v3 section 7, compared with the v2 column of the same run):

- **A2:** 8 of 15 confirmed fakes `Fake` (v2: 15/15). The guard fired on the other 7.
- **A1:** W1–W3 `Suspicious` (`HighBandGaps`), but W4 still `Fake`.
- **A8:** the guard fired on a large share of the detectable transcodes of every setting, and the `Fake` rate fell accordingly:

| setting | cbr128 | cbr192 | cbr256 | cbr320 | v2 | aac128 | opus128 | v0 | aac256 |
|---|---|---|---|---|---|---|---|---|---|
| `FullBand` with v3 (detectable) | 625/1424 | 404/1422 | 581/1419 | 410/1398 | 848/1416 | 837/1424 | 1046/1365 | 1316/1387 | 1065/1134 |
| `Fake` % with v2 | 99.6 | 99.9 | 98.2 | 92.1 | 99.6 | 99.1 | 97.3 | 16.9 | 0.3 |
| `Fake` % with v3 | 55.9 | 71.6 | 58.7 | 64.7 | 40.1 | 41.2 | 23.4 | 2.7 | 0.1 |

Negatives with v3: `clean` 0 `Fake`, `primary` 1 (W4), `reference` 1 (v2: 0, 4 and 3).

The measured upper-window values (same window in v3 and v4):

| id | watch id | `U` (dB) | `B` (dB) | v3 `FullBand` |
|---|---|---|---|---|
| FP1 | W1 | −56.64 | 9.53 | true |
| FP2 | W2 | −44.74 | 30.60 | true |
| FP3 | W3 | −51.46 | 10.76 | true |
| FP4 | W4 | −47.06 | 2.86 | false (`B` < 3) |

**No `(U, B)` threshold can work.** The 7 confirmed fakes that v3 guarded have `U` = −34.7 to −49.2 dB and `B` = 3.8 to 20.3 dB, with `fe` = Nyquist: higher than W1 and W3 on both axes. No region of the `(U, B)` plane keeps W1–W3 guarded and leaves those 7 unguarded, so with any `min_level_db` / `min_burst_db` A2 is at most 10/15 (a simulated grid of 64 `(U, B)` points found the same). The transcodes are no better separated: inside the tightest box around W1–W3 (`U` ≥ −56.64, `B` ≥ 9.53) lie 8–45 % of the detectable transcodes of each protected setting (section 7), 77.5 % of `v0` and 51.7 % of `aac256`. Above a lossy low-pass, transcodes and these confirmed fakes carry bursty residue in the upper window. **Hypothesis, not verified:** it is intermittent decoder-side distortion, such as clipping or overshoot of loud passages, which would explain the high `B`.

**What does separate them is where the deepest step is.** W1–W3 have their deepest cutoff step **above** the lossy range (`fs` = 20450, 21450 and 21250 Hz: a mastering low-pass at 20.4–21.5 kHz); their v2 `Fake` came from `G` alone. The guarded confirmed fakes have their deepest step **inside** it (`fs` = 15750–19850 Hz, `D` = 5.63–23.14 dB), and so do the transcodes (their low-pass, `D` median 22–34 dB per setting). Inside the `(U, B)` box around W1–W3 **and** with `fs` ≥ 20450 Hz lie 0.0–0.1 % of the detectable transcodes of each protected setting (section 7) and 0 of the 15 confirmed fakes (55.9 % of `v0`, 25.2 % of `aac256`). The guard is physically meaningful only when the track has no lossy cutoff: if there is a step in the lossy range, bursty content above it is residue, not music. v4 adds exactly that condition.

**The window stays.** The v3 run's tool re-measured `U` and `B` with four other windows ([20600, 21000), [20800, 21000), [20900, 21000) and [21000, 21500) Hz). The transcode residue is broadband, so no window escapes it: with the v4 condition, every window from 20600 to 20900 Hz gives the same A2 and watch-list verdicts, and transcode `Fake` rates within 0.3 percentage points; [21000, 21500) loses W1 and W3 at `U` ≥ −60 dB. v4 keeps [20700, 21000).

## Why v5's lossless rules were replaced (v6)

v4 (and v5, which keeps its lossless rules) left W4 `Fake` through V4 as a known limitation (Q7, section 6): `D` = 14.22 dB at `fs` = 19450 Hz, in the lossy range. Clearing it "needs a new measurement, not a threshold". The evidence below comes from local analyses (2026-10-22 and 2026-10-23) with throwaway scripts outside the repository, validated against the tool: an offline port of `measure()` and `decide()` reproduces the v4 verdicts of the v4 run with **0 mismatches on the 14,241 rows** it pairs with `tracks.csv`. Only aggregates and the measurements of W1–W6 appear here (ADR-0022).

**What makes W4's step.** Around the step (`M` and `Q` relative to `L_ref`, dB):

| band centre (Hz) | 18950 | 19050 | **19150** | **19250** | **19350** | 19450 | 19550 | 19650 | 19750 | 20450 | 21450 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `M` (mean) | −25.8 | −26.8 | **−18.6** | **−19.3** | **−20.0** | −31.8 | −32.6 | −33.9 | −34.0 | −41.4 | −51.2 |
| `Q` (median) | −34.5 | −35.6 | −36.0 | −37.1 | −37.4 | −38.1 | −38.6 | −39.5 | −40.2 | −46.4 | −51.3 |

The three bands below `fs` stand 8–12 dB above their neighbours in `M` while `Q` is smooth: a short, narrow-band sound present in a few frames only. `d(i)` finds 14.22 dB there, but `Q` falls only 0.32 dB across the same bands, and above the step `M` keeps falling for another ~17 dB before it reaches the floor at ~21 kHz. A low-pass does the opposite: right above it the spectrum is at the floor. The same album bought as 320 kbps MP3 from the same store matches the lossless files within 0.3 dB up to 19.5 kHz (full-track Welch spectra, six tracks), so the lossless files cannot come from a lossy source cut at 19.5 kHz. W5 shows the same pattern at 19.6–20.1 kHz.

| id | v5 verdict (rule) | `D` (dB) | `fs` (Hz) | `G` (dB) | `U` (dB) | `B` (dB) | `D_Q` (dB) | `R` (dB) | v6 verdict |
|---|---|---|---|---|---|---|---|---|---|
| W4 | `Fake` (V4) | 14.22 | 19450 | −0.25 | −47.06 | 2.86 | 0.32 | 17.48 | `Suspicious` `AmbiguousCutoff` (`EdgeStep`) |
| W5 | `Suspicious` `AmbiguousCutoff` (V6) | 9.88 | 20150 | −2.51 | −32.64 | 17.29 | 1.02 | 22.35 | unchanged (V4 needs `D ≥ 10`) |
| W6 | `Suspicious` `HighBandGaps` (V5) | 7.39 | 16550 | 5.65 | −51.13 | 2.37 | −0.24 | 26.72 | unchanged (Q15) |

W1–W3 have their deepest step above the lossy range (`fs` ≥ 20450 Hz), so `EdgeStep` is false for them and their v5 verdicts (`Suspicious`, `HighBandGaps`) do not change.

**Neither measurement works alone.** Below an MP3 or AAC low-pass the quantiser leaves holes, so the per-frame median is low on both sides of the cutoff and many genuine transcodes have `D_Q` ≤ 0; and some transcodes have a large `R`. On W4's album (91 transcodes, seven `calibration.v2.toml` settings) only the conjunction separated W4 and W5 from the 78 transcodes with a low-pass.

**The conjunction alone fails on the corpus.** With the cache of the v4 run (12,825 transcodes, all decoded to 16 bits), `EdgeStep` = `InRange(fs)` ∧ `D_Q < 3` ∧ `R ≥ 12` (the proposal) releases W4 but takes `Fake` from 24 cbr192 transcodes (1.7 pp) and 17 aac128 ones (1.2 pp), against a gate of 0.5 pp (E6). No point of `x` ∈ {2, 3, 5} × `y` ∈ {12, …, 15} passes. The released transcodes have their step far deeper than W4's: in `tune`, 33 of the 43 detectable V4-only transcodes of the protected settings with `D_Q < 3` and `R ≥ 12` have `D` ≥ 17 dB (up to 44.5 dB). A content edge like W4's is a few frames of a narrow sound and raises `M` by tens of dB at most a few bands; a lossy low-pass removes everything. Hence the third condition, a cap on `D` (`D < edge.max_step_db`).

**Choice of the values on `tune`, check on `holdout`.** The rows carry the split of the v4 run (W4 and the 15 confirmed fakes are all in `tune`). Undetectable transcodes (evaluation.md section 2) are left out. On `tune`, with any cap from 15 to 20 dB the worst protected-setting loss is 0.43 pp at `y` = 12, 0.32 pp at `y` = 13–14 and 0.21 pp at `y` = 16; at `y` = 15 it is 0.21 pp with a cap of 15–16 dB and 0.32 pp with 17–20 dB. Without a cap it is 0.53–1.50 pp at `y` = 12–16; at `y` = 16.5–17 dB (within 1 dB of W4's `R`) it is 0.43 pp on `tune` but 1.24 pp on `holdout` (aac128), so a larger `y` is no substitute for the cap. `x` from 2 to 5 changes it by at most 0.01 pp. The rule fixed before looking at `holdout`: the worst protected setting on `tune` at most 0.25 pp (half the gate), then the largest smallest margin of W4 over the three conditions, over the grid `x` ∈ {2, 3, 4, 5}, `y` ∈ {10, 11, …, 16} and cap ∈ {15, 16, 17, 18, 20, none} dB. That gives `x` = 3, `y` = 15, cap 16 dB (W4's margins: `D_Q` 2.68 dB, `R` 2.48 dB, `D` 1.78 dB; the cap sits in a gap of the `tune` releases, none of the protected settings between `D` = 14.8 and 17.0 dB). The choice is a point on a plateau, not a unique optimum: on a 0.25 dB grid the same rule gives (3, 14.75, 16.75) with the same losses. Lost `Fake` per setting, detectable transcodes, in percentage points (count):

| split | cbr128 | cbr192 | cbr256 | cbr320 | v2 | aac128 | opus128 | v0 | aac256 |
|---|---|---|---|---|---|---|---|---|---|
| `tune` | 0 | 0.21 (2) | 0.21 (2) | 0.11 (1) | 0 | 0.11 (1) | 0 | 0.33 (3) | 0.27 (2) |
| `holdout` | 0 | **0.41 (2)** | 0 | 0 | 0 | 0 | 0 | 0.21 (1) | 0 |
| `all` | 0 | 0.28 (4) | 0.14 (2) | 0.07 (1) | 0 | 0.07 (1) | 0 | 0.29 (4) | 0.18 (2) |

Detectable transcodes per protected setting: 897–938 in `tune`, 464–484 in `holdout`. With these values: 15 of 15 confirmed fakes `Fake` and none `EdgeStep`; `primary` 1 → 0 `Fake` (W4), `clean` 0, `reviewed` 0, `reference` 2 → 2; no verdict change other than `Fake → Suspicious`. The proposal's (3, 12) with a cap of 18 dB (the first candidate, seen on the whole corpus) gives 0.43 pp on `tune` (cbr256) and 0.41 pp on `holdout`; it was not chosen because of its `tune` figure.

**Bit depth.** The whole cache is at 16 bits (`decode_args` `s16`) and stereo, but a fake lossless file is often written at 24 bits. 773 transcodes of the corpus were made again with the `calibration.v2.toml` arguments and decoded both to 16 and to 24 bits: every detectable V4-only transcode with `D_Q < 6` and `D < 25` at 16 bits (390), 30 random V4-only ones per protected setting, and an earlier random sample. Above a low-pass a 24-bit decode keeps the decoder's own residue, which can lie far below the 16-bit rounding floor (absolute `F_top` at 24 bits: 5th percentile −119 dB, against a lowest −104.23 dB for any 16-bit setting of the cache, which matches the rounding noise of two channels averaged, `10·log10(2⁻³⁰/12) − 3` = −104.11 dB). `R` then grows (change at the same `fs`: median +0.11 dB, 95th percentile +11.0 dB, maximum +28.5 dB), and so can `EdgeStep`: without a correction, 4 transcodes (3 cbr128, 1 cbr320) are `EdgeStep` only at 24 bits, all because `fs` moved (for example `D` 25.0 → 10.7 dB and `R` 0.6 → 19.3 dB). Never taking `F_top` below −104 dB (section 2) gives them `R` = 4.8–12.4 dB and `Fake`, and leaves no 24-bit-only release: with it the clamped `R` is within −5.9 dB (5th percentile) and +3.1 dB (maximum) of the 16-bit `R`, and every 24-bit loss is a transcode already lost at 16 bits (cbr192 2, cbr256 1, cbr320 1, aac128 1; 0 for cbr128, v2 and opus128). Raising the floor to −101 or −98 dB changes nothing on the protected settings at 24 bits, except that −98 dB removes one cbr192 loss. Applied to the 16-bit cache, the −104 dB floor moves `F_top` by at most 0.23 dB on 319 rows and changes no `EdgeStep` (212 → 212) and no verdict. W4 is unaffected (padded to 24 bits it has the same `F_top`, −80.0 dB, and is still released). On the 154 genuine 24-bit corpus tracks the floor changes nothing (9 `EdgeStep` with and without it, at −104, −101 or −98 dB; none `Fake`).

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

Levels are relative to `L_ref` = median of `M(b)` over the bands whose centre lies in `[ref_band_low_hz, ref_band_high_hz]`. Below, `M` means `M − L_ref` unless a row says otherwise. `top` is the last band whose centre is at or below `nyquist − nyquist_guard_hz`. **`InRange(f)`** means `lossy_min_cutoff_hz ≤ f < fake_max_cutoff_hz` (the lossy range); `InRange` of an undefined `fs` is false.

| Measurement | Definition |
|---|---|
| Noise floor `F` (reported, not used to decide) | the `noise_floor_percentile` percentile of `M` over the bands with centre in `[ref_band_low_hz, nyquist − nyquist_guard_hz]` |
| High-frequency content `H` | median of `M` over the bands with centre in `[hf_band_low_hz, hf_band_high_hz]` |
| Content extent `fe` (only used for `GradualRolloff` and as the reported cutoff when there is no step) | scanning down from `top`, the lower edge of the first band of the first run of bands with `M > −content_drop_db` at least `content_min_width_hz` wide. If the run starts at `top`, `fe = nyquist` |
| Step `d(i)` at band `i` (mean spectrum) | with `w = step_window_bands`: `below = mean(M[i−w … i−1])`, `above = mean(M[i+1 … i+w])`, `slope = max(0, mean(M[i−2w … i−w−1]) − below)`; `d(i) = below − above − slope`. Band `i` itself is the transition band and is left out of both windows. `slope` discounts a roll-off that was already under way |
| Rebound | band `i` is **not** a candidate if any band from `i+w+1` up to `top` has `M > above + step_rebound_db`: content comes back higher up, so the drop is not where the content ends |
| Cutoff step `D` at `fs` | over the candidate bands `i` with `c(i) ∈ [step_search_low_hz, nyquist − step_search_top_margin_hz]`, `D` = the largest `d(i)` and `fs = c(i)` for that band (the lowest such band on ties). If there is no candidate or the largest `d(i)` is ≤ 0: `D = 0`, `fs` undefined. So whenever `fs` is defined, `D > 0` |
| Gap step `G` at `gap_edge_hz` (median spectrum) | with `e = gap_edge_hz`, `g = gap_band_hz` and `q(lo, hi)` = median of `Q` over the bands with centre in `[lo, hi)`: `G = (q(e−g, e) − q(e, e+g)) − max(0, q(e−2g, e−g) − q(e−g, e))`. Taking the median across bands stops a single tone from counting as a step. If `nyquist < e + g`, `G = 0` |
| **Upper window** (v3) | the bands with centre in `[full_band.low_hz, full_band.high_hz)` and at or below `top`. With v3/v4 values at 44.1 kHz: the 3 bands centred at 20750, 20850 and 20950 Hz. If the params have no `[full_band]` section, or the window holds no band (for example at 32 kHz, Nyquist 16 kHz), `U` and `B` are **undefined** |
| **Upper level `U`** (v3) | median of `M` (relative to `L_ref`, as everywhere in this table) over the upper window |
| **Upper burstiness `B`** (v3) | median over the upper window of `M(b) − Q(b)`, both in absolute dB (the difference does not depend on `L_ref`). A stationary floor (dither, rounding noise) gives under 0.5 dB with 100 Hz bands at N = 4096; intermittent content gives several dB (≈ 14 dB for the −20 dB, 75 % gate of fixtures.md; ≈ 2.4 dB for the −6 dB gate of `sfb21_gaps_weak`) |
| **Lossy step `LossyStep`** (v4) | the `[full_band]` section has `max_lossy_step_db`, `fs` is defined, `InRange(fs)` and `D ≥ full_band.max_lossy_step_db`. Otherwise `false`; in particular always `false` for a params file whose `[full_band]` section has no `max_lossy_step_db` (v3 files) |
| **`FullBand`** (v4) | the params have a `[full_band]` section, `U` and `B` are defined, `U ≥ full_band.min_level_db`, `B ≥ full_band.min_burst_db` **and not `LossyStep`**. Otherwise `false`. It is evaluated by the verdict step from `U`, `B`, `D`, `fs` and the params, so without a `[full_band]` section it is `false` whatever values it is given, and without `max_lossy_step_db` it is the `FullBand` of v3 |
| **Median step `D_Q`** (v6) | defined when `fs` is defined and the params have an `[edge]` section. With `i` the band whose centre is `fs` and `w = step_window_bands`: `d(i)` computed on the median spectrum `Q` instead of `M`, at the same band `i`: `below_Q − above_Q − max(0, mean(Q[i−2w … i−w−1]) − below_Q)`, with `below_Q = mean(Q[i−w … i−1])` and `above_Q = mean(Q[i+1 … i+w])`. `L_ref` cancels in the differences. There is no candidate search and no rebound test: it is evaluated only at the `fs` chosen on `M` |
| **Top floor `F_top`** (v6) | defined when `D_Q` is and the **floor window** (the bands with centre in `[edge.floor_low_hz, edge.floor_high_hz)` and at or below `top`) holds at least one band; undefined otherwise (for example at 32 kHz, Nyquist 16 kHz). `F_top` = the median of `M` over the floor window, relative to `L_ref`; then, **whatever the bit depth of the file**, `F_top = max(F_top, edge.floor_min_db − L_ref)`, where `L_ref` is, as everywhere, a level in the absolute dB of section 1 (power of samples with full scale 1.0), so `edge.floor_min_db − L_ref` is that absolute floor relative to `L_ref`. With v6 values at 44.1 kHz the window is the 11 bands centred 20950 … 21950 Hz |
| **Post-step residual `R`** (v6) | `above(i) − F_top`, where `above(i) = mean(M[i+1 … i+w])` is the `above` of `d(i)` at `fs` (relative to `L_ref`). Defined exactly when `F_top` is |
| **`EdgeStep`** (v6) | the params have an `[edge]` section, `fs` is defined, `InRange(fs)`, `D_Q` and `R` are defined, `D_Q < edge.max_median_step_db`, `R ≥ edge.min_residual_db` **and** `D < edge.max_step_db`. Otherwise `false`; always `false` for a params file without `[edge]` (v2–v5 files). Like `FullBand`, it is evaluated by the verdict step from `D_Q`, `R`, `D`, `fs` and the params, so without `[edge]` it is `false` whatever values it is given |

`gap_edge_hz` is an absolute frequency, not relative to Nyquist: MP3 encodes at 44.1 and 48 kHz both start sfb21 at about 16 kHz, and a hi-res file made from an MP3 inherits that edge. The upper window is absolute for the same reason: a lossy origin keeps its low-pass at or below 20.5 kHz whatever the sample rate.

**`LossyStep` uses the cutoff step `D` at `fs`**, the deepest step of the search range, not every step. A track whose deepest step lies above the lossy range (FP1–FP3: `fs` = 20450, 21450, 21250 Hz) keeps the guard even if a smaller step exists inside the range. Since `D > 0` whenever `fs` is defined, `max_lossy_step_db = 0` would mean "any cutoff step in the lossy range removes the guard".

**Medians** over an even number of values take the upper middle value (nearest-index percentile, rank `round(½ · (n − 1))` rounding halves up, as the v2 code already does): with 2 values, the larger; with 4, the third smallest. It matters for the upper window whenever a params file gives it an even number of bands: `full_band.low_hz = 20600` (4 bands) and `20800` (2 bands) in calibrate-grid.v2.toml.

**Headroom.** FP1's highest content band (centre 20950 Hz, `fe` = 20900 Hz) is the last band of the window, so `full_band.high_hz` cannot be raised for FP1 without adding bands where it has no content. Measured with v3: `U` = −56.64 dB and `B` = 9.53 dB, margins of 13.4 dB and 6.5 dB.

`U` alone would not do: like `fe`, it cannot tell a transcode's floor from music. Its threshold only rules out residues near digital silence (for example in a 24-bit transcode), where `B` would be meaningless. `B` alone would count those residues as content. Neither tells bursty residue above a lossy low-pass from music: that is what `LossyStep` adds.

**Reported cutoff.** `CutoffEstimate { frequency_hz, steepness_db_per_khz, noise_floor_db: F }` is reported for every track that has spectra, whatever the verdict (it is `null` only for `Error` and for a silent track, section 1.6). Unchanged from v2:
- if `D ≥ step_suspicious_db`: `frequency_hz = fs`, `steepness_db_per_khz = D / ((step_window_bands + 1) · band_hz / 1000)` (the distance between the centres of the two windows);
- otherwise: `frequency_hz = fe`, `steepness_db_per_khz = 0`.

`U` and `B` are optional fields of the core's `Measurements` (undefined = absent; v4 adds none). They, `LossyStep` and `FullBand` are not part of `TrackAnalysis` (the contract does not change), but the calibration tool reports them (calibration/evaluation.md sections 4, 5 and 8).

**Edge step (v6).** A lossy low-pass removes everything above it, so right above the step the spectrum is at the floor (`R` near 0), and the step is deep (`D` of 20–45 dB on most transcodes). A content edge like W4's (section "Why v5's lossless rules were replaced") is a short narrow-band sound just below a natural roll-off: it raises `M` but not the per-frame median `Q` (`D_Q` small), the spectrum above it keeps falling for many dB before it reaches the floor (`R` large), and its step is moderate. `EdgeStep` needs all three. Each one alone also matches some transcodes: below an MP3 or AAC low-pass the quantiser leaves holes, so `D_Q` ≤ 0 is common; and a few transcodes keep a residue well above the floor right above the low-pass.

- **The floor window is absolute**, like the upper window and `gap_edge_hz`. At 88.2/96 kHz a window near Nyquist would sit far above a lossy residue (an upsampled transcode would get a large `R` and be cleared); at 21–22 kHz it sits in that residue. For a genuine hi-res file with music at 21–22 kHz, `F_top` is higher and `R` smaller, so `EdgeStep` is less likely: the error goes the conservative way. The window always lies above the `above` window of an in-range step (`edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz`, section 4).
- **Bit depth.** At 16 bits the rounding floor (about −104 dB absolute for two channels with independent rounding, −101 dB for one channel or for two identical ones) is the lowest level `M` can reach above a low-pass. At 24 bits the decoder's residue can lie far lower, and `R` grows (by up to 11 dB at the 95th percentile, 28.5 dB at most, for the same audio). The floor `floor_min_db` makes `R` what it would be if the file had been written at 16 bits; it only ever lowers `R`, so it can only keep a `Fake`. **It does not read the bit depth:** at 16 bits `M` practically never goes below it (the lowest cached value is −104.23 dB; the floor moves `F_top` by at most 0.23 dB and changes no verdict of the evidence), so applying it always is equivalent on every measured file, and the measurement needs only the spectra.
- **`floor_min_db` is the only absolute level in the rules.** Every other level is relative to `L_ref`. The absolute scale is the one section 1 fixes (power per FFT bin `|X|² / Σw²` of samples decoded to `f32` with full scale 1.0); for white noise of variance `σ²` it gives `10·log10(σ²)` in every bin, so 16-bit rounding (`σ² = 2⁻³⁰/12`) gives −101.1 dB, and −104.1 dB after averaging two channels with independent rounding.
- **`EdgeStep` only acts on V4** (section 3). It does not change `D`, `fs`, `LossyStep`, `FullBand` or the reported cutoff.
- `D_Q`, `F_top`, `R` and `EdgeStep` are not part of `TrackAnalysis` (no contract change). The calibration tool reports them (calibration/evaluation.md sections 4, 5 and 8).

## 3. Verdict (RF-04)

The rules are evaluated in order; the first match wins. The table is the one of v3 (the guard in bold); v4 changes only the definition of `FullBand` (section 2), and v6 adds "and not `EdgeStep`" to V4. One table serves v2 to v6 params files.

| # | Condition | Verdict | Reasons |
|---|---|---|---|
| V1 | Decoding failed or too short | `Error` | — (the `TrackError` explains it) |
| V2 | `H < low_hf_threshold_db` (little treble: a cappella, soft instrumental) | `Suspicious` | `LowHighFrequencyContent` (never `Fake` only because of the spectrum shape, 2.5 of the planning doc) |
| V3 | `G ≥ gap_fake_db` **and not `FullBand`** | `Fake` | — |
| V4 | `D ≥ step_fake_db` and `InRange(fs)` **and not `FullBand`** **and not `EdgeStep`** (v6) | `Fake` | — |
| V5 | `G ≥ gap_suspicious_db` | `Suspicious` | `HighBandGaps` |
| V6 | `D ≥ step_suspicious_db` and `InRange(fs)` | `Suspicious` | `AmbiguousCutoff` |
| V7 | `D ≥ step_low_suspicious_db` and `fs < lossy_min_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V8 | `D ≥ step_high_suspicious_db` and `fake_max_cutoff_hz ≤ fs < ambiguous_max_cutoff_hz` | `Suspicious` | `AmbiguousCutoff` |
| V9 | `D < step_suspicious_db` and `fe < gradual_min_cutoff_hz` | `Suspicious` | `GradualRolloff` |
| V10 | Otherwise | `Real` | — |

Consequences of the order (no new rule is needed; they rely on the load-time checks of section 4):
- A track that V3 would have made `Fake` but is `FullBand` falls through. `gap_suspicious_db ≤ gap_fake_db`, so V5 makes it `Suspicious` with `HighBandGaps` (FP1–FP3).
- **V4 is never guarded with v4 parameters.** V4 needs `InRange(fs)` and `D ≥ step_fake_db`, and `max_lossy_step_db ≤ step_fake_db` (section 4), so `LossyStep` holds and the track is not `FullBand`. The spec keeps "and not `FullBand`" in V4 formally rather than removing it, because it still acts with a v3 params file (no `max_lossy_step_db`), which must reproduce the v3 verdicts; with a v4 file the condition is always true. FP4 (W4) is therefore `Fake` by V4 with v4 and v5 files, as in v2; with v6 it is `EdgeStep` (below).
- With v3 parameters, a track that V4 would have made `Fake` but is `FullBand` falls through to V6 (`AmbiguousCutoff`), unless V5 matches first, exactly as in v3.
- **Relation between versions.** For the same spectra and window, `FullBand` of v4 implies `FullBand` of v3, while v2 is never `FullBand`. The guard only skips V3 and V4, so every v4 verdict equals the track's v2 verdict or its v3 verdict:
  - against v2, v4 only turns `Fake` into `Suspicious`, and only on tracks that v3 also turned; no verdict becomes `Real` or `Error`, and no `Real` or `Suspicious` verdict of v2 changes, so the caught rate (`Fake` + `Suspicious`) of every setting is the same as with v2;
  - against v3, v4 only turns `Suspicious` back into `Fake`: the tracks that v3 guarded although their deepest step lies in the lossy range with `D ≥ max_lossy_step_db`.
- The upper window always lies above the lossy range (`fake_max_cutoff_hz < full_band.low_hz`, section 4), so a step in the lossy range is always below the window.
- **`EdgeStep` (v6).** A track that V4 would have made `Fake` but is `EdgeStep` falls through. V5 matches if `G ≥ gap_suspicious_db` (`HighBandGaps`); otherwise V6 does, because `D ≥ step_fake_db ≥ step_suspicious_db` and `InRange(fs)`: `Suspicious` with `AmbiguousCutoff`. It can never become `Real` (W4).
- `EdgeStep` does not touch V3: a track with `G ≥ gap_fake_db` that is not `FullBand` is still `Fake`. It does not touch `LossyStep` either, so an `EdgeStep` track still has no full-band guard.
- **Relation to v5.** For the same spectra, `EdgeStep` only skips V4. Every v6 verdict therefore equals the v5 verdict, except `Fake → Suspicious` on V4 tracks; the caught rate of every setting is the same as with v5, and the reported cutoff does not change. Lossy files are not affected (lossy-source.md rule L3 does not read `EdgeStep`, Q16).

Higher sample rates (88.2/96/192 kHz) use the same absolute frequencies: a lossy origin still shows its step at or below ~20.3 kHz, its gaps at 16 kHz and no music in the upper window.

## 4. Invariants and parameter validation

- Deterministic: the same file and parameters always give the same result.
- Read-only: the file is opened for reading only (RNF-05).
- `algorithm_version` in every `TrackAnalysis` equals the `algorithm_version` of the parameters used (8 with the defaults since detection v8, lossy-source.md section 12, RF-14/RF-15; it was 7 with detection v7, lossy-source.md section 11, RF-16, 6 with detection v6, 5 with detection v5, lossy-source.md, and 4 before).
- The code's default parameters equal `detection-params.v8.toml` (a test checks it), whose sections and values other than `lossy.cutoffs.opus` are those of `detection-params.v7.toml`, whose sections other than `[lossy.streaming]` are those of `detection-params.v6.toml`, whose sections other than `[edge]` are those of `detection-params.v5.toml`. `[lossy.streaming]` and `lossy.cutoffs.opus` are read by lossy files only, so every lossless result with v8 equals the v7 and v6 results.
- **v2 and v3 stay loadable.** `[full_band]` is optional, and `max_lossy_step_db` is optional inside it. A params file without `[full_band]` (such as `detection-params.v2.toml`) loads with `FullBand` always `false` and gives exactly the v2 verdicts, reasons and cutoffs. A params file whose `[full_band]` has no `max_lossy_step_db` (such as `detection-params.v3.toml`) loads with `LossyStep` always `false` and gives exactly the v3 verdicts, reasons and cutoffs. Tests check both on every fixture. This lets the calibration tool compare v2, v3 and v4 in one run (calibration/evaluation.md section 9).
- **v4 and v5 stay loadable (v6).** `[edge]` is optional. A params file without it (such as `detection-params.v5.toml`) loads with `EdgeStep` always `false` and gives exactly its own verdicts, reasons and cutoffs; `D_Q`, `F_top` and `R` are undefined. Tests check it on every fixture, so the calibration tool compares v5 and v6 in one run.
- **`[edge]` (v6).** Shape: if present, its six fields `floor_low_hz`, `floor_high_hz`, `floor_min_db`, `max_median_step_db`, `min_residual_db` and `max_step_db` are all required, and unknown fields are rejected. Values, checked by `DetectionParams::validate` after the checks above, in this order, each named as written: `edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz` (20600 Hz with v6 values: the floor window never overlaps the `above` window of an in-range step); `edge.floor_low_hz < edge.floor_high_hz`; `edge.floor_min_db < 0`; `edge.max_median_step_db is finite`; `edge.min_residual_db > 0`; `edge.max_step_db > step_fake_db` (otherwise `EdgeStep` could never act on V4, which needs `D ≥ step_fake_db`: a file that wants no edge step omits the section). A comparison with a NaN operand does not hold, so NaN in any of these fields is rejected.
- **Who checks a params file.** The core has no file loader: `DetectionParams` is deserialised with serde by whoever reads a file (today the calibration tool and tests; the app uses `DetectionParams::default()`). Two layers:
  1. **Shape** (serde, at deserialisation): every section rejects unknown fields; `[full_band]` may be absent, but if present its four fields `low_hz`, `high_hz`, `min_level_db`, `min_burst_db` are all required, and `max_lossy_step_db` is optional.
  2. **Values:** a pure function `DetectionParams::validate(&self) -> Result<(), ParamsError>` in `fakeflac-core` (no I/O; `ParamsError` names the inequality that does not hold). It checks `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db`; when `[full_band]` is present, `fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz`; and (v4) when `max_lossy_step_db` is present, `0 ≤ full_band.max_lossy_step_db ≤ step_fake_db`, named as `full_band.max_lossy_step_db ≥ 0` and `full_band.max_lossy_step_db ≤ step_fake_db`. A comparison with a NaN operand does not hold, so NaN in any of these fields is rejected. The section 3 consequences depend on these checks. Every loader calls it after deserialising and refuses an invalid file (the calibration tool: exit 2, evaluation.md section 9). A test checks that the defaults are valid. When `DetectionParams` reaches the settings file (H2), the settings loader calls it too.
- **Why `0 ≤ max_lossy_step_db`, not `0 <`.** 0 is a meaningful value: since `D > 0` whenever `fs` is defined, it means "any cutoff step in the lossy range removes the guard" (the analysis behind v4 simulated it). A negative value would behave exactly like 0, so it can only be a mistake and is rejected. **Why `≤ step_fake_db`.** It is what makes "V4 is never guarded" (section 3) an invariant instead of a property of today's values. A file that wants the v3 guard on V4 omits `max_lossy_step_db`.
- `detection-params.v1.toml` stays in the repository as the record of version 1. The code does not have to load it.
- `[input]` and `[fft]` are the same as in v2 and v3, so v4 needs no new extraction: `fakeflac-audio::EXTRACTION_VERSION` does not change and cached spectra are reused. The app's result cache is invalidated by the new `algorithm_version` (ADR-0008). The same holds for v6: `D_Q`, `F_top` and `R` come from the existing `M` and `Q` only (no bit depth, no new input to the measurement).
- `algorithm_version` numbers are allocated by the files in `specs/detection/`. A parameter file produced elsewhere (a `/calibrate` proposal, a test candidate) takes the next free number, never one already used here (calibration/evaluation.md section 7). With `detection-params.v4.toml` in the repository, the next free number was 5; detection v5 (`detection-params.v5.toml`, lossy-source.md section 1) took it, and detection v6 (`detection-params.v6.toml`, ADR-0042) took 6; detection v7 (`detection-params.v7.toml`, lossy-source.md section 11, RF-16) took 7, and detection v8 (`detection-params.v8.toml`, lossy-source.md section 12, Opus) took 8, so the next free number is **9**.

## 5. Test signals

Tests use synthetic signals only (`specs/detection/fixtures.md`). The fixtures `lp16k_dither`, `lp20k_floor` and `sfb21_gaps` reproduce the real failure of v1 (v1 returns `Real` for all three). The full-band pairs separate a stationary floor from intermittent content above the lossy range: `sfb21_gaps_lp20k5` (`Fake`) against `sfb21_gaps_aa21k` (`Suspicious`, `FullBand`), and `shelf19k5` (`Fake`) against `shelf19k5_bursts`. With v4, `sfb21_gaps` and `shelf19k5_bursts` pass the `U` and `B` thresholds but have their deepest step in the lossy range, so they exercise `LossyStep` and are `Fake` (fixtures.md gives the derivation and what the test stage must confirm). Rule-level scenarios with the FP1–FP4 measurements, the `LossyStep` boundaries and the parameter checks: `specs/features/detection-full-band.feature`. Acceptance against the real corpus runs locally with `just accept` (section 7). Tests derived from v3 that v4 changes are listed in ADR-0039 ("Consequences").

**v6.** Three edge-step fixtures (fixtures.md, "Edge step"): `edge_bump19k3` (a W4-like content edge: `EdgeStep`, `Suspicious` with v6, `Fake` with v5), `edge_bump19k3_lp19k5` (the same signal behind a low-pass: `Fake`) and `holes_lp19k5` (a low-pass with quantiser holes below it, so `D_Q` and `D` pass but `R` does not: `Fake`; a rule without `R` would clear it). Rule-level scenarios with the W4–W6 measurements, the three boundaries, the 16-bit floor and the `[edge]` checks: `specs/features/detection-edge-step.feature`.

## 6. Known limitations (v4, kept by v5 and v6)

Detection (`Fake`) rates per encoder setting with v2 in the v3 run (detectable transcodes, split `all`): cbr128 99.6 %, cbr192 99.9 %, cbr256 98.2 %, cbr320 92.1 %, v0 16.9 %, v2 99.6 %, aac128 99.1 %, **aac256 0.3 %**, opus128 97.3 %. These figures are context only: acceptance compares against the v2 column of its own run (section 7).

**v6.** The first bullet below is what v6 addresses: W4 becomes `Suspicious` through `EdgeStep` (section 2). Its own limitations:

- **The evidence is one false positive.** The rule is physically motivated (a low-pass leaves the floor right above it; a content edge does not), but W4 is the only `Fake` it was built to clear. Its margins are 2.68 dB on `D_Q`, 2.48 dB on `R` and 1.78 dB on `D`; the values were chosen on `tune`, and on `holdout` the worst protected setting (cbr192, 0.41 pp) is one track away from the 0.5 pp gate (Q19).
- **A real W4-like edge with a deep step stays `Fake`.** The cap `max_step_db` = 16 dB is what keeps the deep low-pass steps of transcodes `Fake`. A genuine track whose bursty band raises `M` by 16 dB or more just below a natural roll-off is still `Fake` by V4.
- **A lossy source whose low-pass sits just above a W4-like bursty band** could get `EdgeStep`; it would be `Suspicious`, not `Real`. Those are the transcodes the corpus analysis loses (0.07–0.41 pp per protected setting and split).
- **Mono and dual-mono files above 16 bits.** The floor is the one of two averaged channels with independent rounding (−104 dB); a mono file, or a stereo file with identical channels (L = R, same rounding), has a 16-bit floor 3 dB higher, so its floored `R` can be up to 3 dB larger than at 16 bits. The corpus has no such file (every cached row is stereo), so this is not measured.
- **Dithered or noise-shaped 16-bit files** have a higher floor, so a smaller `R`: the error goes the conservative way (theory, not measured).
- **The 64 × 4-frame sampling** (ADR-0010) decides how often a short sound is caught: W4's bump comes from a few frames. Shifting the 64 positions by a fraction of their spacing (same audio) moves `D`, `D_Q` and `R` of the transcodes near the conjunction by σ ≈ 3.8, 2.7 and 3.1 dB, more than W4's margins. W4 itself, over 20 shifts, is `Fake` with v5 in 9 and with v6 in none; the 14 transcodes released in the cache are released in 18 of 126 shifted analyses. The verdict on W4 is robust; the exact set of lost transcodes is not.
- W5 and W6 stay `Suspicious` (Q12, Q15).

- **FP4 (W4) stays `Fake` with v4 and v5 (Q7, accepted).** Its deepest step (14.22 dB at 19.45 kHz) lies in the lossy range, where a cbr256 transcode (low-pass 19.5 kHz) puts its own; 470 detectable transcodes of the v3 run with a lossy-range step of at least 10 dB have `U` and `B` at or above W4's. A separate guard on V4 (`U` ≥ −50 dB and `B` ≥ 2.5 dB on top of v4) would clear W4 but was simulated to cost A2 (13/15) and up to 19.5 percentage points of `Fake` rate (opus128 77.8 %, aac128 87.9 %, v2 93.3 %, cbr256 92.7 %, cbr320 87.4 %), with a margin of 0.36 dB on W4's `B`: fitting one track, rejected. More generally, a lossless track whose deepest step lies in the lossy range with `D ≥ step_fake_db` is `Fake`, as in v2, whatever its upper window holds. Clearing it needs a new measurement, not a threshold: for example the continuity of the spectrum across the step (the level just above `fs` against the upper-window level), or the per-frame statistics below.
- **Genuine tracks near `step_fake_db` are back to v2's margins.** v3 kept some W4-type genuine tracks away from V4 only through a guard that also fired on 28–95 % of transcodes. With v4 they are as close as with v2: in the v3 run, 1 `primary` track at 0.12 dB and 1 `clean` track at 0.27 dB below the `Fake` side's threshold (both with `fs` = 20.15 kHz, `D` ≈ 9.8 dB), and 1 `clean` track at 0.51 dB. This is the main residual risk of v4; it is the risk of v2, not a new one.
- **Genuine full-band tracks with a small lossy-range step lose the guard.** At `max_lossy_step_db` = 3 dB, 14.1 % of the `primary`, 17.8 % of the `clean` and 22.7 % of the `reference` tracks that are `FullBand` with v3 are not with v4. They are exposed only to V3 (`G ≥ gap_fake_db`); in the v3 run none reaches it (the tracks with a lossy-range step and `G ≥ 4` are 1 `primary` and 4 `clean`, all with `G` < 6, so `Suspicious` by V5 as with v2).
- **W1's protection rests on 150 Hz.** W1's deepest step (`fs` = 20450 Hz) lies 1.5 bands above `fake_max_cutoff_hz` (W2: 1150 Hz, W3: 950 Hz). That boundary is v2's own, not a value fitted for v4, but a lossless track like W1 whose mastering low-pass sits 200 Hz lower would be `Fake`.
- **LAME V0 (16.9 % detected by v2).** The calibration config records V0 without a low-pass (`lowpass_hz = 22050`, calibration.v1.toml; Q5), and VBR codes sfb21, so a V0 transcode usually has neither a cutoff step nor a 16 kHz gap. Its band spectra look like those of a full-band lossless file. Any rule on `G`, `D`, `fs`, `fe`, `U` or `B` that caught it would also catch lossless tracks like FP1–FP3, which are what the guard protects. The V0 transcodes that v2 catches are caught through `G`; those without a lossy-range step that carry intermittent content in the upper window are guarded by v4 and become `Suspicious` (simulated: 138 of the 234 V0 transcodes `Fake` with v2). That is the intended behaviour: with these measurements they are indistinguishable from FP1–FP3. They stay caught, so the caught rate that A5 gates does not change; the drop of V0's `Fake` rate is reported by A5 as information.
- **AAC 256, ffmpeg's native encoder (0.3 % detected).** It cuts at about 20 kHz and leaves no sfb21 gap (`G` ≈ 0), so only V4 could catch it. Doing so would mean lowering `step_fake_db` or moving `fake_max_cutoff_hz` near 20 kHz. No v2 threshold reaches these transcodes (docs/agents/progress.md, "Detection v3 findings"). FP4 shows that a step near 20 kHz is not lossy evidence on its own. Steps of `step_suspicious_db` to `step_fake_db` in the lossy range are already `Suspicious` (V6).
- **Lossy sources without a low-pass** (V0 above; MP3 encoders run with the low-pass disabled) are `Suspicious` at best when they show a 16 kHz gap and no lossy-range step, for the same reason.
- **Lossless tracks without content above 20.7 kHz** (masters band-limited at or below 20.5 kHz) with a gap or a lossy-range step are still `Fake`, as in v2.

Catching V0 and AAC 256 needs new measurements, not new thresholds; clearing W4 did too, and v6 uses the continuity of the spectrum across the cutoff step for it (`D_Q`, `R`). Candidates recorded for a later version, not specified here: per-frame statistics beyond the median in the 16–20 kHz bands (how often a band falls far below its median: the holes a VBR or AAC quantiser leaves), and codec-frame structure in time (MP3 granules of 576 samples, AAC frames of 1024). They need a new extraction (an `EXTRACTION_VERSION` bump) and fit better with the codec-domain analysis of RF-15 (H2).

## 7. Acceptance criteria (v4)

Measured locally on the real corpus with `just accept --params specs/detection/detection-params.v2.toml --params specs/detection/detection-params.v4.toml` (v2 is the baseline), split `all` unless a row says otherwise. A third file, `detection-params.v3.toml`, may be added for reference; it is not judged. Every comparison is against the v2 column **of the same run**, not against the figures of sections 6 and 7.1. The aggregate figures from `summary.md` go into the implementation PR (ADR-0022).

"Protected settings" are the seven settings with a low-pass that v2 detects: `cbr128`, `cbr192`, `cbr256`, `cbr320`, `v2`, `aac128` and `opus128`. `v0` (no low-pass) and `aac256` (v2 detects 0.3 %) are reported, not gated, in A8.

| # | Criterion | Pass |
|---|---|---|
| A1 | FP1–FP4 (watch list W1–W4) | W1, W2 and W3 are not `Fake` (the gate); with v4 this means `Suspicious` with `HighBandGaps`, which is reported. W4 is expected `Fake` (known limitation, section 6, Q7); its verdict, `U`, `B`, `D` and `fs` are reported and do not decide A1 |
| A2 | Confirmed fakes (`confirmed_fake` row) | 15 of 15 `Fake` with v4, as with v2 |
| A3 | Negatives | `clean`: 0 `Fake` (as v2). `primary`: at most 1 `Fake`, and if there is one it is W4 (v2: 4, exactly W1–W4). Target T1 therefore reports `FAIL` with count 1: accepted (W4). `reference`: reported, not a gate |
| A4 | No track becomes `Real` or leaves `Real` | In the verdict-change table v2 × v4, every off-diagonal count other than `Fake → Suspicious` is 0 (it holds by construction, section 3) |
| A5 | Caught rate per encoder setting | **Gate:** for every setting, the v4 caught rate (`Fake` + `Suspicious` over detectable transcodes) is at least the v2 caught rate of the same run (by construction it is equal; the run confirms it). **Information, not a gate:** per setting, the number of positives that change `Fake → Suspicious` and the resulting drop of the `Fake` (detection) rate, v2 minus v4, in percentage points |
| A6 | Targets | For each of T1, T2 and T3, the v4 count is at most the v2 count of the same run, on split `all` and on `holdout`. The `PASS`/`FAIL` status itself is reported, not judged: v2 already fails T2 and T3 on split `all` (Q9), and the guard can neither create nor remove a `Real`, so T2 and T3 equal v2's by construction; T1 is expected to fall from 4 to 1 |
| A7 | Synthetic | Every scenario of `detection-lossless.feature`, `detection-full-band.feature` and the `@RF-04` scenarios of `calibration.feature` pass (`just test`) |
| A8 | Full-band guard on transcodes | **Gate:** for each protected setting, the `FullBand` count of the detectable transcodes (per-setting table, evaluation.md section 4) is at most **5 %** of the detectable transcodes. **Report:** the same rate for `v0` and `aac256`; per setting the median, 95th percentile and maximum of `B`; `U`, `B` and `FullBand` of W1–W4 |

**Why A8 is a gate now.** The guard firing on transcodes was v3's failure, and on a protected setting a `FullBand` transcode is the only way v4 can lose a `Fake` verdict, so the `FullBand` rate bounds the setting's `Fake`-rate drop. 5 % is five times the highest simulated value (cbr320, 1.0 %) and far below v3's 28–77 % on the same settings. `v0` and `aac256` are not gated: the guard is expected to fire on most of them (section 7.1), and it costs at most their small v2 `Fake` rate, which A5 reports.

If A1 (W1–W3), A2, A3, A5 or A8 fails, the implementer does not tune the parameters: the run's figures (and, locally, the listed tracks with `U`, `B`, `D` and `fs`) go back to G1, where the owner decides between new values (a new params version) and accepting the result. A4 and A6 hold by construction (section 3), so a failure of either is an implementation bug: it is fixed in the `implement` stage and the run is repeated. An A7 failure is a failing test: it is handled like any red test (implementation bug, or a spec or test error reported back to G1).

### 7.1 Predicted results (offline simulation, not results)

These figures are **predictions**. They come from an offline re-implementation of V1–V10 and `FullBand` (a throwaway script outside the repository) applied to the measurements that the v3 run wrote to `tracks.csv` (2 decimals). Validation of the script against the tool's own verdicts: 1 mismatch in 14,265 rows for the v2 params and 1 for the v3 params (plus 1 on `full_band`), all rounding artefacts at a threshold. v4 itself has not been run through the Rust tool; `just accept` decides.

| setting | cbr128 | cbr192 | cbr256 | cbr320 | v0 | v2 | aac128 | aac256 | opus128 |
|---|---|---|---|---|---|---|---|---|---|
| `Fake` % v2 (v3 run) | 99.6 | 99.9 | 98.2 | 92.1 | 16.9 | 99.6 | 99.1 | 0.3 | 97.3 |
| `Fake` % v4 (predicted) | 99.6 | 99.9 | 98.2 | 91.9 | 6.9 | 99.6 | 99.1 | 0.3 | 97.3 |
| `FullBand` v4 (predicted, detectable) | 0/1424 | 1/1422 | 0/1419 | 14/1398 | 1070/1387 | 0/1416 | 0/1424 | 826/1134 | 1/1365 |

| # | Predicted |
|---|---|
| A1 | W1–W3 `Suspicious` (`HighBandGaps`), W4 `Fake` |
| A2 | 15/15 `Fake`; 0 of 15 `FullBand`. The closest confirmed fake has a lossy-range step of `D` = 5.63 dB, 2.63 dB above `max_lossy_step_db` |
| A3 | `clean` 0 `Fake`; `primary` 1 (W4); `reference` 2 (v2: 3, v3: 1). The `reference` track that returns to `Fake` against v3 (unverified truth) has a 17.7 dB step at 20.05 kHz, a cbr320-like profile |
| A4 | Only `Fake → Suspicious` against v2: 160 tracks of the evaluation set (none of them a v3 `Fake`) |
| A5 | Caught rate equal to v2 on every setting. `Fake`-rate drop: v0 −10.0 pp (138 tracks), cbr320 −0.2 pp (2), cbr192 −0.1 pp (1), 0 elsewhere and for the confirmed fakes |
| A6 | T1 1 (v2: 4); T2 4 and T3 8, as v2 |
| A8 | ≤ 1.0 % on every protected setting (cbr320 14 tracks, cbr192 1, opus128 1); v0 77 %, aac256 73 % |

`tune` and `holdout` behave alike in the simulation (v4 `Fake` % on tune / holdout: cbr128 99.6 / 99.6, cbr320 91.4 / 93.0, v2 99.5 / 100.0, opus128 97.2 / 97.4, v0 7.7 / 5.5, matching v2's split figures). W2 is in `holdout` and is `Suspicious`; all confirmed fakes are in `tune`.

### 7.2 Robustness caveats (stated for G1)

- **`max_lossy_step_db` = 3 dB is set by one confirmed fake** (the smallest lossy-range step among the confirmed fakes that v3 guarded: `D` = 5.63 dB). Any value in (0, 5.63] gives 15/15; 3 dB keeps a 2.63 dB margin. A lower value removes the guard from more genuine full-band tracks (share of the v3-`FullBand` tracks that lose it, `primary` / `clean` / `reference`: at 0 dB 15.6 / 26.6 / 30.6 %; at 3 dB 14.1 / 17.8 / 22.7 %; at 5 dB 6.2 / 5.9 / 7.9 %; at 7 dB 4.7 / 1.1 / 1.9 %); those tracks are exposed only to V3 (section 6).
- **W1–W3 margins with v4:** `U` 13.4 / 25.3 / 18.5 dB above −70 dB; `B` 6.5 / 27.6 / 7.8 dB above 3 dB; `fs` 150 / 1150 / 950 Hz above `fake_max_cutoff_hz`. W1's 150 Hz is the thinnest (section 6).
- **v2's thin margins return** for genuine tracks with a lossy-range step near `step_fake_db` (0.12, 0.27 and 0.51 dB, section 6).
- **Small evidence base.** The positive side of the rule rests on 3 false positives from 3 albums, the 7 relevant confirmed fakes (known-truth material from 1–2 albums) and the generated transcodes. The rule is simple and explainable, but few albums support it.
- **The cause of the bursty residue** in the upper window of transcodes (decoder clipping or overshoot) is a hypothesis.
- An alternative with `min_level_db` −60 dB and `min_burst_db` 6 dB (and the same `max_lossy_step_db`) gives the same A1–A3 with slightly better v0 and cbr320 `Fake` rates (8.1 % and 92.1 %), but thinner margins on W1 (3.4 dB on `U`, 3.5 dB on `B`). v4 keeps the v3 values: fewer changed values and wider margins on the side that protects genuine tracks.

## 8. Acceptance criteria (v6)

Measured locally on the real corpus with `just accept --config specs/calibration/calibration.v3.toml --params specs/detection/detection-params.v5.toml --params specs/detection/detection-params.v6.toml` (v5 is the baseline). Every comparison is against the v5 column **of the same run**. The aggregate figures from `summary.md` go into the implementation PR (ADR-0022). Before the run the owner adds W5 and W6 to `confirmed.txt` (Q18).

"Protected settings" are the seven of section 7 (`cbr128`, `cbr192`, `cbr256`, `cbr320`, `v2`, `aac128`, `opus128`) and the four 24-bit variants of `calibration.v3.toml` (`cbr128_s24`, `cbr192_s24`, `cbr256_s24`, `cbr320_s24`; Q21). `v0` and `aac256` are reported, not gated.

| # | Criterion | Pass |
|---|---|---|
| E1 | Watch list | **Gate:** W4 is not `Fake`; W1–W3, W5 and W6 keep their v5 verdicts. Reported: verdict, `D`, `fs`, `D_Q`, `R` and `EdgeStep` of each |
| E2 | Confirmed fakes | 15 of 15 `Fake`, as with v5, and 0 of them `EdgeStep` |
| E3 | Negatives | `clean`: 0 `Fake`. `primary`: 0 `Fake` (v5: 1, W4), so T1 passes. `reference`: reported |
| E4 | Only `Fake → Suspicious` | In the verdict-change table v5 × v6 every off-diagonal count other than `Fake → Suspicious` is 0 (it holds by construction, section 3; a failure is an implementation bug) |
| E5 | Caught rate | Equal to v5 per setting (by construction) |
| E6 | **`Fake`-rate drop per protected setting** (detectable transcodes) | **Gate:** per protected setting, on split `all` the drop is at most **0.5 percentage points**; on `holdout` the lower bound of its 95 % Wilson interval is at most **0.5 percentage points** (Q23). Report: the drop and its interval for every split, the same for `v0` and `aac256`, on `tune` too, and the edge report of evaluation.md section 4 |
| E7 | Synthetic | Every scenario of `detection-edge-step.feature`, `detection-lossless.feature` and `detection-full-band.feature`, and the `@RF-04` scenarios of `calibration.feature`, pass (`just test`) |

If E1, E2, E3 or E6 fails, the parameters are not tuned in the implement stage: the figures (and, locally, the listed tracks with `D`, `fs`, `D_Q` and `R`) go back to G1. E4 and E5 hold by construction, so a failure of either is an implementation bug, fixed in the `implement` stage before the run is repeated. An E7 failure is a failing test. 0.5 pp is a choice: a loss here is a `Fake` that becomes `Suspicious` (still caught), but the target is for `Fake` on a transcode to stay the rule; v4's A8 allowed `FullBand` on up to 5 % of transcodes.

**Why the two splits are gated differently (Q23).** The drop of a setting on a split is `k / n`: `k` detectable transcodes that are `Fake` with v5 and not with v6, out of the `n` detectable transcodes of that setting and split. On `all` (about 1,420 per setting) the point figure is precise enough to gate: one track is 0.07 pp. On `holdout` (about 390 per setting) one track is 0.26 pp, so a point gate of 0.5 pp fails on two lost tracks, a result that 8.2 shows happens often with nothing wrong in the rule. There `holdout` gates only the question it can answer: whether it shows a drop **above** 0.5 pp. The interval is Wilson's at 95 % (`z` = 1.96): lower bound `(p + z²/2n − z·√(p(1−p)/n + z²/4n²)) / (1 + z²/n)` with `p = k / n`. With `n` = 392 it passes up to `k` = 4 (1.02 pp, lower bound 0.40 pp) and fails from `k` = 5 (1.28 pp, lower bound 0.55 pp); the proposal's 1.7 pp regression (7 of 392, lower bound 0.87 pp) still fails. The `all` gate is the strict one; `holdout` guards against a large drop that `tune` hid.

### 8.1 Predicted results (offline analysis, not results)

From the analysis of "Why v5's lossless rules were replaced" (16-bit transcodes of the v4 run's cache; the 24-bit variants have only the 267-transcode sample):

| # | Predicted |
|---|---|
| E1 | W4 `Suspicious` (`AmbiguousCutoff`); W1–W3 `Suspicious` (`HighBandGaps`), W5 `Suspicious` (`AmbiguousCutoff`), W6 `Suspicious` (`HighBandGaps`), as with v5 |
| E2 | 15/15 `Fake`, 0 `EdgeStep` |
| E3 | `clean` 0, `primary` 0, `reference` 2 (as v5) |
| E6 | `all`: at most 0.28 pp (cbr192); `holdout`: at most 0.41 pp (cbr192); `v0` 0.29 pp, `aac256` 0.18 pp on `all`. 24-bit variants: from the 773-pair sample, 0–0.14 pp per setting (cbr192 the highest); a joint estimate for the transcodes the sample does not cover gives about a 21 % chance that one of the four fails |

### 8.2 Statistical caveats (stated for G1)

Measured on 2026-10-24 with an album-level bootstrap (2,000 resamples of the 115 albums that are transcode sources, 81 in `tune` and 34 in `holdout`) and Wilson intervals; aggregates only.

- **The gate is coarse.** With about 480 detectable transcodes per setting in `holdout`, one track is 0.21 pp and the gate allows at most 2. 0/484 has a 95 % Wilson interval of 0–0.79 pp and 3/484 (a fail) 0.21–1.81 pp, so the gate cannot tell 0.1 pp from 1 pp; it catches large regressions such as the proposal's 1.7 pp. cbr192 at the chosen point: `all` 4/1420, 0.11–0.72 pp; `holdout` 2/484, 0.11–1.49 pp.
- **Generalisation.** Resampling albums and drawing a new 30 % `holdout`, a corpus like this one fails E6 on `all` in about 8 % of the draws, on `holdout` in 27 %, and on one of the two in 30 %, with nothing wrong in the rule. The 16-bit figures of this run are not at risk: the run reuses the cache. (After this was written, the library was reorganised and the run of 8.3 was a cold run with a new split: its `all` figures repeat these, its `holdout` figures do not.)
- **Stability of the choice.** On 200 album-level 70/30 splits, the selection rule picks (3, 15, 16) 10 times; with the point fixed, 15 % of the splits fail their own `holdout`. Leaving out one `tune` album changes the selection for 4 of 81 albums. The 14 lost transcodes on `all` come from 10 albums.

### 8.3 Results of the corpus run (2026-10-25)

Run `20261025T010130Z`, the command of section 8, tool commit `d45830b`; aggregates only (ADR-0022). The owner reorganised the library after the v4 run, so this was a **cold run**: cache entries are keyed by path, size and mtime, so the corpus was extracted again and every transcode generated again (generation.md sections 3 and 4). The split was drawn again from the new album ids (inputs.md section 4): `holdout` now has 392 transcode sources instead of 484, and mixes in albums of the old `tune`, where the values of Q19 were chosen. The owner chose, before the run, to judge E6 on it anyway (2026-10-24). 1,425 transcode sources, 13 lossless settings, 14 lossy groups; 0 generation failures.

| # | Result |
|---|---|
| E1 | **Pass.** W4 `Suspicious` (`AmbiguousCutoff`; `D_Q` 0.32, `R` 17.48, `D` 14.22 dB, `EdgeStep`). W5 `Suspicious` (`AmbiguousCutoff`; `D_Q` 1.02, `R` 22.35, `EdgeStep`) and W6 `Suspicious` (`HighBandGaps`; `D_Q` −0.24, `R` 26.72, `EdgeStep`), as with v5. W1–W3 `Suspicious` (`HighBandGaps`), not `EdgeStep`, as with v5. W5's margin to `Fake` grows from 0.12 to 8.51 dB; the smallest `primary` margin is now W6's 0.35 dB |
| E2 | **Pass.** 15/15 `Fake`, 0 `EdgeStep` |
| E3 | **Pass.** `clean` 0 `Fake`, `primary` 0 (T1 passes on `all` and `holdout`), `reference` 2 (as v5). No clean album turns `Fake` |
| E4 | **Pass.** 21 `Fake → Suspicious` (20 transcodes and W4); every other off-diagonal count is 0. Lossy rows: identical to v5 |
| E5 | **Pass.** `Real` and `Fake` + `Suspicious` equal to v5 on every setting |
| E6 `all` | **Pass.** cbr192 4/1422 (0.28 pp), cbr256 2/1419 (0.14 pp), cbr192_s24 2/1422 (0.14 pp); cbr320, aac128, cbr256_s24 and cbr320_s24 one each (0.07 pp); cbr128, v2, opus128 and cbr128_s24 none. Reported: v0 4/1387 (0.29 pp), aac256 2/1134 (0.18 pp). The 16-bit figures are those predicted in 8.1 |
| E6 `holdout` | **Pass with the gate of Q23; fails the original one.** cbr192 and cbr192_s24 2 of at most 392 each (0.51 pp, interval 0.14–1.84 pp); cbr320 and cbr320_s24 1 each (0.26 pp); the other protected settings none. The four 0.51 pp losses are two source tracks, from two albums, at 16 and 24 bits. Reported: v0 2 |
| E6 `tune` | Reported: cbr192 2, cbr256 2, cbr256_s24 1, aac128 1; v0 2, aac256 2; the rest none |
| E7 | **Pass.** `just test` on commit `d45830b` plus this spec change: 624 Rust tests in 45 suites and 279 frontend tests, 0 failed |

The 24-bit variants lose no more than their 16-bit settings, and on the same sources (Q21). Of the 20 transcodes that change verdict, 8 are in `holdout` and 12 in `tune`; two of the `tune` ones (v0) are undetectable and do not count in E6.

## Acceptance criteria of v3 (superseded)

v3's criteria A1–A8 (same layout as section 7, compared with v2) required none of FP1–FP4 `Fake`, 15/15 confirmed fakes, 0 `Fake` on `clean` and `primary`, only `Fake → Suspicious` changes, caught rate ≥ v2 per setting, T2 and T3 `PASS`, the synthetic scenarios, and a report of `U`, `B` and `FullBand`. The v3 run failed A1 (W4 `Fake`), A2 (8/15) and, in substance, A8 ("Why v3 was replaced"); A6's premise was wrong, because v2 already fails T2 and T3 on split `all` (Q9). v3 stays loadable for comparison (section 4).

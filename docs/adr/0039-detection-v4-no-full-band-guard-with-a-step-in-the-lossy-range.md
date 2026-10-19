# ADR-0039: Detection v4: the full-band guard does not apply when there is a step inside the lossy range (amends ADR-0038)

- Status: accepted
- Date: 2026-10-19

## Context and rationale

Detection v3 (ADR-0038) added a full-band guard to the gap rule V3 and the step rule V4: a track with level `U` ≥ −70 dB and burstiness `B` ≥ 3 dB in the upper window [20.7, 21.0) kHz is `Suspicious` instead of `Fake`. It was meant to clear the four purchased tracks confirmed as false positives of v2, FP1–FP4 (watch ids W1–W4), without losing transcodes (RF-04; primary target T1).

The real-corpus acceptance run of v3 (RP-11, run `20261019T091918Z`, split `all`, v2 baseline) failed (aggregates only, ADR-0022; details in `specs/detection/rules.md`, "Why v3 was replaced"):

- A2: 8 of 15 confirmed fakes `Fake` (v2: 15/15). The guard fired on the other 7.
- A1: W1–W3 cleared, W4 still `Fake` (its `B` = 2.86 dB).
- A8: the guard fired on 28–77 % of the detectable transcodes of the seven settings v2 detects (and 94–95 % of v0 and aac256), and their `Fake` rates fell by 27–74 percentage points.

An offline analysis of that run's measurements (all aggregates; a throwaway re-implementation of V1–V10 validated against the tool to 1 rounding mismatch in 14,265 rows) showed that **no `(U, B)` threshold can work**: the 7 guarded confirmed fakes have `U` and `B` higher than W1 and W3 on both axes, so A2 is at most 10/15 for any `min_level_db` / `min_burst_db`, and transcodes carry similar bursty residue above their low-pass. Four other upper windows were re-measured; none does better. What **does** separate them is where the deepest cutoff step lies: W1–W3 have it above the lossy range (`fs` = 20450, 21450, 21250 Hz, a mastering low-pass), while the guarded confirmed fakes (`fs` 15750–19850 Hz, `D` 5.63–23.14 dB) and the transcodes (their low-pass) have it inside. With a step in the lossy range, bursty content above it is residue, not music.

The owner approved the analysis's proposal S1 as detection v4 at G1 (2026-10-19).

## Decision

1. **One more condition in `FullBand`.** `FullBand` = `U ≥ full_band.min_level_db` ∧ `B ≥ full_band.min_burst_db` ∧ ¬(`InRange(fs)` ∧ `D ≥ full_band.max_lossy_step_db`), where `D` at `fs` is the existing cutoff step (the deepest step of the search range) and `InRange` the existing lossy-range test of V4 ([14500, 20300) Hz). `max_lossy_step_db` = 3.0 dB. The window and the other values stay those of v3 (20700, 21000, −70, 3).
2. **The verdict table does not change.** V3 and V4 keep "and not `FullBand`". With v4 values V4 is never guarded (it needs an in-range `D ≥ step_fake_db ≥ max_lossy_step_db`), so in practice the guard acts only on V3 and W1–W3 are `Suspicious` with `HighBandGaps`. The V4 guard is kept formally, not removed, because one table then serves v2, v3 and v4 files and v3 files reproduce their own verdicts.
3. **Data, not code** (ADR-0011): `max_lossy_step_db` is an optional field of `[full_band]` in `specs/detection/detection-params.v4.toml`, `algorithm_version` 4. Without it (a v3 file) `FullBand` is v3's, without `[full_band]` (a v2 file) it is always false: v2 and v3 files stay loadable and give exactly their own verdicts, so the calibration tool compares all three in one run. **v4 becomes the default** (`DetectionParams::default()` equals the file). `[input]` and `[fft]` are unchanged: no `EXTRACTION_VERSION` bump.
4. **Validation.** `DetectionParams::validate()` also checks, when the field is present, `0 ≤ full_band.max_lossy_step_db ≤ step_fake_db`. 0 is allowed because it is meaningful ("any lossy-range cutoff step"); negative values would act as 0 and are rejected as mistakes. The upper bound makes "V4 is never guarded" an invariant; a file that wants the v3 guard on V4 omits the field.
5. **W4 (FP4) stays `Fake`**, accepted as a known limitation (Q7): its step (14.22 dB at 19.45 kHz) is indistinguishable from a cbr256 transcode's with these measurements.
6. **Acceptance v4** (`rules.md` section 7, `just accept --params …v2.toml --params …v4.toml`, against the v2 column of the same run): W1–W3 not `Fake`, W4 `Fake` reported (A1); 15/15 confirmed fakes (A2); `clean` 0 and `primary` ≤ 1 `Fake`, W4 only (A3); only `Fake → Suspicious` (A4); caught rate ≥ v2 per setting (A5, gate) with the `Fake`-rate drop reported; T1–T3 counts not worse than v2 (A6, reformulated because v2 already fails T2/T3 on split `all`); synthetic scenarios (A7); and, **now a gate**, `FullBand` on at most 5 % of the detectable transcodes of each of the seven settings v2 detects (A8). The analysis's figures are recorded in `rules.md` section 7.1 as predictions of an offline simulation, not results.

## Alternatives considered

- **Retune `min_level_db` / `min_burst_db` (keep v3's logic).** Rejected: no `(U, B)` region keeps W1–W3 guarded and the 7 confirmed fakes unguarded (A2 ≤ 10/15), and every setting that keeps W1 guarded still guards 20–50 % of transcodes.
- **Move or narrow the upper window.** Rejected: re-measured for [20600, 21000), [20800, 21000), [20900, 21000) and [21000, 21500) Hz; the residue is broadband, so transcode `Fake` rates move by under 1 pp, and the highest window loses W1 and W3.
- **Drop the guard from V4 only (no step condition).** Simulated A2 13/15: the gap rule still guards confirmed fakes whose deepest step is in the lossy range.
- **The step condition alone, without `U` and `B`.** Simulated A2 10/15: every track without a lossy-range step would be guarded, stationary floors included. `U` and `B` still rule those out and are kept.
- **Other `max_lossy_step_db` values.** Any value in (0, 5.63] dB gives 15/15; 3 dB keeps a 2.63 dB margin below the smallest lossy-range step of a guarded confirmed fake, while 5 dB would leave 0.6 dB and 7 dB loses one confirmed fake (14/15).
- **`min_level_db` −60, `min_burst_db` 6 with the same condition (S2).** Same A1–A3, slightly better v0 and cbr320 `Fake` rates, but margins on W1 of 3.4 dB (`U`) and 3.5 dB (`B`) against 13.4 and 6.5: rejected for thinner protection of genuine tracks and more changed values.
- **A separate guard on V4 to clear W4** (`U` ≥ −50 dB, `B` ≥ 2.5 dB on top of v4). Clears W4, but costs A2 (13/15) and up to 19.5 pp of `Fake` rate (opus128), with a margin of 0.36 dB on W4's `B`: fitting one track, rejected.
- **Go back to v2** (W1–W4 `Fake`). The owner's fallback if v4 fails acceptance; v4 strictly improves `primary` against v2 (predicted 4 → 1 `Fake`) and only loses `Fake` verdicts that stay caught as `Suspicious` (v0 above all).
- **A new reason for guarded tracks** (Q3). Not taken: a contract change; with v4 the only guarded reason is `highBandGaps`.

## Consequences

- `fakeflac-core`: `FullBandParams` gains `max_lossy_step_db: Option<f64>` (serde default `None`, unknown fields still rejected); `is_full_band` adds the `LossyStep` condition from `Measurements.step_db` / `step_hz` and `[verdict]`; `validate()` gains the two inequalities; `Default` equals v4. No new measurement, port, dependency or contract type; `just gen` gives the same bindings; `algorithmVersion` becomes 4 (`specs/contracts/commands.md`).
- `algorithm_version` 4 invalidates every cached app result (ADR-0008). The calibration cache is not invalidated (same extraction fingerprint).
- Calibration tool (`specs/calibration/evaluation.md`): new `tracks.csv` column `m_full_band_step_db` and the `nearest_fake_margin_db` formula with the guard margin `g`; `full_band.max_lossy_step_db` accepted in grids only with a baseline that has the field, and the new invariant skips points; the next free `algorithm_version` becomes **5** (proposals, `candidate.toml`). The tool's defaults stay on v2; no grid with a v4 baseline is added while `/calibrate` has no feasible point on the real corpus (`rules.md` Q10).
- Fixtures: `sfb21_gaps` and `shelf19k5_bursts` go back to `Fake` (their deepest step lies in the lossy range); only `sfb21_gaps_aa21k` stays guarded. On the sample corpus v2 and v4 give the same matrices (`fxv0` `Fake`). These verdicts are derived from the fixture definitions (`specs/detection/fixtures.md`, "Verdicts with v4"); the test stage confirms that `sfb21_gaps` has `D` ≥ 3 dB at `fs` in the lossy range and stops for G1 if not.
- **Protected tests that v4 changes** (updated by the test author in the test stage, never by the implementer). Names and lines as on the local branch `feat/RF-04`, which holds the v3 implementation and will be rebased onto this spec:
  - `crates/fakeflac-core/tests/detect_rules.rs`:
    - `:810` `rf04_sfb21_gaps_are_suspicious_without_a_cutoff_step`: its spectra have a 6 dB mean step at 16 kHz (`fs` 15950 Hz, in range), so with v4 defaults `Fake`, no reasons; cutoff still 22050 Hz.
    - `:985` `rf04_v3_confirmed_false_positives_are_suspicious`: the FP4 row (in-range step 14.22 dB) is not full band and `Fake` with v4 (scenario "FP4 stays Fake"); FP1–FP3 now carry their measured `U` and `B`.
    - `:1123` `rf04_v3_the_guard_only_turns_fake_into_suspicious`: the row "gap and lossy step: V5 comes first" is `Fake` with v4; it moves to "A v3 params file still gives the v3 verdicts" and "A cutoff step in the lossy range removes the guard".
    - `:1216` `rf04_v3_defaults_are_v3`: `algorithm_version` 4 and `FullBandParams { …, max_lossy_step_db: Some(3.0) }`; every `FullBandParams { … }` literal in tests needs the new field.
    - The other v3 tests (`:894`, `:920`, `:946`, `:957`, `:1009`, `:1077`, `:1095`, `:1205`, `:1233`–`:1294`) keep their assertions under v4 defaults; their names and comments mention v3.
  - `crates/fakeflac-core/tests/params_validate.rs:13` `rf04_validate_rejects_nan_in_every_ordered_field`: gains `full_band.max_lossy_step_db` (`step_fake_db` now also bounds it).
  - `crates/fakeflac-audio/tests/detection_lossless.rs`:
    - `:139` `verdict_sfb21_gaps_flac` and `:175` `verdict_shelf19k5_bursts_flac`: `Fake`, no reasons.
    - `:287` `guard_bursts_sfb21_gaps_is_full_band` and `:295` `guard_bursts_shelf19k5_bursts_is_full_band`: full band with v3 parameters, not with v4, `Fake` (new outline); `:291` keeps `sfb21_gaps_aa21k`, and `hf_bursts` joins it (full band, `Real`).
    - `:324` `v2_params_change_only_the_guarded_verdicts`: only `sfb21_gaps_aa21k` differs between v2 and v4.
    - `:420` `default_params_equal_versioned_file`: compares with `detection-params.v4.toml`, `algorithm_version` 4.
    - `:430` `every_analysis_carries_algorithm_version_3`: 4.
    - `:36` helper `is_full_band`: needs the params as an argument (v3 and v4).
  - `crates/fakeflac-audio/tests/params_files.rs:72` `default_params_are_accepted_and_have_a_full_band_section`: also asserts `max_lossy_step_db` = 3.0; the v3-text tests stay as they are.
  - `tools/fakeflac-calibrate/tests/sample_corpus.rs:35` `rp11_sample_fixtures_have_the_v2_verdicts`: the `sfb21_gaps` row (`:50–55`) goes back to `Fake`, no reasons.
  - `tools/fakeflac-calibrate/tests/grid.rs:54–81` and `tests/full_band.rs:488–492`: they compute the proposal's version with `next_free_version()` (`tests/common/mod.rs:50–63`), which becomes 5 once `detection-params.v4.toml` exists; no code change, but the comment at `grid.rs:58` ("algorithm_version 4") and the `candidate.toml` doc comment at `common/mod.rs:840–841` ("never 3") become stale. The scenarios now name v5 / 5.
  - `tools/fakeflac-calibrate/tests/full_band.rs` v3 tests (`:53` side by side, `:249` columns, validation, grid from a v3 baseline): unchanged, since v3 stays loadable; new tests derive from the new v4 scenarios.
- `docs/architecture/backend.md` (pipeline) is to mention the v4 condition once this ADR is accepted. ADR-0038 is marked "amended by ADR-0039".
- Accepted risks (stated at G1, `rules.md` sections 6 and 7.2): `max_lossy_step_db` rests on one confirmed fake (`D` 5.63 dB); W1's protection rests on its step lying 150 Hz above `fake_max_cutoff_hz`; genuine tracks with an in-range step near `step_fake_db` return to v2's thin margins (0.12, 0.27, 0.51 dB in the v3 run); 14–23 % of the genuine v3-`FullBand` tracks lose the guard and are exposed to V3 (none reaches `G` ≥ 6 on this corpus); W4 stays `Fake`; v0 loses about 10 pp of `Fake` rate (still caught). The positive-side evidence comes from few albums.

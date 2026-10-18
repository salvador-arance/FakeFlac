# ADR-0038: Detection v3: a full-band guard (level and burstiness above 20.7 kHz) on the gap and step rules; v3 becomes the default

- Status: accepted
- Date: 2026-10-18

## Context and rationale

The first acceptance run of detection v2 (ADR-0035) with the Rust pipeline on the real corpus (RP-11, ADR-0036, 2026-10-18) met most of its goals: 0/546 `Fake` on `clean` tracks, 15/15 confirmed fakes `Fake`, detection rates of 91.6–99.9 % for the LAME CBR settings, V2, AAC 128 and Opus 128. But 4 of 67 purchased tracks were `Fake`, and the owner confirmed all four as false positives (RF-04, primary target T1). They are called FP1–FP4 here; the 2026-10-18 report lists them as watch ids W1–W4 (rules.md, "Names used here"). "Content up to `fe`" uses the content extent of rules.md section 2: `fe` is the lower edge of the highest band with content.

- FP1–FP3 are `Fake` by V3: a 16 kHz gap step `G` of 10.3–18.4 dB, the same range as 320 kbps transcodes, so no `gap_fake_db` value separates them (calibration/evaluation.md section 7). Their content goes up to `fe` = 20.9–21.4 kHz, past the highest lossy low-pass evaluated (320 kbps LAME, 20.5 kHz).
- FP4 is `Fake` by V4: a cutoff step of 14.2 dB at 19.45 kHz, with content above it up to `fe` = Nyquist.

The obvious guard, "content extent `fe` above ~20.5 kHz", does not work. A decoded transcode keeps a floor above its low-pass (dither or rounding noise of the decoder output) that usually sits less than 60 dB below the reference level, so most transcodes also have `fe` = Nyquist. That is exactly why v1 failed (ADR-0035), and the fixtures `lp16k_dither` and `lp20k_floor` would be cleared. A guard on `fs` (largest step above the lossy range) clears FP2 and FP3 but not FP1 (its step is at 20.45 kHz, where 320 kbps transcodes have theirs) nor FP4.

What a lossy origin does not have above its low-pass is **coded audio**. The floor there is stationary: its per-frame band level hardly changes, so the per-frame median `Q` is within a fraction of a dB of the mean `M` (under 0.5 dB with 100 Hz bands at N = 4096). Music comes and goes with transients, so `M` is several dB above `Q`. Both spectra already exist (ADR-0035), so this needs no new extraction.

## Decision

1. **New measurements** on the existing band spectra, in an absolute upper window of bands centred in `[full_band.low_hz, full_band.high_hz)` = [20.7, 21.0) kHz: the level `U` (median of `M − L_ref`) and the burstiness `B` (median of `M − Q`). `FullBand` ⇔ the params have a `[full_band]` section, `U ≥ full_band.min_level_db` (−70 dB) and `B ≥ full_band.min_burst_db` (3 dB). The window is above the 320 kbps LAME low-pass and ends where FP1's highest content band ends (21.0 kHz; FP1 has no headroom). `B` decides; `U` only rules out residues near digital silence.
2. **Guard.** V3 (`G ≥ gap_fake_db`) and V4 (`D ≥ step_fake_db` in the lossy range) apply only when the track is not `FullBand`. Otherwise the existing rules V5/V6 make it `Suspicious` with the existing reasons `HighBandGaps` / `AmbiguousCutoff`. The guard can only turn `Fake` into `Suspicious`, so the caught rate of every setting is unchanged.
3. **Data, not code** (ADR-0011): a new optional section `[full_band]` in `DetectionParams`, in `specs/detection/detection-params.v3.toml`, `algorithm_version` = 3. A params file without the section (v2) loads with the guard disabled and gives exactly the v2 results, so the calibration tool compares v2 and v3 in one run and v2 stays a member of the same rule family.
4. **Validation in the core.** The core still has no file loader. A pure `DetectionParams::validate(&self) -> Result<(), ParamsError>` in `fakeflac-core` checks `gap_suspicious_db ≤ gap_fake_db`, `step_suspicious_db ≤ step_fake_db` and `fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz`; every loader (the calibration tool today, the settings file in H2) calls it after serde deserialisation, which checks the shape (required fields, unknown fields).
5. **v3 becomes the default** parameters of the code (a test checks equality with the file). `[input]` and `[fft]` are unchanged: `EXTRACTION_VERSION` does not change and cached spectra are reused.
6. **Version numbers belong to `specs/detection/`.** Parameter files produced elsewhere (a `/calibrate` proposal, the sample corpus's `candidate.toml`) take the next free `algorithm_version` (1 + the highest in `specs/detection/` and the baseline), never 3 again.
7. **No new rule for LAME V0 (18.2 % detected) or AAC 256 (0.6 %).** They are recorded as known limitations (rules.md section 6): no measurement of the band spectra separates them from full-band or band-limited lossless tracks without new false positives. They need new measurements (a new extraction), probably together with RF-15.

Exact rules: `specs/detection/rules.md` (v3). Acceptance criteria A1–A8: rules.md section 7, measured with `just accept --params …v2.toml --params …v3.toml`. Owner decisions at G1: approve without pre-measuring `U` and `B` (A1/A5 decide, A8 reports them); A5 gates the caught rate against the v2 column of the same run and reports the `Fake`-rate drop as information.

## Alternatives considered

- **Guard on `fe`** (content extent above ~20.5 kHz). Rejected: most transcodes have `fe` = Nyquist because of their floor; it would clear `lp16k_dither`, `lp20k_floor` and the real transcodes they model.
- **Guard on the position of the largest step** (`fs` above the lossy range). Rejected: clears FP2 and FP3 only. FP1's step is at 20.45 kHz, the same place as a 320 kbps transcode's, and FP4's is inside the lossy range.
- **Level `U` alone** in the upper window. Rejected for the same reason as `fe`: the floor of a 16-bit transcode can be as high as music there.
- **Raise `gap_fake_db` or `step_fake_db`.** Rejected: FP1–FP3 have `G` of 10–18 dB and FP4 has `D` of 14.2 dB, inside the range of real transcodes. The grid of RP-11 found no threshold-only fix (evaluation.md section 7).
- **A new reason** (for example `fullBandGaps`) for the guarded tracks. Not taken now: it is a contract change (`SuspicionReason`, bindings, UI text) and the existing reasons already say what was seen. Open question Q3.
- **Rules for V0 and AAC 256 with the existing measurements** (lower `step_fake_db` near 20 kHz, lower `gap_fake_db`). Rejected: FP4 shows a lossless track with a 14.2 dB step at 19.45 kHz, and V0 transcodes look like FP1–FP3 once the low-pass is gone; either rule would add false positives on purchased discs, the metric that weighs most (fp = 10).
- **Validation only in the calibration tool.** Rejected: the invariants are part of the detection rules (section 3 of rules.md depends on them), so they live next to `DetectionParams` and every loader shares them.

## Consequences

- `fakeflac-core`: `DetectionParams` gains `full_band: Option<FullBandParams>` and `validate()` with a `ParamsError` (thiserror); `Measurements` gains two optional fields for `U` and `B`; the verdict step evaluates `FullBand` from them and the params; V3 and V4 gain the guard; `Default` equals v3. No port, no dependency and no contract type changes; `just gen` gives the same bindings. When `DetectionParams` reaches the settings file (H2, data-model.md), `[full_band]` is part of it.
- `algorithm_version` 3 invalidates every cached app result (ADR-0008). The calibration cache is not invalidated (same extraction fingerprint).
- Fixtures: `sfb21_gaps` (gated content up to Nyquist) changes from `Fake` to `Suspicious` under v3; four new fixtures (exact frequency-domain low-pass after gating) model the 320 kbps transcode, FP1–FP3, a 19.5 kHz transcode and FP4.
- **Protected tests that v3 changes** (updated by the test author in the test stage, never by the implementer):
  - `crates/fakeflac-core/tests/detect_rules.rs:803` `rf04_sfb21_gaps_are_fake_without_a_cutoff_step`: expects `Fake`; with v3 defaults its spectra (M −6 dB, Q −19 dB above 16 kHz) give `B` = 13 dB and `U` = −6 dB, so `Suspicious` with `HighBandGaps`.
  - `detect_rules.rs:370` `clean()` and every other `Measurements { … }` struct literal in that file (lines 386–711): they no longer compile once `Measurements` has the `U` and `B` fields.
  - `crates/fakeflac-audio/tests/detection_lossless.rs:117` `verdict_sfb21_gaps_flac` (expects `Fake`).
  - `detection_lossless.rs:239` `deterministic` (the scenario now uses `shelf19k5_bursts` instead of `lp19k`).
  - `detection_lossless.rs:245–251` `default_params_equal_versioned_file` (compares with `detection-params.v2.toml` and `algorithm_version` 2) and `:255–258` `every_analysis_carries_algorithm_version_2`.
  - `tools/fakeflac-calibrate/tests/grid.rs:54–75` (proposal `detection-params.v3.toml` with `algorithm_version` 3 from the v2 baseline; now `v4` / 4) and `tools/fakeflac-calibrate/tests/common/mod.rs:815–821` (`candidate.toml` with `algorithm_version = 3`; now the next free version).
  - `tools/fakeflac-calibrate/tests/sample_corpus.rs:35–56` `rp11_sample_fixtures_have_the_v2_verdicts`: analyses with `DetectionParams::default()` and expects `sfb21_gaps` to be `Fake` (`:47`); with v3 defaults it is `Suspicious` with `HighBandGaps` (sample-corpus.md).
  - Any other test whose synthetic spectra, with v3 defaults, give `U ≥ −70` and `B ≥ 3` in the 20.7–21.0 kHz window together with `G ≥ 6` or a V4 step. A search of `detect_rules.rs` found only `:803` (the `mean_only` spectra have `Q` flat at 0 dB and a lower `M`, so `B` ≤ 0).
- The calibration tool calls `validate()`, reports `U`, `B`, `full_band`, the full-band margins, caught rates and per-setting `Fake → Suspicious` counts, accepts `[full_band]` keys in grids with a v3 baseline (`calibrate-grid.v2.toml`), numbers proposals with the next free version, and redefines `nearest_fake_margin_db` to account for the guard (unchanged for v2 files). Its defaults stay on v2.
- `docs/architecture/backend.md` (pipeline) mentions the guard, marked pending until this ADR is accepted.
- Accepted risks: the values of `U` and `B` for FP1–FP4 and for the real transcodes have not been measured yet (A8 reports them; `cbr320`'s low-pass is only 200 Hz below the window). A1 (FP1–FP4 not `Fake`) may fail, and then the figures go back to G1. A5 gates the caught rate, which the guard cannot lower; the price of the guard is the `Fake` rate, which can drop for transcodes with bursty content in the window (V0 above all, and lossy sources with the low-pass disabled). A5 reports that drop per setting for the owner to judge. Lossless masters band-limited at or below 20.5 kHz with a gap or a lossy-range step stay `Fake`, as in v2.

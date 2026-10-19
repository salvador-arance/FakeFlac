# ADR-0041: Detection v5: source-bitrate rules for lossy files (RF-15) as an optional `[lossy]` section of `DetectionParams`; v5 becomes the default; contract version 3

- Status: proposed
- Date: 2026-10-19

## Context and rationale

RF-15 (high, H2): for MP3, M4A and Opus, estimate the real source bitrate and flag files whose content corresponds to a lower bitrate than declared (a 320 kbps MP3 made from 128 kbps). ADR-0031 fixes the verdict model for every format (`Real` / `Suspicious` / `Fake` relative to the declared quality, plus `estimated_source_bitrate_kbps`); ADR-0011 and ADR-0025 require thresholds and tables to be data; `docs/architecture/backend.md` section 9 names the main signal (spectral cutoff against declared bitrate, a cutoff → bitrate table per codec and encoder, versioned in `DetectionParams`) and complementary ones (LAME/Xing tag, declared low-pass, per-frame bitrate, sfb21 energy).

The lossless rules (detection v4, `specs/detection/rules.md`, ADR-0039) cannot be reused as they are: every honest MP3 has a low-pass by design, so rule V4 would call it `Fake`. The question for a lossy file is not "is there a lossy cutoff" but "is the cutoff where an honest encode at this bitrate would put it".

Owner input for the evaluation (docs/agents/progress.md, 2026-10-18; aggregates only, ADR-0022): about 170 streaming-site MP3 320 rips whose sample rate tells the upstream stream (44.1 kHz from AAC 128, 48 kHz from Opus), labels that are upper bounds, 14 honest AAC 128 `.m4a` files, and one album with a full chain from one master (FLAC → AAC 128 → MP3 320) whose FLAC version holds FP4.

## Decision

1. **Rules** (`specs/detection/lossy-source.md` sections 3–5). For a lossy codec: the content edge `fc` is the reported cutoff of rules.md (the deepest step `fs` if `D ≥ step_suspicious_db`, else the content extent `fe`); the expected cutoff `c_exp` is the per-codec table row of the declared bitrate (largest row ≤ the declared or average bitrate), lowered by the LAME tag's low-pass when present and capped below Nyquist; the shortfall is `s = c_exp − fc`. `Fake` when `s ≥ fake_shortfall_hz` with a step of at least `fake_min_step_db`; `Suspicious` with the new reason `cutoffBelowDeclared` (clear step) or `gradualRolloff` (no clear step) when `s ≥ suspicious_shortfall_hz`; `Suspicious` with `lowHighFrequencyContent` when there is little treble; otherwise `Real`. The estimate is the declared bitrate when no shortfall rule fires, otherwise the table row nearest to `fc`, at most the declared bitrate. `G`, `U`, `B` and per-frame bitrates are not used in v5.
2. **Data, one file.** `DetectionParams` gains an optional `[lossy]` section (three thresholds and `[lossy.cutoffs]` with `mp3` and `aac` rows; `opus` optional and absent in v5). `specs/detection/detection-params.v5.toml` = the v4 file with every value unchanged plus `[lossy]`, `algorithm_version` 5. **v5 becomes the default.** v2–v4 files stay loadable: they judge lossless files exactly as before and give lossy files `Error(unsupportedFormat)`. The next free `algorithm_version` becomes 6. The table and thresholds are **indicative starting values, not measured**; acceptance B1–B11 (lossy-source.md section 9) decides, and a failed gate returns to G1.
3. **Contract version 3** (`specs/contracts/commands.md`): `TrackInfo.codec` (`pcm`, `flac`, `alac`, `mp3`, `aac`, `opus`), `TrackInfo.bitrateMode` (`cbr`, `vbr`, `abr`), the reason `cutoffBelowDeclared`, `declaredBitrateKbps` and `estimatedSourceBitrateKbps` filled for lossy codecs, `algorithmVersion` 5. ALAC in M4A is judged as lossless.
4. **Core** (`fakeflac-core`, no I/O, no new dependency, no new port): domain enums `AudioCodec` and `BitrateMode`; `AudioProbe` gains `codec`, `declared_bitrate_kbps`, `bitrate_mode`, `encoder`, `encoder_lowpass_hz`; `analyze_track` sends lossy codecs to the lossy rules and the others to the v4 rules; `DetectionParams::validate()` gains the `[lossy]` checks of lossy-source.md section 6.
5. **Calibration** (`specs/calibration/lossy.md`): lossy files as labelled tracks, honest encodes and cascades generated with ffmpeg, the label attributes, chains of versions, split units, and lossy tables kept apart from every lossless figure; `calibration.v2.toml` becomes the default config.

## Alternatives considered

- **A separate lossy params file with its own version.** A `TrackAnalysis` and the cache key (ADR-0008) carry one `algorithm_version`; two files would need two numbers in the contract and the key, and the lossy rules reuse the lossless measurement parameters, which would then live in two places. Rejected (lossy-source.md Q1).
- **The 16 kHz gap step `G` as lossy evidence.** LAME 320 kbps encodes already show `G` as large as a transcode's (rules.md, "Why v2 was replaced"), so it cannot separate an honest 320 from a 320 made from 128. Recorded as a candidate for non-MP3 codecs (lossy-source.md section 10).
- **The LAME tag alone** (encoder settings and low-pass). It describes the last encode, not the source; ffmpeg-made files often leave the low-pass byte at 0. It is used only to lower `c_exp` (Q6).
- **The `esds` average bitrate of M4A.** Encoders fill it inconsistently; the average over the track's samples is used.
- **Per-encoder tables now.** The encoder string is often absent or generic (ffmpeg's `Lavc…`); one table per codec, conservative for AAC, is the starting point (section 10 candidate).
- **MDCT-domain double-compression fingerprints now.** They are the robust signal of H3 (backend.md section 9) and need a codec-domain extraction; out of scope for H2.
- **The nearest table row as the estimate in every case.** It would show "Real, ≈256 kbps" on an honest 300 kbps VBR file. Rejected (Q5).

## Consequences

- `algorithm_version` 5 invalidates every cached app result once (ADR-0008); lossless verdicts, reasons and cutoffs are the v4 ones (B1).
- `just gen` produces new bindings; the UI gains two columns, the molecule `BitrateValue`, a longer detail line and new texts (`specs/ui/lossy-formats.md`); `src/lib/testing/fixtures.ts` gains the new `TrackInfo` fields.
- **Protected tests that v5 and contract version 3 change** (updated by the test author in the test stage, never by the implementer; lines as on `main` at c63cf20):
  - `crates/fakeflac-audio/tests/detection_lossless.rs:531` `default_params_equal_versioned_file`: compares with `detection-params.v5.toml`, `algorithm_version` 5.
  - `crates/fakeflac-audio/tests/detection_lossless.rs:541` `every_analysis_carries_algorithm_version_4`: 5.
  - `crates/fakeflac-audio/tests/params_files.rs:83` `v3_file_equals_the_v4_defaults_without_max_lossy_step_db`: the expected value is built from the defaults, which now have `[lossy]`; it needs `lossy: None` (a v3 file has none). `:102` `default_params_are_accepted_and_have_a_full_band_section` keeps its assertions.
  - `crates/fakeflac-core/tests/detect_rules.rs:1246` `rf04_v3_defaults_are_v3`: `algorithm_version` 5. `:894` `v2_params()` and `:904` `v3_params()` are built from the defaults and now carry `[lossy]`; they only judge lossless measurements, so their results do not change, but they no longer equal the v2/v3 files exactly (set `lossy: None` if a test compares them with a file).
  - `crates/fakeflac-core/tests/detect_rules.rs:869` `supported_extensions`: `mp3` and `m4a` map to `Mp3` and `M4a` (ADR-0040); `opus` stays `None`.
  - Struct literals that gain fields: `crates/fakeflac-core/tests/analyze_folder.rs:96` (`AudioProbe`), `crates/fakeflac-core/tests/album_summary.rs:11` (`TrackAnalysis`/`TrackInfo`), `src-tauri/tests/contract.rs:15` and `:30` (`TrackInfo`, `TrackAnalysis`; the contract JSON gains `codec` and `bitrateMode`).
  - `tools/fakeflac-calibrate/tests/full_band.rs:886` `rf04_v4_margins_include_the_lossy_step` asserts `DetectionParams::default()` equals the v4 file: it must load the v4 file instead (the defaults are v5).
  - `tools/fakeflac-calibrate/tests/inputs.rs:71` (expects `trusted.txt:6: not a FLAC or WAV file`) and `:263` (expects `confirmed.txt:2: unknown kind 'dudoso' (expected fake or false-positive)`): the two messages change (`specs/calibration/lossy.md`, C11) to `not a supported audio file (FLAC, WAV, MP3, M4A or Opus)` and `unknown kind 'dudoso' (expected fake, real or false-positive)`.
  - `src/lib/components/organisms/ResultsTable.test.ts:39–52` "is a grid with the eight columns …": ten columns, `Bitrate` and `Real bitrate` after `Cutoff` (`specs/ui/lossy-formats.md`). `:150` (aria-sort on index 6, Duration) keeps its assertions.
  - `src/lib/i18n/index.test.ts:26`, `src/lib/pages/AnalysisPage.test.ts:503` and `:521`: `error.NO_AUDIO_FOUND` becomes "No supported audio files in this folder".
  - `src/lib/pages/AnalysisPage.test.ts:220`: its negative regex `/No WAV or FLAC|…/` would pass vacuously with the new text; it must match the new text (`/No supported audio|…/`).
  - Not failing but stale: `src/lib/components/organisms/StatusBar.test.ts:83–85` and `StatusBar.stories.ts:51` pass the old text as a `message` prop; the test author may update the literal.
  - `src/lib/testing/fixtures.ts` (test support): `TrackInfo` gains `codec` and `bitrateMode`, so the fixtures need them for `svelte-check`.
  - New component tests and stories derive from the new `@RF-06`/`@RF-07` scenarios of `decode-lossy.feature` (`BitrateValue`, the lossy detail line, the column header tooltip, `M4A (ALAC)`, `error.unsupportedFormat`); `ColumnHeader` gains an optional prop, so its existing tests keep passing.
  - `tools/fakeflac-calibrate/tests/grid.rs:58` and `tests/common/mod.rs:846`: the proposal's version is computed by `next_free_version()`, which becomes 6 once `detection-params.v5.toml` exists; no code change, comments ("algorithm_version 5", "5 today") become stale. `tests/environment.rs:197`: the default config becomes `calibration.v2.toml`; the scenario (exit 3 when ffmpeg is missing) still holds.
- Adding `detection-params.v5.toml` in the spec PR does not break `main` before the implementation: the tool and its tests read only `algorithm_version` from every params file as a plain TOML table (`tools/fakeflac-calibrate/src/run.rs:472`, `tests/common/mod.rs:57`), and no test deserialises every file into `DetectionParams`.
- `docs/architecture/backend.md` (pipeline and section 9) is to name detection v5 as the default once this ADR is accepted.
- Accepted risks (stated at G1, lossy-source.md sections 8 and 9): the tables and thresholds are not measured; close bitrates, Opus → MP3 and AAC 256 → MP3 320 chains and sources without a low-pass are mostly `Real`; genuine sources that end early can be `Suspicious` or `Fake`; the estimate is expressed in the declared codec's terms.

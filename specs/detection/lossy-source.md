---
id: SPEC-DETECTION-LOSSY
requirements: [RF-14, RF-15, RF-16]
status: proposed
algorithm_version: 8
params: specs/detection/detection-params.v8.toml
lossless_rules: specs/detection/rules.md (v4 rules and values, unchanged in v5; v6 adds [edge]; unchanged in v7 and v8)
adr: [ADR-0040, ADR-0041, ADR-0043, ADR-0047]
history: algorithm_version 5 (detection-params.v5.toml, sections 1-10); algorithm_version 7 (detection-params.v7.toml, RF-16, adds rule L6a, section 11); algorithm_version 8 (detection-params.v8.toml, RF-14/RF-15, Opus stream information in section 2.4 and Opus rows, section 12)
---

# Detection rules v5, v7 and v8: lossy files (MP3, M4A, Opus) and their source bitrate

> For an MP3 or an AAC file in M4A: read what the stream **declares** (codec, bitrate, bitrate mode, the LAME tag's low-pass), measure where its content **ends** with the band spectra and measurements of `rules.md` (unchanged), compare that edge with the **expected cutoff** of an honest encode at the declared bitrate, and decide **real / suspicious / fake** with an **estimated source bitrate**. The expected cutoffs are a per-codec table in `DetectionParams` (`[lossy.cutoffs]`), and every threshold is a named parameter of `detection-params.v5.toml`; the code never hard-codes them (S4, ADR-0011). Design decision: ADR-0041. Decoding (RF-14 H2): ADR-0040.

The verdict answers ADR-0031's question for lossy files: *is the content consistent with the declared quality?* A 320 kbps MP3 whose content stops at 17 kHz is `Fake`, with an estimated source of ≈128 kbps (planning section 2.5, `docs/architecture/backend.md` section 9).

**What changes in v7 (RF-16).** One rule, L6a (section 4), between L5 and L6, and an optional section `[lossy.streaming]` with five parameters (section 11). A lossy MP3 or AAC file that L6 would call `Real` becomes `Suspicious` with the new reason `streamingSampleRate` and no estimate when its decoded sample rate is a streaming rate (48 kHz) and its cutoff lies at Opus's 20 kHz band edge: the signature of a rip of a full-band lossy streaming source, typically Opus from YouTube. Against v6, for the same spectra, the only possible verdict change is `Real → Suspicious` on lossy files; lossless files are unchanged. Design decision: ADR-0043 (accepted at G1, #78). Sections 1–10 describe v5 and stay valid under v7 except where they point to section 11.

**What changes in v8 (RF-14, RF-15: Opus).** Opus files (`.opus`, Ogg Opus, channel mapping family 0) are decoded by the adapter with `rusty-opus` and an Ogg reader of its own (ADR-0047; stream information and errors in section 2.4), the scanner lists them, and `detection-params.v8.toml` adds the `opus` rows of `[lossy.cutoffs]` (section 12). For an Opus file the shortfall rules give no estimate (section 12.4). Against v7, for the same spectra, **nothing changes for any file that is not Opus**: WAV, FLAC, ALAC, MP3 and AAC get their v7 verdicts, reasons, cutoffs and estimates, with `algorithm_version` 8. Opus files were never analysed before (v5–v7 make them `Error(unsupportedFormat)` by L0). Sections 1–11 stay valid under v8 except where they point to section 12.

**Names used here.** `b` is the declared bitrate in kbps (section 2), `fc` the measured content edge, `c_exp` the expected cutoff, `s` the shortfall `c_exp − fc` (section 3). `D`, `fs`, `fe`, `H`, `G` are the measurements of `rules.md` section 2. "Lossy codec" means `mp3`, `aac` or `opus`; "lossless codec" means `pcm` (WAV), `flac` or `alac` (ALAC in M4A). **Opus** was specified only as far as the rules go until v8 (its scenarios waited in `specs/pending/opus.feature`, Q9); since v8 it is decoded and judged, and its scenarios are in `specs/features/opus.feature` (section 12).

## Questions for G1 (v8, RF-14/RF-15: Opus): open

ADR-0047 was accepted at G1 on 2026-11-01 with its three defaults (the decoder `rusty-opus =1.0.2`, family 0 only and only `.opus` scanned, `EXTRACTION_VERSION` 2). The questions below are about the spec built on it. The figures they rest on were **measured before G1** (2026-11-01) and are in section 12.8 (F1–F7).

| # | Question | Status |
|---|---|---|
| Q29 | **Estimate of an Opus track that L2–L5 judge.** Opus keeps its 20 kHz full band at every bitrate of the table (measured from 16 kbps up, section 12.8 F1, F2), so every Opus row has the same cutoff and the nearest-row rule of section 5 would give the lowest such row (24 kbps) for any early edge: an "≈24 kbps source" for an Opus 256 made from an MP3 128, which means nothing. Options: (a) **null** for Opus whenever L2–L5 fire (the verdict says "fake", the Real bitrate cell shows "—"), as L6a already does for a table that cannot express the source (Q19); (b) an **MP3-equivalent** estimate: `r*` read from `lossy.cutoffs.mp3` for Opus files (needs a new data key, for example `lossy.estimate_rows.opus = "mp3"`, a UI text change, since `help.realBitrate` says "in terms of the file's own codec", and probably its own ADR); (c) section 5 unchanged (gives the meaningless 24) | Open. **Default: (a) null** (section 12.4) |
| Q30 | **The Opus rows** (section 12.3): eight rows, 24, 32, 48, 64, 96, 128, 160 and 192 kbps, each with `cutoff_hz` = 20000 (libopus's full band, the top CELT band edge), and **no row below 24 kbps**, so a lower declared bitrate leaves `c_exp` undefined and the file is not judged by the shortfall rules. **Measured before G1** (section 12.8): native libopus 1.6.1 decodes of 69 corpus sources report `fc` 20050 or 20150 Hz in 69 of 69 at every bitrate from 16 to 320 kbps (F1, F2; 12 kbps is wideband and falls into L2), and with these rows 0 of 828 honest encodes at 24–320 kbps are `Fake` or `Suspicious` (F1). The full band holds from **16 kbps** (F2); the first row stays at 24 to keep a margin for older libopus versions, which were not measured and may choose a narrower bandwidth just above 16 kbps. Cost of 24 (F3): libopus's VBR payload at a 24 kbps target has a median of 23.9 kbps, so about half of the honest 24 kbps encodes fall below the table and are `Real`, not judged by the shortfall rules. **Alternative:** a first row at **16 kbps** (cutoff 20000), which judges the 20 and 24 kbps encodes too, measured full band in 69 of 69. One row would behave the same as eight with Q29 (a); the eight rows let a calibration run change one bitrate alone | Open. Default: as written (24–192 kbps at 20000 Hz); alternative: first row 16 kbps |
| Q31 | **Declared bitrate of an Opus file** (section 2.4.3): the total size of the **audio packets** over the duration (Ogg page headers, segment tables and the two header packets excluded), rather than the file size over the duration (what ffprobe shows). The packet size is what the encoder's bitrate targets; the Ogg overhead adds about 0.3–0.6 kbps at 50 packets per page | Open. Default: audio packets |
| Q32 | **Bitrate mode and encoder of an Opus file**: `bitrate_mode` absent (Ogg Opus does not signal CBR or VBR; as AAC), `encoder` and `encoder_lowpass_hz` absent (the OpusTags vendor string is checked but not reported). Reading the vendor string into `encoder` later would change the adapter's output (`EXTRACTION_VERSION`) | Open. Default: all three absent |
| Q33 | **Strictness of the Ogg reader** (section 2.4.4): the listed `corruptStream` cases; a second logical stream (another serial number: chained or multiplexed files) is `unsupportedFormat`; the end-of-stream flag is **not** required (a file whose last page lacks it is read up to its last page); no resynchronisation after damage and no leading bytes before the first `OggS` | Open. Default: as written |
| Q34 | **Contract version.** No type changes: `AudioFormat` and `AudioCodec` already hold `opus`, `bitrateMode` stays null, and `just gen` produces the same bindings. The contract stays **version 7** and `commands.md` gains the section "Opus files (detection v8)" (new supported extension, `NO_AUDIO_FOUND` wording, `algorithmVersion` 8, the null estimate of Q29). A new version 8 would only rename it | Open. Default: stays 7 |
| Q35 | **Format label** in the table: `OPUS` (the existing rule, `format` in upper case, `ui/lossy-formats.md`), or `Opus` (the codec's own spelling; a new text `format.opus` and a UI change) | Open. Default: `OPUS` (no UI change) |
| Q36 | **Calibration** (`calibration/lossy.md` C14–C17): Opus rows evaluated by params files with Opus rows (v8) and not evaluated by params files without them (v5–v7, as Q8 does for files without `[lossy]`); `.opus` included by `formats=lossy` and `formats=all`; Opus allowed as the last stage of a lossy entry; a new `calibration.v4.toml` (v3 plus four Opus settings, four honest Opus entries and five cascades ending in Opus; the new settings have `lossless_transcode = false`, so the lossless tables do not change); the default `--config` stays `calibration.v2.toml` | Open. Default: yes to all |
| Q37 | **Acceptance O1–O8** (section 12.7) and their gate values: honest generated Opus encodes and the negative cascade `cbr320_opus128` `Fake` ≤ 1 % (measured, F1 and F4: 0 of 828 honest encodes and 0 of 69 `cbr320_opus128` `Fake`); the O4 cascades `cbr128_opus256` and `aac128_opus256` caught ≥ **85 %** and `Fake` ≥ **80 %** of their detectable rows (measured over all 69 rows, F4: caught 94 % and `Fake` 94 %; caught 91 % and `Fake` 88 %); the rest reported, `cbr192_opus256` and `cbr128_opus160` included. The O4 cascades end at **256 kbps, not 160**: at 160 kbps or less libopus hides the source's edge by band folding (section 12.6; measured `cbr128_opus160` caught 15 of 69, `aac128_opus160` 6 of 69), so a 160 cascade cannot carry a gate; `cbr128_opus160` stays as a report that documents the limitation. B5's 95 % and 90 % are not proposed: the measured `Fake` rates at 256 kbps (94 % and 88 %) lie below them, so the gates leave a margin under the measurements instead | Open. Default: as written |
| Q38 | **Changes to approved specs and protected tests** in this PR (section 12.9): the defaults and `algorithmVersion` become v8/8 and the next free `algorithm_version` becomes **9**; `.opus` is scanned; `specs/pending/opus.feature` moves to `specs/features/opus.feature`; the listed scenarios of `decode-lossy.feature`, `detection-lossy.feature`, `detection-lossless.feature`, `detection-edge-step.feature`, `detection-streaming-source.feature`, `calibration.feature` and `calibration-lossy.feature` change; and the protected tests listed in section 12.9 change in the test stage. Agree? | Open. Default: yes to all |
| Q39 | **ADR for detection v8.** Every detection version so far has its ADR (Q26). v8 adds data rows to a key that ADR-0041 already defined (`lossy.cutoffs.opus`, optional) and one rule detail (Q29 (a), the null estimate, which follows the precedent of Q19); ADR-0047 ("Consequences") already announces the new params version. Proposal: **no new ADR**; ADR-0047's "Notes from implementation" record the version. With Q29 (b) a new ADR would be needed (a cross-codec estimate is a new kind of rule) | Open. Default: no new ADR |
| Q40 | **The "lower-bitrate source" fixture** (`lossy-fixtures.md` `opus_160_src16k`) is encoded at full band from a signal **low-passed at 16 kHz**, because Opus's own bandwidths (4, 6, 8, 12 and 20 kHz) cannot give an edge between 16 and 17.5 kHz, where the `Fake` rule is tested without little treble (an edge at 12 kHz falls into L2, `opus_160_swb`). ADR-0047 point 3 names `max_bandwidth` as what sets a fixture's edge; this fixture sets it through its input instead. **Measured** (section 12.8 F4): the edge comes from the low-passed input and survives the encode: mono `lp16k` encoded by `rusty-opus` at 160 kbps reports `fc` 16050 Hz with `D` ≈ 33 dB (the edge survives in mono from 96 kbps; in independent-channel stereo only from 192 kbps, because of band folding, section 12.6), so the fixture stays **mono** (a stereo variant would need ≥ 192 kbps) | Open. Default: yes |
| Q41 | **`EXTRACTION_VERSION` stays 2** (ADR-0047 point 5) | **Decided at G1 (owner, 2026-11-01, ADR-0047)** |
| Q42 | **Determinism of the fixture encoder across SIMD levels** (ADR-0047 point 3, `lossy-fixtures.md` section 1.3). ADR-0047 assumed that `rusty-opus` gives the same bytes on every SIMD level. **Measured** (section 12.8 F7): same-ISA encodes are byte-identical run to run, but `RUSTY_OPUS_ISA=scalar` (and `sse2`, `avx`) gives **different bytes** from the default AVX2 path for mono fixtures (11 of 18 recipes differed, VBR and CBR; stereo VBR 160, the spike's case, was identical); the decoded difference is about 62 dB SNR (different coding decisions, not rounding), and `fc`, `D` and `H` are unchanged. Proposal: the rule becomes (a) the same recipe and seed give the same bytes within one process and between two runs on the same machine; (b) encoded in a child process with `RUSTY_OPUS_ISA=scalar`, every encoded fixture keeps its **truth** (verdict, reasons, cutoff within the fixture tolerance, sample counts, declared bitrate within 1 kbps of its target), not its bytes. No test asserts fixture bytes against a stored hash, so a CI runner with another SIMD level gets the same verdicts. ADR-0047 point 3 is amended in the same PR | Open. **Default: accept** (rule (a) + (b); ADR-0047 point 3 amended) |

## Questions for G1 (v7, RF-16): all decided

The owner decided the approach (2026-10-25): the cheap, deterministic 48 kHz indicator, `Suspicious` only, every threshold as data, MDCT fingerprints left as future work. The owner then decided every detail below at G1 (2026-10-25): the proposed defaults for Q18–Q20 and Q23–Q27, the measured window for Q21, stricter acceptance for Q22 (K4 from N = 30 labels and a K5 gate) and the shorter reason text for Q28. The spec is written with these decisions.

| # | Question | Status |
|---|---|---|
| Q18 | **Name of the new reason.** `streamingSampleRate` describes the evidence (a streaming sample rate with a full-band edge), as the existing reasons describe theirs (`cutoffBelowDeclared`, `highBandGaps`, `gradualRolloff`). The alternative `possibleStreamingSource` names the conclusion instead | **Decided at G1 (owner, 2026-10-25):** `streamingSampleRate` |
| Q19 | **Estimate of an L6a track.** `null` (shown "—"): the spectrum agrees with the declared bitrate, so it says nothing about the source's bitrate, and an Opus 128–160 kbps source has no equivalent row in the MP3 or AAC table. Alternatives: `b` (shows "320 kbps" next to `Suspicious`, which reads as "the source was 320") or `min(b, r*)` as L3–L5 (gives ≈256 kbps for the motivating case, `fc` = 20050 Hz being nearest the 19700 Hz row: a number with no meaning here) | **Decided at G1 (owner, 2026-10-25):** `null` |
| Q20 | **Where the parameters live.** A subsection `[lossy.streaming]` of `[lossy]` (optional; absent in v5 and v6) rather than a top-level `[streaming]`: the rule only exists where the lossy rules run, a top-level section without `[lossy]` would mean nothing, and the calibration tool already rejects `lossy.*` grid keys (calibration/lossy.md C10) | **Decided at G1 (owner, 2026-10-25):** `[lossy.streaming]` |
| Q21 | **Values of `[lossy.streaming]`** (section 11.3). Proposed as indicative: `sample_rates_hz = [48000]`, `codecs = ["mp3", "aac"]`, `min_declared_bitrate_kbps = 192`, and a cutoff window first written as `[19500, 21000)`, with 20300 as the alternative upper bound. **Measured before G1** (section 11.3.1): what FakeFlac reports for an Opus edge is 20050 or 20150 Hz (band centres, 100 Hz grid); honest 48 kHz LAME 320 reports 20250–20450 Hz (median 20350 for native 48 kHz masters, 20450 for masters resampled to 48 kHz); LAME 224/256 reports 19450–19750 Hz. The gap between Opus and LAME 320 in reported-cutoff terms is 100–400 Hz (paired per source, median 300 Hz), not the about 500 Hz between the nominal low-passes that the first draft assumed. `[19500, 21000)` flags 96–100 % of honest 48 kHz LAME 320 encodes and 88–91 % of LAME 224/256; `[19500, 20300)` still flags 12.5–13.5 % of honest LAME 320. Honest AAC 256 at 48 kHz from ffmpeg's native encoder reports a median of 21750 Hz: 7 % of it falls in `[19500, 21000)`, 0–1.9 % in `[19800, 20200)` (the first draft's "edge about 19.5–20 kHz" was not measured and does not hold for this encoder); other AAC encoders were not measured | **Decided at G1 (owner, 2026-10-25) with measured data (11.3.1): window `[19800, 20200)`** (`min_cutoff_hz` = 19800, `max_cutoff_hz` = 20200). `sample_rates_hz`, `codecs` and `min_declared_bitrate_kbps` unchanged and still indicative |
| Q22 | **Gate values** of acceptance K (section 11.7): K3 (generated `opus128_cbr320` caught of its detectable rows), K4 (confirmed lossy fakes at 48 kHz, strong labels, caught; applied only from N such labels, below that reported as n/a without blocking) and whether K5 (false positives on honest lossy files) is a gate or only a report. They are targets, not measurements, as Q12 for v5. K4's labels come from `upstream=by-rate` (calibration/lossy.md section 1.5), the same observation L6a uses, so **K4 measures recall only** (on the owner's own rips), never false positives; K5 is the criterion that bounds false positives. Measured before G1 with the decided window (11.3.1): K3 1418 of 1425 rows (99.5 %); K4 134 of 134 strong labels; honest 48 kHz LAME 320 flagged 2.7–4.7 %, LAME 256 0 %. The first draft proposed N = 10 and K5 as a report only. With a 90 % gate a rule whose true recall is 80 % still passes 38 % of the time with N = 10 (69 %: 13 %) and 12 % with N = 30 (69 %: 0.7 %); neither N changes this corpus, which has 134 strong 48 kHz labels | **Decided at G1 (owner, 2026-10-25):** K3 ≥ **90 %**; K4 ≥ **90 %**, applied from **N = 30** strong 48 kHz labels (with fewer: "n/a (fewer than 30 strong 48 kHz fakes)", reported, does not block); **K5 is a gate**: with v7, the `streaming` rate of the honest generated sub-row `cbr320 @ 48000` is at most **10 %** and that of `cbr256 @ 48000` at most **2 %** (calibration/lossy.md 6.8); the per-sample-rate report part of K5 stays |
| Q23 | **Contract and CSV.** A new `SuspicionReason` value changes the generated bindings, so the contract becomes **version 6** (`specs/contracts/commands.md`, "Streaming sample rate (version 6)"). The CSV's column 12 gains one possible identifier, but its encoding (identifiers joined with `;`) does not change, so `csv_format_version` **stays 1** (csv-export.md section 3). Agree? | **Decided at G1 (owner, 2026-10-25):** contract version 6, `csv_format_version` 1 |
| Q24 | **Calibration report additions** (calibration/lossy.md section 6.8): a lossy verdict-change table between params files, a `streaming` count column and a `streaming rate` column (the share K5's gate reads, Q22) in the per-group table, per-sample-rate rows for the negative groups and `fc` percentiles per group; and the per-upstream and per-sample-rate rows of confirmed lossy fakes hold **strong labels only** (weak labels only in the `weak` row, lossy.md 6.3), so that K4 reads strong labels as it says (before this, `confirmed_fake 48000` on the owner's corpus held 152 rows: 134 strong and 18 weak). Without them, K2, K5 and K6 cannot be read from `summary.md` (the existing verdict-change table covers lossless rows only, lossy.md section 6.1) | **Decided at G1 (owner, 2026-10-25):** yes, with the `streaming rate` column that the K5 gate (Q22) needs (same as calibration/lossy.md C13) |
| Q25 | **Changes to approved specs and protected tests** in this PR: (a) `rules.md` section 4 and its introduction: the defaults and `algorithmVersion` become v7/7 and the next free `algorithm_version` becomes **8**; no lossless rule or value changes; `detection-edge-step.feature`'s header says that the v7 defaults give v6's lossless results and that a step checking `algorithmVersion 6` uses the v6 file; (b) `detection-lossless.feature`: "Parameters come from the versioned file" names `detection-params.v7.toml` and `algorithmVersion 7`; (c) `detection-lossy.feature`: its header says that "DetectionParams v5" means a params set **without** `[lossy.streaming]` (`detection-params.v5.toml` or `.v6.toml`), not the v7 defaults, because the row "48 kHz MP3 from Opus (known limitation)" and the fixture `mp3_320_48k_lp20k` are `suspicious` with v7; (d) `decode-lossy.feature`: header and "A lossy row has the same fields as a lossless row" expect `algorithmVersion 7`; (e) `calibration.feature`: the proposals of "The grid search proposes new parameters from tune data" and "The full-band guard can be searched from a v3 baseline" are `detection-params.v8.toml` / `algorithm_version 8`; `calibration/evaluation.md` section 7 and `calibration/sample-corpus.md` sections 4–5: next free version 8; `calibration/evaluation.md` section 9: the v7 file belongs to the v2 rule family and its `[lossy.streaming]` checks fail with exit 2 like the others; (f) `contracts/commands.md` version 6 (section "Streaming sample rate (version 6)"); `contracts/csv-export.md` section 3 (a note, no format change); (g) `ui/lossy-formats.md` (the reason text); (h) `calibration/lossy.md` sections 6.3 and 6.8 (Q24; the strong-labels-only rows change the calibration tool's group selection, `tools/fakeflac-calibrate/src/lossy.rs`, but no existing test: `tools/fakeflac-calibrate/tests/lossy_acceptance.rs` reads the `weak` row and the strong `confirmed_fake aac` row, never a per-upstream or per-sample-rate row holding a weak label), and `calibration-lossy.feature`'s header, which says that "Lossy table per group" ("contains the rows") stays valid unchanged: its rows keep their counts, the new `@ <bucket>` sub-rows are extra rows (only `@ 44100`, every lossy fixture of the variant being 44.1 kHz), the step compares only the columns it names, and the run's v2 and v5 files have no `[lossy.streaming]`; (i) `lossy-fixtures.md`: three new fixtures, a v7 truth column note and the window-margin paragraph, with an extra condition in check 2 of its section 3 for the two 48 kHz fixtures with a 20000 Hz edge; (j) the protected tests that change in the test stage, as a declared consequence: `crates/fakeflac-core/tests/detect_lossy.rs` (its `v5()` returns the defaults; it must return them with `lossy.streaming = None`, since the crate cannot read a file (it has no `toml` dependency), or the 48 kHz rows, `rf15_known_limitation_a_48k_mp3_from_opus_is_real` included, fail; plus the two `algorithm_version` assertions, 6 → 7, of `rf15_analyze_track_fills_the_estimate_and_the_cutoff_of_a_320_made_from_128` and `rf15_analyze_track_of_a_silent_lossy_track_has_no_cutoff_and_no_estimate`), `crates/fakeflac-audio/tests/detection_lossy.rs` (`rf15_the_default_params_give_the_same_analysis_as_the_v5_file`), `crates/fakeflac-audio/tests/detection_lossless.rs` (`default_params_equal_versioned_file`, including the derived v5 params, which also need `lossy.streaming` cleared, and `every_analysis_carries_algorithm_version_6`), `crates/fakeflac-audio/tests/detection_edge_step.rs` (`rf04_v6_gives_the_v5_verdicts_on_every_other_fixture`, which takes the defaults as its v6 params and checks `algorithm_version` 6), `crates/fakeflac-core/tests/{detect_rules,params_lossy}.rs` (`rf04_v3_defaults_are_v3`, `rf15_the_defaults_hold_the_v5_lossy_values`) and `src-tauri/tests/run_analysis_lossy_v5.rs` (`rf15_a_lossy_row_has_the_same_fields_as_a_lossless_row`, `rf15_a_variable_bitrate_is_marked_in_the_table`). The calibration tests need no change (they compute the next free version from the files); only the comment at `tools/fakeflac-calibrate/tests/grid.rs:58` ("algorithm_version 6") is stale. `crates/fakeflac-audio/tests/params_edge.rs` needs no change (it reads the v6 file, not the defaults). `tools/fakeflac-calibrate/tests/lossy_acceptance.rs` reads rows by name and does not compare the whole text, so the 6.8 sub-rows do not change it. The test author lists the exact set in the `test(RF-16)` commit; assertions on other values do not change. Agree? | **Decided at G1 (owner, 2026-10-25):** yes to all, (a) to (j) |
| Q26 | **ADR.** Every detection version so far has its ADR (v2 ADR-0035, v3 ADR-0038, v4 ADR-0039, v5 ADR-0041, v6 ADR-0042). Using the **sample rate as evidence** about a lossy file's history is a new kind of signal (not spectral, not from the stream's declared quality) and a new reason in the contract. So it has its ADR: `docs/adr/0043-detection-v7-streaming-sample-rate-as-evidence.md` (ADR-0043, accepted at G1, #78), written in the spec PR and approved with it at G1 | **Decided at G1 (owner, 2026-10-25):** ADR-0043, approved with this spec |
| Q27 | **Priority and milestone** of RF-16: `medium`, H2 (an indicator that only produces `Suspicious`; RF-15 stays the main lossy rule) | **Decided at G1 (owner, 2026-10-25):** medium, H2 |
| Q28 | **The reason's text** (`ui/lossy-formats.md`). The first draft was "Full band at 48 kHz: possibly ripped from a streaming site such as YouTube. Genuine 48 kHz releases look the same." (114 characters; the longest existing reason has 45); the alternative was one shorter sentence that keeps the "why only `Suspicious`" clause. Either names "48 kHz" literally, so a later params version that adds another rate changes the text too (a spec change anyway). With the decided window (Q21), "Full band" stays accurate in Opus's terms (Opus's full band ends at 20 kHz, the edge the rule looks for). Spanish is not added to the catalogue now: the Spanish catalogue only overrides the verdicts until RF-13; a proposed Spanish text is recorded for RF-13 | **Decided at G1 (owner, 2026-10-25):** the short text, exactly "Full band at 48 kHz: possibly a streaming rip; genuine 48 kHz files look the same." (82 characters), copied literally into the two scenarios of `detection-streaming-source.feature` that assert it (tooltip and detail line). Proposed Spanish for RF-13: "Banda completa a 48 kHz: posible ripeo de streaming; los archivos auténticos a 48 kHz se ven igual." |

## Open questions for G1 (v5)

| # | Question | Status |
|---|---|---|
| Q1 | **One params file or two?** This spec extends `DetectionParams` with an optional `[lossy]` section in a new `detection-params.v5.toml` (v4 plus `[lossy]`, `algorithm_version` 5) instead of a separate lossy params file with its own version. Reasons in section 1 | Open. Default: one file (v5) |
| Q2 | Approve the **indicative** table and thresholds of section 6 without pre-measuring them, as Q1 of `rules.md` v3 did? Acceptance B (section 9) decides; a failed gate returns to G1 with the run's figures, and the implementer never tunes values | Open. Default: yes |
| Q3 | **New reason `cutoffBelowDeclared`** for a moderate shortfall with a clear step (rule L4). It is a contract change (bindings, UI text, `specs/ui/lossy-formats.md`). A lossy `Fake` carries no reason, as a lossless one; a shortfall without a clear step reuses `gradualRolloff` | Open. Default: yes |
| Q4 | **Contract additions** `TrackInfo.codec` (needed to tell AAC from ALAC inside M4A) and `TrackInfo.bitrateMode` (`cbr`/`vbr`/`abr`, shown next to the declared bitrate). Take both, only `codec`, or neither? | Open. Default: both |
| Q5 | `estimatedSourceBitrateKbps` of a lossy track with no shortfall rule firing is the **declared** bitrate (section 5), not the table row nearest to `fc`. Avoids showing "Real, ≈256 kbps" on a 300 kbps VBR file | Open. Default: declared |
| Q6 | The **LAME tag's low-pass** may only lower `c_exp` (section 3): a user-chosen low-pass is honoured, so a 320 kbps file encoded with `--lowpass 16` and content to 16 kHz is `Real` | Open. Default: yes |
| Q7 | `G`, `U`, `B`, per-frame bitrates and padding are **not used** by the lossy rules in v5 (reported by the calibration tool only). LAME 320 kbps encodes already show large `G` (rules.md, "Why v2 was replaced"), so `G` cannot separate an honest 320 from a 320-from-128 | Open. Default: not used |
| Q8 | A lossy track analysed with params **without `[lossy]`** (v2–v4 files) is `Error(unsupportedFormat)` with detail `these DetectionParams have no [lossy] section`. The app always uses v5; the calibration tool does not evaluate lossy rows with such files (calibration/lossy.md section 6) | Open. Default: yes |
| Q9 | **Opus is pending.** The scanner keeps skipping `.opus` files. The Opus scenarios are written in `specs/pending/opus.feature`. Pending features live in `specs/pending/`, which neither `just trace` (`scripts/trace.mjs` reads `specs/features/*.feature` only, not recursively) nor the test stage reads, so no test is derived from them and their tags are not checked. After the ADR-0030 spike, a later spec PR moves the file to `specs/features/` (dropping its `@pending-opus-decoder` tag, which is only a marker). RF-14 and RF-15 keep their scenarios in `specs/features/`, so `just trace` still passes. Optional note for the owner (agents may not edit CLAUDE.md files): `specs/CLAUDE.md` could say "Features in `specs/pending/` are specified but not testable yet; they move to `features/` in a later spec PR." | Open. Default: yes. **Superseded by v8** (section 12, Q38): ADR-0047 chose the decoder, `.opus` is scanned and the scenarios live in `specs/features/opus.feature` |
| Q10 | **HE-AAC** (SBR or PS signalled in the AudioSpecificConfig: object types 5 and 29) and MP3 layers I/II are `Error(unsupportedFormat)` (symphonia decodes AAC-LC and Layer III; ADR-0040) | Open. Default: yes |
| Q11 | Lossy test files are written by **bitstream fixture writers in test support**, with no new dependency (`lossy-fixtures.md`, ADR-0040), rather than an encoder dev-dependency or committed binary files | Open. Default: writers |
| Q12 | The **gate values** of section 9 (honest encodes `Fake` ≤ 1 %, high-contrast cascades caught ≥ 95 % and `Fake` ≥ 90 %, confirmed AAC-origin fakes caught ≥ 90 %). They are targets, not measurements | Open. Default: as written |
| Q13 | **Changes to approved specs and protected tests** in this PR: (a) `rules.md`: the default params and `algorithmVersion` become v5/5 and the next free `algorithm_version` becomes **6** (its Q8(d) said 5); its lossless rules and values do not change; (b) `detection-lossless.feature`: the scenario "Parameters come from the versioned file" names `detection-params.v5.toml` and `algorithmVersion 5`; (c) `calibration.feature`: the proposals of "The grid search proposes new parameters from tune data" and "The full-band guard can be searched from a v3 baseline" are `detection-params.v6.toml` / `algorithm_version 6`, the `trusted.txt` error for `folder.jpg` reads `not a supported audio file (FLAC, WAV, MP3, M4A or Opus)` the `confirmed.txt` unknown-kind error lists `fake, real or false-positive` (calibration/lossy.md C11), and "ffmpeg missing when a transcode is needed" uses the default config, now `calibration.v2.toml`; (d) `calibration/evaluation.md` section 7 and `calibration/sample-corpus.md` sections 4–5: next free version 6; (e) `contracts/commands.md` version 3 (section "Lossy formats"); (f) `ui/main-screen.md` and `ui/components.md`: two new table columns, the `BitrateValue` molecule, an optional `tooltip` prop of `ColumnHeader`, the lossy detail line and the `error.NO_AUDIO_FOUND` text (`ui/lossy-formats.md`); (g) the calibration specs as listed in `calibration/lossy.md`; (h) the protected tests listed in ADR-0041 ("Consequences") change in the test stage: Rust tests of the core, the audio adapter and `src-tauri`, the calibration tool's `inputs.rs` (the C11 messages), and the frontend tests of `ResultsTable` (ten columns), `i18n` and `AnalysisPage` (the `error.NO_AUDIO_FOUND` text). Agree? | Open. Default: yes to all |
| Q14 | Only `.m4a` is scanned as MP4 audio; `.mp4`, `.m4b` and raw ADTS `.aac` stay out of scope | Open. Default: yes |
| Q15 | ADR-0040 enables the symphonia feature `alac` as well as `mp3`, `aac` and `isomp4`: ALAC in M4A must be decoded to be treated as lossless (backend.md section 8) | Open. Default: yes |
| Q16 | **A tag frame whose frames or bytes field is 0** (section 2.1). A stream with audio cannot have 0 frames or 0 bytes, so the field is broken (an encode that never updated its tag). Treat it as absent, so the stream is counted? Today the adapter takes 0 literally: a frame count of 0 gives `durationMs` 0, so the file is `tooShort` after being decoded; a bytes field of 0 on a VBR or ABR file gives no declared bitrate, so it is `corruptStream` (`no bitrate`). Changing it is an adapter change (`EXTRACTION_VERSION`) | Open. Default: treat 0 as absent |
| Q17 | **Free-format MP3** (bitrate index 0, section 2.1). The decoder (symphonia 0.6.1) does not decode free bitrate, so such a file never gets a verdict. Report it as `unsupportedFormat` (a recognised but not decoded codec, section 2.3, like Layers I and II), with detail `free-format MP3 is not supported`? Today it ends as `corruptStream` (the decoder finds no frame it can read) or, if it finds one, `tooShort` (the adapter counts 0 frames) | Open. Default: `unsupportedFormat` |

## 1. Scope and versions

- **In:** MP3 (MPEG-1, 2 and 2.5 Layer III) and M4A with AAC-LC, judged against their declared bitrate; ALAC in M4A, judged as lossless with `rules.md`; the source-bitrate estimate.
- **Opus** (since v8): Ogg Opus with channel mapping family 0, judged against its declared bitrate with the `opus` rows of `detection-params.v8.toml` (sections 2.4 and 12). No `[lossy.cutoffs]` row exists for it in v5, v6 or v7 (rule L0). Until v8 it was specified but pending (`specs/pending/opus.feature`, Q9).
- **Out:** double-compression fingerprints in the MDCT domain (H3, backend.md section 9), upsamples (RF-12), FLAC-to-FLAC quality loss, and grid search over `[lossy]` values (calibration/lossy.md, C10).

**Why one file (Q1).** v5 is `detection-params.v4.toml` with every section and value unchanged plus the optional `[lossy]` section:
1. A `TrackAnalysis` carries one `algorithmVersion`, and the app's cache key has one (ADR-0008). Two files with two versions would need a second number in the contract and in the key.
2. The lossy rules reuse the lossless measurements and their parameters (`[input]`, `[fft]`, `[bands]`, `[extent]`, `[step]`, `verdict.low_hf_threshold_db`, `verdict.step_suspicious_db`). A separate file would have to repeat them or point at another file, and the two could disagree.
3. The calibration tool already compares whole files side by side (calibration/evaluation.md section 9); one file per version keeps that.

The cost: a change to a lossy value is a new version for lossless results too, so cached lossless results are recomputed once. Accepted.

**Version numbering.** `algorithm_version` numbers are allocated by the files in `specs/detection/` (rules.md section 4). This spec allocates **5**. With `detection-params.v5.toml` in the repository, the **next free number is 6**: `/calibrate` proposals and the sample corpus's `candidate.toml` take 6 (Q13 (a), (c), (d)); the tool and its tests already compute it from the files (`next_free_version()`), so nothing in the code allocates it by hand. A later change to either part (lossless or lossy) takes 6, and both `rules.md` and this spec then cite the file that introduced it. **v5 becomes the default**: `DetectionParams::default()` equals `detection-params.v5.toml`. **Since detection v6** (rules.md, ADR-0042), which took 6, the default is `detection-params.v6.toml`: its `[lossy]` section and every value these rules read are v5's, so lossy files get exactly their v5 verdicts, reasons and estimates, with `algorithm_version` 6; the next free number is **7**. **Since detection v7** (section 11, RF-16), which takes 7, the default is `detection-params.v7.toml` (v6 plus `[lossy.streaming]`); lossy files get their v6 results except where rule L6a fires, lossless files get exactly their v6 results, and the next free number is **8**.

**v2, v3 and v4 stay loadable.** `[lossy]` is optional. Without it, lossless files get exactly their v2/v3/v4 verdicts (nothing in `rules.md` reads `[lossy]`), and lossy files are `Error(unsupportedFormat)` (Q8).

## 2. Stream information (RF-14)

The audio adapter (`fakeflac-audio`) reports, besides the band spectra, what the stream declares. It reads the file read-only (RNF-05) and never needs an external tool. New fields of `AudioProbe` (core types, no I/O in the core): `codec`, `declared_bitrate_kbps`, `bitrate_mode`, `encoder` and `encoder_lowpass_hz`. The first three reach the contract (`specs/contracts/commands.md`, Q4); `encoder` and `encoder_lowpass_hz` stay in the backend and the calibration tool.

| Field | WAV, FLAC | MP3 | M4A, AAC-LC | M4A, ALAC | Opus (v8, 2.4) |
|---|---|---|---|---|---|
| `codec` | `pcm`, `flac` | `mp3` | `aac` | `alac` | `opus` |
| `declared_bitrate_kbps` | absent | section 2.1 | section 2.2 | absent | section 2.4.3 |
| `bitrate_mode` | absent | `cbr`, `vbr` or `abr` (2.1) | absent (AAC does not signal it reliably) | absent | absent (Ogg Opus does not signal it; Q32) |
| `encoder`, `encoder_lowpass_hz` | absent | from the LAME tag (2.1) | absent (iTunes tags are not read in v5) | absent | absent (Q32) |
| `bit_depth` | as in H1 | absent | absent | from the ALAC configuration | absent |

The **sample rate** of a lossy file is the decoded rate (44.1 or 48 kHz for most files; always 48 kHz for Opus). **Channels** are mixed to mono for the analysis, as in H1 (rules.md section 1.1).

### 2.1 MP3

- **Tag frame.** The first frame of the stream may be a tag frame instead of audio: `Xing` or `Info` (written by LAME and ffmpeg) or `VBRI` (Fraunhofer). The adapter parses it itself from the bytes of that frame (no tag library; ADR-0040). A tag frame is not audio. A frames or bytes field equal to 0 is treated as absent, so the value is counted from the audio frames instead (Q16).
- **LAME extension.** When an `Xing`/`Info` frame carries the LAME extension (a 9-byte encoder field of printable ASCII after the Xing fields), the adapter reads: `encoder` = that field with trailing spaces and NUL bytes removed (for example `LAME3.100`, `Lavc60.31`); the **VBR method** (low 4 bits of the next byte); and `encoder_lowpass_hz` = the following byte × 100 Hz, **absent when the byte is 0**.
- **Mode.**
  - `Info` → `cbr`.
  - `Xing` → `vbr`, unless the LAME VBR method is 1 or 8 (`cbr`) or 2 or 9 (`abr`).
  - `VBRI` → `vbr`.
  - No tag frame: `cbr` when every audio frame of the file has the same bitrate index, otherwise `vbr`. A free-format stream (bitrate index 0) has no mode: it is an error (last item of this section).
- **Declared bitrate** (kbps, an integer):
  - `cbr`: the bitrate of the frame header of the first audio frame.
  - `vbr` and `abr`: the average, `round(8 · bytes / duration_s / 1000)` (halves away from zero), where `bytes` is the bytes field of the tag frame when it has one (as LAME writes it: every frame of the stream **including the tag frame**, ID3 tags excluded; the adapter uses the value as written) and otherwise the total size of the audio frames, and `duration_s` = `frames · samples_per_frame / sample_rate` with `frames` from the tag frame's frames field when it has one (the number of **audio** frames, the tag frame excluded) and otherwise counted. Counting the tag frame's bytes raises the average by about one frame in a thousand, far below the rounding to 1 kbps.
- **Duration.** `frames · samples_per_frame / sample_rate`, with `frames` from the tag frame's frame count when present; otherwise the stream is counted as in rules.md section 1.2 (a missing length is never reported as silence). Encoder delay and padding are not subtracted, so `durationMs` may exceed the audible length by the encoder delay plus padding (`enc_delay + enc_padding` of the LAME extension; up to about 2.5 frames for LAME).
- **Decoded length.** The decoder trims the encoder delay and padding, so it may return fewer samples than `frames · samples_per_frame`. The stream is `corruptStream` (it ended early, RNF-04) only when the decoded samples fall short of that length by more than a tolerance: `enc_delay + enc_padding` plus two frames when the LAME extension's encoder field starts with `LAME`, `Lavf` or `Lavc` (the encoders whose delay and padding fields, bytes 21 to 23 after the Xing fields, the adapter reads), otherwise three frames.
- Layer I and II streams are `Error(unsupportedFormat)` (Q10).
- A **free-format** stream (bitrate index 0) is not decoded, so it has no declared bitrate, duration or mode: `Error(unsupportedFormat)` with detail `free-format MP3 is not supported` (Q17).

### 2.2 M4A

- The first audio track of the MP4 container. The codec comes from its sample entry: `mp4a` with AAC object type 2 (LC) → `aac`; `mp4a` with object type 5 or 29 (HE-AAC v1, v2) or any other object type → `Error(unsupportedFormat)` with detail `HE-AAC is not supported` or `AAC object type N is not supported` (Q10); `alac` → `alac`; any other sample entry → `Error(unsupportedFormat)`.
- **AAC declared bitrate:** `round(8 · Σ sample sizes of the track / duration_s / 1000)`, with `duration_s` from the track's media header. The `esds` average bitrate field is not used: encoders fill it inconsistently.
- **ALAC** is lossless: it is analysed exactly like FLAC with `rules.md` (`TrackInfo.format` stays `m4a`, `codec` is `alac`).

### 2.3 Errors

As in H1 (ADR-0013, RNF-04): an unreadable file is `unreadable`; a container or bitstream that cannot be parsed or decoded is `corruptStream`; a codec that is recognised but not decoded is `unsupportedFormat`; a track shorter than `min_duration_s` is `tooShort`. **The duration is checked before the declared bitrate**: a stream shorter than `min_duration_s`, a zero-duration stream included, is `tooShort`, whatever its bitrate. The order of these errors against rule L0 is in section 4. **A lossy stream that is long enough but has no declared bitrate after section 2.1 or 2.2 is `corruptStream`** (`no bitrate`); with the fallbacks above this should not happen, so it is a guard, not an expected case. A panic in a decoder is caught and becomes `corruptStream` (as in H1).

### 2.4 Opus (detection v8, ADR-0047)

An `.opus` file is read as **one Ogg logical stream that carries Opus** (RFC 3533, RFC 7845). The Ogg layer is read by a module of `fakeflac-audio` itself (no `ogg` crate, ADR-0047 point 2); the packets are decoded with `rusty-opus` `=1.0.2`. The adapter first reads and checks the **whole Ogg structure** (2.4.1 and 2.4.2), and only then decodes (2.4.3), so a structural error is always reported before any decoder error, and the declared bitrate and duration are known before decoding. The file is opened read-only (RNF-05).

#### 2.4.1 Pages and packets

- **Page.** A 27-byte header, a segment table and a body (RFC 3533 section 6): capture pattern `OggS`, stream structure version, header type flags (`0x01` continued packet, `0x02` beginning of stream, `0x04` end of stream), granule position (signed 64-bit LE), serial number (u32 LE), page sequence number (u32 LE), CRC (u32 LE), number of segments, then that many lacing values. The body length is the sum of the lacing values.
- **CRC-32.** Polynomial `0x04C11DB7`, initial value 0, no bit reflection, no final XOR, computed over the whole page with the CRC field taken as zero; it must equal the CRC field.
- **Layout.** The file starts with `OggS` at byte 0. Pages follow each other with no bytes between them, and the last page ends at the end of the file. There is no resynchronisation: bytes that do not start a page are an error (2.4.4).
- **Packets.** A packet is the concatenation of consecutive segments; a lacing value below 255 ends it. A page whose last lacing value is 255 leaves a packet **unfinished**; the next page continues it and must have the continued flag; a page with the continued flag must follow a page that left a packet unfinished.
- **One logical stream.** The first page has the beginning-of-stream flag. Every page has the serial number of the first page. Each page's sequence number is the previous page's plus 1. No page follows a page with the end-of-stream flag; the end-of-stream flag itself is **not required** (a file whose last page lacks it is read up to its last page, Q33).
- **Granule positions.** Only the pages that complete at least one **audio** packet are checked: their granule positions never decrease. The granule positions of the header pages and of pages that complete no packet are not checked.

#### 2.4.2 Header packets

- **OpusHead** (packet 0) is the only packet of page 0 and is complete on it. Magic `OpusHead`; length at least 19 bytes; version byte with major version (high 4 bits) 0 (minor versions 0–15 are accepted); channel count at least 1; pre-skip (u16 LE); input sample rate (u32 LE, **informational only**: an Opus stream always decodes at 48 kHz, so it never reaches `TrackInfo`); output gain (i16 LE, Q7.8 dB); channel mapping family.
  - **Family 0** (mono and stereo): channel count 1 or 2. Bytes after the 19th are ignored (RFC 7845 allows them for later minor versions).
  - **Any other family** (multichannel, ADR-0047 point 4): `Error(unsupportedFormat)` with detail `channel mapping family N is not supported`, whatever the rest of the header holds; the packets are never decoded.
- **OpusTags** (packet 1) is the first packet of page 1, may span several pages, and the page where it ends completes no other packet and leaves none unfinished (the audio starts on a new page). Magic `OpusTags`; vendor string length (u32 LE) and vendor string, user comment count (u32 LE), and for each comment its length (u32 LE) and bytes, all inside the packet; bytes after the last comment are allowed. Its contents are checked, not reported (Q32).
- **Audio packets**: every packet after OpusTags. There must be at least one.

#### 2.4.3 Stream information and decoding

Names: `pre_skip` from OpusHead; `S` = the number of samples (per channel, at 48 kHz) that all audio packets decode to, from each packet's TOC (RFC 6716 section 3.1); `G` = the granule position of the **last page that completes an audio packet**; `P` = the total size in bytes of the audio packets (page headers, segment tables and the two header packets excluded). An audio packet whose TOC (or, for code 3, its frame-count byte) cannot be parsed while computing `S` (RFC 6716 section 3.2, for example code 3 with a frame count of 0) is reported as **R22**, with detail `Opus packet i: <reason>` and `i` counted from audio packet 0; this check runs before the stream checks below (R20, R21), so such a file never reaches the decoder.

- **Stream checks** (after the structure, in this order): `G ≥ pre_skip`, otherwise `corruptStream` (`granule position below pre-skip`); `G ≤ S`, otherwise `corruptStream` (`granule position beyond the audio`).
- **Duration.** `L = G − pre_skip` samples per channel; `durationMs = floor(1000 · L / 48000)`. The pre-skip and the end trim (`S − G`) are excluded, so this is the audible length (unlike MP3, section 2.1).
- **Declared bitrate** (kbps, integer): `round(8 · P / (L / 48000) / 1000)`, halves away from zero (Q31). With `L` = 0 the track is `tooShort` first (section 2.3).
- **Sample rate** 48000 Hz always; **channels** = the OpusHead channel count (1 or 2); no bit depth; codec `opus`; no bitrate mode, encoder or encoder low-pass (Q32).
- **Decoding.** One `rusty-opus` decoder at 48 kHz with the header's channel count, `f32` output, every audio packet in stream order (no seeking in v8; ADR-0047 "Speed"). From the concatenated output the adapter **drops the first `pre_skip` samples** per channel and **keeps exactly `L`**, dropping the end trim; then it multiplies every kept sample by the **output gain** `10^(gain / 5120)` (gain in Q7.8 dB, so `gain / 256` dB). The result is mixed to mono for the analysis (rules.md section 1.1). The decoded length is always `L`, so the MP3 "decoded length" tolerance of section 2.1 has no Opus equivalent.
- A decoder error on audio packet `i` (counted from 0) is `corruptStream` with detail `Opus packet i: <the decoder's error>`; a panic is caught and becomes `corruptStream` (section 2.3).
- An Opus track whose decode fails before its stream was read keeps `format` `opus` and `codec` `opus` (from the extension, as `.mp3` does, `commands.md` version 3).

#### 2.4.4 Errors (`corruptStream` unless stated)

Pages are read in file order, and the checks of one page run in the order below; a header packet is checked as soon as it is complete; the end-of-file checks run after the last page; the stream checks of 2.4.3 run last. The first failure is reported. Each detail is fixed text (scenarios: `opus.feature`, "Broken Ogg or Opus structure is corruptStream"; fixtures: `lossy-fixtures.md` section 2.2).

| # | Condition | Detail |
|---|---|---|
| R1 | The bytes where a page should start (byte 0, or right after a page) are not `OggS`; an empty file | `not an Ogg stream` |
| R2 | The file ends inside a page header, its segment table or its body | `truncated Ogg page` |
| R3 | Stream structure version ≠ 0 | `unsupported Ogg page version` |
| R4 | CRC mismatch | `Ogg page CRC mismatch` |
| R5 | A serial number other than the first page's (a chained or multiplexed file) | **`unsupportedFormat`**: `multiplexed or chained Ogg streams are not supported` |
| R6 | The first page lacks the beginning-of-stream flag | `first Ogg page is not a beginning of stream` |
| R7 | A page after a page with the end-of-stream flag | `Ogg page after end of stream` |
| R8 | A sequence number other than the previous page's plus 1 | `Ogg page out of sequence` |
| R9 | The continued flag is set with no unfinished packet, or clear while a packet is unfinished | `Ogg packet continuation mismatch` |
| R10 | Packet 0 is not a complete, lone packet of page 0 with magic `OpusHead` | `no OpusHead` |
| R11 | OpusHead shorter than 19 bytes, major version ≠ 0, channel count 0, or family 0 with more than 2 channels | `invalid OpusHead` |
| R12 | OpusHead family ≠ 0 (after R11's length, version and channel-count checks) | **`unsupportedFormat`**: `channel mapping family N is not supported` |
| R13 | Packet 1 does not start page 1 or its magic is not `OpusTags` | `no OpusTags` |
| R14 | An OpusTags length field runs past the end of the packet | `invalid OpusTags` |
| R15 | The page where OpusTags ends completes another packet or leaves one unfinished | `audio data on the OpusTags page` |
| R16 | An audio page that completes a packet has a granule position lower than the previous such page's | `Ogg granule position decreases` |
| R17 | End of file with an unfinished packet | `unfinished Ogg packet at end of stream` |
| R18 | End of file with fewer than two packets (the stream ends before OpusTags is complete) | `no OpusTags` |
| R19 | End of file with no audio packet | `no Opus audio packets` |
| R20 | `G < pre_skip` | `granule position below pre-skip` |
| R21 | `G > S` | `granule position beyond the audio` |
| R22 | The TOC of audio packet `i` cannot be parsed while computing `S` (section 2.4.3, checked before R20 and R21), or the decoder rejects audio packet `i` | `Opus packet i: <the reason or the decoder's error>` |

These are the reading errors of rule L1a (section 4). After them the order of section 4 applies as for the other lossy formats: L0 (`no [lossy.cutoffs] rows for opus` with params v5–v7), then L1b (`tooShort` when `durationMs` is below `min_duration_s`; a missing declared bitrate cannot happen once `L` > 0).

## 3. Measurements

Every lossy file gets the spectra of `rules.md` section 1 and the measurements of `rules.md` section 2 with the same parameters: `L_ref`, `F`, `H`, `fe`, `D` at `fs`, `G`, `U`, `B`. A silent track (no frame above `silence_dbfs`) is `Suspicious` with `lowHighFrequencyContent`, no cutoff and no estimate, as in rules.md section 1.6. Then:

| Quantity | Definition |
|---|---|
| **Content edge `fc`** | the **reported cutoff frequency** of rules.md section 2: `fc = fs` when `D ≥ step_suspicious_db`, otherwise `fc = fe`. So the cutoff shown in the table is the edge the lossy rules judge |
| **Table cutoff `T(codec, b)`** | the `cutoff_hz` of the row of `lossy.cutoffs.<codec>` with the largest `bitrate_kbps ≤ b`. **Undefined** when `b` is below the first row's bitrate (nothing can be expected of a stream below the table) |
| **Tag low-pass `L_tag`** | `encoder_lowpass_hz` when present and in `[cutoff_hz of the codec's first row, nyquist]`; otherwise undefined (a corrupt or meaningless byte is ignored). Always undefined for AAC and Opus (section 2) |
| **Expected cutoff `c_exp`** | `min(T(codec, b), L_tag, nyquist − step_search_top_margin_hz)`, ignoring an undefined `L_tag`. **Undefined** when `T(codec, b)` is undefined |
| **Shortfall `s`** | `c_exp − fc` when `c_exp` is defined; undefined otherwise. Positive when the content stops below the expected cutoff |

- **Variable bitrate.** `b` of a VBR or ABR file is its average (section 2.1), looked up in the same table. LAME's VBR presets keep a low-pass at or above that of a CBR file of their average bitrate, so the lookup errs on the lenient side (to be confirmed by B2 for `v0` and `v2`).
- **Between rows.** A declared bitrate between two rows uses the lower row, so a 300 kbps average is judged against the 256 kbps row: lenient on purpose.
- **Nyquist cap.** At 32 kHz (Nyquist 16 kHz) a 320 kbps row's 20.5 kHz cannot be reached; `c_exp` is capped at the top of the step search range, where `fs` can still be found.
- **`L_tag` only lowers `c_exp`** (Q6): an encoder told to keep 16 kHz at 320 kbps declares that its content stops there. It can never make a verdict stricter than the table.
- **Mono.** The tables assume stereo encodes. Encoders give a mono stream of the same bitrate a higher or equal low-pass, so for mono files the rules can only miss a fake, never invent one (section 8).
- **Where it is computed.** `fakeflac-core` exposes these quantities, the verdict of section 4 and the estimate of section 5 as **pure public functions** of the measurements, the stream information of section 2, the sample rate and the params (no I/O), next to `measure`, `decide` and `cutoff_estimate` in `fakeflac_core::detect`. `analyze_track` and the calibration tool both call them, so the tool reimplements no detection step (ADR-0036); exact names and signatures are left to the implementation.

## 4. Verdict (RF-15)

For a track whose `codec` is lossy. Rules are evaluated in the order of the table rows (L1a before L0); the first match wins. Lossless codecs (`pcm`, `flac`, `alac`) use `rules.md` section 3 unchanged.

| # | Condition | Verdict | Reasons | `estimatedSourceBitrateKbps` |
|---|---|---|---|---|
| L1a | Reading, probing or decoding failed (section 2.3: `unreadable`, `corruptStream`, or `unsupportedFormat` from sections 2.1, 2.2 and 2.4, such as HE-AAC, MP3 Layer II or an Opus channel mapping family other than 0) | `Error` | — (the `TrackError` explains it) | null |
| L0 | The params have no `[lossy]` section (Q8): `unsupportedFormat` with detail `these DetectionParams have no [lossy] section`; or `lossy.cutoffs` has no row for the codec (Opus in v5, v6 and v7): `unsupportedFormat` with detail `no [lossy.cutoffs] rows for <codec>` (for example `no [lossy.cutoffs] rows for opus`) | `Error` | — | null |
| L1b | Too short, or no declared bitrate, in that order (section 2.3) | `Error` | — (the `TrackError` explains it) | null |
| L2 | `H < low_hf_threshold_db` (little treble) | `Suspicious` | `lowHighFrequencyContent` | null |
| L3 | `s` defined, `s ≥ fake_shortfall_hz` and `D ≥ fake_min_step_db` | `Fake` | — | section 5 |
| L4 | `s` defined, `s ≥ suspicious_shortfall_hz` and `D ≥ step_suspicious_db` | `Suspicious` | `cutoffBelowDeclared` | section 5 |
| L5 | `s` defined and `s ≥ suspicious_shortfall_hz` | `Suspicious` | `gradualRolloff` | section 5 |
| L6a | (v7, RF-16) The params have `[lossy.streaming]`, and `StreamingRate` holds (section 11.2): codec, decoded sample rate, declared bitrate and `fc` all match that section | `Suspicious` | `streamingSampleRate` | null |
| L6 | Otherwise | `Real` | — | `b` |

A silent track is decided after L1b and before L2 (section 3), with the same result as L2 but no cutoff. L0 depends only on the codec and the params, never on the spectra. It comes **after** L1a for every lossy format, MP3 included: an M4A has to be probed to know whether it holds AAC or ALAC, so both formats follow one order, and a broken file keeps the more informative `corruptStream` (or `unreadable`, or the `unsupportedFormat` of an undecoded codec) whatever the params. It comes **before** L1b: with params that cannot judge the codec, a short or bitrate-less lossy file is `unsupportedFormat`, not `tooShort` or `corruptStream`. In v5–v7 the app never meets an Opus file, because the scanner skips `.opus` (Q9); the row exists so that the core has a defined answer. Since v8 the scanner lists `.opus` and the defaults have Opus rows, so L0 for Opus is met only with an older params file (the calibration tool, tests).

Consequences (they rely on the checks of section 6):
- **`Fake` always rests on a clear step:** L3 needs `D ≥ fake_min_step_db ≥ step_suspicious_db`, so `fc = fs`. A track whose content fades without a step (`D < step_suspicious_db`, `fc = fe`) is at most `Suspicious` (L5), whatever its shortfall, in the spirit of planning section 2.5 ("never `Fake` only because of the spectrum shape").
- **L4 covers both a moderate shortfall with a clear step and a large shortfall with a step that is clear but not deep** (`step_suspicious_db ≤ D < fake_min_step_db`).
- **Content above the expected cutoff is never held against a file** (`s < 0`): it is `Real` (since v7, `Suspicious` by L6a when its sample rate and cutoff match `[lossy.streaming]`; that is evidence about the sample rate, not a shortfall).
- **A declared bitrate below the table** (for example a 24 kbps stream) has no `c_exp`, so the file is `Real` unless L2 (or, since v7, L6a) fires.
- **The lossless rules never apply to a lossy codec**, and the lossy rules never apply to a lossless codec: an honest MP3 has a low-pass by design, so V4 of rules.md would call every one `Fake`.
- **No `FullBand`, gap or lossy-range logic.** `G`, `U`, `B` and the range `[lossy_min_cutoff_hz, fake_max_cutoff_hz)` of rules.md are not used here (Q7).
- **L6a (v7) only splits L6.** It is reached only when L1a–L5 did not match, so it never overrides an `Error`, `Fake` or `Suspicious` of an earlier rule and never produces `Fake`. With a params file without `[lossy.streaming]` (v5, v6) it never fires and every v5 consequence above holds unchanged (section 11).

**Reported cutoff.** `CutoffEstimate` exactly as in rules.md section 2 ("Reported cutoff"), so its `frequencyHz` equals `fc`. It is `null` only for `Error` and silent tracks.

## 5. Estimated source bitrate (`estimatedSourceBitrateKbps`)

- **Lossless codecs, `Error`, silent tracks and L2:** `null`. Nothing reliable can be said.
- **L6a (v7):** `null` (section 11.2, Q19). The content agrees with the declared bitrate, so it says nothing about the source's bitrate.
- **L6 (`Real`):** the declared bitrate `b` (Q5): the content is consistent with it.
- **L3, L4, L5 of an Opus track (v8):** `null` (section 12.4, Q29). Every Opus row has the same cutoff, so no Opus bitrate corresponds to an early edge.
- **L3, L4, L5** (MP3 and AAC): `min(b, r*)`, where `r*` is the `bitrate_kbps` of the row of `lossy.cutoffs.<codec>` whose `cutoff_hz` is **nearest to `fc`** (on a tie, the lower bitrate). An `fc` below the first row gives the first row's bitrate, so the value reads as "at most".

The estimate is expressed **in the declared codec's terms**: an MP3 320 made from a streaming site's AAC 128 stream (content to about 16 kHz) shows the MP3 bitrate whose low-pass is nearest, ≈112–128 kbps, not "AAC 128". The contract and the UI call it the real bitrate, and the help explains it (`ui/lossy-formats.md`).

Examples with the v5 table: a 320 kbps MP3 with `fc` = 17000 Hz → `Fake`, estimate 128; with `fc` = 18500 Hz → `Suspicious` (`s` = 2000), estimate 192 (nearest row 18600); a 256 kbps AAC with `fc` = 15500 Hz → `Fake` (`s` = 3500), estimate 128.

## 6. Parameters and validation

**Defaults** (`detection-params.v5.toml`). All values are **indicative starting values, not measured on the corpus** (Q2):

| Parameter | Value | Why this starting value |
|---|---|---|
| `lossy.suspicious_shortfall_hz` | 1000 | Larger than the gap between a band-centre `fs` and an encoder's nominal low-pass (LAME's transition band is about 500 Hz wide; `fs` is a 100 Hz band centre), so an honest encode is not flagged; small enough that a 192 kbps MP3 from a 128 kbps source (`s` ≈ 1600) is `Suspicious` |
| `lossy.fake_shortfall_hz` | 2500 | With the v5 table it makes a 320 from ≤ 160 kbps (`s` ≥ 3000) and a 256 from ≤ 128 kbps (`s` ≥ 2700) `Fake`. A 320 from 192 (`s` ≈ 1900), a 320 from 224 (`s` ≈ 1100) and a 256 from 160 (`s` ≈ 2200) stay `Suspicious` |
| `lossy.fake_min_step_db` | 10 | The value of `step_fake_db`: a `Fake` needs the same depth of step as on lossless files |
| `lossy.cutoffs.mp3` | 32 → 5500 … 320 → 20500 Hz (14 rows) | LAME 3.99–3.100's default low-pass per CBR bitrate, as documented for LAME (not verified in this repository). The calibration config records lower `lowpass_hz` for `cbr128` (16000) and `cbr256` (19500); those values only decide undetectability there. B2 measures honest LAME encodes of every setting and decides |
| `lossy.cutoffs.aac` | 64 → 11000 … 320 → 19500 Hz (7 rows) | A conservative value across common AAC-LC encoders: honest AAC files must not fall below it. B2 (ffmpeg's native encoder) and B3 (streaming-site AAC 128 copied without re-encoding) decide |
| `lossy.cutoffs.opus` (v8 only) | 24, 32, 48, 64, 96, 128, 160, 192 → 20000 Hz (8 rows) | libopus's full band ends at 20 kHz from 24 kbps up (section 12.3, Q30); acceptance O decides (section 12.7) |

**Shape** (serde, at deserialisation; unknown fields rejected in every table): `[lossy]` may be absent; if present, `suspicious_shortfall_hz`, `fake_shortfall_hz`, `fake_min_step_db` and `[lossy.cutoffs]` are required. In `[lossy.cutoffs]`, `mp3` and `aac` are required arrays and `opus` is optional; any other key is an error. Each row is `{ bitrate_kbps: integer, cutoff_hz: float }`, both required.

**Values** (`DetectionParams::validate`, rules.md section 4, extended; the error names the inequality, written as below):
- `lossy.suspicious_shortfall_hz > 0`
- `lossy.suspicious_shortfall_hz ≤ lossy.fake_shortfall_hz`
- `step_suspicious_db ≤ lossy.fake_min_step_db` (makes "`Fake` rests on a clear step", section 4, an invariant)
- for each codec present: `lossy.cutoffs.<codec> is not empty`, `lossy.cutoffs.<codec>: bitrate_kbps > 0`, `lossy.cutoffs.<codec>: cutoff_hz > 0`, `lossy.cutoffs.<codec>: bitrate_kbps increases` (strictly, in file order) and `lossy.cutoffs.<codec>: cutoff_hz does not decrease`.

**Order.** `validate()` runs the checks of rules.md section 4 first, then these in the order listed, codec by codec in the order `mp3`, `aac`, `opus`, and reports the **first** check that fails (so swapping the 112 and 128 kbps rows of `mp3`, which breaks both row checks, is reported as `lossy.cutoffs.mp3: bitrate_kbps increases`). A comparison with a NaN operand does not hold, so NaN is rejected. Every loader calls `validate()` (the calibration tool exits 2, evaluation.md section 9); a test checks that the defaults are valid.

**v7.** The optional subsection `[lossy.streaming]` has its own shape and value checks, run after the ones above (section 11.3).

**Invariants.** Deterministic; read-only; `algorithm_version` of every `TrackAnalysis` = that of the params (5 with v5; 6 with v6; 7 with v7; 8 with the defaults since detection v8); the defaults equal `detection-params.v8.toml` since detection v8, whose `[lossy]` section is v5's plus `[lossy.streaming]` (v7) plus the `opus` rows of `[lossy.cutoffs]` (v8) (a test checks it, rules.md section 4); `[input]` and `[fft]` are unchanged, so the spectra of lossless files are the same as with v2–v4 (calibration/lossy.md section 5 on `EXTRACTION_VERSION`).

## 7. Test signals and scenarios

- Lossy test files are generated inside the tests by small bitstream writers (`specs/detection/lossy-fixtures.md`, ADR-0040, Q11): MP3 and AAC streams whose coded spectrum is noise up to a chosen edge and nothing above it, with chosen bitrates, tag frames and LAME fields. No encoder runs in CI and no audio is committed (ADR-0022).
- Scenarios: `specs/features/decode-lossy.feature` (RF-14 H2: scan, decoding, stream information, ALAC, errors, Unicode, read-only, spectrogram, table), `specs/features/detection-lossy.feature` (RF-15: verdicts on fixtures, rule-level boundaries from given measurements, parameter files, v4 compatibility), `specs/features/calibration-lossy.feature` (calibration and labels).
- External encoders (LAME, ffmpeg's AAC, libopus) are exercised only by the calibration tool on the local corpus (section 9). The one encoder that runs in tests is `rusty-opus`'s, for the Opus fixtures (ADR-0047 point 3, `lossy-fixtures.md` section 1.3).
- v7 (RF-16): `specs/features/detection-streaming-source.feature` (section 11.8).
- v8 (RF-14, RF-15, Opus): `specs/features/opus.feature` (section 12.10).

## 8. Known limitations (v5)

Shown in the help as "suspicious with the reason" where they produce `Suspicious`, as backend.md section 9 asks.

- **Close bitrates cannot be told apart.** A 320 kbps MP3 made from 256 kbps (`s` ≈ 800 Hz) is `Real` with the declared bitrate as estimate; one made from 224 kbps (`s` ≈ 1100 Hz) is at most `Suspicious` (`cutoffBelowDeclared`), never `Fake`.
- **Opus → MP3 (48 kHz) and AAC 256 → MP3 320.** Opus keeps a full-band edge of about 20 kHz at common bitrates (`s` ≈ 500 Hz against LAME's 320 kbps low-pass), so Opus → MP3 320 is `Real` with v5 and v6. AAC 256 cuts at 19–20 kHz (`s` ≈ 500–1500 Hz), so AAC 256 → MP3 320 is `Real` or at most `Suspicious`. B6 and B7 report them. **Since v7** (section 11, RF-16) an Opus rip kept at 48 kHz is `Suspicious` with `streamingSampleRate` by its sample rate, not by its spectrum; it is still never `Fake`, and its own limitations are in section 11.6. AAC 256 → MP3 320 at 44.1 kHz is unchanged. Proving either needs codec-domain measurements (section 10).
- **Sources without a low-pass.** A lower-bitrate source encoded without a low-pass (LAME V0 through ffmpeg, rules.md Q5; `--lowpass -1`) leaves no edge: the re-encode is `Real`. This is the H3 double-compression case.
- **Little treble.** A track with `H < low_hf_threshold_db` is `Suspicious` (`lowHighFrequencyContent`) with no estimate.
- **Genuine sources that end early.** A recording whose own mastering low-pass or natural roll-off lies more than `suspicious_shortfall_hz` below `c_exp` is `Suspicious`, and `Fake` if its edge is a sharp step `fake_shortfall_hz` below (R1). The calibration tool counts these "source-limited" honest encodes apart (calibration/lossy.md section 6), and B2 reports them.
- **Other MP3 encoders.** The MP3 table is LAME's. An encoder with a lower low-pass at the same bitrate (old Fraunhofer encoders cut at about 16 kHz at 128 kbps) can be `Suspicious` at its threshold. Encoder-specific tables keyed by `encoder` are a candidate (section 10).
- **Mono files** can only be missed, not wrongly flagged (section 3).
- **HE-AAC** is `unsupportedFormat` (Q10). An M4A that uses **implicit** SBR signalling (only inside the bitstream), or the backward-compatible explicit signalling (object type 2 in the AudioSpecificConfig, with the SBR extension after it), is decoded as AAC-LC at half its bandwidth and may be judged `Fake`: the adapter reads only the first object type of the AudioSpecificConfig.
- **Sample rates below 32 kHz** (MPEG-2 and 2.5 Layer III) are decoded and judged with the same table, but no acceptance figure covers them.
- **The estimate is MP3- or AAC-equivalent** (section 5), not the codec of the source.
- **A user-chosen low-pass** recorded in the LAME tag is honoured (Q6), so a fake made with an explicit low-pass equal to its content edge is `Real`.

## 9. Acceptance criteria (v5)

Measured locally on the real corpus with `just accept --config specs/calibration/calibration.v2.toml --params specs/detection/detection-params.v4.toml --params specs/detection/detection-params.v5.toml` (v4 is the baseline; it evaluates no lossy row, Q8), split `all` unless a row says otherwise. Groups, tables and columns are those of `specs/calibration/lossy.md` section 6. Only aggregate figures from `summary.md` go into the implementation PR (ADR-0022).

"Detectable" excludes undetectable cascades; "non-source-limited" excludes honest encodes whose source's own edge is below `c_exp − suspicious_shortfall_hz` (calibration/lossy.md section 6).

| # | Criterion | Pass |
|---|---|---|
| B1 | Lossless results unchanged | **Gate.** On the lossless evaluation set, the verdict-change table v4 × v5 is diagonal, and every reason and cutoff is equal (it holds by construction, section 1) |
| B2 | Honest generated lossy encodes: the single-stage entries `cbr128`, `cbr192`, `cbr256`, `cbr320`, `v0`, `v2`, `aac128`, `aac256` and the negative cascade `cbr320_cbr128` | **Gate:** for each, `Fake` ≤ **1 %** of its non-source-limited rows. **Report:** the `Suspicious` rate, the source-limited count and, per entry, the median and 5th percentile of `s` |
| B3 | Confirmed honest lossy files (`confirmed.txt` kind `real`) | **Gate:** 0 `Fake`. **Report:** `Suspicious` count |
| B4 | Trusted lossy files of class `primary` | **Gate:** 0 `Fake` (n/a when there are none). Other classes: reported |
| B5 | High-contrast cascades `cbr128_cbr320` and `aac128_cbr320` | **Gate:** caught (`Fake` + `Suspicious`) ≥ **95 %** and `Fake` ≥ **90 %** of the detectable rows of each |
| B6 | Other cascades: `cbr192_cbr320`, `cbr128_aac256`, `opus128_cbr320` | **Report:** caught and `Fake` rates (known limitations, section 8) |
| B7 | Confirmed lossy fakes | **Gate:** strong labels with `upstream = aac`: caught ≥ **90 %**. **Report:** their `Fake` rate; the same figures for `upstream = opus`, `mp3` and `unknown`; weak labels in their own row |
| B8 | Source-bitrate estimate | **Gate:** the median estimate of the `Fake` rows of `cbr128_cbr320` is 128, and that of `aac128_cbr320` is at most 128. **Report:** for confirmed fakes with `source_max_kbps`, the share whose estimate is at most that bound |
| B9 | Chains (one master in several versions) | **Report** per chain: pairs found, verdict of each version, and the median `|fc(mp3) − fc(aac)|` over the pairs that have both |
| B10 | Synthetic | Every scenario of `decode-lossy.feature`, `detection-lossy.feature` and `calibration-lossy.feature` passes (`just test`); `detection-lossless.feature` and `detection-full-band.feature` still pass |
| B11 | Time (RNF-01) | **Report:** median and maximum analysis time per MP3 and M4A track in a release build, in the implementation PR |

If B2, B3, B4, B5, B7 or B8 fails, the implementer does not tune the parameters: the run's figures (and, locally, the listed tracks with `fc`, `c_exp`, `s` and `D`) go back to G1, where the owner decides between new values (a new params version) and accepting the result. B1 holds by construction, so its failure is an implementation bug. A B10 failure is handled like any red test.

The acceptance criteria of v7 (RF-16) are K1–K7 in section 11.7. B1–B11 are not re-run for v7: K1 and K2 show that v7 changes nothing they measure except `Real → Suspicious` on lossy rows matched by L6a.

## 10. Candidates for a later version (not specified here)

- The 16 kHz gap step `G` as evidence of an **MP3 origin in an AAC or Opus file** (an AAC 256 made from an MP3 inherits its sfb21 holes).
- **Per-frame bitrate and padding** of VBR MP3s: a VBR encode of a band-limited source spends fewer bits.
- **Encoder-specific tables** keyed by `encoder` (LAME, Fraunhofer, ffmpeg's AAC, Apple).
- **Opus rows below 24 kbps** (libopus's narrower bandwidths at low bitrates) and an **estimate for Opus fakes** (Q29 (b)); done in part by v8 (section 12), which adds the rows from 24 kbps up.
- **MDCT-domain double-compression fingerprints** (H3), which also cover sources without a low-pass and close bitrates. They are also the way to turn the v7 streaming-rate indicator (section 11) into evidence: an Opus origin leaves traces of its own transform (band energies quantised per CELT band, a hard 20 kHz band edge) that a fingerprint could confirm, and they would catch an Opus rip resampled to 44.1 kHz, which v7 cannot see (section 11.6). Not specified (owner's decision, 2026-10-25).
- **The streaming window for other encoders.** The v7 window `[19800, 20200)` was measured with LAME 3.100, ffmpeg's native AAC encoder and libopus 1.6.1 (section 11.3.1). Other MP3 encoders (Fraunhofer and others), Apple, FDK or Nero AAC and real streaming-site Opus at other settings may need other bounds or encoder-specific windows; acceptance K6 keeps reporting the `fc` distributions.

## 11. Streaming sample rate (detection v7, RF-16)

### 11.1 What and why

Streaming sites serve music as lossy streams. YouTube's highest-quality audio stream is Opus, and **Opus always decodes at 48 kHz**. Opus keeps its **full-band edge at 20 kHz** at every bitrate measured (24–192 kbps, section 11.3.1), so when such a stream is re-encoded to MP3 320 or AAC 256 the result has an edge close to what an honest encode of that bitrate keeps, the shortfall `s` stays below `suspicious_shortfall_hz` and rule L6 calls it `Real` with the declared bitrate as estimate. A ripper that does not resample writes the MP3 or AAC file at **48 kHz**, while most music is released at 44.1 kHz (the CD rate), and the streaming sites' AAC streams are usually 44.1 kHz (the observation that `[labels.upstream_by_rate]` of `calibration.v2.toml` already encodes as data: a 48 kHz rip comes from the Opus stream).

So **a 48 kHz MP3 or AAC file whose content ends at Opus's 20 kHz band edge is a cheap, deterministic hint** of an Opus (streaming) origin. It is only a hint: legitimate 48 kHz releases exist (section 11.6). That is why the rule gives `Suspicious` with a reason that says what was seen, never `Fake`, and only where the spectral rules found nothing (owner's decision, 2026-10-25).

**The case that motivated it** (described without names, ADR-0022): nine MP3 CBR 320 kbps files at 48 kHz, most likely rips of a streaming site's Opus stream, each with a reported cutoff of 20 050 Hz and a steepness of 69–99 dB/kHz. They are a sample of a larger set: the owner's corpus holds **152 labelled 48 kHz rips** (134 strong and 18 weak labels, all MP3 CBR 320) with the same signature. Every strong one reports 20 050 Hz (77 %) or 20 150 Hz (23 %) (section 11.3.1). Under v6 those are `Real` with estimate 320 (`c_exp` = 20500, `s` = 450 or 350 < 1000); of the 152, v6 calls 149 `Real` and 3 weak-labelled ones `Fake`. Under v7 each of the 149 is `Suspicious` with `streamingSampleRate` and no estimate: `mp3` is a listed codec, 48000 a listed rate, 320 ≥ 192 and 19800 ≤ `fc` < 20200 (measured: 152 of 152 caught by L6a or earlier rules).

### 11.2 Rule L6a

`StreamingRate` holds when the params have a `[lossy.streaming]` section `P` and **all** of:

1. the track's codec is in `P.codecs` (`mp3` or `aac`; Opus can never be listed, section 11.3);
2. the track's **decoded sample rate** (`TrackInfo.sampleRate`, section 2) equals one of `P.sample_rates_hz` exactly;
3. the declared bitrate `b ≥ P.min_declared_bitrate_kbps` (for VBR and ABR files `b` is the average, section 2.1);
4. the reported cutoff `fc` (section 3) satisfies `P.min_cutoff_hz ≤ fc < P.max_cutoff_hz` (with the defaults, `[19800, 20200)`: an edge at Opus's 20 kHz band edge, section 11.3.1).

Otherwise, and always for a params file without `[lossy.streaming]` (v5, v6), it is `false`.

Rule L6a (section 4): when no rule from L1a to L5 matched and `StreamingRate` holds, the verdict is **`Suspicious`** with reasons **`[streamingSampleRate]`** and `estimatedSourceBitrateKbps` **null** (Q19). The reported cutoff is the one of section 4 ("Reported cutoff"), unchanged.

Consequences:
- **Never `Fake`**, and **never overrides an earlier rule**: an `Error` (L1a, L0, L1b), a silent track, little treble (L2), a `Fake` (L3) and a `Suspicious` from L4 or L5 keep their own verdict, reasons and estimate, whatever the sample rate. A 48 kHz MP3 320 made from an MP3 128 is still `Fake` with estimate 128.
- **Only `Real → Suspicious`.** For the same spectra and stream information, every v7 result equals the v6 result except tracks that L6 made `Real` and L6a now makes `Suspicious`; those gain the reason and lose their estimate (`b` → null). Apart from that and `algorithmVersion`, nothing changes (the reported cutoff included).
- **Reached only with no larger shortfall.** L6a is evaluated after L5, so it only sees tracks with `s < suspicious_shortfall_hz` or `s` undefined (a declared bitrate below the table): the files whose content reaches what their declared bitrate keeps, which would otherwise be `Real`.
- **Content above the window is not flagged.** A 48 kHz MP3 whose edge lies at 20.25 kHz or above (LAME 320 reports 20250–20450 Hz at 48 kHz; an encode without a low-pass reports more) has content that an Opus source (20 kHz) does not hold, so it is not an Opus rip and stays `Real`. An edge below the window (an encoder low-pass such as LAME 256's, at most 19750 Hz) carries no evidence about the source.
- **Lossless codecs are never judged by it** (`pcm`, `flac`, `alac` use rules.md only, section 4): a 48 kHz FLAC or ALAC is not flagged. **Opus files are out of scope**: they are natively 48 kHz, `opus` cannot be listed in `P.codecs`, and in v7 an Opus track is `Error(unsupportedFormat)` by L0 anyway (no `[lossy.cutoffs]` rows for it). Since v8 (section 12) an Opus track is judged by L1a–L6 with the Opus rows, and still never by L6a.
- **Where it is computed.** `StreamingRate` and L6a are part of the pure lossy verdict function of `fakeflac_core::detect` (section 3, "Where it is computed"), which already receives the codec, the sample rate, the declared bitrate, `fc` and the params. No new measurement, no I/O, no new input from the adapter.

### 11.3 Parameters (`[lossy.streaming]`)

**Values** of `detection-params.v7.toml`. `sample_rates_hz`, `codecs` and `min_declared_bitrate_kbps` are **indicative starting values, not measured on the corpus**; `min_cutoff_hz` and `max_cutoff_hz` were **measured** before G1 and decided by the owner (Q21, section 11.3.1). Acceptance K (section 11.7) checks all five, and a failed gate returns to G1 with the run's figures (the implementer never tunes values).

| Parameter | Value | Why this value |
|---|---|---|
| `lossy.streaming.sample_rates_hz` | `[48000]` | The only rate Opus decodes at. 44100 would flag almost every MP3 and AAC file; 32000 and 96000 are not streaming rates of lossy music |
| `lossy.streaming.codecs` | `["mp3", "aac"]` | The formats a rip is saved as (RF-16). AAC at 48 kHz is less common than MP3 (streaming AAC is usually 44.1 kHz, section 11.1), so it is listed for the Opus → AAC re-encode |
| `lossy.streaming.min_declared_bitrate_kbps` | 192 | Excludes the 128-class files. It is not what separates honest 224–256 kbps files: their reported low-pass (19 450–19 750 Hz) lies below `min_cutoff_hz`, which is what excludes them (measured: 0 of 111 and 0 of 257 honest 48 kHz LAME 256 encodes flagged; 88–91 % with a lower bound of 19500). VBR and ABR files are compared by their average `b` (section 2.1) |
| `lossy.streaming.min_cutoff_hz` | 19800 | Measured (11.3.1): libopus 24–192 kbps re-encoded to MP3 320 reports 19850–20150 Hz in 95.3–100 % of files, mostly 20050 or 20150 (lowest common value 19850; the rest end earlier on their own); LAME 224/256 reports at most 19750 Hz. 19800 is the midpoint of the two grid values |
| `lossy.streaming.max_cutoff_hz` | 20200 | Midpoint of 20150 (highest Opus value) and 20250 (lowest common value of honest 48 kHz LAME 320, whose reported cutoff is 20250–20450 Hz in 96 % of the encodes measured); content past 20.2 kHz is more than an Opus source holds (section 11.2) |

**Shape** (serde, at deserialisation; unknown fields rejected): `[lossy.streaming]` may be absent. If present, all five fields are required. `sample_rates_hz` is an array of non-negative integers; `codecs` is an array whose elements are exactly `"mp3"` or `"aac"`, so any other value, `"opus"` included, is a deserialisation error naming that value; `min_declared_bitrate_kbps` is a non-negative integer; `min_cutoff_hz` and `max_cutoff_hz` are floats. A `[lossy.streaming]` without `[lossy]` cannot be written (it is a subtable of `[lossy]`; TOML creates `[lossy]` implicitly, which then fails for its missing required fields).

**Values** (`DetectionParams::validate`, after the checks of section 6, in this order; the error names the check, written as below; a comparison with a NaN operand does not hold, so NaN is rejected):
1. `lossy.streaming.sample_rates_hz is not empty`
2. `lossy.streaming.sample_rates_hz: sample rate > 0`
3. `lossy.streaming.sample_rates_hz increases` (strictly, in file order, so a repeated rate is rejected)
4. `lossy.streaming.codecs is not empty`
5. `lossy.streaming.codecs has no duplicates`
6. `lossy.streaming.min_declared_bitrate_kbps > 0`
7. `lossy.streaming.min_cutoff_hz > 0`
8. `lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz`

A file that wants no streaming-rate rule omits the section (an empty list is rejected rather than meaning "disabled", so "disabled" has one spelling). The calibration tool's grid search rejects `lossy.streaming.*` keys like every `lossy.*` key (calibration/lossy.md C10), and a proposal from a v7 baseline copies the section unchanged.

#### 11.3.1 Measured basis (G1, 2026-10-25)

The window `[min_cutoff_hz, max_cutoff_hz)` = `[19800, 20200)` was read off measurements made before G1. Aggregates only (ADR-0022): no file, folder or artist name.

**How `fc` was measured.** With a throwaway program outside the repository that calls the same public functions of `fakeflac_core::detect` and `fakeflac-audio`'s analyser as the app, with `detection-params.v6.toml`. **Validation** against the project's own tool: on the 152 labelled 48 kHz MP3s of the owner's 2026-10-25 acceptance run, its `fc`, verdict and declared bitrate equal those of `tracks.csv` for 152 of 152 rows, and `D` agrees within 0.005 dB (the CSV's rounding). L6a is not implemented, so a row counts as flagged when its v6 verdict is `Real` (L6 is "otherwise", so `Real` is exactly "L1a–L5 did not match"), its codec is `mp3` or `aac`, its rate is 48000, `b` ≥ 192 and `fc` is in the window: exact for the rule as specified, not a run of v7 code.

**The grid of `fc`.** Bands are `fft.band_hz` = 100 Hz wide. When `D ≥ step_suspicious_db`, `fc = fs`, the centre of a band, so it always ends in 50; otherwise `fc = fe`, a multiple of 100. Every file of the clusters below has `D` ≥ 8.3 dB (real rips 24.5–41.3 dB), so every reported value ends in 50. All window bounds are band edges, so a bound is exactly 50 Hz from the two neighbouring possible values: ± 50 Hz is the largest margin the grid allows, and both bounds have it.

**Data.** Tools: ffmpeg 8.1 with libopus 1.6.1, LAME 3.100 and ffmpeg's native AAC encoder (no FDK or Apple AAC was available).
- **D1**, the owner's acceptance run of 2026-10-25 (params v5 × v6): 1 425 sources × 14 lossy entries; 152 labelled 48 kHz rips (134 strong and 18 weak labels, all MP3 CBR 320); the honest generated groups at 48 kHz come from the 111 sources that are natively 48 kHz. Full tracks.
- **D2**, controlled cascades: 257 lossless sources (240 at 44.1 kHz resampled to 48 kHz, 13 native 48 kHz, 4 from 96 kHz; one track per folder, seeded random choice; 60 s centred excerpts) × 24 recipes (Opus 64/96/128/160/192 kbps → MP3 CBR 320, V0, V2, CBR 256/192, AAC 256/192 at 48 kHz; honest LAME 320/256/224/192, V0, V2, AAC 256/192 at 48 and 44.1 kHz), plus 80 sources × Opus 24/32/48/64 kbps → MP3 320. An independent replication of D1 on new sources (same encoder builds).
- **D3**, the owner's library: the 638 MP3 files at 48 kHz with `b` ≥ 192 (152 labelled rips and 486 unlabelled).

**`fc` distributions** (v6 measurement; Hz):

| Population | n | min | p5 | median | p95 | max | Set |
|---|---|---|---|---|---|---|---|
| Real-world rips, strong labels, MP3 CBR 320 at 48 kHz | 134 | 20050 | 20050 | 20050 | 20150 | 20150 | D1 |
| Opus 128 → MP3 320 generated (`opus128_cbr320`, B6) | 1425 | 11850 | 20050 | 20050 | 20150 | 20650 | D1 |
| Opus 128 → MP3 320, controlled | 257 | 17950 | 20050 | 20050 | 20150 | 20450 | D2 |
| Opus 64 / 96 / 160 / 192 → MP3 320, controlled | 257 each | 11950 / 19650 / 15750 / 15750 | 20050 / 20050 / 19950 / 19890 | 20050 (all) | 20150 (all) | 20550 / 20150 / 20450 / 20450 | D2 |
| Opus 24 / 32 / 48 / 64 → MP3 320 | 80 each | 20050 / 19950 / 20050 / 20050 | 20050 (all) | 20150 / 20150 / 20150 / 20050 | 20150 (all) | 20150 (all) | D2 |
| Opus 128 → AAC 256 (ffmpeg native) | 257 | 20050 | 20050 | 20050 | 20150 | 21150 | D2 |
| Opus 128 → V0 | 257 | 15950 | 19950 | 20050 | 20150 | 20550 | D2 |
| Honest LAME 320 at 48 kHz, native 48 kHz masters | 111 | 19750 | 20250 | 20350 | 20450 | 20450 | D1 |
| Honest LAME 320 at 48 kHz, controlled | 257 | 15750 | 19750 | 20450 | 20450 | 20450 | D2 |
| Honest LAME 320 at 44.1 kHz | 1314 | 16050 | 19950 | 20150 | 20150 | 20250 | D1 |
| Honest LAME 256 at 48 kHz (D1 / D2) | 111 / 257 | 18650 / 15750 | 19400 / 18250 | 19650 / 19650 | 19650 / 19750 | 19750 / 19750 | D1 / D2 |
| Honest AAC 256 at 48 kHz, ffmpeg native (D1 / D2) | 111 / 257 | 20200 / 12550 | 20900 / 17110 | 21750 / 21750 | 24000 / 24000 | 24000 / 24000 | D1 / D2 |
| Honest AAC 192 at 48 kHz | 257 | 15750 | 19550 | 21050 | 21150 | 21150 | D2 |
| Honest V0 at 48 kHz (D1 / D2) | 111 / 257 | 17450 / 12550 | 19450 / 16050 | 22150 / 21350 | 24000 / 24000 | 24000 / 24000 | D1 / D2 |

Paired per source (D2, 257 sources): `fc` of the honest LAME 320 at 48 kHz minus `fc` of Opus 128 → LAME 320 at 48 kHz has median +300 Hz, 5th percentile −120 Hz and 95th percentile +400 Hz; 231 of 257 sources have a gap of at least 200 Hz, and 19 are inverted or equal (10 of them sources that end below 19.5 kHz on their own). libopus keeps the 20 kHz edge from 24 to 192 kbps: `fc` lies between 19850 and 20150 Hz for 100 % of the files at 24–48 kbps, 98.8 % at 64, 99.6 % at 96, 99.2 % at 128, 96.1 % at 160 and 95.3 % at 192; the rest are sources that end earlier on their own, mostly caught by L3–L5. Steepness does not separate the populations (median `D` 33.6 dB for Opus-origin files, 33.8 dB for honest LAME 320), so no further parameter is added.

**Window options** (positives: caught by L6a or already by L3–L5; negatives: rows flagged; 95 % Wilson intervals in parentheses). A = the first draft `[19500, 21000)`, B = its alternative `[19500, 20300)`, **C = decided `[19800, 20200)`**:

| Population | n | A | B | C |
|---|---|---|---|---|
| **Positives (Opus origin)** | | | | |
| Real-world rips, strong labels | 134 | 134 (100 %; 97.2–100) | 134 | 134 |
| Real-world rips, weak labels | 18 | 18 | 18 | 18 |
| Generated Opus 128 → MP3 320 (K3 population) | 1425 | 1425 (100 %) | 1421 (99.7 %) | 1418 (99.5 %; 99.0–99.8) |
| Controlled Opus 64–192 → MP3 320 (each) | 257 | 257 (100 %) | 255–257 (99.2–100 %) | 255–256 (99.2–99.6 %) |
| Controlled Opus 24–64 → MP3 320 (each) | 80 | 100 % | 100 % | 100 % |
| Controlled Opus 128 → V0 / → AAC 256 | 257 | 98.1 % / 99.2 % | 97.7 % / 99.2 % | 97.3 % / 98.8 % |
| **Negatives (honest, 48 kHz): rows flagged** | | | | |
| LAME 320, native 48 kHz masters (D1) | 111 | 111 (100 %) | 15 (13.5 %) | 3 (2.7 %; 0.9–7.6) |
| LAME 320, controlled (D2) | 257 | 246 (95.7 %) | 32 (12.5 %) | 12 (4.7 %; 2.7–8.0) |
| LAME 256 (D1 / D2) | 111 / 257 | 91.0 % / 87.9 % | 91.0 % / 87.9 % | 0 % / 0 % |
| LAME 224 (D2) | 257 | 88.3 % | 88.3 % | 0 % |
| LAME 192, V2, AAC 128 (D1, D2) | | 0 % | 0 % | 0 % |
| V0 (D1 / D2) | 111 / 257 | 13.5 % / 20.2 % | 7.2 % / 6.2 % | 4.5 % / 5.1 % |
| AAC 256, ffmpeg native (D1 / D2) | 111 / 257 | 7.2 % / 7.0 % | 0.9 % / 2.7 % | 0 % / 1.9 % |
| AAC 192 (D2) | 257 | 28.4 % | 25.3 % | 0 % |
| **Owner's library (D3): 48 kHz MP3 with `b` ≥ 192, files flagged** | 638 | 506 (79 %) | 324 (51 %) | 274 (43 %) |
| of which labelled rips / unlabelled | | 149 / 357 | 149 / 175 | 149 / 125 |

Moving the upper bound from 20200 to 20300 buys 0–0.4 points of recall and costs 6.6–9.9 points of false positives on honest 48 kHz LAME 320, so 20200 is preferred; moving the lower bound from 19500 to 19800 removes the whole LAME 224/256 class at a cost of 0–0.4 points of recall. The labels cannot say which of the unlabelled library files flagged by C are right (91 of the 125 lie in the Opus cluster, 20.0–20.2 kHz).

**Caveats.**
- **Circularity.** The bounds were read off the corpus whose rips are also the positives. D2 replicates both clusters and the same recall and false-positive rates on new sources, but with the same encoder builds. K4's labels come from `upstream=by-rate` (calibration/lossy.md section 1.5), the same observation L6a uses: K4 measures recall on the owner's rips only, never false positives (section 11.7).
- **Encoders not measured.** Fraunhofer and other MP3 encoders; Apple, FDK and Nero AAC; real streaming-site Opus streams at other settings (only libopus 1.6.1 through ffmpeg, and the 134 + 18 real rips, which match its cluster); mono and multichannel sources; 32 and 22.05 kHz.
- **Negatives.** The honest encodes are ffmpeg encodes of the owner's lossless files; the corpus holds no labelled honest 48 kHz lossy file from a store, so the false-positive rate on commercial 48 kHz releases is unknown. The AAC branch has generated data only (0 of the owner's 162 M4A files are 48 kHz).
- Rows of one album or master are correlated, so the intervals are optimistic.

### 11.4 Versions

- **v7 = `detection-params.v6.toml` with every section and value unchanged plus `[lossy.streaming]`**, `algorithm_version = 7`. It was the default until detection v8 (section 12.5, whose `detection-params.v8.toml` keeps `[lossy.streaming]` unchanged): `DetectionParams::default()` equalled `detection-params.v7.toml` (a test checked it). The next free `algorithm_version` becomes **8** (`/calibrate` proposals and the sample corpus's `candidate.toml` take 8; the tool computes it from the files, so no code allocates it by hand).
- **v5 and v6 stay loadable.** `[lossy.streaming]` is optional; a params file without it gives exactly its own verdicts, reasons, cutoffs and estimates on every file (L6a never fires). Tests check it, so the calibration tool compares v6 and v7 in one run.
- **Lossless files** get exactly their v6 results (nothing in rules.md reads `[lossy.streaming]`), with `algorithm_version` 7.
- `[input]` and `[fft]` are unchanged and L6a uses only values already computed, so `fakeflac-audio::EXTRACTION_VERSION` does not change and cached spectra are reused; the app's result cache is invalidated by the new `algorithm_version` (ADR-0008).
- **Contract**: version 6 (`specs/contracts/commands.md`, "Streaming sample rate (version 6)"): the new reason `streamingSampleRate`, `algorithmVersion` 7 with the defaults, and a new case where `estimatedSourceBitrateKbps` is null. **CSV**: `csv_format_version` stays 1 (Q23). **UI**: the reason's text (`specs/ui/lossy-formats.md`, "Streaming sample rate (RF-16)"); no new component.

### 11.5 Relation to calibration labels

`upstream=by-rate` (calibration/lossy.md section 1.5) and L6a encode the same observation in two places: the first labels the owner's streaming-site rips for evaluation, the second judges files in the app. They are independent data (`calibration.v*.toml` and `detection-params.v7.toml`): changing one does not change the other, and acceptance K4 is read on labels resolved by the first. Because both rest on the same observation, K4 measures recall only (section 11.7).

### 11.6 Known limitations (v7)

Shown in the reason's text, which says that genuine 48 kHz releases look the same (`ui/lossy-formats.md`).

- **Legitimate 48 kHz files whose content ends in the window are flagged (false positives by design).** Audio from video productions (soundtracks, concerts, video game music, podcasts recorded for video), web stores and labels that sell or deliver 48 kHz masters, and a user's own MP3 or AAC encodes of 48 kHz or hi-res masters (an encoder fed 96 kHz may resample to 48 kHz) all have a 48 kHz rate. When their reported cutoff lies in `[19800, 20200)` (a master that ends near 20 kHz, or an encoder whose low-pass lands there) they are `Suspicious` with `streamingSampleRate`, never `Fake`, and the reason says so; nothing in v7 tells them apart from an Opus rip. Measured (11.3.1): 2.7 % of honest 48 kHz LAME 320 encodes of native 48 kHz masters and 4.7 % of the controlled ones, 4.5–5.1 % of V0, 0–1.9 % of ffmpeg AAC 256, 0 % of LAME 192–256 and AAC 192; those are ffmpeg encodes of the owner's lossless files, so the rate on commercial 48 kHz releases is unknown. Acceptance K5 counts them on the honest groups and bounds them on the honest generated MP3 320 and 256 rows at 48 kHz (at most 10 % and 2 %, section 11.7).
- **An Opus rip resampled to 44.1 kHz is not caught.** A ripper or converter that resamples to 44.1 kHz (a common default) removes the only evidence v7 reads; such a file is `Real` as under v6, and its cutoff (20050 or 20150 Hz) is indistinguishable from that of an honest LAME 320 at 44.1 kHz (median 20150 Hz). Catching it needs codec-domain measurements (section 10).
- **Opus at low bitrates is covered.** Measured (11.3.1): libopus 24–192 kbps keeps the 20 kHz edge (re-encoded to MP3 320, `fc` 19850–20150 Hz in 95.3–100 % of the files; the rest are sources that end earlier on their own, mostly caught by L3–L5). Measured with libopus 1.6.1 through ffmpeg, not with a streaming site's own streams; the 134 strong-labelled real rips match the same cluster.
- **Opus rips re-encoded at MP3 224 or 256, at AAC 192, or below 192 kbps** (for example to MP3 128) are not flagged: the encoder's own low-pass (LAME 224/256 report 19450–19750 Hz) lies below `min_cutoff_hz` and hides the Opus edge, and honest encodes at those settings report the same cutoffs (section 11.3). In the owner's library only 18 of 653 48 kHz MP3 files lie between 192 and 319 kbps.
- **Other 48 kHz lossy sources.** A 48 kHz rip of another lossy source whose edge lies in the window (for example a 48 kHz AAC stream of a video site) is flagged the same way; the reason does not name the source codec and the text says "possibly".
- **The estimate is null** for flagged files (Q19): the table cannot express an Opus source in MP3 or AAC terms.
- **AAC files at 48 kHz.** Measured only for ffmpeg's native encoder (11.3.1): honest AAC 256 reports a median of 21750 Hz and 0–1.9 % of it falls in the window (AAC 192: 0 %). Other AAC encoders (Apple, FDK, Nero) were not measured; one whose honest low-pass at 256–320 kbps lies between 19800 and 20200 Hz would be flagged like an MP3 (the AAC table is conservative, section 6). The AAC branch has no real-world data: none of the owner's M4A files is 48 kHz.
- **Exact rates only.** A rate of 47 999 Hz or a 48 kHz file reported at another rate by a broken header is not matched; rates come from the decoder (section 2), so this needs a broken file.

### 11.7 Acceptance criteria (v7)

Measured locally on the real corpus with `just accept --config specs/calibration/calibration.v3.toml --params specs/detection/detection-params.v6.toml --params specs/detection/detection-params.v7.toml` (v6 is the baseline), split `all` unless a row says otherwise. Groups, tables and columns are those of `specs/calibration/lossy.md` sections 6.3 and 6.8. Only aggregate figures from `summary.md` go into the implementation PR (ADR-0022): no file, folder or artist name. "Detectable" is as in section 9.

| # | Criterion | Pass |
|---|---|---|
| K1 | Lossless results unchanged | **Gate.** On the lossless evaluation set, the verdict-change table v6 × v7 is diagonal, and every reason and cutoff is equal (by construction, section 11.4; a failure is an implementation bug) |
| K2 | Lossy changes are only `Real → Suspicious` by L6a | **Gate.** In the lossy verdict-change table v6 × v7 (calibration/lossy.md 6.8) every off-diagonal count other than `Real → Suspicious` is 0, and its `Real → Suspicious` count equals its "of which `streamingSampleRate`" count (every changed row carries the new reason with v7). By construction (section 11.2); a failure is an implementation bug |
| K3 | Generated Opus → MP3 320 (`opus128_cbr320`, 48 kHz) | **Gate:** with v7, caught (`Fake` + `Suspicious`) ≥ **90 %** of its detectable rows (Q22). **Report:** caught and `Fake` rates with v6 and v7, the `streaming` count, and the same caught rate over all its rows, undetectable included |
| K4 | Confirmed lossy fakes at 48 kHz (the per-sample-rate row 48000 of section 6.3, strong labels; and the `upstream = opus` row) | **Gate:** with v7, caught ≥ **90 %** of the strong labels of the 48000 row (Q22), applied once that row has at least **N = 30** strong labels (Q22). With fewer, K4 is **reported and does not block**: `summary.md` shows the count, and the gate reads "n/a (fewer than 30 strong 48 kHz fakes)" (so 0 such files is also n/a, not a failure). **Strong labels:** the per-upstream and per-sample-rate rows of confirmed fakes hold strong labels only; weak labels appear in the `weak` row (calibration/lossy.md 6.3, 6.8), so the gate is read on the row as it stands. **Recall only:** K4's labels come from `upstream=by-rate`, the same observation L6a uses (section 11.5), so K4 measures recall on the owner's rips and cannot detect false positives (K5 gates and reports those). **Report:** caught and `Fake` rates with v6 and v7 for both rows, and their `streaming` counts |
| K5 | False positives on honest lossy files | **Gate** (Q22): with v7, the `streaming rate` (calibration/lossy.md 6.8) of the honest generated sub-row **`cbr320 @ 48000`** is at most **10 %** and that of **`cbr256 @ 48000`** at most **2 %** (measured before G1: 2.7–4.7 % and 0 %, section 11.3.1). The bound is checked on the row's counts (`streaming` ≤ bound × the rate's denominator), not on the rounded percentage. A sub-row that is absent or whose denominator is 0 is n/a for its bound and does not block. **Report:** with v7, the `streaming` count, the `streaming rate` and the `Suspicious` rate of each honest group per sample rate (6.8: single-stage generated entries, the negative cascade `cbr320_cbr128`, trusted lossy files per class, `confirmed_real`), and their change against v6. **Gate (unchanged from v5):** B2's `Fake` ≤ 1 %, B3's and B4's 0 `Fake` still hold (by construction: L6a produces no `Fake`) |
| K6 | Where the cutoffs lie (checks the measured window of Q21 and section 11.3.1 on each run) | **Report:** the median, 5th and 95th percentile of `fc` (6.8) of `opus128_cbr320`, of the honest `cbr320` and `aac256` rows at 48 kHz, and of the confirmed lossy fakes at 48000 |
| K7 | Synthetic | Every scenario of `detection-streaming-source.feature` passes (`just test`), and `detection-lossy.feature`, `decode-lossy.feature`, `detection-lossless.feature`, `detection-edge-step.feature`, `calibration.feature` and `calibration-lossy.feature` still pass |

If K3, K4 or K5's `streaming rate` gate fails (K4 only counts as failed when it applies, that is with at least N = 30 strong 48 kHz labels; an n/a K4 or an n/a K5 bound is listed in the PR with its count and blocks nothing), the implementer does not tune the parameters: the run's figures (and, locally, the listed tracks with codec, sample rate, `b`, `fc`, `s` and `D`) go back to G1, where the owner decides between new values (a new params version) and accepting the result. K1, K2 and K5's gate unchanged from v5 hold by construction, so their failure is an implementation bug. A K7 failure is handled like any red test. No timing criterion: L6a adds a few comparisons and no measurement (RNF-01 unaffected).

### 11.8 Test signals and scenarios

- Fixtures (`lossy-fixtures.md`): the existing `mp3_320_48k_lp20k` (an MP3 320 at 48 kHz with edge 20000 Hz: the motivating case), and three new ones: `mp3_320_lp20k` (the same edge at 44.1 kHz), `m4a_aac_256_48k_lp20k` (AAC 256 at 48 kHz, edge 20000 Hz) and `mp3_320_48k_short` (a 2 s cut of `mp3_320_48k_lp20k`, `tooShort`).
- Scenarios: `specs/features/detection-streaming-source.feature` (tag `@RF-16`): verdicts on fixtures, rule-level cases from given measurements (the 44.1 kHz control, L3–L5 at 48 kHz, every boundary of the window and of the bitrate), params without the section, Opus, lossless codecs, parameter checks, error tracks (RNF-04), a real run with a Unicode name (RNF-09) that leaves the file unchanged (RNF-05), the CSV cell, the UI text, the calibration report additions, and K5's gate read from the 48 kHz sub-rows (passing and failing).

## 12. Opus (detection v8, RF-14 and RF-15)

### 12.1 What and why

ADR-0047 (accepted at G1, #102) chose the decoder (`rusty-opus =1.0.2`), the adapter's own Ogg reader, family 0 only and `.opus` only. Detection v8 makes `.opus` files analysable: the scanner lists them, the adapter reads them (section 2.4), and the params gain Opus rows so that rule L0 no longer rejects them.

The rule is the v5 rule (section 4) with one more table. Honest libopus encodes keep their **full band up to 20 kHz** (the top edge of CELT's last band) at every bitrate from 16 to 320 kbps: measured before G1 on native decodes of 69 corpus sources, `fc` 20050 or 20150 Hz in 69 of 69 at every bitrate (section 12.8 F1, F2; section 11.3.1 had found the same edge through MP3 re-encodes). So an Opus file whose content stops clearly below 20 kHz was made from a source with less bandwidth: typically an MP3 or AAC at 128 kbps or less converted to Opus. That is what RF-15 asks to flag. The edge shows only where libopus keeps it: at 160 kbps or less (stereo music) it fills the empty bands above a band-limited source's edge with folded energy and hides it, from 192 kbps it keeps it (section 12.6, F4).

### 12.2 Rules

For a track with codec `opus`, rules L1a–L6 of section 4 apply unchanged, with:

- `c_exp = min(T(opus, b), nyquist − step_search_top_margin_hz)` = **20000 Hz** for every `b` ≥ 24 kbps (the Nyquist cap is 23600 Hz at 48 kHz, so it never applies). An Opus stream has no tag low-pass (section 2).
- `b` < 24 kbps: `T` is undefined, so `c_exp` and `s` are undefined and the track is `Real` unless L2 fires (section 4, "A declared bitrate below the table").
- **L6a never applies**: `opus` cannot be listed in `lossy.streaming.codecs` (section 11.3).
- The estimate: section 12.4.

With the v5 thresholds this means, for a clear step (`D` ≥ 10 dB): `fc` ≤ 17500 Hz → `Fake`; 17500 < `fc` ≤ 19000 Hz → `Suspicious` (`cutoffBelowDeclared`); `fc` > 19000 Hz → `Real`. Examples (medians measured before G1, section 12.8 F4): an Opus 256 made from an MP3 128 (`fc` ≈ 16650) or from an AAC 128 (`fc` ≈ 17250) is `Fake` with no estimate; an Opus 256 made from an MP3 192 (`fc` ≈ 18750) is `Suspicious` (`cutoffBelowDeclared`); an Opus 128 made from an MP3 320 is `Real`. An Opus 160 or 128 made from an MP3 128 is mostly `Real`: libopus hides the source's edge at those bitrates (band folding, section 12.6). An Opus file whose edge is 12 kHz or lower has little treble (`H` is the median of the 10–16 kHz bands) and is `Suspicious` with `lowHighFrequencyContent` by L2 (measured for a 12 kHz super-wideband encode: `H` −62.4 dB, F5).

### 12.3 The Opus rows (Q30)

| `bitrate_kbps` | 24 | 32 | 48 | 64 | 96 | 128 | 160 | 192 |
|---|---|---|---|---|---|---|---|---|
| `cutoff_hz` | 20000 | 20000 | 20000 | 20000 | 20000 | 20000 | 20000 | 20000 |

- **Why 20000.** It is the nominal full-band edge of libopus. An honest Opus file reports `fc` at the band centre of its edge: **measured** on native decodes (section 12.8 F1), 20050 or 20150 Hz in 69 of 69 sources at every bitrate from 16 to 320 kbps (5th percentile 20050, 95th 20150), with a median `D` of 23–25 dB, so `fc = fs` and `s` is −50 or −150 Hz, far below `suspicious_shortfall_hz` (1000 Hz): honest files keep a margin of about 1000 Hz before `Suspicious` and 2500 Hz before `Fake`. With these rows 0 of the 828 honest encodes at 24–320 kbps are `Fake` or `Suspicious` (F1).
- **Why no row below 24 kbps.** libopus's default bandwidth depends on the bitrate: at low bitrates it codes only super-wideband (12 kHz), wideband (8 kHz) or narrowband (4 kHz). **Measured** (F2): libopus 1.6.1 stays full band for music from **16 kbps** (69 of 69 at 16, 20 and 24 kbps); at 12 kbps it is wideband, with `H` < −60 dB in 69 of 69 (`Suspicious` by L2). The first row stays at 24 kbps, keeping a margin of one step for older libopus versions (not measured), whose bandwidth choice just above 16 kbps may differ; a first row at 16 kbps is the alternative of Q30. A row below 16 kbps would need the bandwidth libopus picks at each bitrate, which also depends on the content (speech or music) and the channel count. Without a row such files are never `Fake` or shortfall-`Suspicious`.
- **The border at 24 kbps** (F3). The declared bitrate is the payload (section 2.4.3), and libopus's VBR payload at a 24 kbps target has a median of 23.9 kbps (16: 15.1; 20: 19.8; from 32 kbps up it lies above the target: 32.4 at 32, 330.2 at 320), so **about half of the honest 24 kbps encodes fall below the table**: they are `Real` (unless L2 fires) and are not judged by the shortfall rules. Harmless for honest files; the cost is that a transcode declared below 24 kbps is never judged by the shortfall rules either, which a first row at 16 kbps would change.
- **Why several equal rows.** With the null estimate of section 12.4 one row from 24 kbps would give the same verdicts; the eight rows are the bitrates the calibration measures, so a later version can change one bitrate alone. A declared bitrate above 192 kbps (an Opus 256 or 320) uses the 192 row, also 20000 Hz.
- **Status: measured before G1** (section 12.8), with libopus 1.6.1 through ffmpeg on 69 corpus sources; acceptance O3 checks the rows on every run.

### 12.4 Estimate of an Opus track (Q29)

- **L6 (`Real`):** `b`, as for every lossy codec (section 5).
- **L2, L3, L4, L5:** `null`. Every Opus row has the same cutoff, so the nearest-row rule of section 5 would always return the first row (24 kbps): an "≈24 kbps source" for an Opus 256 made from an MP3 128, which is not true in any codec's terms. The verdict still says `Fake` or `Suspicious`; the Real bitrate cell shows "—" (`ui/lossy-formats.md`). This follows the precedent of L6a (Q19): when the table cannot express the source, no estimate is given.
- Where it is computed: in the pure lossy functions of `fakeflac_core::detect` (section 3), from the codec; no new parameter.

### 12.5 Versions, extraction and contract

- **v8 = `detection-params.v7.toml` with every section and value unchanged plus the `opus` array of `[lossy.cutoffs]`**, `algorithm_version = 8`. It is the default: `DetectionParams::default()` equals `detection-params.v8.toml` (a test checks it). The next free `algorithm_version` becomes **9** (`/calibrate` proposals and the sample corpus's `candidate.toml` take 9; the tool computes it from the files).
- **v5, v6 and v7 stay loadable.** With them an Opus track is `Error(unsupportedFormat)` (`no [lossy.cutoffs] rows for opus`, L0), so the calibration tool compares v7 and v8 in one run.
- **Every file that is not Opus** gets exactly its v7 verdict, reasons, cutoff and estimate with v8 (nothing but an Opus track reads `lossy.cutoffs.opus`), with `algorithm_version` 8. Tests check it on every lossless signal and every MP3 and M4A fixture.
- **`EXTRACTION_VERSION` stays 2** (ADR-0047 point 5, Q41): the decoding of WAV, FLAC, MP3 and M4A does not change, and Opus results never existed before. `[input]` and `[fft]` are unchanged. The app's result cache is invalidated by the new `algorithm_version` (ADR-0008).
- **Contract** (Q34): no type change; `commands.md` stays version 7 and documents Opus in "Opus files (detection v8)". **CSV**: the `format` and `codec` columns already allow `opus` (csv-export.md); `csv_format_version` stays 1. **UI**: no new component, prop or text (`ui/lossy-formats.md`, "Opus files (detection v8)").

### 12.6 Known limitations (v8)

- **Band folding hides a band-limited source at 160 kbps or less.** At 160 kbps or less (stereo music), libopus's CELT fills the empty bands above the source's edge with folded energy (about 20 dB below an honest encode's level), so the step the rules find is Opus's own 20 kHz band edge and the file is `Real`. From 192 kbps the source's edge is kept. Measured before G1 (section 12.8 F4, 69 corpus sources, `Fake` of 69): MP3 128 → Opus 128: 0; → Opus 160: 15; → Opus 192: 55; → Opus 256: 65 (`fc` median 16650 Hz). AAC 128 → Opus 160: 6; → Opus 192: 43; → Opus 256: 61. MP3 192 → Opus 160: 0 `Fake` and 0 `Suspicious`. So the common case of a 128 kbps file converted to Opus 128 or 160 is **not caught**, and only conversions to 192 kbps or more are; the gated cascades of acceptance O4 end at 256 kbps for this reason, and `cbr128_opus160` is reported (section 12.7). In mono the edge survives from 96 kbps (the synthetic `lp16k` measured with `rusty-opus`; in independent-channel stereo only from 192 kbps). Codec-domain measurements (section 10) are the way to see such files.
- **Opus made from a full-band or nearly full-band source** (an MP3 320 or V0, an AAC 256, another Opus) keeps an edge at or above 19.5 kHz and is `Real`: the edge cannot show it. Measured (F4): MP3 320 → Opus 128 69 of 69 `Real`; Opus 64 → 160, 96 → 192 and 128 → 256 207 of 207 `Real`. An Opus made from an MP3 224 or 256 (edge 19.4–19.7 kHz) is `Real` too, and one made from an MP3 192 is mostly `Suspicious` at Opus 256 (measured 2 `Fake`, 50 `Suspicious`, 17 `Real` of 69, `fc` median 18750 Hz) and `Real` at Opus 160 (folding). Codec-domain fingerprints (section 10) are the way to see them.
- **No estimate for Opus fakes** (section 12.4): the table says nothing about the source's bitrate.
- **Low bitrates** (`b` < 24 kbps) are not judged by the shortfall rules, about half of the honest 24 kbps encodes included (their payload is just below 24 kbps, section 12.3); speech and podcasts encoded with a narrow bandwidth are `Suspicious` (`lowHighFrequencyContent`) when their edge is low enough for L2, as low-bitrate MP3s are (measured: libopus at 12 kbps is wideband, `H` < −60 dB in 69 of 69 music sources, F2).
- **A user-chosen bandwidth** (opusenc `--bandwidth`, ffmpeg's libopus `-cutoff`) is not recorded in the stream, so an honest encode limited to 12 kHz or less is `Suspicious` (`lowHighFrequencyContent`) like a transcode (measured, F5: a super-wideband encode reports `fc` 12050 Hz, `D` ≈ 26 dB, `H` −62.4 dB, both with `rusty-opus` `max_bandwidth = Superwideband` and with ffmpeg's libopus `-cutoff 12000`); unlike a LAME low-pass (Q6), nothing can excuse it.
- **Encoders other than libopus 1.6.1** (through ffmpeg 8.1) **and `rusty-opus` 1.0.2**, its port, were not measured (section 12.8): older libopus versions may choose other bandwidths at low bitrates, and the folding threshold (160 versus 192 kbps) may differ between versions; ffmpeg's native experimental `opus` encoder has its own bandwidth choices.
- **Duration assumes a stream that starts at granule 0.** A stream cut out of a live broadcast, whose first audio page implies a start offset, reports a longer duration and a slightly lower declared bitrate (RFC 7845 section 4.5). Files saved by encoders start at 0.
- **Out of scope:** channel mapping families other than 0 (multichannel), chained or multiplexed Ogg streams (`unsupportedFormat`), and Opus in `.ogg`, `.oga`, `.webm` or `.mka` files (not scanned; ADR-0047 point 4).
- **Sequential decoding.** The whole stream is decoded to analyse the sampled windows (no seeking in v8); acceptance O8 reports the time (measured before G1, F6: about 0.7 s of decoding per 360 s of stereo Opus 128 plus less than 0.1 s of spectral analysis, well under RNF-01's 3 s).

### 12.7 Acceptance criteria (v8)

Measured locally on the real corpus with `just accept --config specs/calibration/calibration.v4.toml --params specs/detection/detection-params.v7.toml --params specs/detection/detection-params.v8.toml` (v7 is the baseline; it evaluates no Opus row, calibration/lossy.md C14), split `all` unless a row says otherwise. Groups, tables and columns are those of `specs/calibration/lossy.md` sections 6.3 and 6.8. Only aggregate figures from `summary.md` go into the implementation PR (ADR-0022). "Detectable" and "non-source-limited" are as in section 9. The corpus holds no `.opus` file today, so the generated entries of `calibration.v4.toml` (calibration/lossy.md section 8) carry O3–O5.

| # | Criterion | Pass |
|---|---|---|
| O1 | Lossless results unchanged | **Gate.** On the lossless evaluation set, the verdict-change table v7 × v8 is diagonal, and every reason and cutoff is equal (by construction, section 12.5; a failure is an implementation bug) |
| O2 | MP3 and AAC results unchanged | **Gate.** In the lossy verdict-change table v7 × v8 (calibration/lossy.md 6.8, over the rows both files evaluate, which excludes Opus rows with v7), every off-diagonal count is 0, and every reason, cutoff and estimate is equal (by construction) |
| O3 | Honest generated Opus: the single-stage entries `opus64`, `opus96`, `opus128`, `opus160` and the negative cascade `cbr320_opus128` | **Gate:** for each, `Fake` ≤ **1 %** of its non-source-limited rows. **Report:** the `Suspicious` rate, the source-limited count, the median and 5th percentile of `s`, and the median, 5th and 95th percentile of `fc` per entry (they check the 20000 Hz rows, Q30). Measured before G1 (section 12.8 F1, F4): 0 of 828 honest libopus encodes at 24–320 kbps `Fake` or `Suspicious`; `cbr320_opus128` 0 of 69 `Fake` (69 `Real`) |
| O4 | High-contrast Opus cascades `cbr128_opus256` and `aac128_opus256` (an Opus 256 made from an MP3 128 or an AAC 128; 256 kbps because at 160 kbps or less libopus hides the source's edge, section 12.6) | **Gate:** caught (`Fake` + `Suspicious`) ≥ **85 %** and `Fake` ≥ **80 %** of the detectable rows of each (Q37). Measured before G1 (F4, over all 69 rows, detectability not computed): `cbr128_opus256` caught 94 %, `Fake` 94 % (65 of 69, `fc` median 16650 Hz); `aac128_opus256` caught 91 %, `Fake` 88 % (61 `Fake` and 2 `Suspicious` of 69, `fc` median 17250 Hz). **Report:** their `fc` percentiles |
| O5 | Other Opus cascades: `cbr192_opus256` (an Opus 256 made from an MP3 192; truth `fake` by the bitrate rule of calibration/lossy.md 4.2, since 192 < 256; its MP3 edge makes it mostly `Suspicious` by design) and `cbr128_opus160` (an Opus 160 made from an MP3 128; truth `fake`; documents the band-folding limitation of section 12.6) | **Report:** their verdict counts, caught and `Fake` rates and `fc` percentiles (known limitations, section 12.6). Measured before G1 (F4, of 69): `cbr192_opus256` 2 `Fake`, 50 `Suspicious`, 17 `Real` (`fc` median 18750 Hz); `cbr128_opus160` 15 `Fake`, 0 `Suspicious`, 54 `Real` (`fc` median 20150 Hz) |
| O6 | Labelled Opus files (trusted lossy Opus files, `confirmed_real` and `confirmed_fake` with codec `opus`) | **Gate:** 0 `Fake` on trusted `primary` and `confirmed_real` Opus files (n/a when there are none, as today). **Report:** every Opus group's counts |
| O7 | Synthetic | Every scenario of `opus.feature` passes (`just test`), and `decode-lossy.feature`, `detection-lossy.feature`, `detection-streaming-source.feature`, `detection-lossless.feature`, `detection-edge-step.feature`, `calibration.feature` and `calibration-lossy.feature` still pass |
| O8 | Time (RNF-01) | **Report:** median and maximum analysis time per Opus track in a release build (`just perf <folder of Opus encodes>`, folder mode, performance.md section 12), in the implementation PR. Context, measured before G1 (section 12.8 F6): decoding with `rusty-opus` takes about 1.9 ms per second of stereo Opus 128 (about 0.7 s per 360 s), and the spectral analysis of the decoded samples less than 0.1 s per track (one thread, release); O8 stays a report. A median above RNF-01's 3 s for a typical-length track goes back to G1 with the figures; the levers of performance.md section 7 (seeking with RFC 7845's 80 ms pre-roll) need a spec change because they alter the decoded samples (`EXTRACTION_VERSION`) |

If O3, O4 or O6 fails, the implementer does not tune the parameters: the run's figures (and, locally, the listed tracks with `b`, `fc`, `s` and `D`) go back to G1, where the owner decides between new values (a new params version) and accepting the result. O1 and O2 hold by construction, so their failure is an implementation bug. An O7 failure is handled like any red test. The v7 criteria K1–K7 are not re-run: O1 and O2 show that v8 changes nothing they measure.

### 12.8 Figures measured before G1 (2026-11-01)

Lesson of RF-16 and RNF-01 (`docs/agents/lessons.md`): every figure that can be measured locally is measured before G1, with a throwaway program outside the repository that calls FakeFlac's own public functions, as section 11.3.1 did. Aggregates only (ADR-0022): no file, folder, track or artist name.

**How.** The Opus streams were decoded to `f32` at 48 kHz with `rusty-opus` 1.0.2 and analysed with FakeFlac's band spectra and `fakeflac_core::detect` (`measure`, `cutoff_estimate`) with the values of `detection-params.v7.toml` (v8 changes no value these functions read); verdicts were **simulated** with the v7 lossy thresholds and an `opus` table at 20000 Hz (the rows of section 12.3), so they are exact for the rules as specified, not a run of v8 code. Declared bitrate as in section 2.4.3: the Ogg payload after the two header packets over (last granule − pre-skip) / 48000.

**Tools.** ffmpeg 8.1 with libopus 1.6.1 (default VBR, application `audio`), LAME 3.100 and ffmpeg's native AAC encoder for the cascades; `rusty-opus` 1.0.2's encoder and decoder for the synthetic fixtures and for decoding. Machine: AMD Ryzen 7 5800X (AVX2), Windows 11.

**Data.**
- **Corpus sample:** 69 lossless sources of the owner's corpus (60 at 44.1 kHz, 9 at 48 kHz; one per folder, seeded random choice; 60 s excerpts from 30 s), encoded honestly by libopus and through the cascades of F4.
- **Synthetic:** the fixtures' signals at 48 kHz (white noise, −12 dBFS RMS, 30 s; `full_band` and `lp16k`), encoded by `rusty-opus` in mono and in stereo with independent channels.

**F1 (honest encodes) and F3 (payload).** libopus 1.6.1 at the target bitrates below, 69 sources each:

| target kbps | 16 | 20 | 24 | 32 | 48 | 64 | 80 | 96 | 112 | 128 | 160 | 192 | 256 | 320 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| payload kbps (median) | 15.1 | 19.8 | 23.9 | 32.4 | 49.2 | 64.9 | 80.5 | 96.3 | 112.5 | 129.4 | 162.8 | 195.5 | 263.8 | 330.2 |

- `fc` at every bitrate from 16 to 320 kbps: 20050 or 20150 Hz in 69 of 69 (5th percentile 20050, 95th 20150); median `D` 23–25 dB.
- With rows at 20000 Hz: **0 of the 828 honest encodes at 24–320 kbps** (12 bitrates × 69) are `Fake` or `Suspicious`.
- VBR payload (F3): below the target up to 24 kbps (median 23.9 at 24, so about half of the 24 kbps encodes fall below the first row, section 12.3), above it from 32 kbps. `rusty-opus` on the synthetic signals: CBR exactly the target, VBR the target plus 0.4–0.8 kbps.

**F2 (lowest full-band bitrate): 16 kbps.** 69 of 69 report `fc` ≥ 20050 Hz at 16, 20 and 24 kbps. At 12 kbps libopus is wideband, with `H` < −60 dB in 69 of 69 (`Suspicious` by L2).

**F4 (band-limited sources).** Cascades of the same 69 sources, verdicts with rows at 20000 Hz (`—`: payload not recorded):

| Recipe | payload kbps | `fc` p5 / median / p95 (Hz) | Fake | Suspicious | Real |
|---|---|---|---|---|---|
| MP3 128 → Opus 128 | 126 | 20050 / 20050 / 20150 | 0 | 0 | 69 |
| MP3 128 → Opus 160 (`cbr128_opus160`) | 158 | 16550 / 20150 / 20150 | 15 | 0 | 54 |
| MP3 128 → Opus 192 | 191 | 16150 / 16650 / 20150 | 55 | 0 | 14 |
| MP3 128 → Opus 256 (`cbr128_opus256`) | 258 | 16150 / 16650 / 16750 | 65 | 0 | 4 |
| MP3 192 → Opus 160 | — | 20050 / 20050 / 20150 | 0 | 0 | 69 |
| MP3 192 → Opus 256 (`cbr192_opus256`) | 261 | 18450 / 18750 / 20150 | 2 | 50 | 17 |
| MP3 320 → Opus 128 (`cbr320_opus128`) | — | 20050 / 20050 / 20150 | 0 | 0 | 69 |
| AAC 128 → Opus 160 | — | 17350 / 20150 / 20150 | 6 | 0 | 63 |
| AAC 128 → Opus 192 | 192 | 17250 / 17350 / 20150 | 43 | 0 | 26 |
| AAC 128 → Opus 256 (`aac128_opus256`) | — | 17250 / 17250 / 20050 | 61 | 2 | 6 |
| Opus 64 → 160, 96 → 192, 128 → 256 | — | 20050 / 20050 / 20150 | 0 | 0 | 207 |

Cause: at 160 kbps or less (stereo music) CELT fills the empty bands above the source's edge with folded energy (about 20 dB below an honest encode's level), so the step the rules find is the 20 kHz band edge (section 12.6).

Synthetic, `rusty-opus`, reported `fc` (Hz) per bitrate (kbps):

| Input | Channels | 64 | 96 | 128 | 160 | 192 | 256 | 320 |
|---|---|---|---|---|---|---|---|---|
| `full_band` | mono / stereo | 20050 | 20050 | 20050 | 20050 | 20050 | 20050 | 20050 |
| `lp16k` | mono | 20050 | **16050** | 16050 | 16050 | 16050 | 16050 | — |
| `lp16k` | stereo, independent channels | 20050 | 20050 | 20150 | 20150 | **16050** | 16050 | 16050 |

`lp16k` with its edge kept: `D` ≈ 33 dB. `full_band`: `D` ≈ 26 dB, `H` ≈ 0 dB. So the fixture `opus_160_src16k` (mono, 160 kbps) reports `fc` 16050 Hz with `D` ≈ 33 dB (`Fake`), and a stereo variant would need at least 192 kbps (Q40).

**F5 (super-wideband).** `rusty-opus` with `max_bandwidth = Superwideband` (mono and stereo, 64–256 kbps): `fc` 12050 Hz, `D` ≈ 26 dB (the 12 kHz step is the one found), `H` −62.4 dB, so L2 `lowHighFrequencyContent`. The same with ffmpeg's libopus `-cutoff 12000`. This is the truth of `opus_160_swb` (lossy-fixtures.md section 2.2).

**F6 (time).** `rusty-opus` decodes 119–282 s of stereo libopus 128 in 187–548 ms (about 1.9 ms per second of audio, about 0.7 s per 360 s); FakeFlac's spectral analysis of the decoded samples takes 45–79 ms (one thread, release build). Context for O8 (section 12.7), which stays a report.

**F7 (encoder delay and SIMD).**
- `rusty-opus` 1.0.2 has **no public lookahead getter**. Its encoder delay, measured by cross-correlating the decode with the input, is **120 samples** (2.5 ms) in mono and stereo; libopus's (through ffmpeg) is 312, and decoding a libopus file with its own pre-skip leaves no residual offset. With a pre-skip of 312, a `rusty-opus` fixture decodes 192 samples early, so the fixtures use **120** (lossy-fixtures.md section 1.3).
- Encodes on the same ISA level are byte-identical run to run. `RUSTY_OPUS_ISA=scalar` (and `sse2`, `avx`) give **different bytes** from the default AVX2 path for mono fixtures (VBR and CBR: 11 of 18 recipes differed); stereo VBR 160, the ADR-0047 spike's case, was identical. The decoded difference is about 62 dB SNR (different coding decisions, not rounding); `fc`, `D` and `H` are unchanged. So the fixtures are deterministic per machine, not across SIMD levels (Q42).

**What they settle.** The `opus` rows and their first row (Q30, section 12.3); the gate values of O3 and O4 and the choice of 256 kbps for the O4 cascades (Q37); the band-folding limitation (section 12.6); the truths of `opus_160_src16k` and `opus_160_swb` and the mono choice (Q40, lossy-fixtures.md section 2.2); the fixtures' pre-skip and the determinism rule (Q42, lossy-fixtures.md section 1.3); the context of O8.

**Caveats.**
- Two encoders only (libopus 1.6.1 through ffmpeg 8.1, `rusty-opus` 1.0.2); older libopus versions and other encoders were not measured (section 12.6).
- 69 sources, one per folder, 60 s excerpts, stereo music; no speech, no mono music, and no real-world Opus file (the corpus holds none). Rows of one album are correlated, so the rates are optimistic.
- The cascade rates are over all 69 rows: detectability (section 9) was not computed, so the gated rates of O4 over detectable rows may differ slightly.
- Verdicts are simulated (see "How"), not produced by v8 code; acceptance O3–O5 re-measures them on every run.

### 12.9 Changes to approved specs and protected tests (Q38)

(a) `rules.md` section 4 and its introduction: the defaults and `algorithmVersion` become v8/8, the next free `algorithm_version` becomes **9**; no lossless rule or value changes. (b) `detection-lossless.feature`: "Parameters come from the versioned file" names `detection-params.v8.toml` and `algorithmVersion 8`. (c) `detection-lossy.feature`: its header and "A lossy codec without table rows cannot be judged" (no longer "the scanner never lists an Opus file"), and "The defaults hold the v5 lossy values", whose defaults now have the eight Opus rows. (d) `decode-lossy.feature`: the header, "MP3 and M4A files are found; Opus files are not yet" (now Opus files are found), "A folder with only Opus files has no supported audio yet" (becomes a folder with only `.ogg` and `.oga` files), and "A lossy row has the same fields as a lossless row" (`algorithmVersion 8`). (e) `detection-streaming-source.feature`: its header says that "DetectionParams v7" means the parameters of `detection-params.v7.toml` (no longer the defaults); "Opus tracks are not judged by L6a…", "An Opus track with the defaults is still an error…" (now "with DetectionParams v7"), "The defaults are the v7 file" (now about the v7 file, whose `[lossy.streaming]` the defaults keep), and the real run "A 48 kHz rip with a Unicode name…" (`algorithmVersion 8`). (f) `detection-edge-step.feature`: header (the defaults are v8). (g) `calibration.feature`: the proposals of "The grid search proposes new parameters from tune data" and "The full-band guard can be searched from a v3 baseline" are `detection-params.v9.toml` / `algorithm_version 9`; `calibration/evaluation.md` section 7 and `calibration/sample-corpus.md` section 4: next free version 9. (h) `calibration/lossy.md` (C14–C17), `calibration-lossy.feature` (the Opus corpus file is an Opus fixture; Opus decoded; `formats=lossy` includes `.opus`; Opus allowed as a last stage; a v8 run of the lossy variant), the new `calibration/calibration.v4.toml`. (i) `contracts/commands.md` ("Opus files (detection v8)", "Supported files", `algorithmVersion` 8), `contracts/performance.md` (Opus no longer out of scope for the folder report), `ui/lossy-formats.md` (the Opus rows of the table and the detail line). (j) `lossy-fixtures.md` sections 1.3, 1.4, 2.2 and 3. (k) The protected tests that change in the test stage, as a declared consequence (the test author lists the exact set in the `test(RF-14)` commit; assertions on other values do not change):
  - `crates/fakeflac-core/tests/{params_lossy,params_streaming,detect_rules,detect_lossy,detect_streaming}.rs`: the defaults' `algorithm_version` (7 → 8) and `lossy.cutoffs.opus` (`None` → the eight rows); the core cannot read a file, so a test that builds "v5" or "v7" params from the defaults must also clear `lossy.cutoffs.opus` and set `algorithm_version` (for example `rf15_a_lossy_codec_without_table_rows_cannot_be_judged` and the RF-16 test "An Opus track with the defaults is still an error"); `supported_extensions` gains `opus`;
  - `crates/fakeflac-audio/tests/{detection_lossless,detection_lossy,detection_streaming,params_streaming,params_files,detection_edge_step}.rs`: the defaults equal the v8 file; the "v7" side of the v6 × v7 comparisons loads the v7 file instead of taking the defaults; `algorithm_version` 7 → 8 where the defaults are used;
  - `crates/fakeflac-fs/tests/scan_lossy.rs`: `.opus` is listed;
  - `src-tauri/tests/{run_analysis_lossy,run_analysis_lossy_v5,run_analysis_streaming,export_csv}.rs`: the Opus scan cases and `algorithmVersion` 8 with the defaults;
  - `tools/fakeflac-calibrate/tests/{lossy_inputs,lossy_sample,lossy_acceptance,lossy_rows,lossy_streaming}.rs` and `tests/common/mod.rs`: Opus decoded and evaluated by v8, not evaluated by v5–v7; `formats=lossy` includes `.opus`; the "Opus output is not supported yet" error is gone; the comment in `grid.rs` (version 8) is stale.

### 12.10 Test signals and scenarios

- Fixtures: `lossy-fixtures.md` sections 1.3, 1.4 and 2.2 (Opus encoded by `rusty-opus` in test support, written by a test-support Ogg writer; the test stage first confirms section 3).
- Scenarios: `specs/features/opus.feature` (tags `@RF-14`, `@RF-15`): scan, stream information, pre-skip, end trim and output gain, packets across pages, verdicts on fixtures and from given measurements (every boundary of the shortfall rules, the table borders, L6a never), the v8 file and its checks, v8 against v7 on every other file, every `corruptStream` and `unsupportedFormat` case of section 2.4.4 (RNF-04), the table, the detail line and the spectrogram, read-only (RNF-05), Unicode names (RNF-09), and the fixture encoder's determinism on one machine and the truths of the fixtures encoded on the scalar path (Q42).
- Calibration: `specs/features/calibration-lossy.feature` (Opus rows in the lossy variant of the sample corpus, C14–C17).

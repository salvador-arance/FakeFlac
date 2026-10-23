---
id: SPEC-DETECTION-LOSSY
requirements: [RF-14, RF-15]
status: proposed
algorithm_version: 5
params: specs/detection/detection-params.v5.toml
lossless_rules: specs/detection/rules.md (v4 rules and values, unchanged in v5)
adr: [ADR-0040, ADR-0041]
---

# Detection rules v5: lossy files (MP3, M4A) and their source bitrate

> For an MP3 or an AAC file in M4A: read what the stream **declares** (codec, bitrate, bitrate mode, the LAME tag's low-pass), measure where its content **ends** with the band spectra and measurements of `rules.md` (unchanged), compare that edge with the **expected cutoff** of an honest encode at the declared bitrate, and decide **real / suspicious / fake** with an **estimated source bitrate**. The expected cutoffs are a per-codec table in `DetectionParams` (`[lossy.cutoffs]`), and every threshold is a named parameter of `detection-params.v5.toml`; the code never hard-codes them (S4, ADR-0011). Design decision: ADR-0041. Decoding (RF-14 H2): ADR-0040.

The verdict answers ADR-0031's question for lossy files: *is the content consistent with the declared quality?* A 320 kbps MP3 whose content stops at 17 kHz is `Fake`, with an estimated source of ≈128 kbps (planning section 2.5, `docs/architecture/backend.md` section 9).

**Names used here.** `b` is the declared bitrate in kbps (section 2), `fc` the measured content edge, `c_exp` the expected cutoff, `s` the shortfall `c_exp − fc` (section 3). `D`, `fs`, `fe`, `H`, `G` are the measurements of `rules.md` section 2. "Lossy codec" means `mp3`, `aac` or `opus`; "lossless codec" means `pcm` (WAV), `flac` or `alac` (ALAC in M4A). **Opus** is specified only as far as the rules go: its decoder is pending the ADR-0030 spike, and every Opus scenario lives in `specs/pending/opus.feature`, outside `specs/features/` (Q9).

## Open questions for G1

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
| Q9 | **Opus is pending.** The scanner keeps skipping `.opus` files. The Opus scenarios are written in `specs/pending/opus.feature`. Pending features live in `specs/pending/`, which neither `just trace` (`scripts/trace.mjs` reads `specs/features/*.feature` only, not recursively) nor the test stage reads, so no test is derived from them and their tags are not checked. After the ADR-0030 spike, a later spec PR moves the file to `specs/features/` (dropping its `@pending-opus-decoder` tag, which is only a marker). RF-14 and RF-15 keep their scenarios in `specs/features/`, so `just trace` still passes. Optional note for the owner (agents may not edit CLAUDE.md files): `specs/CLAUDE.md` could say "Features in `specs/pending/` are specified but not testable yet; they move to `features/` in a later spec PR." | Open. Default: yes |
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
- **Specified but pending:** Opus (`specs/pending/opus.feature`, Q9). No `[lossy.cutoffs]` row exists for it in v5.
- **Out:** double-compression fingerprints in the MDCT domain (H3, backend.md section 9), upsamples (RF-12), FLAC-to-FLAC quality loss, and grid search over `[lossy]` values (calibration/lossy.md, C10).

**Why one file (Q1).** v5 is `detection-params.v4.toml` with every section and value unchanged plus the optional `[lossy]` section:
1. A `TrackAnalysis` carries one `algorithmVersion`, and the app's cache key has one (ADR-0008). Two files with two versions would need a second number in the contract and in the key.
2. The lossy rules reuse the lossless measurements and their parameters (`[input]`, `[fft]`, `[bands]`, `[extent]`, `[step]`, `verdict.low_hf_threshold_db`, `verdict.step_suspicious_db`). A separate file would have to repeat them or point at another file, and the two could disagree.
3. The calibration tool already compares whole files side by side (calibration/evaluation.md section 9); one file per version keeps that.

The cost: a change to a lossy value is a new version for lossless results too, so cached lossless results are recomputed once. Accepted.

**Version numbering.** `algorithm_version` numbers are allocated by the files in `specs/detection/` (rules.md section 4). This spec allocates **5**. With `detection-params.v5.toml` in the repository, the **next free number is 6**: `/calibrate` proposals and the sample corpus's `candidate.toml` take 6 (Q13 (a), (c), (d)); the tool and its tests already compute it from the files (`next_free_version()`), so nothing in the code allocates it by hand. A later change to either part (lossless or lossy) takes 6, and both `rules.md` and this spec then cite the file that introduced it. **v5 becomes the default**: `DetectionParams::default()` equals `detection-params.v5.toml`.

**v2, v3 and v4 stay loadable.** `[lossy]` is optional. Without it, lossless files get exactly their v2/v3/v4 verdicts (nothing in `rules.md` reads `[lossy]`), and lossy files are `Error(unsupportedFormat)` (Q8).

## 2. Stream information (RF-14)

The audio adapter (`fakeflac-audio`) reports, besides the band spectra, what the stream declares. It reads the file read-only (RNF-05) and never needs an external tool. New fields of `AudioProbe` (core types, no I/O in the core): `codec`, `declared_bitrate_kbps`, `bitrate_mode`, `encoder` and `encoder_lowpass_hz`. The first three reach the contract (`specs/contracts/commands.md`, Q4); `encoder` and `encoder_lowpass_hz` stay in the backend and the calibration tool.

| Field | WAV, FLAC | MP3 | M4A, AAC-LC | M4A, ALAC |
|---|---|---|---|---|
| `codec` | `pcm`, `flac` | `mp3` | `aac` | `alac` |
| `declared_bitrate_kbps` | absent | section 2.1 | section 2.2 | absent |
| `bitrate_mode` | absent | `cbr`, `vbr` or `abr` (2.1) | absent (AAC does not signal it reliably) | absent |
| `encoder`, `encoder_lowpass_hz` | absent | from the LAME tag (2.1) | absent (iTunes tags are not read in v5) | absent |
| `bit_depth` | as in H1 | absent | absent | from the ALAC configuration |

The **sample rate** of a lossy file is the decoded rate (44.1 or 48 kHz for most files). **Channels** are mixed to mono for the analysis, as in H1 (rules.md section 1.1).

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

## 3. Measurements

Every lossy file gets the spectra of `rules.md` section 1 and the measurements of `rules.md` section 2 with the same parameters: `L_ref`, `F`, `H`, `fe`, `D` at `fs`, `G`, `U`, `B`. A silent track (no frame above `silence_dbfs`) is `Suspicious` with `lowHighFrequencyContent`, no cutoff and no estimate, as in rules.md section 1.6. Then:

| Quantity | Definition |
|---|---|
| **Content edge `fc`** | the **reported cutoff frequency** of rules.md section 2: `fc = fs` when `D ≥ step_suspicious_db`, otherwise `fc = fe`. So the cutoff shown in the table is the edge the lossy rules judge |
| **Table cutoff `T(codec, b)`** | the `cutoff_hz` of the row of `lossy.cutoffs.<codec>` with the largest `bitrate_kbps ≤ b`. **Undefined** when `b` is below the first row's bitrate (nothing can be expected of a stream below the table) |
| **Tag low-pass `L_tag`** | `encoder_lowpass_hz` when present and in `[cutoff_hz of the codec's first row, nyquist]`; otherwise undefined (a corrupt or meaningless byte is ignored) |
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
| L1a | Reading, probing or decoding failed (section 2.3: `unreadable`, `corruptStream`, or `unsupportedFormat` from sections 2.1 and 2.2, such as HE-AAC or MP3 Layer II) | `Error` | — (the `TrackError` explains it) | null |
| L0 | The params have no `[lossy]` section (Q8): `unsupportedFormat` with detail `these DetectionParams have no [lossy] section`; or `lossy.cutoffs` has no row for the codec (Opus in v5): `unsupportedFormat` with detail `no [lossy.cutoffs] rows for <codec>` (for example `no [lossy.cutoffs] rows for opus`) | `Error` | — | null |
| L1b | Too short, or no declared bitrate, in that order (section 2.3) | `Error` | — (the `TrackError` explains it) | null |
| L2 | `H < low_hf_threshold_db` (little treble) | `Suspicious` | `lowHighFrequencyContent` | null |
| L3 | `s` defined, `s ≥ fake_shortfall_hz` and `D ≥ fake_min_step_db` | `Fake` | — | section 5 |
| L4 | `s` defined, `s ≥ suspicious_shortfall_hz` and `D ≥ step_suspicious_db` | `Suspicious` | `cutoffBelowDeclared` | section 5 |
| L5 | `s` defined and `s ≥ suspicious_shortfall_hz` | `Suspicious` | `gradualRolloff` | section 5 |
| L6 | Otherwise | `Real` | — | `b` |

A silent track is decided after L1b and before L2 (section 3), with the same result as L2 but no cutoff. L0 depends only on the codec and the params, never on the spectra. It comes **after** L1a for every lossy format, MP3 included: an M4A has to be probed to know whether it holds AAC or ALAC, so both formats follow one order, and a broken file keeps the more informative `corruptStream` (or `unreadable`, or the `unsupportedFormat` of an undecoded codec) whatever the params. It comes **before** L1b: with params that cannot judge the codec, a short or bitrate-less lossy file is `unsupportedFormat`, not `tooShort` or `corruptStream`. In v5 the app never meets an Opus file, because the scanner skips `.opus` (Q9); the row exists so that the core has a defined answer.

Consequences (they rely on the checks of section 6):
- **`Fake` always rests on a clear step:** L3 needs `D ≥ fake_min_step_db ≥ step_suspicious_db`, so `fc = fs`. A track whose content fades without a step (`D < step_suspicious_db`, `fc = fe`) is at most `Suspicious` (L5), whatever its shortfall, in the spirit of planning section 2.5 ("never `Fake` only because of the spectrum shape").
- **L4 covers both a moderate shortfall with a clear step and a large shortfall with a step that is clear but not deep** (`step_suspicious_db ≤ D < fake_min_step_db`).
- **Content above the expected cutoff is never held against a file** (`s < 0`): it is `Real`.
- **A declared bitrate below the table** (for example a 24 kbps stream) has no `c_exp`, so the file is `Real` unless L2 fires.
- **The lossless rules never apply to a lossy codec**, and the lossy rules never apply to a lossless codec: an honest MP3 has a low-pass by design, so V4 of rules.md would call every one `Fake`.
- **No `FullBand`, gap or lossy-range logic.** `G`, `U`, `B` and the range `[lossy_min_cutoff_hz, fake_max_cutoff_hz)` of rules.md are not used here (Q7).

**Reported cutoff.** `CutoffEstimate` exactly as in rules.md section 2 ("Reported cutoff"), so its `frequencyHz` equals `fc`. It is `null` only for `Error` and silent tracks.

## 5. Estimated source bitrate (`estimatedSourceBitrateKbps`)

- **Lossless codecs, `Error`, silent tracks and L2:** `null`. Nothing reliable can be said.
- **L6 (`Real`):** the declared bitrate `b` (Q5): the content is consistent with it.
- **L3, L4, L5:** `min(b, r*)`, where `r*` is the `bitrate_kbps` of the row of `lossy.cutoffs.<codec>` whose `cutoff_hz` is **nearest to `fc`** (on a tie, the lower bitrate). An `fc` below the first row gives the first row's bitrate, so the value reads as "at most".

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

**Shape** (serde, at deserialisation; unknown fields rejected in every table): `[lossy]` may be absent; if present, `suspicious_shortfall_hz`, `fake_shortfall_hz`, `fake_min_step_db` and `[lossy.cutoffs]` are required. In `[lossy.cutoffs]`, `mp3` and `aac` are required arrays and `opus` is optional; any other key is an error. Each row is `{ bitrate_kbps: integer, cutoff_hz: float }`, both required.

**Values** (`DetectionParams::validate`, rules.md section 4, extended; the error names the inequality, written as below):
- `lossy.suspicious_shortfall_hz > 0`
- `lossy.suspicious_shortfall_hz ≤ lossy.fake_shortfall_hz`
- `step_suspicious_db ≤ lossy.fake_min_step_db` (makes "`Fake` rests on a clear step", section 4, an invariant)
- for each codec present: `lossy.cutoffs.<codec> is not empty`, `lossy.cutoffs.<codec>: bitrate_kbps > 0`, `lossy.cutoffs.<codec>: cutoff_hz > 0`, `lossy.cutoffs.<codec>: bitrate_kbps increases` (strictly, in file order) and `lossy.cutoffs.<codec>: cutoff_hz does not decrease`.

**Order.** `validate()` runs the checks of rules.md section 4 first, then these in the order listed, codec by codec in the order `mp3`, `aac`, `opus`, and reports the **first** check that fails (so swapping the 112 and 128 kbps rows of `mp3`, which breaks both row checks, is reported as `lossy.cutoffs.mp3: bitrate_kbps increases`). A comparison with a NaN operand does not hold, so NaN is rejected. Every loader calls `validate()` (the calibration tool exits 2, evaluation.md section 9); a test checks that the defaults are valid.

**Invariants.** Deterministic; read-only; `algorithm_version` of every `TrackAnalysis` = that of the params (5 with the defaults); the defaults equal `detection-params.v5.toml` (a test checks it); `[input]` and `[fft]` are unchanged, so the spectra of lossless files are the same as with v2–v4 (calibration/lossy.md section 5 on `EXTRACTION_VERSION`).

## 7. Test signals and scenarios

- Lossy test files are generated inside the tests by small bitstream writers (`specs/detection/lossy-fixtures.md`, ADR-0040, Q11): MP3 and AAC streams whose coded spectrum is noise up to a chosen edge and nothing above it, with chosen bitrates, tag frames and LAME fields. No encoder runs in CI and no audio is committed (ADR-0022).
- Scenarios: `specs/features/decode-lossy.feature` (RF-14 H2: scan, decoding, stream information, ALAC, errors, Unicode, read-only, spectrogram, table), `specs/features/detection-lossy.feature` (RF-15: verdicts on fixtures, rule-level boundaries from given measurements, parameter files, v4 compatibility), `specs/features/calibration-lossy.feature` (calibration and labels); and, not tested yet, `specs/pending/opus.feature` (Q9).
- Real encoders (LAME, ffmpeg's AAC, libopus) are exercised only by the calibration tool on the local corpus (section 9).

## 8. Known limitations (v5)

Shown in the help as "suspicious with the reason" where they produce `Suspicious`, as backend.md section 9 asks.

- **Close bitrates cannot be told apart.** A 320 kbps MP3 made from 256 kbps (`s` ≈ 800 Hz) is `Real` with the declared bitrate as estimate; one made from 224 kbps (`s` ≈ 1100 Hz) is at most `Suspicious` (`cutoffBelowDeclared`), never `Fake`.
- **Opus → MP3 (48 kHz) and AAC 256 → MP3 320.** Opus keeps a full-band edge of about 20 kHz at common bitrates (`s` ≈ 500 Hz against LAME's 320 kbps low-pass), so Opus → MP3 320 is `Real`. AAC 256 cuts at 19–20 kHz (`s` ≈ 500–1500 Hz), so AAC 256 → MP3 320 is `Real` or at most `Suspicious`. B6 and B7 report them. Catching them needs codec-domain measurements (section 10).
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

## 10. Candidates for a later version (not specified here)

- The 16 kHz gap step `G` as evidence of an **MP3 origin in an AAC or Opus file** (an AAC 256 made from an MP3 inherits its sfb21 holes).
- **Per-frame bitrate and padding** of VBR MP3s: a VBR encode of a band-limited source spends fewer bits.
- **Encoder-specific tables** keyed by `encoder` (LAME, Fraunhofer, ffmpeg's AAC, Apple).
- **Opus rows** with the Opus decoder (ADR-0030 spike).
- **MDCT-domain double-compression fingerprints** (H3), which also cover sources without a low-pass and close bitrates.

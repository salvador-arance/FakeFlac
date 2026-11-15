---
id: SPEC-FAKE-EVIDENCE
requirements: [RF-17, RF-04, RF-15, RF-10, RF-11, RF-09]
status: proposed
algorithm_version: unchanged (8 with the defaults; RF-17 adds no detection version)
params: unchanged (specs/detection/detection-params.v8.toml; no new section, no new value)
---

# Evidence of a `Fake` verdict (RF-17)

> When a track is `Fake`, say **which evidence** made it `Fake` and **how strong** that evidence is, so the user can judge the verdict instead of trusting it blindly. Today a `Fake` carries an empty reasons list by design (rules V3 and V4 of `rules.md` section 3 and rule L3 of `lossy-source.md` section 4 return `Fake` with reasons `—`), while `Suspicious`, the weaker accusation, always has a reason. RF-17 adds a separate list, `evidence`, to every `TrackAnalysis`: for a `Fake` track it names each `Fake` rule whose condition holds, with the measured value, the threshold it was compared with and the **margin** between them. It also adds, in the interface only, an **album note** that puts an isolated `Fake` in the context of its album.
>
> **RF-17 changes no verdict, reason, cutoff, estimate, measurement, threshold or version.** For the same file, verdict, reasons, error, cutoff, estimated source bitrate and `algorithmVersion` are identical before and after RF-17 (section 5). `DetectionParams` gets no new section or value, so `detection-params.v8.toml` stays the default, `algorithm_version` stays 8 and `EXTRACTION_VERSION` stays 2.
>
> Contract: `specs/contracts/commands.md`, "Fake evidence (version 9)". Interface and texts: `specs/ui/fake-evidence.md`. Cache: `specs/contracts/result-cache.md` section 3 (one more re-check). Scenarios: `specs/features/fake-evidence.feature` (tag `@RF-17`).

## 0. Open questions for G1

Each question has a default, and the rest of this spec (and the UI spec, the contract section and the scenarios) is written with that default. Changing a default changes only what its row names.

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| Q1 | **A separate field or the existing `reasons`.** (a) **A new field `evidence: FakeEvidence[]`** of `TrackAnalysis`, filled only for `Fake`; `reasons` and `SuspicionReason` do not change. (b) New `SuspicionReason` values (for example `gapStep`, `cutoffStep`, `shortfall`) put in `reasons` of a `Fake`. (b) cannot carry the numbers that the strength needs (Q2), so it would need a numeric field anyway; and it would break approved invariants and outputs that read `reasons`: "a `Fake` carries no reason" (`commands.md` "Lossy formats (version 3)", `components.md` `DetailPanel`), `streamingSampleRate` "never on a `fake`", the CSV `reasons` column (`fake` rows would gain identifiers), the calibration tool's `tracks.csv` `reasons` column and the verdict-change tables that compare reasons between params files. With (a) every one of those, and every CSV, `tracks.csv` and cache row written before RF-17, stays valid as it is | (a) separate field | 2, 6; contract "Fake evidence (version 9)" |
| Q2 | **Strength: a margin, not a label.** (a) **Each evidence item carries its margin**, `value − threshold`, in the unit of the measurement (dB or Hz), computed in the core from the measurement and the `DetectionParams` value the rule compared it with. No new parameter, no label, no new detection version. (b) A label such as "marginal" when the margin is below a boundary (for example 4 dB): the boundary would be a new parameter, so a new params file `detection-params.v9.toml` (v8 plus, for example, `[evidence] marginal_margin_db`), `algorithm_version` 9, a new cache engine, and the version changes listed under Q5 (b). The 2026-11-14 figures (section 1) show the margin itself separating the two groups (38 % of the isolated fakes within 3–4 dB of a threshold, about 3 % of the whole-album fakes), and a fixed label would hide how close a track is | (a) margin only | 3 |
| Q3 | **Which items a lossless `Fake` lists.** (a) **Every `Fake` rule whose whole condition holds**: V3 (`gapStep`) and V4 (`cutoffStep`), both when both hold, although the verdict table stops at V3 (first match). (b) Only the rule that decided the verdict (V3 when both hold). (a) matches the owner's analysis ("by both V3 and V4", section 1) and shows that two independent signs agree; (b) would hide V4 behind V3 on 218 of the 501 fakes | (a) every rule that holds | 2.1 |
| Q4 | **Lossy evidence (L3).** L3 needs two conditions together, the shortfall `s ≥ lossy.fake_shortfall_hz` and the step `D ≥ lossy.fake_min_step_db`, so a lossy `Fake` lists **both**: `shortfall` (Hz) and `cutoffStep` (dB). Alternative: the shortfall only (the step is a precondition) | Both items | 2.2 |
| Q5 | **Album note boundary.** (a) **No tunable boundary.** The interface shows the note on a `Fake` track whenever its album has at least 2 analysed tracks and not all of them are `Fake` (`fake < real + suspicious + fake`), with the counts in the text ("the only Fake of the 14 analysed tracks", "3 of the 14 analysed tracks"), so the user judges "few" from the numbers. The comparison of two counts of the album summary selects a text; it is not a threshold, so nothing goes into `DetectionParams`, the contract's `AlbumUpdate` does not change and the album verdict rule stays parameter-free (`album-summary.md`, "Non-goals"). (b) **A share boundary as data:** the note only when `fake / (real + suspicious + fake) < album.isolated_max_fake_share` (0.25, the boundary of the owner's "isolated" group). It would be computed in the core (the frontend never decides on album data, `commands.md` "Album summary (version 5)"), so `AlbumUpdate` would gain a flag (contract change), the use case would read the value, and the value would be a new section of `DetectionParams`: `detection-params.v9.toml`, `algorithm_version` 9, a new cache engine (every cached result recomputed once), defaults and "next free version" moving in `rules.md` section 4, `lossy-source.md` sections 1 and 6, `calibration/{calibration,evaluation,sample-corpus}.md`, `calibration.feature` (proposals become v10), `detection-lossless.feature`, `decode-lossy.feature`, `opus.feature`, `detection-streaming-source.feature`, and every protected test that checks the defaults or `algorithmVersion` 8, with acceptance proving that v9 gives the v8 verdicts (as K1/K2 did for v7). (c) The same boundary in a new, separate params type outside `DetectionParams` (no algorithm version, no cache effect): a second params mechanism, which needs an ADR extending ADR-0011 | (a) no boundary | 4; UI spec section 3 |
| Q6 | **Error tracks and the album note.** The counts of the note leave out `errors` (an unreadable file says nothing about how the album was made): "analysed" = `real + suspicious + fake` | Errors left out | 4 |
| Q7 | **Partial albums.** During a run, or after a cancel, the note uses the counts of the latest album update received (the counts so far); the text says "analysed", so it stays true. Alternative: no note until `summary.total == files` (a cancelled run would never show it) | Counts so far | 4 |
| Q8 | **CSV.** (a) **No change**: `evidence` is not exported, `csv_format_version` stays 1, and every CSV, scenario and test of RF-09 stays as it is (`fake` rows keep an empty `reasons` cell). (b) Append a column 22 `fake_evidence` (items in order, joined with `;`, each `<kind>:<value>:<threshold>`, dB values with one decimal and Hz values in whole Hz, as section 4 of `csv-export.md` formats numbers; empty for a track that is not `fake`), `csv_format_version` 2. (b) would let a spreadsheet sort the fakes by margin and count them by kind (it would also make check M2's counts readable from the app's own export); it changes `csv-export.md` sections 1, 3 and 7 (21 → 22 fields), the scenarios "The header row", "The document for four stubbed tracks", "Export after a finished analysis" and "Large result sets" of `export-csv.feature`, "The CSV record of a flagged track" of `detection-streaming-source.feature`, and the exact-byte tests of `fakeflac-export` and `src-tauri`. A version-1 file stays a valid version-1 file either way | (a) no change | 6.2 |
| Q9 | **Cache.** The cache schema (`user_version` 1) and the engine do not change (no params, extraction or algorithm version changes). The core's re-check of every hit (`result-cache.md` section 3) gains one condition: **a returned analysis whose `evidence` is empty while its verdict is `fake`, or not empty while it is not, is a miss.** `evidence` deserialises as an empty list when the key is absent, so a row written before RF-17 parses: a `Real`, `Suspicious` or `Error` row is still a hit (it is still correct), and a `Fake` row is analysed again and replaced. The first release with RF-17 recomputes every result anyway, because the engine includes the app version (ADR-0044); the re-check is what protects development builds, which keep the app version, and any cache an older build of the same version wrote. Alternatives: no re-check (a development build would show cached fakes without evidence until the cache is cleared), or a schema version 2 (every row of every verdict thrown away for one field) | Re-check, no schema or engine change | 6.3; `result-cache.md` 3 |
| Q10 | **Contract version.** A new field of `TrackAnalysis` and two new types change the generated bindings, so the contract becomes **version 9** (`commands.md`, "Fake evidence (version 9)") | Version 9 | 6.1 |
| Q11 | **Where the evidence is shown.** In the detail panel (one line per item, plus the gap-only sentence and the album note) and in the State cell's tooltip, where reasons are shown today. Not in the Albums panel, not as a table column, not in the status bar. A sortable "margin" column would be a later requirement | Detail panel and tooltip | UI spec 1 |
| Q12 | **The texts** of `ui/fake-evidence.md` section 6, English and Spanish, in particular the gap-only sentence ("The evidence is the gap pattern at 16 kHz, not the cutoff.") and the two album notes. They name "16 kHz" literally, as `reason.highBandGaps` does: a params version that moves `gaps.edge_hz` changes these texts too | As written | UI spec 6 |
| Q13 | **ADR.** No dependency, crate, port or layer changes: a field of a core type computed by a pure core function (as `estimated_source_bitrate_kbps` was), one more cache re-check and an interface note. Proposal: **no new ADR**; the reasons are Q1, Q2, Q5 and Q9 of this spec (as `csv-export.md` Q15 did). With Q2 (b) or Q5 (b) there is a new detection version (`rules.md` and `lossy-source.md` give every lossless and lossy rule change its ADR, except v8, `lossy-source.md` Q39); with Q5 (c) an ADR extending ADR-0011 is needed | No ADR | — |
| Q14 | **Changes to approved specs and protected tests** in this PR (section 9): pointer notes only in `rules.md`, `lossy-source.md`, `csv-export.md`, `ui/{components,main-screen,lossy-formats,album-summary,verdict-usability,translations}.md`; the new section and types of `commands.md` (version 9); one re-check in `result-cache.md` section 3; and, in the test stage, the protected tests that build a `TrackAnalysis` field by field or compare its JSON gain the `evidence` field, without changing any other assertion. No scenario of an existing feature file changes. Agree? | Yes | 9 |
| Q15 | **Priority and milestone** of RF-17: `medium`, H2 (an explanation of existing verdicts; no detection change) | medium, H2 | `requirements/RF-17.md` |
| Q16 | **What a `Fake` can claim (added after the rip-log check of section 1, "Authentic releases").** (a) **Keep the verdict label "Fake" and add the audio note** `evidence.audioNote` as the last evidence line of every `Fake` ("These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio."), with the known limitation of section 7; no evidence text asserts that someone converted the file. (b) Also rename the verdict label (for example "Lossy traces"): a change to `verdict.fake`, the status bar, filters, CSV texts and every screenshot, better as its own requirement | (a) | 1, 7, `ui/fake-evidence.md` sections 2 and 6 |

**Decided defaults (not questions).** They follow from the scope or from approved decisions, and are listed so that G1 sees them.
- **D1. Only `Fake` carries evidence.** `evidence` is empty for `Real`, `Suspicious` and `Error`, also when a `Fake` condition holds on a track that an earlier rule decided (V2 little treble, L2, a silent track). A `Fake` always has at least one item (section 2.4).
- **D2. Evidence follows the params the track was analysed with.** It is computed with the same `DetectionParams` as the verdict, for any params file the core loads (v2 to v8), so the calibration tool and tests that use an older file get that file's evidence (for example `edge_bump19k3` is `Fake` with `cutoffStep` evidence under `detection-params.v5.toml` and `Suspicious` without evidence under v6 and later).
- **D3. Nothing about the guards that did not fire.** The evidence names the rules that made the track `Fake`. It does not describe why the full-band guard (`FullBand`) or the edge step (`EdgeStep`) did not protect it.
- **D4. Values are `f32`**, like `CutoffEstimate`, so the cache reads them back bit for bit (`result-cache.md` section 2.1). The margin is computed before the conversion (section 2.4), so it is never negative.
- **D5. Fixed order.** Lossless: `gapStep`, then `cutoffStep` (the order of V3 and V4). Lossy: `shortfall`, then `cutoffStep`.

## 1. What and why

**Evidence from the owner's library (2026-11-14).** Aggregates only; no file, folder or artist name (ADR-0022). The owner analysed the whole library with the release app (11,396 tracks). Of the 4,233 lossless tracks, 501 were `Fake`. Re-measuring them with the shipped engine and the default params (detection v8) showed which rule fired:

| Group | Fakes | Both V3 and V4 | V4 only (step) | V3 only (gap) |
|---|---|---|---|---|
| All lossless fakes | 501 | 218 | 188 | 95 |
| In the 24 albums where at least 75 % of the tracks are `Fake` ("whole-album") | 223 | 117 | 84: median `D` 24.7 dB; 5 with `D` < 13 dB | 22 |
| In the 92 albums where fewer than 25 % of the tracks are `Fake` ("isolated"; 52 of those albums have exactly one) | 142 | 33 | 74: median `D` 13.2 dB; 35 with `D` < 13 dB | 35: median `G` 9.6 dB; 19 with `G` < 10 dB; 19 report a cutoff at about 22 kHz (full band) |

The "V3 only" figure of the whole-album group and the "both" figure of the isolated group are the differences of the other figures; the remaining 136 fakes lie in albums with 25 % to 75 % `Fake` tracks. With `step_fake_db` = 10 dB and `gap_fake_db` = 6 dB, `D` < 13 dB is a margin under 3 dB and `G` < 10 dB a margin under 4 dB: **about 38 % of the isolated fakes are within 3–4 dB of a threshold, against about 3 % of the whole-album fakes.** The app shows both groups the same way ("Fake", no reason). The gap-only fakes whose reported cutoff is full band are the hardest to read: "Fake, cutoff 22 kHz" says nothing about why, because their evidence is the gap pattern at 16 kHz, not the cutoff (the reported cutoff is the content extent `fe` when `D < step_suspicious_db`, `rules.md` section 2).

**Authentic releases (rip-log check and independent review, 2026-11-14).** Of the 135 albums with at least one lossless `Fake`, 6 have a CD rip log. In 5 of them, 35 `Fake` tracks are bit-exact matches of the log's copy CRC: the file is the untouched rip. 8 of those 35, in 2 albums, are also confirmed by the AccurateRip or CTDB databases, so they are rips of a published disc that other people own (a log alone does not prove that: a CD-R burnt from MP3 files gives a matching log too). Their evidence is not marginal: in one database-verified album 6 of 11 tracks are `Fake` with `G` from 14 to 35 dB, and a log-matched album is 16 of 16 `Fake` with `G` about 50 dB and `D` about 30 dB. An independent review of 177 of the 501 fakes confirmed 3 as genuine (all gap-only, database-verified rips), none as transcodes, and left 174 undecided. **So a `Fake` states that the audio carries the traces of lossy compression. It cannot tell a file that someone converted from an authentic release whose master was made from lossy material** (sampled or MP3-sourced production, a label that mastered from lossy files). No threshold clears such tracks, and RF-17 does not try: it says what was seen and what that cannot tell (Q16, section 7). The corpus side (a separate class for these releases, outside acceptance target T1) is specified apart, as an RP-11 follow-up.

**Little ground truth.** Apart from the tracks above, nobody knows yet which of these 501 are transcodes. RF-17 does not decide that: it gives the user the evidence and its strength, and the album context, so that the user can review the doubtful ones (with the spectrogram, RF-07).

## 2. Evidence items

`TrackAnalysis.evidence` is a list of `FakeEvidence { kind, value, threshold, margin }`. `kind` is one of:

| `kind` | `value` | `threshold` | Unit | Meaning |
|---|---|---|---|---|
| `gapStep` | `G`, the gap step at `gaps.edge_hz` (`rules.md` section 2) | `verdict.gap_fake_db` | dB | Intermittent empty bands above 16 kHz, as an MP3 leaves them (rule V3) |
| `cutoffStep` | `D`, the cutoff step at `fs` (`rules.md` section 2) | lossless: `verdict.step_fake_db`; lossy: `lossy.fake_min_step_db` | dB | A sharp cutoff: in the lossy range (V4), or at a lossy file's early edge (L3) |
| `shortfall` | `s = c_exp − fc` (`lossy-source.md` section 3) | `lossy.fake_shortfall_hz` | Hz | The content ends far below what the declared bitrate keeps (L3) |

`margin = value − threshold` (≥ 0, section 2.4).

### 2.1 Lossless codecs (`pcm`, `flac`, `alac`; `rules.md` section 3)

For a track whose verdict is `Fake`, with the measurements and params of its analysis:
- **`gapStep`** is listed **if and only if** V3's condition holds: `G ≥ gap_fake_db` and not `FullBand`.
- **`cutoffStep`** is listed **if and only if** V4's condition holds: `D ≥ step_fake_db` and `InRange(fs)` and not `FullBand` and not `EdgeStep`.

Both are evaluated whatever the order of the verdict table (Q3): a track that V3 made `Fake` also lists `cutoffStep` when V4's condition holds. `FullBand`, `InRange`, `LossyStep` and `EdgeStep` are those of `rules.md` section 2, evaluated with the same params (without a `[full_band]` section `FullBand` is false; without `[edge]` `EdgeStep` is false).

### 2.2 Lossy codecs (`mp3`, `aac`, `opus`; `lossy-source.md` section 4)

A lossy `Fake` comes only from L3 (`s` defined, `s ≥ fake_shortfall_hz` and `D ≥ fake_min_step_db`). It lists exactly two items, in this order (Q4):
- **`shortfall`**: value `s`, threshold `lossy.fake_shortfall_hz`;
- **`cutoffStep`**: value `D`, threshold `lossy.fake_min_step_db`.

`G`, `FullBand` and the lossy range are not part of the lossy rules (`lossy-source.md` section 4), so a lossy track never lists `gapStep`, whatever its `G`. The estimate, the reported cutoff (`fc`) and the declared bitrate are already in `TrackAnalysis`; `c_exp` is `fc + s`.

### 2.3 Every other track

`evidence` is empty (D1): `Real`, `Suspicious` (any rule, any reason), `Error` (any `TrackError`), and a silent track. An `Error` from L0 or L1 has no measurements, and a `Suspicious` from V2, L2 or the silent-track rule is not `Fake` even when a `Fake` condition would hold.

### 2.4 Values and invariants

- **`value`** is the measurement exactly as the rule compared it; **`threshold`** is the params value it was compared with; **`margin`** is `value − threshold` computed in the precision of that comparison. All three are then converted to `f32` (D4). Since the rule held (`value ≥ threshold`), `margin ≥ 0`, and the conversion keeps it `≥ 0`. A `NaN` value never satisfies a rule, so every value is finite.
- **Non-empty exactly for `Fake`.** A lossless `Fake` comes from V3 or V4 (V1, V2 and the silent-track rule come first and give other verdicts), so at least one of the two conditions holds and the list has one or two items. A lossy `Fake` comes from L3 and has two items.
- **No duplicates**, in the fixed order of D5.
- **Deterministic and pure.** The same measurements, stream information and params always give the same evidence.

### 2.5 Where it is computed

A pure public function of `fakeflac_core::detect`, next to `decide`, `cutoff_estimate` and the lossy verdict functions (`lossy-source.md` section 3, "Where it is computed"). It receives the measurements, the params, the codec, the shortfall (when defined) and the verdict already decided, and returns the list. `analyze_track` calls it **after** the verdict and stores its result in `TrackAnalysis.evidence`. **No rule reads it**: `decide`, the lossy verdict, `cutoff_estimate` and the estimate keep their inputs and outputs. No I/O. Exact names and signatures are left to the implementation. The calibration tool does not need it (its `tracks.csv` already has signed margins against every threshold, `calibration/evaluation.md` section 8: for a lossless `Fake`, `gapStep`'s margin is `−m_gap_fake_db` and `cutoffStep`'s is `−m_step_fake_db`).

## 3. Strength

The strength of the evidence is its **margins** (Q2): how far each measured value lies past the threshold of the rule it satisfies. A `Fake` whose only item has a margin of 0.4 dB is a borderline call; one with `gapStep` +13 dB and `cutoffStep` +15 dB is overwhelming. Two items mean two independent signs agree. There is no label, no aggregate score and no boundary: the core gives the numbers, the interface shows them (`ui/fake-evidence.md` section 2), and the user judges. Margins are in the unit of their measurement; a dB margin on `G` and one on `D` are not the same physical quantity, and a `shortfall` margin is in Hz, so the interface never adds or compares them.

## 4. Album context (interface only)

Transcodes usually affect whole albums (a ripped or downloaded album comes from one source); a `Fake` among genuine tracks of the same folder deserves a look at its spectrogram. The album is the one of RF-10 (`detection/album-summary.md`: the tracks with the same direct parent folder), and its counts are the `AlbumSummary` of the latest `AlbumUpdate` received for it (`commands.md`, "Album summary (version 5)"; `ui/album-summary.md`).

For a selected track whose verdict is `Fake`, with `f = summary.fake` and `n = summary.real + summary.suspicious + summary.fake` (analysed tracks; errors left out, Q6; counts so far during a run, Q7):
- `n ≤ 1`: no note (nothing to compare with).
- `f = n` (every analysed track of the album is `Fake`): no note (the pattern of a transcoded album).
- `f = 1 < n`: the note "the only Fake of the `n` analysed tracks" (`evidence.albumOnly`).
- `2 ≤ f < n`: the note "`f` of the `n` analysed tracks are Fake" (`evidence.albumSome`).

Texts and placement: `ui/fake-evidence.md` section 3. **The note never changes the track's verdict, the album's verdict or any count**, and it is not part of `TrackAnalysis`, `AlbumSummary`, the CSV or the cache. The comparison of `f` and `n` only selects a text (Q5 (a)): there is no tunable boundary, so nothing goes into `DetectionParams`, and `summarize_album` stays without parameters (`album-summary.md`, "Non-goals"). The panel's counts and the note cover every track of the album in the run, whatever the table's sort and filters (as the Albums panel, `ui/sort-filter.md` section 6).

## 5. Versions: what does not change

- **No new detection version.** `DetectionParams` has no new section or value; `detection-params.v8.toml` stays the default; `algorithm_version` stays 8; `EXTRACTION_VERSION` stays 2; the next free `algorithm_version` stays 9 (`rules.md` section 4).
- **Verdicts unchanged, by construction.** The evidence function runs after the verdict, reads only values the verdict already used, and writes only `evidence` (section 2.5). So for the same file and params, `info`, `verdict`, `cutoff`, `estimatedSourceBitrateKbps`, `reasons`, `error` and `algorithmVersion` equal those of the code before RF-17.
- **How it is shown** (section 8): every scenario of the existing detection, decoding, album, cache and CSV feature files passes unchanged (X1); every fixture keeps the truth of its fixture spec (X2); `just accept` on the owner's corpus gives a `summary.md` identical to the last run before RF-17, run metadata aside (X3); and an app export of the owner's library is byte-identical to the 2026-11-14 export (M2).
- **Cache engine unchanged** (Q9): RF-17 itself invalidates nothing; the release that ships it recomputes every result through the app version (ADR-0044); in between, only cached `Fake` rows without evidence are analysed again (section 6.3).

## 6. Contract, CSV, cache, calibration

### 6.1 Contract (version 9)

`TrackAnalysis` gains `evidence: FakeEvidence[]`; new types `EvidenceKind` (`'gapStep' | 'cutoffStep' | 'shortfall'`) and `FakeEvidence` (`kind`, `value`, `threshold`, `margin`, all numbers). Nothing else changes. Full text: `commands.md`, "Fake evidence (version 9)".

### 6.2 CSV (Q8)

No change: `evidence` is not exported, `csv_format_version` stays 1, and the record of a `fake` track is byte-identical to its record before RF-17 (its `reasons` cell stays empty). Old CSV files and every RF-09 scenario stay valid.

### 6.3 Result cache (Q9)

- The stored value is the core `TrackAnalysis` as before (`result-cache.md` section 2.1), now with `evidence`; `f32` values round-trip bit for bit.
- `evidence` deserialises as an empty list when the key is absent, so a row written before RF-17 still parses.
- The core's re-check of a hit (`result-cache.md` section 3) adds: **`evidence` is not empty exactly when `verdict` is `fake`.** Otherwise the hit is a miss (not a failure): the file is analysed again and its row replaced on the next `put`.
- No schema version change (`user_version` stays 1), no engine change, no new `CacheStatus`.

### 6.4 Calibration tool

No change. It does not use `analyze_folder` or the app cache, its `tracks.csv` already has the signed margins (section 2.5) and its `reasons` column, which RF-17 does not touch, and its verdict-change tables compare verdicts and reasons, which do not change. If the tool builds `TrackAnalysis` values field by field, it adds the field (an implementation change with no effect on its outputs).

## 7. Known limitations

- **A `Fake` is about the audio, not about the file's history.** An authentic CD rip or store purchase is `Fake` when the release itself was mastered from lossy audio (section 1, "Authentic releases": 8 database-verified tracks in the owner's library). The evidence texts describe what was measured and never say that the file was converted; the audio note says so on every `Fake` (Q16).
- **A margin is a distance to a threshold, not a probability.** The owner's library has no ground truth for these tracks (section 1); a large margin can still be a false positive (a genuine master with a sharp low-pass in the lossy range, `rules.md` section 6) and a small one a transcode.
- **dB margins of different kinds are not comparable**, and the `shortfall` margin is in Hz (section 3).
- **The album is a folder** (RF-10). A multi-disc album split into `CD1` and `CD2` counts as two albums, a compilation or a folder that mixes sources shows the note on tracks whose context says little, and a single-track folder never gets a note.
- **The note does not tell "few" from "many".** With no boundary (Q5 (a)), an album with 11 of 12 tracks `Fake` shows "11 of the 12 analysed tracks are Fake" too; the counts are in the text so that the user can tell.
- **Guards that did not fire are not explained** (D3). A gap-only `Fake` whose upper window has content but whose cutoff step lies in the lossy range lists `gapStep` only; why the full-band guard did not hold (`LossyStep`) is not shown.
- **"16 kHz" is literal in the texts** (Q12).

## 8. Acceptance criteria

| # | Criterion | Pass |
|---|---|---|
| X1 | Existing behaviour unchanged | **Gate.** Every scenario of `detection-lossless.feature`, `detection-full-band.feature`, `detection-edge-step.feature`, `detection-lossy.feature`, `detection-streaming-source.feature`, `opus.feature`, `decode-lossy.feature`, `album-summary.feature`, `album-summary-app.feature`, `result-cache.feature`, `export-csv.feature`, `sort-filter.feature` and `verdict-usability.feature` passes (`just test`), with no change to any of them. The only edits to their tests are those of section 9 (f) |
| X2 | Fixture truths unchanged | **Gate.** Every synthetic signal of `fixtures.md` (as FLAC) and every fixture of `lossy-fixtures.md` sections 2 and 2.2, analysed with the defaults, has the verdict, reasons, error, cutoff (within its tolerance), estimate and `algorithmVersion` 8 of its fixture spec, and evidence that is non-empty exactly when it is `Fake` (scenarios of `fake-evidence.feature`) |
| X3 | Corpus unchanged | **Gate**, measured locally before the feature PR: `just accept` with its default config and params on the owner's corpus gives a `summary.md` identical to that of the last accepted run before RF-17 except the run metadata (run id, dates, tool commit, timings), as RNF-02's acceptance did. A difference is an implementation bug (the evidence must not touch the verdict path) |
| X4 | Evidence rule | **Gate.** The rule-level and fixture-level scenarios of `fake-evidence.feature` pass: the kinds are exactly the rules of section 2 whose conditions hold, `value`, `threshold` and `margin` match section 2.4, and `margin ≥ 0` |
| X5 | Cache | **Gate.** The cache scenarios of `fake-evidence.feature` pass: a cached `Fake` without evidence is analysed again, other cached rows written before RF-17 are hits, and evidence round-trips bit for bit |
| X6 | Contract and CSV | **Gate.** `just gen-check` passes with the version 9 bindings; the CSV document of stubbed tracks is byte-identical with and without evidence |
| X7 | Interface | **Gate.** The interface scenarios of `fake-evidence.feature` pass in English and Spanish, and the catalogue test covers the keys of `ui/fake-evidence.md` section 6 |
| M1 | Owner walkthrough (release build of the feature branch, the owner's library, English and Spanish) | **Manual.** The owner selects at least: a gap-only `Fake` whose cutoff is full band; a step-only `Fake` with a margin under 3 dB; a `Fake` with both items; a lossy `Fake`; a `Fake` that is the only one in its album; a `Fake` of an album where every track is `Fake`. For each, without project documentation, the owner says which evidence made it `Fake`, how strong it is, and whether the album note appears and why. Pass/fail per language and any confusing wording are recorded; names and paths are not (ADR-0022) |
| M2 | Library unchanged | **Manual.** With unchanged audio files, the CSV that the feature build exports for the owner's library is byte-identical to the one the release build exported on 2026-11-14 (same `algorithm_version` 8; the CSV does not carry evidence, Q8). If the library changed since, the status-bar counts of a run of both builds on the same folder are compared instead and must be equal |

No timing criterion: the evidence is a few comparisons on values already computed, and the note reads counts the page already holds (RNF-01 unaffected). If X1, X2, X3 or M2 fails, it is an implementation bug: it is fixed in the `implement` stage and never by changing a spec, a test or a parameter.

## 9. Changes to approved specs and protected tests (declared, Q14)

(a) `contracts/commands.md`: front matter (version 9, RF-17), the introduction, the `Types` block (two types and one field, marked "version 9 (RF-17)") and the new section "Fake evidence (version 9)".
(b) `contracts/result-cache.md` section 3: one more condition of the core's re-check, marked "(RF-17)"; section 2.1 unchanged in substance.
(c) `contracts/csv-export.md` section 3: a note that the evidence is not exported and `csv_format_version` stays 1.
(d) `detection/rules.md` section 3 and `detection/lossy-source.md` section 4: a pointer note after the verdict table; the reasons columns (`—` for V3, V4 and L3) stay as they are.
(e) `ui/components.md`, `ui/main-screen.md`, `ui/lossy-formats.md`, `ui/album-summary.md`, `ui/verdict-usability.md`, `ui/translations.md`: pointer notes to `ui/fake-evidence.md`. No approved text is rewritten; `components.md`'s "a `Fake` verdict has none" stays true of `reasons`.
(f) **Protected tests, in the test stage (`test(RF-17)`), as a declared consequence.** `TrackAnalysis` gains a field, so tests that build one field by field stop compiling or type-checking, and tests that compare its exact JSON miss the new key. The test author adds `evidence` (empty, or the expected items where the test builds a `Fake` that later reaches the cache or the DTO) and changes no other assertion. Candidates found by the spec stage (struct literals and exact JSON; patterns with `..` need nothing): `crates/fakeflac-core/tests/{result_cache,album_summary,export_csv}.rs` and `tests/common/cache.rs`; `crates/fakeflac-store/tests/sqlite_store.rs`; `crates/fakeflac-export/tests/{csv_streaming.rs,common/mod.rs}`; `crates/fakeflac-audio/tests/{detection_lossless,detection_lossy,detection_edge_step,detection_streaming,decode_lossy,opus}.rs`; `tools/fakeflac-calibrate/tests/sample_corpus.rs`; `src-tauri/tests/{contract,contract_streaming,export_csv,result_cache,run_analysis_streaming,run_analysis_lossy_v5}.rs`; `src/lib/testing/fixtures.ts` (the stub builder gains `evidence: []`, so the frontend tests that use it need nothing more); and `src/lib/i18n/catalogue.test.ts`, which reads the new translations section of `ui/fake-evidence.md` as it reads `ui/resizable-widths.md` section 4. The test author lists the exact set in the `test(RF-17)` commit. Production code that builds `TrackAnalysis` (`fakeflac-core` `analysis.rs`, `src-tauri` `dto.rs`) changes in the `implement` stage; `src/lib/api/bindings.ts` is regenerated.
(g) Outside `specs/` (for `docs-keeper`): `docs/architecture/backend.md` lists the domain types (add `FakeEvidence`, `EvidenceKind`).

## 10. Test signals and scenarios

- No new fixture. Lossless: `sfb21_gaps` (gap only, cutoff at Nyquist: the full-band case of section 1), `sfb21_gaps_lp20k5` (gap only, cutoff 20.5 kHz), `shelf19k5`, `lp19k`, `holes_lp19k5` and `edge_bump19k3_lp19k5` (step only), `lp16k` and `m4a_alac_lp16k` (step, gap not asserted), `edge_bump19k3` (`Fake` with v5, `Suspicious` with the defaults: D2). Lossy: `mp3_320_lp17k`, `m4a_aac_256_lp15k3`, `opus_160_src16k`. The "both" case is covered at rule level (no fixture is specified with both `G ≥ 6 dB` and an in-range step of 10 dB or more).
- Scenarios: `specs/features/fake-evidence.feature` (tag `@RF-17`): the rule from given measurements (lossless and lossy, every boundary), fixtures with the defaults and older params, the invariants, unchanged results, the real run with Unicode names (RNF-09) that leaves the files unchanged (RNF-05) and with a corrupt file (RNF-04), the contract, the cache (old rows), the CSV (unchanged), the calibration report (unchanged), and the interface (detail panel, gap-only sentence, album note, tooltip, empty state, Spanish).

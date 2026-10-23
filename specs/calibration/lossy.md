---
id: SPEC-CALIBRATION-LOSSY
requirements: [RP-11, RF-15, RNF-04, RNF-09]
status: proposed
config: specs/calibration/calibration.v2.toml
---

# Calibration for lossy files (RF-15): labels, cascades and evaluation

> Extends the calibration tool so that RF-15 can be measured: `trusted.txt` and `confirmed.txt` accept MP3, M4A and Opus files with a few attributes; the tool generates honest lossy encodes and **cascades** (a lossy file re-encoded to a higher bitrate) from the trusted lossless tracks; tracks of one master in several versions are **paired**; and lossy rows get **their own tables**. Nothing that exists today changes for lossless rows. Main spec: [calibration.md](calibration.md). Rules being evaluated: `specs/detection/lossy-source.md`; acceptance B1–B11 there (section 9). Scenarios: `specs/features/calibration-lossy.feature`.

## Open questions for G1

| # | Question | Status |
|---|---|---|
| C1 | **Attributes in field 2** of both files (`formats=`, `group=`, `chain=`, `upstream=`, `source_max_kbps=`, `weak`), section 1.2, rather than new fields or new files | Open. Default: attributes |
| C2 | A **new `confirmed.txt` kind `real`** for honest lossy files whose origin is not a trusted store (the 14 streaming-site AAC 128 files copied without re-encoding), class `reviewed`. The one currently labelled `fake` becomes `real` | Open. Default: yes |
| C3 | `trusted.txt` **accepts lossy files** as honest lossy negatives with their origin's class (a store-bought MP3 is `primary`). They are never transcode sources. A folder line keeps including only FLAC and WAV unless it says `formats=lossy` or `formats=all` | Open. Default: yes |
| C4 | `upstream=by-rate` resolves the upstream codec of each track from its sample rate with a config table (`calibration.v2.toml`: 44.1 kHz → AAC, at most 128 kbps; 48 kHz → Opus). It encodes the owner's observation about streaming-site rips as data, not as code | Open. Default: yes |
| C5 | **Pairing across versions** by the leading track number of the file stem, else by the stem reduced to letters and digits (section 6.5) | Open. Default: yes |
| C6 | **Split units**: `group=` (for example one main artist across several albums) and `chain=` make several albums one split unit; units whose tracks are all lossy are stratified apart, so adding lossy labels never moves a lossless album (section 3) | Open. Default: yes |
| C7 | **`EXTRACTION_VERSION` stays 2.** The new `stream` information of a FLAC or WAV entry is fully determined by its format and its spectra do not change, so old cache lines stay valid (section 5). Bumping to 3 would re-extract every track and re-transcode every generated FLAC whose audio was deleted | Open. Default: no bump |
| C8 | **`calibration.v2.toml` becomes the default `--config`**, with 14 lossy entries per source (about 20,000 entries and 29,000 ffmpeg calls on today's corpus, once, then cached; section 4.6). Fewer entries, or keep v1 as the default and pass v2 explicitly? | Open. Default: v2 is the default |
| C9 | Honest generated encodes whose **source** already ends early are counted as **source-limited** and left out of the rates, as undetectable transcodes are on the positive side (section 4.5) | Open. Default: yes |
| C10 | The **grid search ignores lossy rows** and rejects `lossy.*` keys in this version (section 7) | Open. Default: yes |
| C11 | Changed messages of approved scenarios: `not a FLAC or WAV file` becomes `not a supported audio file (FLAC, WAV, MP3, M4A or Opus)`, and `unknown kind 'x' (expected fake or false-positive)` becomes `unknown kind 'x' (expected fake, real or false-positive)` (calibration.feature, "Errors in trusted.txt" and "Errors in confirmed.txt") | Open. Default: yes |
| C12 | No new `[[targets]]` kind for lossy rows: acceptance B is read from the tables of section 6 | Open. Default: yes |

## 0. Compatibility

- **Lossless rows are evaluated exactly as today.** Every table, figure, list and target of `evaluation.md` sections 3–9 (and the grid search) uses lossless-domain rows only (section 6.1). A run with `calibration.v1.toml` and only lossless labels gives the same lossless figures as before this spec.
- `labels.csv` and `tracks.csv` gain columns, appended after the existing ones (sections 2 and 6.4). Runs stay deterministic (calibration.md section 4).
- The split of an album changes only if its unit changes: when one of its lines gains `group=` or `chain=` (section 3).
- **Edits to the approved calibration specs in this PR** (lossy-source.md Q13 (g)): `calibration.md` (front matter, related specs, scope: RF-15 in, `--config` default v2, accepted file types in section 4); `inputs.md` (accepted extensions and pointer to section 1 here, kind `real`, new `labels.csv` columns, split units); `generation.md` (lossy entries in the summary); `evaluation.md` (lossless-only domain in section 2, grid search ignores lossy rows, next free version 6, v5 in the rule family and its checks in section 9); `sample-corpus.md` (next free version 6, pointer to the lossy variant); `calibration.feature` (C11 messages, proposals v6, and the default config, now v2, in "ffmpeg missing when a transcode is needed"). Every other rule of those files is unchanged.
- The 20 lossy `fake` lines that the owner keeps commented out in `confirmed.txt` (docs/agents/progress.md, 2026-10-18) can be enabled once this is implemented: today they fail with `not a FLAC or WAV file`; with this spec they parse, and a params file without `[lossy]` simply does not evaluate them (section 6.1).

## 1. Inputs (extends inputs.md sections 1 and 2)

### 1.1 Accepted files

- **File entries** of `trusted.txt` and `confirmed.txt` may name `.flac`, `.wav`, `.mp3`, `.m4a` or `.opus` files (any letter case). Otherwise: `not a supported audio file (FLAC, WAV, MP3, M4A or Opus)` (C11).
- **Folder entries** include, recursively, the files selected by `formats=` (section 1.2), even though the app's scanner now also lists MP3 and M4A files (the tool filters what it enumerates): `lossless` (the default of `trusted.txt` and `fake` folder lines, as today) → `.flac` and `.wav`; `lossy` (the default of `real` folder lines) → `.mp3` and `.m4a`; `all` → the four. A `real` line accepts only `lossy` (1.2). A folder with no selected file is an error on its line: `no FLAC or WAV files under this folder` (unchanged), `no MP3 or M4A files under this folder`, or `no supported audio files under this folder`.
- **Opus** (decoder pending, ADR-0030): `.opus` files are never enumerated under folders; a file entry naming one is accepted, labelled, and **not evaluated** (section 6.1).
- **Domain.** Each labelled track is `lossless` or `lossy` by its codec, known after extraction (lossy-source.md section 2): `pcm`, `flac`, `alac` → `lossless`; `mp3`, `aac`, `opus` → `lossy`. An `.m4a` that cannot be probed is an `Error` row of the lossy domain. A trusted ALAC `.m4a` is a lossless trusted track like any FLAC, and a transcode source.

### 1.2 Field 2: a word and attributes

Field 2 of an entry is a **word** (the origin in `trusted.txt`, the kind in `confirmed.txt`) followed by zero or more **attributes**, separated by spaces. An attribute is `name=value` or a bare flag. Exclusions (`!path | reason`) take no attributes.

**Which line gives a track its attributes.** The file that gives the track its label: a trusted track (including a trusted track that is also a `false-positive` watch track) takes its attributes from the `trusted.txt` line that decides it; a `fake` or `real` track from the `confirmed.txt` line that decides it. "Decides" is the most specific line of that file (inputs.md section 1.4). Attributes are not inherited from enclosing folder lines, and the two files never combine attributes for one track. **`false-positive` lines take no attributes** (`false-positive lines take no attributes; write them on the trusted.txt line`): a watch track's group and chain are those of its trusted line.

| Attribute | Lines | Value | Meaning |
|---|---|---|---|
| `formats=` | folder entries of `trusted.txt`; `fake` and `real` folder lines of `confirmed.txt` | `lossless` (default of `trusted.txt` and `fake` folders), `lossy` (default of `real` folders), `all`; a `real` line accepts only `lossy` | Which files the folder includes (1.1) |
| `group=` | entries of `trusted.txt`; `fake` and `real` lines of `confirmed.txt` | an id | Split unit (section 3), for example one id per main artist |
| `chain=` | entries of `trusted.txt`; `fake` and `real` lines of `confirmed.txt` | an id | The tracks are versions of one master (section 6.5) |
| `upstream=` | `confirmed.txt` `fake` lines | `aac`, `opus`, `mp3`, `unknown`, `by-rate` | The codec of the lower-bitrate stage the file came from (1.5) |
| `source_max_kbps=` | `confirmed.txt` `fake` lines | an integer from 8 to 640 | An **upper bound** of the source bitrate: the source is at most this (chains may have had 3 or more lossy stages) |
| `weak` | `confirmed.txt` `fake` lines | flag | The provenance is uncertain (for example a bootleg tape compilation); reported in its own row, never gated |

An **id** is 1 to 16 characters among `A–Z`, `a–z`, `0–9`, `_` and `-`. Ids appear in `summary.md` (section 6.6), so they must be opaque (`C1`, `G7`), never names.

`upstream=`, `source_max_kbps=` and `weak` apply only to the lossy tracks of the line; a `fake` line that covers no lossy file and carries one of them is an error. This check runs when the file is parsed, so it goes by extension: `.mp3`, `.m4a` and `.opus` count as lossy.

Errors, reported on the line like every other error of these files (inputs.md section 1.6): `false-positive lines take no attributes; write them on the trusted.txt line`, `unknown attribute 'x'`, `attribute 'x' given twice`, `formats= applies to folders only`, `invalid value 'v' for formats (expected lossless, lossy or all)`, `invalid id 'v' for group (1 to 16 letters, digits, '_' or '-')` (and `for chain`), `invalid value 'v' for upstream (expected aac, opus, mp3, unknown or by-rate)`, `source_max_kbps must be an integer from 8 to 640`, `upstream=, source_max_kbps= and weak apply to fake lines only`, `upstream=, source_max_kbps= and weak apply to lossy files only`, `real applies to lossy files only` (a `real` folder line with `formats=lossless` or `formats=all`, or a `real` file entry naming a `.flac` or `.wav` file; like the check above, it goes by extension).

### 1.3 Kinds of `confirmed.txt`

| Kind | Path | Meaning |
|---|---|---|
| `fake` | folder or file, lossless **or lossy** | Lossless: as today (inputs.md section 2). Lossy: the content comes from a lower bitrate than declared. Label `truth = fake`, `origin = confirmed_fake`, `class = positive`, plus the attributes of 1.2 |
| `real` (new, C2) | folder or file, **lossy only** | An honest lossy file: its content matches its declared bitrate (for example a stream copied without re-encoding). Label `truth = real`, `origin = confirmed_real`, `class = reviewed` |
| `false-positive` | file only | As today, for a trusted track of either domain. A lossy watch track is listed with the lossy measurements (section 6.7) |

- `unknown kind 'x' (expected fake, real or false-positive)` (C11).
- A track included by a `real` line that is also included by `trusted.txt`: `also trusted (trusted.txt line N)`. A track included by both a `real` and a `fake` line: `already listed at line N` on the later line.
- A `real` line whose track turns out to be lossless after extraction (an ALAC `.m4a`) is not an input error (the codec is not known when the file is parsed): the track gets no label row and is listed in the report header as `confirmed.txt line N: real applies to lossy files; the track holds ALAC (excluded)`.

### 1.4 Lossy files in `trusted.txt` (C3)

A trusted lossy file is an **honest lossy negative**: `truth = real`, `origin = trusted`, `trust_origin` and `class` from its origin (inputs.md section 1.5), domain `lossy`. It is never a source for transcodes or cascades. A store-bought MP3 listed with origin `tienda` is class `primary`, so it feeds acceptance B4.

### 1.5 `upstream=by-rate` (C4)

For each lossy track of the line, the tool looks up the decoded sample rate in `[labels.upstream_by_rate]` of the config. A match gives the upstream codec and, if the table has one, the source bound; no match gives `unknown` and no bound. An explicit `source_max_kbps=` on the line wins over the table's bound. `calibration.v2.toml` holds `44100 → aac, 128 kbps` and `48000 → opus, no bound`: on a streaming site, a 44.1 kHz rip comes from its AAC 128 stream and a 48 kHz rip from its Opus stream.

Checks on `[labels.upstream_by_rate]` (exit 2, with the other config errors, before any extraction): each key is a sample rate in Hz written as a positive integer (digits only, no leading zero: `"44100"`); `upstream` is `aac`, `opus`, `mp3` or `unknown` (`by-rate` is not a value of the table); `source_max_kbps`, when present, is an integer from 8 to 640. Messages: `labels.upstream_by_rate: invalid sample rate '<key>' (expected an integer in Hz, for example 44100)`, `labels.upstream_by_rate '<key>': invalid upstream '<v>' (expected aac, opus, mp3 or unknown)`, `labels.upstream_by_rate '<key>': source_max_kbps must be an integer from 8 to 640`. A missing `upstream` or an unknown field is a TOML error, as for the rest of the config.

## 2. New columns of `labels.csv` (extends inputs.md section 3)

Appended after `watch_id`, in this order. Empty when the row says nothing.

| Column | Values |
|---|---|
| `domain` | `lossless` or `lossy` (1.1) |
| `codec` | `pcm`, `flac`, `alac`, `mp3`, `aac`, `opus`; empty if the file could not be probed |
| `declared_bitrate_kbps`, `bitrate_mode` | lossy rows: from the stream (lossy-source.md section 2) |
| `stages` | generated lossy rows: the stage setting ids joined with `>` (`aac128>cbr320`) |
| `source_bitrate_kbps` | generated cascades: the lowest `bitrate_kbps` of the stages before the last; confirmed lossy fakes: the source bound (1.2, 1.5) |
| `upstream` | confirmed lossy fakes: as resolved (1.5); generated cascades: the codec of the first stage with the lowest bitrate, in the vocabulary of 1.2 (`lame` → `mp3`, `aac` → `aac`, `opus` → `opus`), so generated and confirmed rows line up in section 6.3 |
| `label_strength` | `weak` for weak labels |
| `chain_id`, `chain_key` | section 6.5 |
| `split_unit` | section 3 |
| `not_evaluated` | `opus decoder pending` for Opus tracks |

New values in existing columns: `origin` `confirmed_real`; `class` `generated` (honest generated lossy rows); `setting` the `[[generate.lossy]]` id for generated lossy rows; `encoder` the last stage's encoder for them. Rows keep the sort order of inputs.md section 3.

## 3. Split units (extends inputs.md section 4; C6)

- The **unit** of a track is the `group=` of the line that gives it its attributes (section 1.2); otherwise `chain:<id>` if that line has `chain=`; otherwise its album id. Generated rows take the unit of their source.
- Every album id must resolve to one unit, and every chain to one unit (`chain C1 spans split units G2 and chain:C1`, units in byte order). Both are input errors (exit 2), reported with the other input errors (inputs.md section 1.6).
  - The album error is `album has tracks in several split units (<ref>, <ref>, …)`, one error per album, printed without a file prefix. It has one `<ref>` per unit of the album: `<file>:<line>` of the first line that gives one of the album's tracks that unit. The refs are in ascending order: `trusted.txt` before `confirmed.txt`, then by line number (`album has tracks in several split units (trusted.txt:4, confirmed.txt:7)`). Several such errors are ordered by their first ref.
- The hash of inputs.md section 4 is computed on the **unit** instead of the album id; for units that are album ids nothing changes.
- The class of a unit is the class an album would get from all its tracks (inputs.md section 4; a `confirmed_real` track counts as `reviewed`). **Units whose labelled tracks are all lossy are stratified among themselves**. Units are formed before extraction, when the codec is not known yet, so "lossy" goes **by file extension** here (`.mp3`, `.m4a`, `.opus`, any letter case), as the check of 1.2 does (a known limitation, section 11). They use the class keys `lossy:primary`, `lossy:reviewed`, `lossy:clean`, `lossy:reference` and `lossy:positive`, so that adding lossy labels never moves a lossless album. Units with a lossless track use the existing strata.
- The report header counts albums as today and, next to them, units per stratum with their `tune`/`holdout` counts; the holdout warning applies per stratum.

## 4. Generation of lossy entries (extends generation.md)

### 4.1 Configuration

- `[[generate.lossy]]` entries: `id` and `stages`, a non-empty list of `[[generate.settings]]` ids applied in order. The last stage decides the declared format of the file. Checks (exit 2): `id` unique among the lossy entries (it may equal a setting id: the single-stage entry `cbr320` is the lossy file of the setting `cbr320`; the `domain` column tells the two rows apart); every stage id exists; in an entry with two or more stages every stage has `bitrate_kbps`; the last stage's `ext` is `mp3` or `m4a`. Messages: `lossy entry <id> is defined twice`, `lossy entry <id>: no stages`, `lossy entry <id>: unknown stage <setting>`, `lossy entry <id>: stage <setting> has no bitrate_kbps`, `lossy entry <id>: Opus output is not supported yet` (last stage's `ext` is `opus`, any letter case), `lossy entry <id>: the last stage's ext must be mp3 or m4a` (any other `ext`).
- `[[generate.settings]]` gains the optional `lossless_transcode` (default `true`). With `false` the setting is only used as a stage and gets no FLAC transcode, so the lossless tables do not change (used by the sample corpus, section 9).
- **Sources** are those of generation.md section 1: trusted lossless tracks that are not `Error` and not silent. Lossy tracks are never sources.

### 4.2 Truth

- One stage: `truth = real`, `class = generated` (an honest encode).
- Two or more stages: `source_bitrate_kbps` = the lowest `bitrate_kbps` of the stages before the last. `truth = fake`, `class = positive` when it is lower than the last stage's `bitrate_kbps`; otherwise `truth = real`, `class = generated` (for example `cbr320_cbr128`).

### 4.3 ffmpeg calls

Stage 1 is the encode call of generation.md section 2 with stage 1's arguments (and the fragment, if any). Stage `k` > 1 encodes the output of stage `k − 1`:

```
<ffmpeg> -nostdin -hide_banner -loglevel error -i <generated>/<id>.s<k-1>.<ext k-1>
         -map 0:a:0 -map_metadata -1 <stage k args> -y <generated>/<id>.s<k>.<ext k>
```

The last stage writes `<generated>/<id>.<ext>`. There is no decode call. The sample rate is whatever ffmpeg delivers, so a cascade after an Opus stage stays at 48 kHz, as a streaming-site Opus → MP3 rip does. A failure at any stage is a generation failure of the entry (generation.md section 2). `encoder_version` is the version string of each stage (generation.md section 2), joined with ` > `.

### 4.4 Files and recipe

- **Recipe** (keys sorted): `kind` = `"lossy"`, `source`, `stages` (a list of `{ "setting", "args" }`), `fragment_start_s`, `fragment_s`. The id is computed as in generation.md section 3; lossless and lossy recipes never collide (different keys).
- Each intermediate `<id>.s<k>.<ext>` is deleted once the next stage has run. The final lossy file is analysed by the app's pipeline (`SymphoniaAnalyzer`, lossy-source.md section 2), then deleted unless `keep_audio`, or `keep_misclassified` and the row is misclassified (`Fake` on a negative, `Real` on a positive), as in generation.md section 3, with the same JSON sidecar.

### 4.5 Undetectable cascades and source-limited honest encodes

- **Undetectable** (positives): the rule of evaluation.md section 2, with `lowpass_hz` = the lowest `lowpass_hz` of the stages before the last, comparing the source with the final lossy file. Such a cascade removed nothing the last stage would have kept.
- **Source-limited** (honest generated negatives, C9), per params file with `[lossy]`: the source's own reported cutoff (lossless rules of the same params file, rules.md section 2, `fakeflac_core::detect::cutoff_estimate`) is below `c_exp − lossy.suspicious_shortfall_hz`, where `c_exp` is the final file's expected cutoff as computed by the core's lossy functions (lossy-source.md section 3, "Where it is computed"). The lossy rule cannot tell such a file from a fake, so it is counted apart and left out of the rates (lossy-source.md section 8).

### 4.6 Cost

`calibration.v2.toml` has 14 lossy entries (8 single-stage, 6 cascades), so each source adds 20 encode calls, against 18 calls (9 encodes, 9 decodes) for its lossless transcodes. With the about 1,450 sources of the last acceptance runs (rules.md section 6 counts about 1,420 detectable transcodes per setting) that is about 20,000 lossy entries and 29,000 ffmpeg calls on the first run, slightly more than the lossless transcodes took, then nothing while the cache holds (C8).

## 5. Cache (extends generation.md section 4)

- Corpus lossy tracks are keyed like corpus tracks; generated lossy entries by recipe.
- A cache entry's `probe` gains `stream`: `{ "codec", "declared_bitrate_kbps", "bitrate_mode", "encoder", "encoder_lowpass_hz" }`, with `null` for absent values.
- **`EXTRACTION_VERSION` stays 2 (C7).** A line without `stream` is read as a FLAC or WAV entry with codec `flac` or `pcm` (from the extension) and no stream values: exactly what the adapter now reports for those files, whose spectra do not change (enabling more symphonia formats does not change FLAC or WAV decoding). Lossy entries are new and always carry `stream`. A future change to the adapter's lossy output bumps the version as usual.

## 6. Evaluation (extends evaluation.md)

### 6.1 Domains

- Every table, figure, list and target of evaluation.md sections 3–9, and the grid search, use **lossless-domain rows only**, as before this spec.
- Lossy rows appear only in the tables below, and only for params files with a `[lossy]` section. For a params file without it, lossy rows are **not evaluated**: the header gives their count per params file; they are never counted as `Error`.
- Opus rows are not evaluated by any params file while the decoder is pending; the header counts them and `report.md` lists them.

### 6.2 Lossy confusion matrix

Per params file with `[lossy]` and split (`all`, `tune`, `holdout`):

| truth \ verdict | Real | Suspicious | Fake | Undetectable | Source-limited | Error | total |
|---|---|---|---|---|---|---|---|
| real | | | | — | | | |
| fake | | | | | — | | |
| fake (weak) | | | | — | — | | |

- **real**: honest generated encodes (one stage and negative cascades), trusted lossy files and `confirmed_real` files.
- **fake**: generated cascades with `truth = fake` and confirmed lossy fakes with strong labels. **fake (weak)**: weak labels only.
- Rates as in evaluation.md section 4, with `Undetectable`, `Source-limited` and `Error` left out of the denominators.

### 6.3 Per group

One row per `[[generate.lossy]]` entry in config order, then: trusted lossy files per class; `confirmed_real`; confirmed lossy fakes per `upstream` (`aac`, `opus`, `mp3`, `unknown`); confirmed lossy fakes per sample rate (44100, 48000, other); weak labels. Columns: `Real`, `Suspicious`, `Fake`, `Undetectable`, `Source-limited`, `Error`, total, `Fake` rate, caught rate (positives) or FP rate (negatives), the median and 5th percentile of `s`, the median estimate (over the rows with an estimate), and, for positive groups only, the median estimate of the `Fake` rows (empty when there is none; the figure B8 gates) and the share of estimates at most the source bitrate (over the rows with an estimate and a `source_bitrate_kbps`; empty when there is none). A group is **positive** when its rows have `truth = fake`: the generated cascades with `truth = fake` (4.2), the confirmed lossy fakes per `upstream` and per sample rate, and weak labels; every other group is negative and leaves those two columns empty (the negative cascade `cbr320_cbr128` has a source bitrate, 320, but no share). These rows serve lossy-source.md section 9 (B2–B8).

### 6.4 `tracks.csv` and margins

`fc`, `c_exp`, `s`, the verdict and the estimate of a lossy row come from the pure public functions of `fakeflac_core::detect` (lossy-source.md section 3, "Where it is computed"), applied to the cached spectra and `stream` information; the tool computes only the margins below from them, as it does for lossless rows (ADR-0036: the tool reimplements no detection step).

Lossy rows fill the measurement columns of evaluation.md section 8 (`H`, `F`, `fe`, `D`, `fs`, `G`, `U`, `B`) and leave its margin columns empty. New columns, empty on lossless rows and for undefined values: `codec`, `declared_bitrate_kbps`, `bitrate_mode`, `encoder_lowpass_hz`, `fc_hz`, `c_exp_hz`, `shortfall_hz`, `m_lossy_suspicious_hz` (`lossy.suspicious_shortfall_hz − s`), `m_lossy_fake_hz` (`lossy.fake_shortfall_hz − s`), `m_lossy_step_db` (`D − lossy.fake_min_step_db`), `estimate_kbps` and `source_limited`. **Near threshold** (`report.md`): a lossy row with `|m_lossy_suspicious_hz|` or `|m_lossy_fake_hz|` at most `report.near_hz`, or `|m_lossy_step_db|` at most `report.near_db`.

### 6.5 Chains (C5)

- The **version** of a chained track is its codec (`flac`, `pcm`, `alac`, `aac`, `mp3`).
- Its **key** (`chain_key`): when the file stem starts with a run of 1 to 3 ASCII digits, that run as an integer (`03 – x` → `3`). Otherwise, that is, when the stem does not start with a digit or its leading run has 4 or more digits (a year or a catalogue number, not a track number: `2024 – x`), the stem lower-cased with only its letters and digits kept, Unicode included (`2024 – x` → `2024x`, `Ópera` → `ópera`).
- Within one chain, tracks of different versions with the same key form a **pair group**. A key that appears twice within one version is ambiguous: its tracks are listed and not paired.
- **`report.md`**, per chain and key: path, version, verdict, reasons, `fc`, estimate, declared bitrate and watch id. **`summary.md`**, per chain id: keys, pair groups, versions present, verdict counts per version, the median `|fc(mp3) − fc(aac)|` over the keys that have both versions, and the watch ids in the chain (the owner's full-chain album holds FP4, watch id W4). Lossless versions are judged with the lossless rules of the same params file.

### 6.6 Privacy

`summary.md` may show chain ids, group ids, upstream values, sample rates and counts, and nothing else new: never a path, a folder or file name, a key that is a name, or a note (calibration.md section 6). `privacy-check` only catches names of 8 or more characters, so ids must be opaque (1.2).

### 6.7 Watch list

A lossy `false-positive` track is listed in every run with, per params file: verdict, reasons, codec, declared bitrate, `fc`, `c_exp`, `s`, `D` and estimate.

## 7. Grid search (C10)

Lossy rows never enter the grid's data, cost or constraints. A grid key in `[lossy]` is an error (exit 2: `lossy.<field>: lossy parameters cannot be searched yet`). A proposal written from a baseline with `[lossy]` copies it unchanged.

## 8. Configuration v2 and defaults

- `specs/calibration/calibration.v2.toml` = `calibration.v1.toml` with `config_version = 2`, the 14 `[[generate.lossy]]` entries and `[labels.upstream_by_rate]`. Every v1 value is unchanged.
- **`--config` defaults to `calibration.v2.toml`** (C8); calibration.md section 2 changes accordingly. v1 stays loadable (no lossy entries, no table).
- The single-stage entries are the honest negatives of B2; the cascades are `cbr128_cbr320`, `aac128_cbr320` (gated, B5), `cbr192_cbr320`, `cbr128_aac256`, `opus128_cbr320` (reported, B6) and the negative `cbr320_cbr128` (B2).

## 9. Sample corpus, lossy variant (CI; extends sample-corpus.md)

The base sample corpus and its expectations (sample-corpus.md sections 1–5) do not change. Scenarios tagged `@RF-15` in `calibration-lossy.feature` use the **lossy variant**:

- **Config:** the base sample config plus three fixture settings with `lossless_transcode = false` and three lossy entries:

| Setting | `args[0]` (lossy fixture written, `lossy-fixtures.md`) | `ext` | `bitrate_kbps` | `lowpass_hz` |
|---|---|---|---|---|
| `fxmp3_128` | `mp3_128_lp17k` | mp3 | 128 | 17000 |
| `fxmp3_320` | `mp3_320_lp20k5` | mp3 | 320 | 20500 |
| `fxmp3_320_src128` | `mp3_320_lp17k` | mp3 | 320 | 20500 |

| Lossy entry | `stages` | Truth |
|---|---|---|
| `l128` | `fxmp3_128` | real |
| `l320` | `fxmp3_320` | real |
| `l128_320` | `fxmp3_128`, `fxmp3_320_src128` | fake (source 128 < 320) |

  The fixture transcoder writes the fixture of the **last** stage as `<id>.mp3`, ignoring the source and earlier stages, with `encoder_version = "fixture"`.

- **Extra corpus files** (lossy fixtures under Unicode names, RNF-09):

| # | Path under `FAKEFLAC_CORPUS` | Fixture | Listed as |
|---|---|---|---|
| 12 | `Pérdidas/Vídeo/07 – Falso 320.mp3` | `mp3_320_lp17k` | `confirmed.txt` line 4 |
| 13 | `Pérdidas/Vídeo/08 – Señal ’AAC’.m4a` | `m4a_aac_128_lp16k` | `confirmed.txt` line 5 |
| 14 | `Pérdidas/Vídeo/09 – Ópera.opus` | (any bytes) | `confirmed.txt` line 6 |
| 15 | `Cadena/MP3 – Ørsted/01 – Tres.mp3` | `mp3_320_lp17k` | `confirmed.txt` line 7 |

- **`trusted.txt`**: line 7 becomes `Revisión/Ørsted/ | revisado chain=C1 | barras normales y separador final`. **`confirmed.txt`** gains:

```
4  Pérdidas\Vídeo\07 – Falso 320.mp3 | fake upstream=aac source_max_kbps=128 | de AAC 128
5  Pérdidas\Vídeo\08 – Señal ’AAC’.m4a | real | copia sin recodificar
6  Pérdidas\Vídeo\09 – Ópera.opus | fake upstream=opus
7  Cadena\MP3 – Ørsted\01 – Tres.mp3 | fake upstream=aac chain=C1
```

**Expected results** with `--params detection-params.v2.toml --params detection-params.v5.toml`:

- **Lossless:** on split `all`, every lossless table equals the base corpus's (sample-corpus.md section 3), for both files.
- **Labelled rows:** 55 (the base 33, 18 generated lossy rows from the 6 sources, and the 4 corpus lossy files). **Decodes on a cold run:** 54 (the Opus file is never decoded).
- **Not evaluated:** v2: 22 lossy rows; v5: 1 (the Opus file).
- **Lossy matrix (v5, split `all`):** real row 11 `Real`, 2 `Source-limited` (`l128` and `l320` from track 5, `low_hf`, whose own cutoff is about 6 kHz), total 13; fake row 7 `Fake`, 1 `Undetectable` (`l128_320` from track 5), total 8; fake (weak) row empty.
- **What the test stage confirms first** (derived, not measured): that the reported cutoff of `low_hf` with v5 is below 16000 Hz (the source-limited threshold of `l128`, `17000 − 1000`; it is expected near 6 kHz, its `fe`); that the other five sources (`full_band`, `hf_bursts`) report a cutoff above 19500 Hz (the threshold of `l320`); and the checks of `lossy-fixtures.md` section 3 for the three fixtures written. If one does not hold, it stops and returns to G1.
- **Per group (v5):** `l128` 5 Real + 1 source-limited; `l320` 5 Real + 1 source-limited; `l128_320` 5 Fake + 1 undetectable, median estimate of the `Fake` rows 128; `confirmed_real` 1 Real; confirmed fakes `aac` 2 Fake; `opus` 0 rows evaluated. Track 12's estimate (128) is at most its bound (128).
- **Chain `C1`:** keys 1, 2 and 3; one pair group (key 1: the WAV track 7, `Real`, and the MP3 track 15, `Fake`, estimate 128); keys 2 and 3 have the WAV version only (both `Error`).
- The Ørsted album and `Cadena/MP3 – Ørsted` form the unit `chain:C1`.

## 10. Labelling the real corpus (owner, local)

How the owner's RF-15 input (docs/agents/progress.md, 2026-10-18; aggregate figures only, ADR-0022) maps onto this grammar:

- The ~170 streaming-site MP3 320 rips: `fake upstream=by-rate` (44.1 kHz → AAC ≤ 128, 48 kHz → Opus). The labels are upper bounds. The bootleg tape compilation: add `weak`.
- The 14 AAC 128 `.m4a` files copied without re-encoding: `real` (C2); the one now labelled `fake` changes.
- The album with the full chain from one master: its trusted FLAC line, its AAC lines (`real`) and its MP3 lines (`fake upstream=aac`) all get the same `chain=<id>`. Its FLAC version holds FP4 (watch id W4), so the chain report shows W4's other versions.
- Albums that share a main artist: one `group=<id>` per artist on their lines.

## 11. Known limitations

- **Split strata go by extension** (section 3). A unit made only of ALAC `.m4a` files is stratified as lossy (`lossy:<class>`), although its tracks are lossless-domain rows after extraction (1.1) and are evaluated with the lossless tables (6.1). It does not move any other album; it only means such a unit is split among the lossy units.
- **Chain keys of stems without a track number** (section 6.5). A stem whose leading digit run has 4 or more digits gets the letters-and-digits key, so its versions pair only when their whole stems match after that reduction.
- **Stems with a disc number** (section 6.5, owner decision at G1). A stem such as `1-03 – x` gets the key `1` (its leading digit run), so every track of disc 1 of that version shares one key, is ambiguous and is never paired. No `<disc>-<track>` rule in this version.

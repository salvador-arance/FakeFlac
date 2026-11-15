---
id: SPEC-CENSUS
requirements: [RP-12, RNF-04, RNF-05, RNF-09]
status: approved
---

# Library census: the detector's verdicts on a whole folder, in aggregates (RP-12)

> `just census FOLDER` analyses a music folder as the app does and reports what the detector says about it, in aggregates that can be pasted into a PR. It replaces the throwaway probe and scripts of the library diagnosis of 2026-11-14 (`docs/agents/progress.md`). It has no ground truth and no pass or fail: it describes verdicts, it does not judge them. Truth-based acceptance stays in [calibration.md](calibration.md) (`just accept`, RP-11).
>
> This spec defines the command (section 2), the run (section 3), the report (section 4), the per-track file (section 5), privacy (section 6), determinism (section 7), the sample folder (section 8), the layers (section 9) and the acceptance criteria (section 10). It is not a backend ↔ frontend contract: no command, event or type of `specs/contracts/commands.md` changes, no detection rule or parameter changes, and there is no UI change. Scenarios: `specs/features/census.feature` (tag `@RP-12`). Related: `specs/detection/fake-evidence.md` (RF-17: the evidence items and margins that the census counts), `specs/detection/album-summary.md` and `specs/contracts/commands.md` version 5 (the album of a track), ADR-0019, ADR-0022, ADR-0034, ADR-0036.

## 0. Open questions for G1

Decided at G1 (#148) with the defaults below; the rest of this spec is written with them. The table stays as the record of the questions; changing a default later changes only the sections named in its row.

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| CQ1 | **Where it lives.** A new subcommand `census` of the dev-only tool `tools/fakeflac-calibrate` (ADR-0036), which already depends on `fakeflac-core`, `fakeflac-audio` and `fakeflac-fs` and already has the privacy rules of ADR-0022. Alternatives: (a) a script over the app's CSV export, which cannot work because the CSV does not carry the evidence (fake-evidence.md Q8); (b) a new crate, which adds a package for one command | Subcommand of `fakeflac-calibrate` | 2, 9 |
| CQ2 | **The pipeline.** The census calls the app's own use case, `fakeflac_core::analyze_folder`, with the adapters the app uses (`WalkDirScanner`, `SymphoniaAnalyzer`), so its tracks, verdicts, evidence and albums are the app's by construction. It does not use the extraction cache and label lists of `accept` | The app's use case | 3 |
| CQ3 | **Other params.** `--params FILE` analyses with one other `DetectionParams` file (loaded and validated as `accept` loads it), so a proposal can be censused before it ships. Without it, the census uses `DetectionParams::default()` | Yes, one file | 2, 3 |
| CQ4 | **No result cache.** Every census decodes every file, like the app's first run on a folder. The owner's library (11 396 tracks, 150.7 GB) is the reference size; the run time on it is recorded in the feature PR (MC1). Alternative: a cache under `target/`, left for a later change if the time is a problem | No cache | 3, 10 |
| CQ5 | **Where the output goes.** A folder `target/census/<run>/` in the repository (`target/` is ignored by git), with no option to write elsewhere: agents never write outside the repository, and a census never writes into the music folder. `summary.md` is also printed to stdout | `target/census/<run>/` only | 2, 5, 6 |
| CQ6 | **Album strata.** An album with at least one `Fake` is `whole` when at least 75 % of its analysed tracks are `Fake`, `isolated` when fewer than 25 % are, and `mixed` otherwise (the limits of the 2026-11-14 diagnosis). Error tracks are left out of the share, as in the album note of RF-17 | 75 % and 25 % | 4.4 |
| CQ7 | **Margin bands.** The margin of a lossless `Fake` is the largest margin of its `gapStep` and `cutoffStep` items, in dB. Bands: below 1, 1 to 3, 3 to 6, 6 to 12, 12 or more. Lossy fakes (which need a shortfall in Hz and a step in dB together) are counted but not banded; their margins are in `tracks.csv` | Five bands, lossless only | 4.5 |
| CQ8 | **Comparing two censuses.** Out of the tool. `summary.md` and `tracks.csv` are deterministic (section 7), so two runs are compared with any diff tool. A `--compare` option is left for a later change | Not in the tool | 7 |
| CQ9 | **Rows by codec.** Tables group tracks by `TrackInfo.codec` (`flac`, `pcm`, `alac`, `mp3`, `aac`, `opus`), not by container, because an M4A file is lossless (`alac`) or lossy (`aac`) and the rules differ. A track without a codec (an M4A that could not be probed) is `unknown` | By codec | 4 |

**Decided defaults (not questions).**
- **D1. Read-only (RNF-05).** The census opens audio files for reading only. It creates, changes and deletes nothing in `FOLDER`. A `FOLDER` that contains the report folder is refused (section 2), so the report is never written inside `FOLDER`.
- **D2. No environment.** The census does not need `FAKEFLAC_CORPUS` or `FAKEFLAC_CALIBRATION` and does not read `trusted.txt` or `confirmed.txt`.
- **D3. Opt-in.** The recipe is not part of `check`, `check-fast`, `test`, `check-app` or `ci`, and no workflow calls it. Its tests run in `just test` on synthetic fixtures.
- **D4. No detection change.** `DetectionParams`, `algorithm_version`, `EXTRACTION_VERSION`, `commands.md` and the app do not change.

## 1. Scope

- **In:** the subcommand and its recipe; the run; `summary.md`; `tracks.csv`; the sample folder and the tests.
- **Out:** ground truth and acceptance targets (RP-11); a result cache (CQ4); comparing runs inside the tool (CQ8); spectrograms; any figure per album or per track in `summary.md`; the interface.

## 2. The command

| Recipe | Binary call |
|---|---|
| `just census FOLDER [--params FILE]` | `fakeflac-calibrate census FOLDER [--params FILE]` |

The recipe passes its arguments unchanged (positional arguments through Node, like `accept`), so a folder with spaces, `&`, `'` or any Unicode character is safe (RNF-09). The binary runs from the repository root.

- `FOLDER` is required and is used as given. `--params FILE` may appear once, before or after `FOLDER`.
- **Output folder.** `<repository root>/target/census/<run>/`, where `<run>` is the UTC start time `YYYYMMDDTHHMMSSZ`, with `-2`, `-3`… appended if that folder already exists ([inputs.md](inputs.md) section 1 uses the same rule). The repository root is found as the tool already finds it (calibration.md section 3). Outside a repository the census exits 2.
- **The folder must not contain the report folder (D1, CQ5).** The run is refused when the report folder `<repository root>/target/census/` would lie inside `FOLDER` or be `FOLDER` itself: `FOLDER` is the repository root, an ancestor of it, `<repository root>/target` or `<repository root>/target/census` (for example `just census .`, since the binary runs from the repository root). A path lies inside another when the other's components are its first components, so `D:\Música 2` is not inside `D:\Música`. Both paths are compared in their canonical forms: symbolic links and junctions resolved, and the letter case and the `\\?\` prefix of Windows treated the same way on both sides, so that two spellings of one folder compare equal. The canonical form of the report folder is that of its deepest part that already exists (at least the repository root) followed by the parts that do not exist yet, so the check holds before the first census has created `target/census/`. When `<repository root>/target` is a symbolic link or junction that points elsewhere, the report folder is the canonical `<redirect target>/census`: the repository root is then refused only if that redirected report folder lies inside it, while the redirect target itself, its `census` folder and any folder that contains the redirect target are refused. This is the canonical-forms rule applied, not an exception. A `FOLDER` inside the repository that does not contain `target/census/` (for example a fixtures folder) is allowed. When `FOLDER` or the repository root cannot be put in canonical form, this check does not refuse the run; the other checks still apply (a folder that does not exist is refused as such).
- **Order.** The output folder is created only after the run has finished, so a refused run writes nothing. The report-folder check comes after the repository is found and before the run starts: nothing is created before it (no `target/census/<run>/`, and no `target/census/` either), and `FOLDER` is not scanned.

### Exit codes and messages

| Code | When | stdout | stderr (exact) |
|---|---|---|---|
| 0 | The run finished, with or without error tracks | `summary.md`, then the line `elapsed: <seconds, one decimal> s` | – |
| 2 | No `FOLDER`, more than one `FOLDER`, `--params` twice or without a value, or an unknown option | – | `usage: fakeflac-calibrate census FOLDER [--params FILE]` |
| 2 | `FOLDER` does not exist or is not a folder | – | `error: census: the folder does not exist or is not a folder` |
| 2 | The scan finds no supported audio file | – | `error: census: no supported audio files in the folder` |
| 2 | The params file cannot be read or is invalid | – | the message `accept` gives for the same file |
| 2 | The tool is not run inside a repository | – | `error: census: run it from the repository (the report goes under target/)` |
| 2 | `FOLDER` contains the report folder or is it: the repository root, an ancestor of it, `target/` or `target/census/` of the repository (canonical forms, above; with a redirected `target`, as the check says) | – | `error: census: the folder contains the report folder (target/census/ of the repository)` |
| 1 | Internal error | – | `error: census: internal error` and the detail |

No census message of its own contains `FOLDER` or any path under it; the report-folder message names no path at all (`target/census/` is fixed text). A decoding error on a track never stops the run (RNF-04): the track is counted as an error.

## 3. The run

- **Use case.** One call of `fakeflac_core::analyze_folder` with `WalkDirScanner`, `SymphoniaAnalyzer`, the params of CQ3, `FOLDER` as root, and the tool's worker rule (`worker_threads()`: logical CPUs minus 1, at least 1, the app's rule of ADR-0034). No cancellation.
- **Tracks.** Every `TrackDone` of the run is one track, error tracks included. The scan decides which files are tracks, as in the app (other files are ignored).
- **Albums.** A track's album is the `album.index` of its `TrackDone` (the direct parent folder of the track, `commands.md` version 5). The counts of an album are the `summary` of the last `TrackDone` of that index. The census never recomputes a verdict or an album.
- **Groups.** A track is **lossless** when its codec is `pcm`, `flac` or `alac`, and **lossy** when it is `mp3`, `aac` or `opus`. A track without a codec is in neither group.

## 4. `summary.md`

Plain text and Markdown tables, UTF-8, LF line ends, in this order. Counts are plain integers. Every table has all the rows listed for it, with 0 where nothing is counted, unless its section says otherwise.

**Blocks and blank lines.** The report is a sequence of blocks: each of the four header lines of 4.1, each table (its header line, its separator line and its rows), the line `albums with exactly one Fake among 2 or more analysed tracks: <count>` of 4.4, and the line `errors: none` or the error table of 4.7. One blank line follows every block, the last one included: there is one blank line between two blocks, and the file ends with the last line of its last block followed by one blank line. So when stdout is pasted as Markdown, the `elapsed:` line that follows `summary.md` there (section 2) is not read as a row of the last table. There is no blank line inside a block. The code blocks of 4.1 to 4.7 show the lines of the blocks without these blank lines. Blank lines are layout: the tests, and the comparison of two runs of section 7, compare the non-blank lines, and a table line cell by cell with its cells trimmed. No heading and no other text is added before, between or after the blocks.

### 4.1 Header

```
FakeFlac library census (specs/calibration/census.md)
run: <run>
detection: algorithm_version <n>, params <default | custom <id>>
tracks <total> | albums <albums> | <total size in MB, one decimal> MB
```

- `<n>` is the `algorithm_version` of the params used. `<id>` is the tool's 16-character content hash (`id16`) of the params file's bytes; the file's name and path are not printed.
- `<albums>` is the number of distinct album indexes. The size is the sum of `info.size_bytes`, in MB of 1 000 000 bytes.
- The worker count and the elapsed time are not in `summary.md`, so that two runs differ only in the `run:` line (section 7).

### 4.2 Verdicts by codec

```
| codec | tracks | real | suspicious | fake | error |
|---|---|---|---|---|---|
```

One row for each of `flac`, `pcm`, `alac`, `mp3`, `aac`, `opus` and `unknown` that has at least one track, in that order, then a row `total`. An error track is counted under the codec of its `TrackInfo` (set from the file's extension when the file could not be probed).

### 4.3 Evidence of the `Fake` verdicts

```
| group | evidence | tracks |
|---|---|---|
| lossless | gapStep | … |
| lossless | cutoffStep | … |
| lossless | gapStep + cutoffStep | … |
| lossless | other | … |
| lossy | shortfall + cutoffStep | … |
| lossy | other | … |
```

A `Fake` track is counted in the row of the set of kinds in its `evidence` (fake-evidence.md section 2), whatever their order. `other` counts every other set, the empty one included; fake-evidence.md allows no such set, so a number other than 0 there shows a defect.

### 4.4 Albums with a `Fake`

For an album, `f` is its `fake` count and `n` its `real + suspicious + fake` (errors left out). Only albums with `f ≥ 1` are counted. Comparisons are exact, in integers:

| Stratum | Condition |
|---|---|
| `whole` | `4·f ≥ 3·n` |
| `isolated` | `4·f < n` |
| `mixed` | neither |

```
| stratum | albums | fake tracks | lossless | lossy |
|---|---|---|---|---|
| whole | … | … | … | … |
| mixed | … | … | … | … |
| isolated | … | … | … | … |
| total | … | … | … | … |
albums with exactly one Fake among 2 or more analysed tracks: <count>
```

`fake tracks` is the sum of `f` over the stratum's albums; `lossless` and `lossy` split it by the track's group. The last line counts the albums with `f = 1` and `n ≥ 2`, the case of the interface's note "The only Fake of the … analysed tracks in this album" (`specs/ui/fake-evidence.md` section 3), in any stratum.

### 4.5 Margin of the lossless `Fake` verdicts

The margin of a lossless `Fake` track is the largest `margin` among its `gapStep` and `cutoffStep` items: how far its strongest evidence is above its threshold, in dB. A track with a small margin is a `Fake` by a little.

```
| margin dB | whole | mixed | isolated | total |
|---|---|---|---|---|
| < 1 | … | … | … | … |
| 1 – 3 | … | … | … | … |
| 3 – 6 | … | … | … | … |
| 6 – 12 | … | … | … | … |
| ≥ 12 | … | … | … | … |
| none | … | … | … | … |
```

- A band includes its lower limit and excludes its upper one (a margin of exactly 3 is in `3 – 6`). The margin is compared as the `f32` value of the evidence item, widened to `f64`.
- The columns are the stratum of the track's album (4.4).
- `none` counts lossless fakes without a `gapStep` or `cutoffStep` item (0 expected, as `other` in 4.3).
- Lossy fakes are not in this table (CQ7).

### 4.6 Reasons of the `Suspicious` verdicts

```
| reason | lossless | lossy |
|---|---|---|
```

One row for each `SuspicionReason` in the order of the contract (`lowHighFrequencyContent`, `gradualRolloff`, `ambiguousCutoff`, `highBandGaps`, `cutoffBelowDeclared`, `streamingSampleRate`), then a row `none` for `Suspicious` tracks without a reason. A track with two reasons is counted in both rows.

### 4.7 Errors

When no track has an error, the single line `errors: none`. Otherwise:

```
| codec | unreadable | corruptStream | unsupportedFormat | tooShort |
|---|---|---|---|---|
```

One row for each codec of 4.2 that has at least one error track, in the order of 4.2.

## 5. `tracks.csv` (private)

One row per track, sorted by the bytes of its `path` column. UTF-8 without a byte-order mark, LF line ends, fields quoted as RFC 4180 requires (a field with a comma, a quote or a line break is quoted, and a quote is doubled). Header and columns:

```
path,album_index,album_stratum,codec,sample_rate,bit_depth,channels,declared_bitrate_kbps,duration_ms,size_bytes,verdict,reasons,evidence,gap_step_margin_db,cutoff_step_margin_db,shortfall_margin_hz,cutoff_hz,error
```

- `path`: the track's path relative to `FOLDER`, with `/` between components. A name that is not valid Unicode is written with the replacement character; the file is for reading, not for opening files from it.
- `album_stratum`: `whole`, `mixed` or `isolated` for a track of an album with a `Fake`, otherwise empty.
- `codec`, `verdict`, `reasons`, `evidence` and `error` use the camelCase names of the contract. `reasons` and `evidence` (the kinds) are joined with `|` in the order of the analysis. `error` is the error's kind only (no detail text).
- The three margin columns hold the `margin` of the item of that kind, and `cutoff_hz` the cutoff's `frequency_hz`, written with Rust's shortest `f32` form that reads back to the same value. A missing value is an empty field.
- **Empty means not measured.** An empty field means that the value was not measured; a placeholder 0 is never written for a value that was not measured. For an error track whose file could not be probed (the analyzer returned an error, so no value of the file reached the analysis, as for the `corrupt` fixture of section 8), the fields `sample_rate`, `bit_depth`, `channels`, `declared_bitrate_kbps` and `duration_ms` are empty: the analysis holds only the unprobed placeholders there (0 for `sample_rate`, `channels` and `duration_ms`, no value for `bit_depth` and `declared_bitrate_kbps`). A track that was probed keeps its real values in these fields even when it fails afterwards (for example `tooShort`, as the `short` fixture of section 8, with its real sample rate, channels and duration). An error track whose probe reported 0 for one of these fields has that field empty too, since 0 is not a valid measurement of these fields. The rule covers error tracks only: a `0` on a track that is not an error is written as `0`.
- No absolute path appears in the file.

## 6. Privacy (ADR-0022)

- `summary.md` and stdout hold counts and the fixed words of section 4 only: no path, no folder name, no file name, not `FOLDER` itself. They can be pasted into a PR.
- `tracks.csv` holds relative paths. It stays under `target/` (ignored by git) and is never pasted or committed.
- The message of the report-folder check (section 2) names no path: not `FOLDER`, not the repository root, only the fixed text `target/census/`.
- `just privacy-check` does not change. It still guards the branch diff when the labelled corpus is configured.

## 7. Determinism

With the same build, the same params and an unchanged `FOLDER`, two runs give a byte-identical `tracks.csv`, and their `summary.md` files differ only in the `run:` line. The analysis runs in parallel, so the tool sorts its tracks (section 5) and builds every table from counts; nothing depends on the order of the events.

## 8. The census sample folder

Generated inside the tests, in a temporary folder outside the repository, from the fixtures of `specs/detection/fixtures.md` (written by the tool's test support, as [sample-corpus.md](sample-corpus.md) does). Names use accents, `Ñ`, typographic quotes, an en dash, an ampersand and a comma (RNF-09; the comma tests the CSV quoting). The root is `Biblioteca ″Ñandú″ & co`.

| # | Path under the root | Fixture | Verdict with the defaults |
|---|---|---|---|
| 1 | `Álbum A – todo falso/01 – Uno.flac` | `shelf19k5` | Fake, `cutoffStep` |
| 2 | `Álbum A – todo falso/02 – Dos.flac` | `sfb21_gaps` | Fake, `gapStep` |
| 3 | `Álbum A – todo falso/03 – Tres.flac` | `shelf19k5` | Fake, `cutoffStep` |
| 4 | `Álbum A – todo falso/04 – Cuatro.flac` | `sfb21_gaps` | Fake, `gapStep` |
| 5–8 | `Álbum B, aislado/01 – Canción ‘x’.flac` to `04 – Canción ‘x’.flac` | `full_band` | Real |
| 9 | `Álbum B, aislado/05 – Falsa.flac` | `shelf19k5` | Fake, `cutoffStep` |
| 10 | `Álbum C/CD1/01 – Uno.flac` | `full_band` | Real |
| 11 | `Álbum C/CD1/02 – Dos.flac` | `sfb21_gaps` | Fake, `gapStep` |
| 12 | `Álbum C/CD2/01 – Suave.flac` | `low_hf` | Suspicious, `lowHighFrequencyContent` |
| 13 | `Álbum C/CD2/02 – Roto.flac` | `corrupt` | Error, `corruptStream` |
| 14 | `Álbum C/CD2/03 – Corto.flac` | `short` | Error, `tooShort` |
| 15 | `Sueltas/01 – Onda.wav` | `full_band` (WAV) | Real |
| – | `Sueltas/notas.txt` | any small file | ignored: not audio |

**Expected `summary.md`** (every line but `run:`; the size is the sum of the 15 files' sizes):

- Header: `detection: algorithm_version 8, params default`; `tracks 15 | albums 5 | <size> MB`.
- 4.2: `flac` 14 tracks: 5 real, 1 suspicious, 6 fake, 2 error. `pcm` 1 track: 1 real. `total` 15: 6, 1, 6, 2.
- 4.3: lossless `gapStep` 3, `cutoffStep` 3, `gapStep + cutoffStep` 0, `other` 0; lossy 0 and 0.
- 4.4: `whole` 1 album, 4 fake tracks (album A: `f = 4`, `n = 4`); `mixed` 1 album, 1 fake track (CD1: `f = 1`, `n = 2`); `isolated` 1 album, 1 fake track (album B: `f = 1`, `n = 5`, and `4 < 5`); `total` 3 albums, 6 fake tracks; every fake is lossless. Albums with exactly one Fake among 2 or more analysed tracks: 2.
- 4.5: the column totals are `whole` 4, `mixed` 1, `isolated` 1, `total` 6, and `none` is 0 in every column. The band of each track depends on the fixture's measured margin, so the sample checks only these sums; the bands are checked with given analyses (C5).
- 4.6: `lowHighFrequencyContent` lossless 1; every other cell 0.
- 4.7: one row `flac`: `corruptStream` 1, `tooShort` 1, the others 0.

If the test stage finds that a fixture's verdict or evidence with the defaults differs from this table, it stops and reports it; the expected figures are not adjusted (they follow from `fixtures.md` and fake-evidence.md section 10).

## 9. Layers

- **Test stage:** `tools/fakeflac-calibrate/tests/census.rs` (the sample folder, the report from given analyses, the argument and error cases, the wiring check of the recipe). It may extend the tool's test support with the fixtures of section 8.
- **Implement stage:** a new module of `tools/fakeflac-calibrate` (the pure report and CSV builders, which take the tracks and albums of a run and return text, and the run itself), the `census` command in the tool's argument parser, and the `census` recipe in `justfile`:

  ```
  # Library census (RP-12, specs/calibration/census.md): the detector's verdicts on a whole folder, in
  # aggregates. Opt-in: not part of check, ci or any workflow. census FOLDER [--params FILE]
  census *args   (positional arguments, a Node script like `accept`)
  ```

- **Reds at the `test(RP-12)` commit:** the compile (the census entry points do not exist) and the wiring check (the recipe is missing). Nothing else may be red. At the later `test(RP-12)` commit of the report-folder check (C11), only its four refused cases may be red; its two allowed cases pass on the code without the check. The C11 cases added after that commit (a `..` segment, a different letter case, a repository root that does not exist, a redirected `target`) pin the check that already exists: at their `test(RP-12)` commit nothing may be red.
- **Unchanged:** `fakeflac-core`, `fakeflac-audio`, `fakeflac-fs`, `fakeflac-store`, `src-tauri`, the frontend, the bindings, every existing command of the tool and its outputs. No new dependency.
- **Outside `specs/`, for the human** (agent configuration, then `just agents-sync`, ADR-0048): the `census` recipe in the command list of `CLAUDE.md`.

## 10. Acceptance (measurable)

**Automated in `just test` and CI** (`census.rs`, synthetic fixtures):
- **C1 (sample).** A census of the sample folder exits 0, and its `summary.md` equals the expected one of section 8 in every line but `run:`; stdout is that `summary.md` followed by one `elapsed:` line.
- **C2 (per-track file).** `tracks.csv` has the header of section 5 and 15 rows sorted by `path`; row 9 (`Álbum B, aislado/05 – Falsa.flac`) has its `path` quoted, verdict `fake`, evidence `cutoffStep`, stratum `isolated`, a `cutoff_step_margin_db` that is not empty and empty `gap_step_margin_db` and `shortfall_margin_hz`; the row of `02 – Roto.flac` has verdict `error`, error `corruptStream`, empty `sample_rate`, `bit_depth`, `channels`, `declared_bitrate_kbps` and `duration_ms` (not measured, section 5) and empty `reasons`, `evidence`, `gap_step_margin_db`, `cutoff_step_margin_db`, `shortfall_margin_hz` and `cutoff_hz`; the row of `03 – Corto.flac` has verdict `error`, error `tooShort` and a `sample_rate`, `channels` and `duration_ms` that are neither empty nor `0`; no field holds the root's absolute path.
- **C3 (privacy and Unicode, RNF-09).** None of these appears as a substring of `summary.md` or of stdout: the root's name, any folder or file name of the sample, and every component of 4 or more characters of the root's absolute path.
- **C4 (read-only, RNF-05).** After the run, every file of the sample folder has the same size, modification time and content hash, and the folder holds no new file. The only folder created is `target/census/<run>/`, with exactly `summary.md` and `tracks.csv`.
- **C5 (the tables from given analyses).** The pure report builder, given analyses and albums built in the test, prints:
  - lossless margins of 0.0, 0.99, 1.0, 2.99, 3.0, 5.99, 6.0, 11.99, 12.0 and 40.0 dB in the bands `< 1` (2), `1 – 3` (2), `3 – 6` (2), `6 – 12` (2) and `≥ 12` (2);
  - a track with `gapStep` 2.0 dB and `cutoffStep` 7.5 dB in `6 – 12` and in the row `gapStep + cutoffStep`;
  - albums with (`f`, `n`) of (3, 4), (1, 1), (2, 3), (1, 4), (1, 5) and (2, 9) as `whole`, `whole`, `mixed`, `mixed`, `isolated` and `isolated`, and "exactly one Fake among 2 or more" counting 2;
  - an album with 1 fake and 3 error tracks as `whole` (`n = 1`);
  - lossy fakes (`mp3`, `aac`, `opus` with `shortfall` and `cutoffStep`) in the `lossy` row of 4.3 and the `lossy` column of 4.4, and in no cell of 4.5;
  - a `Fake` with empty evidence in `other` (4.3) and `none` (4.5);
  - a suspicious track with two reasons in both rows of 4.6, and one without reasons in `none`;
  - an M4A error track without a codec in the `unknown` rows of 4.2 and 4.7;
  - with no error track, the line `errors: none`.
- **C6 (determinism).** Two censuses of the sample folder give byte-identical `tracks.csv` files and `summary.md` files equal in every line but `run:`. Two runs started in the same second get the folders `<run>` and `<run>-2`.
- **C7 (other params).** With `--params` set to a copy of the shipped params file, `summary.md` says `params custom <id>` with the 16-character hash of that file and has the same tables as C1. With a params file whose `gap_fake_db` is 100, tracks 2, 4 and 11 are not `Fake` and the `gapStep` row of 4.3 is 0.
- **C8 (refused runs).** Each case of the exit-code table of section 2 with code 2 gives its exact stderr line, an empty stdout and no new folder under `target/census/`: no argument, two folders, `--params` without a value, `--params` twice, an unknown option, a folder that does not exist, a file given as the folder, and a folder holding only `notas.txt`. The case of a folder that contains the report folder is C11.
- **C9 (a corrupt track never stops the run, RNF-04).** Covered by C1: tracks 13 and 14 are counted as errors and the run exits 0.
- **C10 (wiring).** The test reads `justfile` as text and checks that a `census` recipe exists, that its body holds `fakeflac-calibrate` and `census`, that none of the recipes `ci`, `check`, `check-fast`, `test` and `check-app` names `census` as a whole word, and that neither `.github/workflows/ci.yml` nor `release.yml` contains `just census`.
- **C11 (the folder must not contain the report folder, RNF-05).** The test makes a stand-in repository root (`Env.repo_root`, as the other tests do) inside the temporary folder `<base>` that also holds the sample folder `<base>/Biblioteca ″Ñandú″ & co`. Each of these `FOLDER`s exits 2 with exactly the stderr line `error: census: the folder contains the report folder (target/census/ of the repository)`, an empty stdout, and the same files and folders under `<base>` before and after the call (no `target/census/<run>/`, and no `target/census/` where there was none): `<base>` (an ancestor of the stand-in root, which also holds the sample's audio), the stand-in root itself, its `target/` (created empty by the test before the call), and its `target/census/` (created empty by the test before the call). Two `FOLDER`s are not refused by this check, and neither stderr holds that line: the sample folder itself (outside the stand-in root) exits 0 as in C1, and a folder `fixtures` inside the stand-in root (beside `target/`, so it does not contain `target/census/`) that holds a copy of `Sueltas/01 – Onda.wav` exits 0. These cases pin the canonical comparison and the fail-open rule of section 2:
  - **(a) A `..` segment.** `FOLDER` spelled with a `..` segment that resolves to the stand-in root (`<stand-in>/../<stand-in name>`) is refused with the same line, exit 2, an empty stdout and the same files and folders under `<base>`.
  - **(b) Letter case (Windows only, RNF-07).** `FOLDER` the stand-in root spelled in a different letter case (for example its last component upper-cased) is refused the same way. On other platforms this case is not run.
  - **(c) Fail-open.** With a repository root that does not exist (`Env.repo_root` set to a folder under `<base>` that the test has not created, so it cannot be put in canonical form) and `FOLDER` the sample folder, the guard does not refuse: stderr does not hold its line, the census exits 0 with the `summary.md` of C1 (every line but `run:`), and, as the tool creates the output folder with its missing parents, exactly one run folder `<that root>/target/census/<run>/` is created, with `summary.md` and `tracks.csv` (the missing root, `target/` and `census/` are created with it). The tool finds the repository root on disk (section 2, "Output folder"), so this case is reachable only through `Env.repo_root`.
  - **(d) A redirected `target` (Windows only, RNF-07).** The test makes `<stand-in>/target` a junction (`mklink /J`) to an empty folder `<base>/redirected`, outside the stand-in root; when the junction cannot be created, the test skips this case and prints the reason. `FOLDER` = `<base>/redirected` is refused with the same line, exit 2, an empty stdout and the same files and folders under `<base>`. `FOLDER` = the stand-in root is not refused by this check (the redirected report folder `<base>/redirected/census` is not inside it): stderr does not hold the guard line; the scan does not follow the junction and the stand-in root holds no audio, so the census exits 2 with exactly the stderr line `error: census: no supported audio files in the folder`, an empty stdout, and the same files and folders under `<base>` (no `<base>/redirected/census/`).

**Manual** (owner, release build, aggregates only in the PR):
- **MC1 (the owner's library).** `just census` on the library of the 2026-11-14 diagnosis. Record `summary.md` and the elapsed time in the feature PR. With detection v8 and an unchanged library, the totals equal the app's run of that day (11 396 tracks: 9 003 real, 1 173 suspicious, 1 156 fake, 64 errors) and the lossless rows of 4.3 equal the diagnosis (95 `gapStep`, 188 `cutoffStep`, 218 both). The strata of 4.4 are not compared with the diagnosis, which counted lossless tracks only.
- **MC2 (read-only on real files).** The file count, sizes and modification times of the library are equal before and after MC1.

## 11. Edits to approved specs

- `specs/traceability.md`: the RP-12 row.
- `specs/calibration/calibration.md` section 2: one line after the command table that points to this spec for `just census` (a marked addition; no existing command changes).

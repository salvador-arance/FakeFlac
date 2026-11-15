---
id: SPEC-CALIBRATION-INPUTS
requirements: [RP-11, RNF-09]
status: approved
---

# Calibration inputs: corpus layout, `trusted.txt`, `confirmed.txt`, `labels.csv`, split

> The user writes two text files (`trusted.txt`, `confirmed.txt`). The tool derives `labels.csv` from them and from the transcodes it generates. `labels.csv` is never edited by hand. Main spec: [calibration.md](calibration.md).

## 0. Layout under `FAKEFLAC_CALIBRATION`

```
FAKEFLAC_CALIBRATION/
  trusted.txt        user: trusted lossless tracks (closed format, section 1)
  confirmed.txt      user, optional: confirmed fakes and confirmed false positives (section 2, OQ-5)
  labels.csv         tool: one row per labelled track (section 3), rewritten by every run
  generated/         tool: transcoded fragments, deletable at any time (generation.md)
  features/          tool: spectra cache (generation.md section 4)
  reports/<run>/     tool: one folder per run (evaluation.md section 1)
```

`<run>` is the UTC start time `YYYYMMDDTHHMMSSZ`, with `-2`, `-3`… appended if that folder already exists.

## 1. `trusted.txt` (closed format; this section only specifies its parser)

### 1.1 Lines

1. The file is read as UTF-8. A UTF-8 BOM at the start is skipped. Invalid UTF-8 is an error on the line that holds the first invalid byte.
2. Lines end with `\n`; a trailing `\r` is removed. Line numbers start at 1.
3. Each line is trimmed (leading and trailing whitespace) and then classified:

| Trimmed line | Kind |
|---|---|
| empty | ignored |
| starts with `#` | comment, ignored |
| `library_root`, optional spaces, `=`, value | root line (1.2) |
| starts with `!` | exclusion (1.4) |
| anything else | entry (1.3) |

A relative path whose first folder name starts with `#` or `!` is written with a `.\` prefix (`.\#1 Hits | cd-log`).

### 1.2 `library_root`

- Exactly one root line, before the first entry or exclusion. Errors: missing (`library_root is missing`, no line number), repeated (`library_root is already set at line N`; the value of a repeated line is not checked), an entry or exclusion before it (`entry before library_root`).
- The value is trimmed and must be an absolute path of an existing folder (`library_root is not an absolute path`, `library_root does not exist`).
- **It must equal `FAKEFLAC_CORPUS`**, compared as canonical paths (so a trailing separator or a difference in letter case on Windows does not matter). Otherwise: `library_root does not match FAKEFLAC_CORPUS`.

### 1.3 Entries: `path | origin | optional note`

- The line is split at every `|` (the character cannot appear in Windows file names). Field 1 is the path, field 2 the origin, and fields 3 onwards, joined again with `|`, are the note. Each field is trimmed. Spaces around `|` are optional.
- **Path.** `\` and `/` are both separators. A trailing separator and a leading `.\` or `./` are ignored. A `..` component is an error (`'..' is not allowed`), checked before the path is resolved. A relative path is joined to `library_root`. An absolute path must lie inside `library_root` (`path is outside library_root`). The path must exist (`path does not exist: <path as written>`). On one line, only the first of these path errors is reported. Names are used exactly as written, with no Unicode normalisation (RNF-09).
- **Folder:** every FLAC and WAV file below it, recursively (hidden entries skipped, links not followed). A folder with no FLAC or WAV file is an error (`no FLAC or WAV files under this folder`).
- **File:** that track only. The extension must be `.flac`, `.wav`, `.mp3`, `.m4a` or `.opus`, ignoring case (`not a supported audio file (FLAC, WAV, MP3, M4A or Opus)`). Lossy files, the `formats=` attribute of folder entries and the other attributes of field 2: `lossy.md` section 1 (RF-15).
- **Origin:** exactly one of the values in 1.5, in lower case (`unknown origin 'x' (expected one of: cd-log, accuraterip, …)`). Missing or empty: `missing origin`. Missing path: `missing path`.
- The same path listed twice (compared as canonical paths) is an error on the later line (`already listed at line N`).

### 1.4 Exclusions: `!path | optional reason`

- The path follows the rules of 1.3 and may be a folder or a file.
- It must lie strictly inside a folder listed by an entry (`exclusion is not inside any listed folder`) and cover at least one FLAC or WAV file (`exclusion matches no FLAC or WAV file`).
- **Most specific line wins.** For each track, the entries and exclusions whose path equals or contains it are compared, and the deepest one decides: an exclusion removes the track, and an entry includes it with its origin. So a folder `analisis-limpio` can hold one file entry `revisado`, and an exclusion inside a folder removes that part. An entry and an exclusion with the same path is an error on the later line (`contradicts line N`).
- Exclusions apply wherever they appear in the file.

### 1.5 Origins and how each is used

| Origin | Class | Negative for tuning | Negative for evaluation | Source for transcodes |
|---|---|---|---|---|
| `cd-log`, `accuraterip`, `bandcamp`, `qobuz`, `tienda` | `primary` | yes | yes: **primary figure** | yes |
| `revisado`, `usuario` | `reviewed` | yes | yes, each origin in its own row | yes |
| `analisis-limpio` | `clean` | yes | yes, its own row; albums that turn `Fake` are listed (evaluation.md section 5) | yes |
| `analisis` | `reference` | no | reference only: its own row, outside the global matrix | yes |

All metrics are broken down by origin. Classes group the rows and decide tuning and the primary figure.

### 1.6 Errors

- The parser reports **every** error in the file, not only the first, as `trusted.txt:<line>: <message>` (or `trusted.txt: <message>` without a line number), sorted by line. Then the run stops with exit code 2 and writes nothing.
- Paths in messages are printed as written in the file, never expanded to absolute paths.
- A file with no included track is an error (`trusted.txt: no tracks listed`). A missing file: `trusted.txt: not found in FAKEFLAC_CALIBRATION`.

## 2. `confirmed.txt` (optional, OQ-5)

Same grammar as `trusted.txt` (1.1 to 1.4 and 1.6, with `confirmed.txt` in the messages, including the `library_root` rule). The second field is a **kind** instead of an origin:

| Kind | Path | Meaning |
|---|---|---|
| `fake` | folder (recursive, exclusions allowed) or file | Confirmed fake, checked by hand on a spectrogram. Label `truth = fake`, `origin = confirmed_fake`. A track that is also included by `trusted.txt` is an error (`also trusted (trusted.txt line N)`). |
| `false-positive` | file only (`must be a file`) | A trusted track that an earlier detector flagged and that the user confirmed lossless. It must be included by `trusted.txt` (`not a trusted track`). It keeps its trusted label and gets a watch id `W1`, `W2`… in line order. Watch tracks are listed one by one in every run (evaluation.md section 5). |
| `real` (RF-15) | lossy folder or file | An honest lossy file: its content matches its declared bitrate. `lossy.md` section 1.3 |
| `lossy-master` (calibration.md OQ-32) | lossless folder (recursive, exclusions allowed) or file | A verified, untouched copy of a published release whose own master carries lossy traces. Label `truth = lossy_master`, `origin = confirmed_lossy_master`, `class = lossy_master`. Section 2.1 |

`fake` lines may also cover lossy files, with the attributes of `lossy.md` section 1.2. Unknown kind: `unknown kind 'x' (expected fake, real, false-positive or lossy-master)` (calibration.md OQ-32; before it: `(expected fake, real or false-positive)`). If the file is missing, there are no confirmed fakes and no watch list.

Known truth for `confirmed.txt` (to be populated by the user once this spec is approved):
- `fake`: tracks 03, 04, 11, 13, 15 and 16 of album F1 and the 9 tracks of album F2 (track 01 excluded because it was never certain).
- `false-positive`: the 4 tracks of purchased discs A–D (watch list `W1`–`W4`).
- In `trusted.txt`, origin `revisado`: the 9 confirmed lossless tracks of album F1.

### 2.1 Kind `lossy-master` (calibration.md OQ-32 to OQ-37)

**Meaning.** The track is a verified, untouched copy of a published release, but its audio carries lossy-compression traces because the release's own master was made from lossy material. For such a track a `Fake` verdict is the detector reporting what is in the audio (rules.md section 6). It is not a trusted negative, not a positive and not a source, so it is listed here and never in `trusted.txt`.

**Proof the owner holds** (the tool cannot check it; it may be recorded in the note, which never reaches `summary.md`, calibration.md section 6):
- a rip verified by a database: AccurateRip or CTDB confirms the listed tracks; or
- a store purchase of the release.

A rip log alone is **not** enough. The log's copy CRC only proves that the rip equals the disc that was in the drive, and a CD-R burnt from MP3 files gives a log whose CRC matches just as well. A Fake verdict on such a disc is exactly what this kind would explain away, so the label needs proof that the disc is the published one.

**What to list.** The whole release, with a folder line (one per disc folder, or the album folder). Never list only the tracks a detector flagged: the report measures which share of an authentic release is accused (evaluation.md section 10), and selecting tracks by their verdict would make it 100 % by construction. A file line is for a release where only some tracks are verified by the database.

**Grammar.** Sections 1.1 to 1.4 and 1.6, with these rules:
- **Lossless only (OQ-36).** A folder line includes the FLAC and WAV files below it (`formats=lossless`, the default). `formats=lossy` or `formats=all`, or a file line naming an `.mp3`, `.m4a` or `.opus` file, is an error: `lossy-master applies to lossless files only`. Like the check of `real` lines (`lossy.md` section 1.2), it goes by extension, so an ALAC `.m4a` cannot be listed. A folder with no FLAC or WAV file: `no FLAC or WAV files under this folder` (unchanged).
- **Attributes.** Those of a `real` line (`lossy.md` section 1.2): `formats=` (only `lossless`), `group=` and `chain=`. `upstream=`, `source_max_kbps=` and `weak`: `upstream=, source_max_kbps= and weak apply to fake lines only` (unchanged message).
- **Exclusions** (`!path`) inside a `lossy-master` folder work as in section 1.4.

**Interactions** (each error is on the `lossy-master` line unless said otherwise, and is reported with the other errors of section 1.6):
- **Also trusted.** A track included by a `lossy-master` line and by `trusted.txt`: `also trusted (trusted.txt line N)`, where N is the `trusted.txt` line that includes the track. One error per `lossy-master` line, for its first such track in path order (the rule of `fake` lines). To list the release here, the owner removes it from `trusted.txt` or excludes it there with `!`.
- **Also fake.** A track included by a `lossy-master` line and a `fake` line: `already listed at line N` on the later of the two lines (as for `real` and `fake`, `lossy.md` section 1.3).
- **Watch list.** A `lossy-master` track is not trusted, so a `false-positive` line naming it gives `not a trusted track` (unchanged).
- **Albums (OQ-33).** An album that holds a `lossy-master` track holds no other labelled track of `trusted.txt` or `confirmed.txt`. Otherwise: `album has lossy-master and other labelled tracks (<ref>, <ref>)`, one error per album, printed without a file prefix. The two refs are `<file>:<line>`: the first line that labels a `lossy-master` track of the album, and the first line that labels any other track of it, where "first" and the order of the two refs are those of the split-unit error of `lossy.md` section 3 (`trusted.txt` before `confirmed.txt`, then by line number). Example: `album has lossy-master and other labelled tracks (trusted.txt:9, confirmed.txt:4)`. This check runs only when neither file has an error of its own, so it never repeats an `also trusted` or `already listed` error. Several such errors, and the split-unit errors of `lossy.md` section 3, are ordered by their first ref. A release shares one master, so its tracks cannot be both negatives (or transcode sources) and lossy-mastered.

**Never** (OQ-34): a source for transcodes or lossy cascades (generation.md section 1; `lossy.md` section 4.1), a tuning negative or positive, part of the grid search cost, ranking or constraints (evaluation.md section 7), a watch track, or selectable by a target (evaluation.md section 6).

## 3. `labels.csv`

Written by every run, after extraction, from `trusted.txt`, `confirmed.txt` and the generated entries. CSV per ADR-0012: RFC 4180 (CRLF line ends, fields quoted when they contain `,`, `"` or a line break), UTF-8 without BOM, comma separator, one header row. Rows are sorted by `album_id`, then `root`, then `path`, then `setting`.

| Column | Values |
|---|---|
| `path` | Path relative to `root`, `/` separators, names exactly as on disk |
| `root` | `corpus` (relative to `FAKEFLAC_CORPUS`) or `generated` (relative to `FAKEFLAC_CALIBRATION/generated`) |
| `truth` | `real` or `fake`; `lossy_master` for `lossy-master` rows (below) |
| `origin` | `trusted`, `generated` or `confirmed_fake`; `confirmed_lossy_master` (below) |
| `trust_origin` | `trusted`: its origin (1.5). `generated`: the origin of its source. `confirmed_fake` and `confirmed_lossy_master`: empty |
| `class` | `primary`, `reviewed`, `clean`, `reference` (negatives); `positive` (generated and confirmed fakes); `lossy_master` (neither, below) |
| `encoder` | `none` (corpus tracks), `lame`, `aac`, `opus`, or `fixture` (CI only, sample-corpus.md) |
| `setting` | `none` or the setting id of the config (`cbr128`, `v0`, `aac256`, `opus128`…) |
| `bitrate_kbps` | Nominal bitrate of the setting; empty for VBR quality settings and for `none` |
| `encoder_version` | Exact version string (generation.md section 2); empty for `none` |
| `sample_rate_hz` | Sample rate of the labelled file as decoded; empty if it could not be decoded |
| `fragment_start_s`, `fragment_s` | Generated only: fragment start and length in seconds (3 decimals). When default full track is used, `fragment_start_s` is `0.000` and `fragment_s` equals the track duration |
| `source_path` | Generated only: `path` of the source track (root `corpus`) |
| `album_id` | Section 4 |
| `split` | `tune` or `holdout` (section 4) |
| `watch_id` | `W1`… for `false-positive` tracks, otherwise empty |

**`lossy-master` rows (section 2.1, calibration.md OQ-34).** No new column. New values in existing columns: `truth` `lossy_master`, `origin` `confirmed_lossy_master` and `class` `lossy_master`. Their `trust_origin` and `watch_id` are empty, `encoder` and `setting` are `none`, and the generated-only columns are empty, as for a confirmed fake. A run without `lossy-master` lines writes none of these values.

RF-15 appends the columns of `lossy.md` section 2 (`domain`, `codec`, …) after `watch_id`. The minimum columns asked for by the issue (`path, truth, origin, encoder, bitrate_kbps, album_id, split`) are all present. A generated entry whose audio was deleted keeps its row (its spectra are cached by recipe).

## 4. Albums and the `tune`/`holdout` split

**Album id.** The path, relative to `FAKEFLAC_CORPUS` with `/` separators, of the folder that holds the track. If that folder's name is a **disc folder**, the album is its parent folder (one level only; OQ-6). A disc folder name matches, ignoring case, one of the `split.disc_folder_prefixes` of the config, then an optional ` `, `-`, `_` or `.`, then 1 to 3 digits, and nothing else (`CD1`, `cd 2`, `Disc-3`, `Disco 1`; not `CD1 Bonus`). A track directly in `FAKEFLAC_CORPUS` has album id `.`. A generated track has the album id of its source.

**Split (OQ-7).** Stratified by class. Albums are grouped by their class (`primary`, `reviewed`, `clean`, `reference`, and `positive` for albums containing only confirmed fakes). An album whose trusted tracks have several classes (for example `CD1` from an `analisis-limpio` folder and a `revisado` file entry in `CD2`) takes the strongest of them, in the order `primary` > `reviewed` > `clean` > `reference`. Confirmed fakes in such an album do not change its class: the album is `positive` only when all its labelled tracks are confirmed fakes. Every row keeps its own class for the metrics; the album class is used only for the split and for the per-class album counts and holdout warnings of the report header.

**Addition (calibration.md OQ-34): class `lossy_master`.** Its albums form a stratum of their own, split by the rules below like any other class (the guarantee on small classes included), so adding one never moves an album of another class. An album with a `lossy-master` track holds no other labelled track (section 2.1), so its class is `lossy_master` exactly when it holds one. In a split unit that joins several albums (`group=`, `lossy.md` section 3), `lossy_master` ranks below `reference` and above `positive`. Whatever its split, a `lossy_master` row is never used for tuning; its split only labels the rows of evaluation.md section 10.

For each album id (with RF-15, for each **split unit**: an album id unless a `group=` or `chain=` attribute joins several albums, `lossy.md` section 3):

```
h = fnv1a64( utf8(split.seed) ++ [0x0A] ++ utf8(album_id) )
u = fmix64(h)
```

- `fnv1a64`: offset basis `0xcbf29ce484222325`, prime `0x100000001b3`; for each byte, `h ^= byte; h *= prime` (wrapping).
- `fmix64` (MurmurHash3 finaliser): `k ^= k >> 33; k *= 0xff51afd7ed558ccd; k ^= k >> 33; k *= 0xc4ceb9fe1a85ec53; k ^= k >> 33` (wrapping).
- `seed`, `holdout_fraction` (0.30) and `min_holdout_albums` (2) are data in `calibration.v1.toml`.

Within each class:
1. Distinct albums in that class are sorted ascending by the 64-bit unsigned hash `u` (breaking ties by `album_id` lexicographically).
2. Let $N$ be the number of distinct albums in the class.
3. The number of albums assigned to `holdout` is (`round` rounds half away from zero):
   - If $\text{holdout\_fraction} = 0$ or $N < 2$: $k = \mathrm{round}(N \cdot \text{holdout\_fraction})$. So `0.0` puts every album in `tune`, and a class with one album goes to `holdout` only when `holdout_fraction` $\ge 0.5$ (with the default 0.30 it stays in `tune`). The report warns of fewer than 2 holdout albums.
   - Otherwise: $k = \min(N, \max(\text{min\_holdout\_albums}, \mathrm{round}(N \cdot \text{holdout\_fraction})))$. So `1.0` puts every album in `holdout`.
4. The first $k$ albums in the sorted order are assigned to `holdout`; the remaining $N - k$ albums are assigned to `tune`.
5. Every track of an album, generated tracks included, has the same split as its album.

Properties:
- **Stability:** Ordering by hash within each class means adding a new album to a class only places that album into the sorted sequence; it does not change the hash of any existing album. Since $k$ grows by at most 1 per added album, existing albums change split only at the holdout boundary: **at most one existing album moves** per added album (from `holdout` to `tune` when the new album lands in `holdout` and $k$ does not grow; from `tune` to `holdout` when it lands in `tune` and $k$ grows). Albums of other classes never move.
- **Guarantee on small classes:** A pure independent hash threshold could leave the 5 purchased discs of class `primary` with 0 or 1 holdout album. The stratified split guarantees at least 2 holdout albums whenever a class has $\ge 2$ albums, ensuring the primary holdout metric has adequate data.
- **`holdout` rows are never used for tuning** (evaluation.md section 7): they never enter the cost or the ranking. The one exception is the hard constraint on confirmed fakes, which checks every confirmed fake, `holdout` included.

---
id: SPEC-CALIBRATION-INPUTS
requirements: [RP-11, RNF-09]
status: proposed
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
- **File:** that track only. The extension must be `.flac` or `.wav`, ignoring case (`not a FLAC or WAV file`).
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

Unknown kind: `unknown kind 'x' (expected fake or false-positive)`. If the file is missing, there are no confirmed fakes and no watch list.

## 3. `labels.csv`

Written by every run, after extraction, from `trusted.txt`, `confirmed.txt` and the generated entries. CSV per ADR-0012: RFC 4180 (CRLF line ends, fields quoted when they contain `,`, `"` or a line break), UTF-8 without BOM, comma separator, one header row. Rows are sorted by `album_id`, then `root`, then `path`, then `setting`.

| Column | Values |
|---|---|
| `path` | Path relative to `root`, `/` separators, names exactly as on disk |
| `root` | `corpus` (relative to `FAKEFLAC_CORPUS`) or `generated` (relative to `FAKEFLAC_CALIBRATION/generated`) |
| `truth` | `real` or `fake` |
| `origin` | `trusted`, `generated` or `confirmed_fake` |
| `trust_origin` | `trusted`: its origin (1.5). `generated`: the origin of its source. `confirmed_fake`: empty |
| `class` | `primary`, `reviewed`, `clean`, `reference` (negatives); `positive` (generated and confirmed fakes) |
| `encoder` | `none` (corpus tracks), `lame`, `aac`, `opus`, or `fixture` (CI only, sample-corpus.md) |
| `setting` | `none` or the setting id of the config (`cbr128`, `v0`, `aac256`, `opus128`…) |
| `bitrate_kbps` | Nominal bitrate of the setting; empty for VBR quality settings and for `none` |
| `encoder_version` | Exact version string (generation.md section 2); empty for `none` |
| `sample_rate_hz` | Sample rate of the labelled file as decoded; empty if it could not be decoded |
| `fragment_start_s`, `fragment_s` | Generated only: fragment start and length in seconds (3 decimals) |
| `source_path` | Generated only: `path` of the source track (root `corpus`) |
| `album_id` | Section 4 |
| `split` | `tune` or `holdout` (section 4) |
| `watch_id` | `W1`… for `false-positive` tracks, otherwise empty |

The minimum columns asked for by the issue (`path, truth, origin, encoder, bitrate_kbps, album_id, split`) are all present. A generated entry whose audio was deleted keeps its row (its spectra are cached by recipe).

## 4. Albums and the `tune`/`holdout` split

**Album id.** The path, relative to `FAKEFLAC_CORPUS` with `/` separators, of the folder that holds the track. If that folder's name is a **disc folder**, the album is its parent folder (one level only; OQ-6). A disc folder name matches, ignoring case, one of the `split.disc_folder_prefixes` of the config, then an optional ` `, `-`, `_` or `.`, then 1 to 3 digits, and nothing else (`CD1`, `cd 2`, `Disc-3`, `Disco 1`; not `CD1 Bonus`). A track directly in `FAKEFLAC_CORPUS` has album id `.`. A generated track has the album id of its source.

**Split (OQ-7).** For each album id:

```
h = fnv1a64( utf8(split.seed) ++ [0x0A] ++ utf8(album_id) )
u = fmix64(h)
split = holdout  if (u >> 11) / 2^53 < split.holdout_fraction,  otherwise tune
```

- `fnv1a64`: offset basis `0xcbf29ce484222325`, prime `0x100000001b3`; for each byte, `h ^= byte; h *= prime` (wrapping).
- `fmix64` (MurmurHash3 finaliser): `k ^= k >> 33; k *= 0xff51afd7ed558ccd; k ^= k >> 33; k *= 0xc4ceb9fe1a85ec53; k ^= k >> 33` (wrapping).
- `seed` and `holdout_fraction` (0.30) are data in `calibration.v1.toml`.

Properties: the split of an album depends only on the seed and its own id, so adding or removing albums never moves another album. Over 10,000 distinct ids the holdout share is between 28 % and 32 %. Every track of an album, generated tracks included, has the same split. **`holdout` rows are never used for tuning** (evaluation.md section 7).

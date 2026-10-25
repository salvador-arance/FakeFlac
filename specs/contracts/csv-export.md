---
id: SPEC-CSV-EXPORT
requirements: [RF-09, RF-16, RNF-04, RNF-05, RNF-09]
status: proposed
csv_format_version: 1
---

# CSV export: file format and layers (RF-09, H2)

> What the file written by "Export CSV" contains, byte for byte, and which layer does what. The command that writes it is `export_csv` (`specs/contracts/commands.md`, version 4); the button and texts are in `specs/ui/export-csv.md`; scenarios in `specs/features/export-csv.feature` (tag `@RF-09`). Format decision: ADR-0012 (RFC 4180, UTF-8 without BOM, comma separator; not tuned for Excel). Acceptance from the planning document (section 2.7): "the CSV export is valid (RFC 4180, UTF-8) and keeps the Unicode characters of the file names when read back".

## Open questions for G1

Each question has a default; the spec below is written with the defaults. The owner decides at G1.

| # | Question | Status / default |
|---|---|---|
| Q1 | **Columns and order** (section 3): 21 columns; the first ten follow the table's column order (State · File · Folder · Format · Samplerate · Bits · Duration · Cutoff · Bitrate · Real bitrate), then the details the table does not show (reasons, error, codec, channels, size, steepness, noise floor, algorithm version, full path). `modifiedUnix` is not exported. Take this set, or a shorter one? | Open. Default: as written |
| Q2 | **Header and values are fixed English identifiers** (`verdict`, `real`, `cutoffBelowDeclared`, `corruptStream`…), the contract's values, never the UI texts ("Genuine", "Corrupt stream"). The CSV is the same whatever the interface language (RF-13) and can be read by scripts. Localised headers instead? | Open. Default: fixed identifiers |
| Q3 | **Line endings: CRLF** after every record, the last one included (RFC 4180 section 2, rules 1 and 2). LF only instead? | Open. Default: CRLF, last record terminated |
| Q4 | **Unknown numbers are empty cells.** `sampleRate`, `channels` and `durationMs` are `0` in a `TrackInfo` whose file could not be probed (an error track); the CSV writes an empty cell instead of `0` for these three fields. Write `0` as stored instead? | Open. Default: empty |
| Q5 | **Formula-looking values are written verbatim.** A file name such as `=1+1.flac` or `@intro.mp3` is not prefixed with `'` or otherwise altered (spreadsheet "CSV injection" mitigation). Altering names would break the round-trip of RNF-09, and ADR-0012 does not tune the file for Excel | Open. Default: verbatim |
| Q6 | **Scope: every analysed track of the latest run, in scan order** (`index`), ignoring the table's sort and, once RF-08 exists, its filters. "Export only the visible rows" could be added with RF-08 (a contract change) | Open. Default: all rows, scan order |
| Q7 | **The destination never comes from the frontend.** `export_csv` takes no path: the backend opens the native save dialog and writes only where the user chose, like `pick_folder` (RF-01). This differs from the planning document's `export_csv(dest: PathBuf)` (section 3.5.1): a page cannot make the app write a file at an arbitrary path (minimal capabilities, RNF-03; read-only audio, RNF-05) | Open. Default: no path input |
| Q8 | **Default file name** `FakeFlac - <analysed folder name>.csv`, computed by the pure function `default_file_name` (section 5.1); `FakeFlac.csv` when the analysed folder has no name of its own (a drive root such as `D:\`, or a UNC share root such as `\\nas\music`: the share name is not used). The app does **not** set the dialog's initial folder: Windows opens the folder last used with a save dialog, else its default (Documents), and never proposes the music folder by itself | Open. Default: as written (UNC share root → `FakeFlac.csv`) |
| Q9 | **A destination whose extension is not `.csv`** (case-insensitive) is refused with `EXPORT_FAILED` and nothing is written. This guarantees the export can never replace an audio file or any other kind of file (RNF-05). The backend never appends `.csv` itself: appending would write a path the dialog did not confirm for overwrite. **A name typed without an extension** (`results`) is completed by the Windows dialog itself, before it returns and before its overwrite prompt: the locked `rfd` 0.16.0 (through `tauri-plugin-dialog` 2.8.3, `Cargo.lock`) calls `IFileDialog::SetDefaultExtension` with the first extension of the first filter, here `csv` (`rfd` `src/backend/win_cid/file_dialog/dialog_ffi.rs`, `add_filters`), so the backend receives `results.csv`. Whether Windows also appends `.csv` to a name typed with another extension (`results.txt`, `results.v2`) is shell behaviour not visible in that source; such a path, if returned, is refused. Both are checked by hand (`ui/export-csv.md`, M6) | Open. Default: refuse; rely on the dialog's default extension for a bare name |
| Q10 | **"No results to export" reuses the existing code `NOT_ANALYZED`** (already in `AppError`, unused so far) instead of a new code. No change to the `AppErrorCode` union | Open. Default: reuse |
| Q11 | **CSV serialisation lives in `fakeflac-export`** (as `docs/architecture/overview.md` and `backend.md` already say: "CSV serialisation", port `CsvExporter`), as a pure function plus the file writer. `fakeflac-core` keeps the port and the `ExportCsv` use case (section 5). Alternative: the pure formatter in `fakeflac-core` and only the write in the adapter | Open. Default: formatter in `fakeflac-export` |
| Q12 | **No new dependency.** The RFC 4180 writer is a few lines (quote a field when it holds `,`, `"`, CR or LF; double the quotes), so the `csv` crate is not added; the save dialog comes from `tauri-plugin-dialog`, already a dependency of `src-tauri` (`blocking_save_file`). Tests read the file back with a small RFC 4180 reader in test support, not a `csv` dev-dependency. If the owner prefers the `csv` crate (runtime or dev), it needs the owner's consent and an ADR before the feature PR | Open. Default: no new dependency, no ADR |
| Q13 | **Safe replacement.** The file is written to a uniquely named temporary file in the destination folder, created so that it never overwrites an existing file, then moved over the destination, so a failed export never leaves a partial or truncated CSV, and an existing file is either fully replaced or left unchanged (section 6). The replaced file's attributes and permissions are not carried over. Write the destination directly instead (simpler, but a failure mid-write leaves a truncated file)? | Open. Default: temporary file + replace |
| Q14 | **Number precision**: cutoff in whole Hz, steepness and noise floor with one decimal, duration in whole ms (section 4) | Open. Default: as written |
| Q15 | **ADR or not.** This spec departs from the planning document in three architectural points: `export_csv` takes no destination (Q7; planning section 3.5.1 has `export_csv(dest: PathBuf)`); the results of the latest run are kept in memory in `src-tauri` (`commands.md`, "Results kept for export"), not in `fakeflac-core` or the store; and the temporary-file-then-replace write policy (Q13, section 6). None adds a dependency or a crate, and ADR-0012 already fixes the format. Record them in an ADR? | Resolved. Owner, 2026-10-25: no ADR; the reasons stay in csv-export.md Q7/Q13 (no consequence line in ADR-0012 either) |

## 1. Encoding (ADR-0012)

- **UTF-8 without BOM.** The file starts with the bytes of the header's first name (`verdict`: `76 65 72 …`), never with `EF BB BF`. Every byte sequence in the file is valid UTF-8.
- **Separator** `,` (U+002C). **Quote** `"` (U+0022).
- **Records end with CRLF** (`0D 0A`), the last record included. There is no empty line after the last record and no LF without a CR outside a quoted field.
- **Header row** first: the column names of section 3, in that order, unquoted. The file has exactly one record per exported track after it, so a file always has at least two records (empty exports are refused, section 5).
- **Every record has 21 fields**, the header included.
- **Text is written as it is.** No Unicode normalisation (NFC stays NFC, NFD stays NFD), no trimming, no case change. Names that are not valid Unicode (rare unpaired surrogates on Windows) are written with U+FFFD replacement characters, as the contract sends them (`commands.md`, "Events").
- **No locale.** Neither the interface language nor the Windows regional settings (decimal comma, digit grouping, date format) change a single byte: the serialiser takes no locale input, numbers follow section 4 and enum values are the contract identifiers (Q2).

## 2. Quoting (RFC 4180 section 2, rules 5–7)

A field is enclosed in double quotes **if and only if** it contains a `,`, a `"`, a CR (U+000D) or an LF (U+000A). Inside a quoted field every `"` is written as `""`; CR and LF are written as they are (a line break inside a quoted field is not turned into CRLF). Any other field is written unquoted, including fields with leading or trailing spaces, `;`, `'`, tabs or non-ASCII characters. An empty value is an empty, unquoted field.

Windows forbids `"`, CR and LF in file names, but not `,`; error details (`TrackError.detail`) can hold any character. The serialiser applies the same rule to every field.

## 3. Columns (`csv_format_version` 1)

One record per `TrackAnalysis`. "Empty" means an empty field.

| # | Header | Value | Example |
|---|---|---|---|
| 1 | `verdict` | `real`, `suspicious`, `fake` or `error` | `fake` |
| 2 | `file` | The last component of `info.path` (file name with extension) | `lp16k.flac` |
| 3 | `folder` | `info.folder` (absolute parent folder) | `C:\Music\Album A` |
| 4 | `format` | `wav`, `flac`, `mp3`, `m4a` or `opus` (container, lower case) | `flac` |
| 5 | `sample_rate_hz` | `info.sampleRate`; empty when `0` (Q4) | `44100` |
| 6 | `bit_depth` | `info.bitDepth`; empty when `null` | `16` |
| 7 | `duration_ms` | `info.durationMs`; empty when `0` (Q4) | `30000` |
| 8 | `cutoff_hz` | `cutoff.frequencyHz` in whole Hz (section 4); empty when there is no cutoff | `16000` |
| 9 | `declared_bitrate_kbps` | `info.declaredBitrateKbps`; empty when `null` | `320` |
| 10 | `bitrate_mode` | `cbr`, `vbr` or `abr`; empty when `null` | `vbr` |
| 11 | `estimated_source_bitrate_kbps` | `estimatedSourceBitrateKbps`; empty when `null` | `128` |
| 12 | `reasons` | The `reasons` identifiers in their order, joined with `;` and no spaces; empty when there are none | `highBandGaps;ambiguousCutoff` |
| 13 | `error` | `unreadable`, `corruptStream`, `unsupportedFormat` or `tooShort`; empty when `error` is `null` | `corruptStream` |
| 14 | `error_detail` | `error.detail`; empty for `tooShort` and when there is no error | `bad frame header` |
| 15 | `codec` | `pcm`, `flac`, `alac`, `mp3`, `aac` or `opus`; empty when `null` | `alac` |
| 16 | `channels` | `info.channels`; empty when `0` (Q4) | `2` |
| 17 | `size_bytes` | `info.sizeBytes` | `5000000` |
| 18 | `cutoff_steepness_db_per_khz` | `cutoff.steepnessDbPerKhz`, one decimal (section 4); empty when there is no cutoff | `42.1` |
| 19 | `noise_floor_db` | `cutoff.noiseFloorDb`, one decimal; empty when there is no cutoff | `-101.9` |
| 20 | `algorithm_version` | `algorithmVersion` | `6` |
| 21 | `path` | `info.path` (absolute) | `C:\Music\Album A\lp16k.flac` |

Header row, exactly:

```
verdict,file,folder,format,sample_rate_hz,bit_depth,duration_ms,cutoff_hz,declared_bitrate_kbps,bitrate_mode,estimated_source_bitrate_kbps,reasons,error,error_detail,codec,channels,size_bytes,cutoff_steepness_db_per_khz,noise_floor_db,algorithm_version,path
```

- **Error rows** (`verdict` `error`) are exported like any other: they carry what was learnt before the error (`commands.md`, "Lossy formats (version 3)"). `codec`, `declared_bitrate_kbps` and `bitrate_mode` are written whenever the stored `TrackInfo` holds them (for example an MP3 that is `tooShort` keeps `mp3`, `320` and `cbr`) and are empty when they are `null` (an `.m4a` that could not be probed). The cutoff cells (8, 18, 19), `estimated_source_bitrate_kbps` (always `null` for an error track) and `reasons` are empty (RNF-04). The serialiser writes what the `TrackAnalysis` holds and never clears a field because of the verdict.
- **Lossy rows** fill columns 9 and 10, and column 11 when there is an estimate; lossless rows leave columns 9–11 empty (`ui/lossy-formats.md`).
- **A non-finite cutoff** (any of the three `CutoffEstimate` values is NaN or infinite) is written as no cutoff: columns 8, 18 and 19 empty, as the contract sends such a track without a cutoff (`src-tauri` DTO conversion).
- Changing the columns, their order or a value encoding bumps `csv_format_version` (a spec change at G1). The version is not written into the file (RFC 4180 has no comment lines).
- **A new contract identifier is not a new encoding.** When the contract adds a value to an enum written here (for example the reason `streamingSampleRate`, contract version 6, RF-16, `commands.md` "Streaming sample rate (version 6)"), the column writes it like the others and `csv_format_version` stays 1. A flagged track's record has `suspicious` in column 1, `streamingSampleRate` in column 12 and an empty column 11 (`specs/features/detection-streaming-source.feature`, "The CSV record of a flagged track").

## 4. Numbers

- **Rounding input.** The three `CutoffEstimate` values are stored as `f32` (`fakeflac-core` `domain.rs`). Every rounding below applies to the stored `f32` value widened exactly to `f64` (`f64::from`), never to a decimal string of it. The examples are stated that way: `16000.4` is the `f32` nearest to 16000.4 (16000.400390625), `16000.5` is exact in `f32`, `42.06` widens to 42.060001373…, `-101.94` to −101.940002441….
- **Integers** (columns 5–7, 9, 11, 16, 17, 20): base-10 ASCII digits, no sign, no leading zeros, no digit grouping, no unit.
- **`cutoff_hz`**: `frequencyHz` rounded to the nearest whole number, halves away from zero (`16000.4` → `16000`, `16000.5` → `16001`).
- **One-decimal values** (columns 18 and 19): the stored value rounded to one decimal place, written with `.` as the decimal point, at least one digit before it, exactly one after it, `-` for negative values and no exponent (`42.06` → `42.1`, `-101.94` → `-101.9`, `-120` → `-120.0`). A value that rounds to zero is written `0.0`, never `-0.0`. Values that are an exact half of the last place are not specified; tests avoid them.

## 5. Layers (hexagonal, `docs/architecture/dependencies.md`)

| Layer | Responsibility | No I/O? |
|---|---|---|
| `fakeflac-core` | Port `CsvExporter` (`export(&self, tracks: &[TrackAnalysis], dest: &Path) -> Result<(), AppError>`, already listed in `docs/architecture/backend.md`, so no ADR). Use case `ExportCsv`: `export_csv(exporter, tracks, dest) -> Result<u32, AppError>` returns the number of rows written. It fails with `NOT_ANALYZED` when `tracks` is empty and with `EXPORT_FAILED` when `dest` is not absolute or its extension is not `csv` (case-insensitive, Q9); in both cases the exporter is not called. Otherwise it calls the exporter once with `tracks` in the order given | Yes (pure; tested with a fake exporter) |
| `fakeflac-export` | A pure function that builds the whole document (sections 1–4) from `&[TrackAnalysis]`, in the order given; the pure function `default_file_name(root: &Path) -> String` (section 5.1); and `CsvFileExporter`, which implements `CsvExporter` by writing exactly those bytes to `dest` (section 6). It opens no file other than the destination and its temporary file: it never reads or opens an audio file (RNF-05) | Formatter and file name yes; the writer is the adapter |
| `src-tauri` | Keeps the results of the latest run (`commands.md`, "CSV export"), takes the snapshot and the export guard before the dialog (`run::begin_export`, below), runs the save dialog (`tauri-plugin-dialog`, already a dependency) with `default_file_name(root)` (Q8) and the filter "CSV (*.csv)", and wires the use case to `CsvFileExporter`. The steps before and after the dialog are plain functions (`run::begin_export` on the kept results; `run::export_csv`, snapshot and destination in) so they are tested without a Tauri runtime; only the dialog call between them is checked by hand | Adapter / composition root |
| Frontend | `exportCsv()` in `src/lib/api/`; the Toolbar button and the status bar notice (`specs/ui/export-csv.md`). It never sees file contents | — |

No capability change: the dialog and the write happen in Rust (`src-tauri/capabilities/default.json` stays as it is; no `fs` or `dialog` permission for the page).

The function names `run::begin_export` and `run::export_csv` are indicative (the test stage may rename them); what is fixed is that both steps run without a Tauri runtime and that the dialog is the only untested step between them.

### 5.1 Default file name

`fakeflac_export::default_file_name(root: &Path) -> String` is pure: it reads no file system and does not resolve, canonicalise or check `root`. `root` is the one the run used (`start_analysis`'s `root`, kept with the results).

- **Name.** The result is `FakeFlac - <name>.csv`, where `<name>` is the last normal component of `root`, as `std::path::Path::file_name` gives it: **trailing separators** (`\` or `/`, one or more) and a trailing `.` component are ignored. The name is taken code point for code point (no trimming, no normalisation, no case change); a name that is not valid Unicode is converted with U+FFFD replacement characters (`to_string_lossy`). Every character allowed in a Windows folder name is allowed in a file name, so nothing is replaced or removed.
- **No name of its own.** When `root` has no such component, the result is `FakeFlac.csv`: a **drive root** (`D:\`, `D:/`, `D:`), a **UNC share root** (`\\nas\music`, `\\nas\music\`; the share name is not a folder name, Q8), their verbatim forms (`\\?\D:\`, `\\?\UNC\nas\music`), and a `root` that ends in `..`.
- **Verbatim and UNC prefixes** do not change the name otherwise: `\\?\C:\Music\Album A` and `\\nas\music\Album A` give `FakeFlac - Album A.csv`.

| `root` | Result |
|---|---|
| `C:\Music\Album A` | `FakeFlac - Album A.csv` |
| `C:\Music\Album A\` | `FakeFlac - Album A.csv` |
| `C:\Music\Album A\\` | `FakeFlac - Album A.csv` |
| `C:/Music/Album A/` | `FakeFlac - Album A.csv` |
| `C:\Music\Álbum ″Ñandú″` | `FakeFlac - Álbum ″Ñandú″.csv` |
| `C:\Music\02 «Ça va» 🎵` | `FakeFlac - 02 «Ça va» 🎵.csv` |
| `C:\Music\Rock, Pop` | `FakeFlac - Rock, Pop.csv` |
| `D:\` | `FakeFlac.csv` |
| `D:` | `FakeFlac.csv` |
| `\\nas\music` | `FakeFlac.csv` |
| `\\nas\music\` | `FakeFlac.csv` |
| `\\nas\music\Album A` | `FakeFlac - Album A.csv` |
| `\\?\C:\Music\Album A` | `FakeFlac - Album A.csv` |
| `\\?\UNC\nas\music` | `FakeFlac.csv` |

The dialog receives this string as its proposed file name; the user can change it (Q9).

### 5.2 Concurrent calls

At most one export is in progress at a time. `run::begin_export` takes an **export guard** together with the snapshot; the guard lives until `export_csv` resolves or rejects (dialog cancelled, file written or failure). While a guard lives, another `export_csv` call fails at once with `INTERNAL` (message `an export is already in progress`): no second dialog opens, nothing is written, and the first export is not affected. The UI never makes such a call (the button is disabled while exporting, `ui/export-csv.md`); the rule covers a stale or duplicated call. `run::begin_export` checks, in this order: an export in progress (`INTERNAL`), a run holding the analysis (`ANALYSIS_RUNNING`), no results (`NOT_ANALYZED`).

## 6. Writing the file

- The file at `dest` ends up holding exactly the document of sections 1–4, nothing before or after it.
- **Overwrite.** The native save dialog asks the user to confirm replacing an existing file (Windows behaviour); once confirmed, the existing file is **replaced as a whole** (never appended to, never left with old bytes at its end).
- **Safe replacement (Q13).** The document is written to a temporary file in the destination folder, then moved over `dest`. When the export fails, a file that already existed at `dest` keeps its previous content and attributes and no file is created at `dest`.
  - **Temporary file name.** `<file name of dest>.<process id>-<n>.tmp` in the folder of `dest`, where `<n>` is the attempt number of this export, from 0 (example: `results.csv.4242-0.tmp`). It never ends in `.csv`. Two exports never run at the same time in one process (section 5.2), and the process id separates two app instances; create-new (below) covers any remaining clash.
  - **Create-new.** The temporary file is created with create-new semantics (`OpenOptions::create_new`, Windows `CREATE_NEW`): creating it never opens, truncates or overwrites an existing file. If the name exists, the next `<n>` is tried, at most 16 names (`0` to `15`) in all; when all exist the export fails with `EXPORT_FAILED` and nothing is written.
  - **Write, then replace.** The whole document is written to the temporary file, flushed to disk (`sync_all`) and closed; then it is renamed onto `dest` (`std::fs::rename`, which on Windows replaces an existing file, as `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`; no flag that ignores the read-only attribute is used). The temporary file and `dest` are in the same folder, so this is a rename on one volume, never a copy: `dest` holds either its old bytes or the whole new document, never a mix.
  - **Replace on Windows.** A missing `dest` is created by the rename. An existing, writable `dest` is replaced as a whole; the new file has the attributes and permissions of a newly created file in that folder (the old file's hidden attribute, timestamps or permissions are not carried over, Q13). A **read-only** `dest` is not replaced: the rename fails (Access is denied), the export fails with `EXPORT_FAILED`, and `dest` keeps its content and its read-only attribute; the backend never clears the attribute. A `dest` that is a folder, or a file open in another program without delete sharing (Excel), also makes the rename fail with `EXPORT_FAILED` and is left unchanged.
  - **Cleanup.** On any failure after the temporary file was created (write, flush or rename), the temporary file is deleted before the error is returned. After a success the temporary file no longer exists (it became `dest`). If deleting it fails as well, the export still fails with `EXPORT_FAILED` and the `message` names the temporary file left behind; this is the only case where a temporary file can remain.
- **Failures** become `EXPORT_FAILED` with an English technical `message` naming the destination and the operating-system reason (for example `cannot write C:\Out\r.csv: Access is denied. (os error 5)`). Examples: the destination folder does not exist or cannot be written; the destination is read-only; it is a folder; it is open and locked by another program (Excel locks an open CSV); the chosen name does not end in `.csv` (checked before any file is created, section 5, Q9). A name typed **without an extension** is completed to `.csv` by the Windows dialog before the backend sees it (Q9), so it is exported normally; the backend itself never appends an extension and refuses a path without one. The UI shows `error.EXPORT_FAILED`, not the message.
- **Only the CSV.** The export creates or modifies no file other than `dest` (and its temporary file while it is written). It reads no audio file: the CSV is made from the results kept in memory, so it works even after the analysed files were moved or deleted (RNF-05).

## 7. Acceptance criteria (measurable)

1. **Exact bytes.** For the four stubbed tracks of the scenario "The document for four stubbed tracks" (`export-csv.feature`), the serialiser produces exactly the document given there.
2. **Round-trip (planning 2.7, RNF-09).** Reading a written CSV back as UTF-8 with an RFC 4180 reader gives the header of section 3 and, for each track, 21 fields whose `file`, `folder` and `path` equal the analysed names code point for code point, including `″` (U+2033), `‘ ’`, `–`, accented letters and characters outside the Basic Multilingual Plane.
3. **Typed failures.** Every case of section 6 "Failures" exercised by the scenarios rejects with `EXPORT_FAILED` and leaves the destination folder as it was.
4. **Read-only.** After exporting into the analysed folder itself, the size, modification time and content hash of every audio file are unchanged and the only new file is the CSV.
5. **Size.** Automated (`just test`): exporting 10 000 stubbed tracks writes a file that reads back as the header and exactly 10 000 records of 21 fields, in the order given. No automated test asserts wall-clock time. Timing: in a release build, exporting 10 000 tracks takes under 1 second from choosing the file in the dialog to the status bar notice, checked by hand before the release (`ui/export-csv.md`, M7).
6. **Default file name.** `default_file_name` returns exactly the result of every row of the table in section 5.1.
7. **One export at a time.** While an export guard lives, a second `run::begin_export` fails with `INTERNAL` and the first export writes its file as if alone (section 5.2).

---
id: SPEC-CSV-EXPORT
requirements: [RF-09, RNF-04, RNF-05, RNF-09]
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
| Q8 | **Default file name** `FakeFlac - <analysed folder name>.csv` (`FakeFlac.csv` when the analysed folder is a drive root). The app does **not** set the dialog's initial folder: Windows opens the folder last used with a save dialog, else its default (Documents), and never proposes the music folder by itself | Open. Default: as written |
| Q9 | **A destination whose extension is not `.csv`** (case-insensitive) is refused with `EXPORT_FAILED` and nothing is written. This guarantees the export can never replace an audio file or any other kind of file (RNF-05). Append `.csv` instead? Appending would write a path the dialog did not confirm for overwrite | Open. Default: refuse |
| Q10 | **"No results to export" reuses the existing code `NOT_ANALYZED`** (already in `AppError`, unused so far) instead of a new code. No change to the `AppErrorCode` union | Open. Default: reuse |
| Q11 | **CSV serialisation lives in `fakeflac-export`** (as `docs/architecture/overview.md` and `backend.md` already say: "CSV serialisation", port `CsvExporter`), as a pure function plus the file writer. `fakeflac-core` keeps the port and the `ExportCsv` use case (section 5). Alternative: the pure formatter in `fakeflac-core` and only the write in the adapter | Open. Default: formatter in `fakeflac-export` |
| Q12 | **No new dependency.** The RFC 4180 writer is a few lines (quote a field when it holds `,`, `"`, CR or LF; double the quotes), so the `csv` crate is not added; the save dialog comes from `tauri-plugin-dialog`, already a dependency of `src-tauri` (`blocking_save_file`). Tests read the file back with a small RFC 4180 reader in test support, not a `csv` dev-dependency. If the owner prefers the `csv` crate (runtime or dev), it needs the owner's consent and an ADR before the feature PR | Open. Default: no new dependency, no ADR |
| Q13 | **Safe replacement.** The file is written to a temporary file in the destination folder and then moved over the destination, so a failed export never leaves a partial or truncated CSV, and an existing file is either fully replaced or left unchanged (section 6). Write the destination directly instead (simpler, but a failure mid-write leaves a truncated file)? | Open. Default: temporary file + replace |
| Q14 | **Number precision**: cutoff in whole Hz, steepness and noise floor with one decimal, duration in whole ms (section 4) | Open. Default: as written |

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

- **Error rows** (`verdict` `error`) are exported like any other: they carry what was learnt before the error (`commands.md`, "Lossy formats (version 3)"), and the cutoff, bitrate estimate and reasons cells are empty (RNF-04).
- **Lossy rows** fill columns 9–11; lossless rows leave them empty (`ui/lossy-formats.md`).
- **A non-finite cutoff** (any of the three `CutoffEstimate` values is NaN or infinite) is written as no cutoff: columns 8, 18 and 19 empty, as the contract sends such a track without a cutoff (`src-tauri` DTO conversion).
- Changing the columns, their order or a value encoding bumps `csv_format_version` (a spec change at G1). The version is not written into the file (RFC 4180 has no comment lines).

## 4. Numbers

- **Integers** (columns 5–7, 9, 11, 16, 17, 20): base-10 ASCII digits, no sign, no leading zeros, no digit grouping, no unit.
- **`cutoff_hz`**: `frequencyHz` rounded to the nearest whole number, halves away from zero (`16000.4` → `16000`, `16000.5` → `16001`).
- **One-decimal values** (columns 18 and 19): the stored value rounded to one decimal place, written with `.` as the decimal point, at least one digit before it, exactly one after it, `-` for negative values and no exponent (`42.06` → `42.1`, `-101.94` → `-101.9`, `-120` → `-120.0`). A value that rounds to zero is written `0.0`, never `-0.0`. Values that are an exact half of the last place are not specified; tests avoid them.

## 5. Layers (hexagonal, `docs/architecture/dependencies.md`)

| Layer | Responsibility | No I/O? |
|---|---|---|
| `fakeflac-core` | Port `CsvExporter` (`export(&self, tracks: &[TrackAnalysis], dest: &Path) -> Result<(), AppError>`, already listed in `docs/architecture/backend.md`, so no ADR). Use case `ExportCsv`: `export_csv(exporter, tracks, dest) -> Result<u32, AppError>` returns the number of rows written. It fails with `NOT_ANALYZED` when `tracks` is empty and with `EXPORT_FAILED` when `dest` is not absolute or its extension is not `csv` (case-insensitive, Q9); in both cases the exporter is not called. Otherwise it calls the exporter once with `tracks` in the order given | Yes (pure; tested with a fake exporter) |
| `fakeflac-export` | A pure function that builds the whole document (sections 1–4) from `&[TrackAnalysis]`, in the order given, and `CsvFileExporter`, which implements `CsvExporter` by writing exactly those bytes to `dest` (section 6). It opens no file other than the destination and its temporary file: it never reads or opens an audio file (RNF-05) | Formatter yes; the writer is the adapter |
| `src-tauri` | Keeps the results of the latest run (`commands.md`, "CSV export"), runs the save dialog (`tauri-plugin-dialog`, already a dependency) with the default file name (Q8) and the filter "CSV (*.csv)", and wires the use case to `CsvFileExporter`. The step after the dialog is a plain function (`run::export_csv`, results and destination in) so it is tested without a Tauri runtime | Adapter / composition root |
| Frontend | `exportCsv()` in `src/lib/api/`; the Toolbar button and the status bar notice (`specs/ui/export-csv.md`). It never sees file contents | — |

No capability change: the dialog and the write happen in Rust (`src-tauri/capabilities/default.json` stays as it is; no `fs` or `dialog` permission for the page).

## 6. Writing the file

- The file at `dest` ends up holding exactly the document of sections 1–4, nothing before or after it.
- **Overwrite.** The native save dialog asks the user to confirm replacing an existing file (Windows behaviour); once confirmed, the existing file is **replaced as a whole** (never appended to, never left with old bytes at its end).
- **Safe replacement (Q13).** The document is written to a temporary file in the destination folder, then moved over `dest`. After a success or a failure no temporary file is left behind. When the export fails, a file that already existed at `dest` keeps its previous content and no file is created at `dest`.
- **Failures** become `EXPORT_FAILED` with an English technical `message` naming the destination and the operating-system reason (for example `cannot write C:\Out\r.csv: Access is denied. (os error 5)`). Examples: the destination folder does not exist or cannot be written; the destination is read-only; it is a folder; it is open and locked by another program (Excel locks an open CSV). The UI shows `error.EXPORT_FAILED`, not the message.
- **Only the CSV.** The export creates or modifies no file other than `dest` (and its temporary file while it is written). It reads no audio file: the CSV is made from the results kept in memory, so it works even after the analysed files were moved or deleted (RNF-05).

## 7. Acceptance criteria (measurable)

1. **Exact bytes.** For the four stubbed tracks of the scenario "The document for four stubbed tracks" (`export-csv.feature`), the serialiser produces exactly the document given there.
2. **Round-trip (planning 2.7, RNF-09).** Reading a written CSV back as UTF-8 with an RFC 4180 reader gives the header of section 3 and, for each track, 21 fields whose `file`, `folder` and `path` equal the analysed names code point for code point, including `″` (U+2033), `‘ ’`, `–`, accented letters and characters outside the Basic Multilingual Plane.
3. **Typed failures.** Every case of section 6 "Failures" exercised by the scenarios rejects with `EXPORT_FAILED` and leaves the destination folder as it was.
4. **Read-only.** After exporting into the analysed folder itself, the size, modification time and content hash of every audio file are unchanged and the only new file is the CSV.
5. **Size.** 10 000 tracks are serialised and written in under 1 second in the test profile (`just test`).

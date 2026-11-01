---
id: SPEC-CALIBRATION-GENERATION
requirements: [RP-11, RNF-04, RNF-05]
status: approved
---

# Transcode generation and the features cache

> Positives are made from trusted tracks: a fragment from the centre of the track is encoded with a lossy encoder through ffmpeg and decoded back to 16-bit FLAC. For RF-15 the tool also keeps **lossy files** as labelled tracks (honest encodes and cascades of several encodes, `[[generate.lossy]]`); they follow `lossy.md` section 4, and the lossy intermediate rule of section 3 below does not apply to them. Spectra of every labelled track are cached, so recalibrating re-applies the rules without decoding. Main spec: [calibration.md](calibration.md). ffmpeg and LAME are development tools and never part of the product (planning section 1.5, ADR-0036).

## 1. What is generated

- **Sources:** every trusted track (all classes, `reference` included, inputs.md 1.5) whose own analysis is not `Error` and not silent (rules.md section 1.6). Confirmed fakes are not sources. A skipped source is listed in the report with its reason.
- **Settings:** the `[[generate.settings]]` of the config. Version 1 has the 9 settings of the issue:

| id | encoder | ffmpeg encoder arguments | ext | `bitrate_kbps` |
|---|---|---|---|---|
| `cbr128` | `lame` | `-c:a libmp3lame -b:a 128k` | mp3 | 128 |
| `cbr192` | `lame` | `-c:a libmp3lame -b:a 192k` | mp3 | 192 |
| `cbr256` | `lame` | `-c:a libmp3lame -b:a 256k` | mp3 | 256 |
| `cbr320` | `lame` | `-c:a libmp3lame -b:a 320k` | mp3 | 320 |
| `v0` | `lame` | `-c:a libmp3lame -q:a 0` | mp3 | (empty) |
| `v2` | `lame` | `-c:a libmp3lame -q:a 2` | mp3 | (empty) |
| `aac128` | `aac` | `-c:a aac -b:a 128k` | m4a | 128 |
| `aac256` | `aac` | `-c:a aac -b:a 256k` | m4a | 256 |
| `opus128` | `opus` | `-c:a libopus -b:a 128k` | opus | 128 |

  `libmp3lame` with `-b:a` and no `-q:a` is CBR. `aac` is ffmpeg's native AAC encoder. `libopus` is VBR by default.
- **Fragment (OQ-10):** duration `generate.fragment_s`.
  - **Default: full track** (`generate.fragment_s` omitted or `null` in config). The entire track is transcoded (`start = 0`, `fragment_s` = track duration). Using full tracks prevents optimistic bias: many real fake transcodes are skits, intros, or quiet sections that a 60 s center fragment might miss or make unnaturally easy to detect.
  - **Fragment option:** when `generate.fragment_s` is set to a number (e.g. 60.0), a centered fragment is extracted: `start = max(0, (duration − fragment_s) / 2)`, rounded down to the millisecond. A track no longer than `fragment_s` is used whole (`start = 0`, `fragment_s` = its duration).
  - Negatives are analysed as whole files, as the app does.

## 2. ffmpeg

Two calls per entry, with arguments passed as a list (`std::process::Command`, no shell, so spaces and Unicode in names are safe). When using full tracks (`fragment_s` omitted/null), `-ss <start> -t <fragment_s>` is omitted:

```
encode: <ffmpeg> -nostdin -hide_banner -loglevel error [-ss <start> -t <fragment_s>] -i <source>
                 -map 0:a:0 -map_metadata -1 <setting args> -y <generated>/<id>.<ext>
decode: <ffmpeg> -nostdin -hide_banner -loglevel error -i <generated>/<id>.<ext>
                 -map 0:a:0 -map_metadata -1 <generate.decode_args> -y <generated>/<id>.flac
```

- `generate.decode_args` (version 1): `-c:a flac -sample_fmt s16`. The output is **16-bit FLAC** with no dither (ffmpeg's default requantisation), with the channels and the sample rate that the decoder delivers. **Opus stays at 48 kHz (OQ-8).** For a hi-res source, the rate is whatever the encoder accepted. The rate is recorded in `labels.csv` (`sample_rate_hz`).
- **Per-setting decode (config version 3, detection v6).** A `[[generate.settings]]` entry may have its own `decode_args` (a list of strings, like `args`), used instead of `generate.decode_args` in its decode call; an entry without it uses `generate.decode_args`, so configs v1 and v2 behave exactly as before. The recipe's `decode_args` (section 3) is the list actually used, so such a setting gets its own cache entries, and the settings without it keep theirs. It applies to lossless transcodes only; a lossy stage never decodes. `calibration.v3.toml` uses it for four settings decoded to **24-bit FLAC** (`-c:a flac -sample_fmt s32 -bits_per_raw_sample 24`, no dither): `cbr128_s24`, `cbr192_s24`, `cbr256_s24` and `cbr320_s24`, each with the encoder arguments, `ext`, `bitrate_kbps` and `lowpass_hz` of the setting it is named after (rules.md section 8, Q21). A fake lossless file is often written at 24 bits, where the decoder's residue above the low-pass can lie below the 16-bit rounding floor (rules.md section 2, `F_top`, whose 16-bit floor these settings exercise).
- `<ffmpeg>` is `FAKEFLAC_FFMPEG` or `ffmpeg` on `PATH`. `ffmpeg_version` is the first line of `ffmpeg -version`, cut before ` Copyright`.
- **Exact versions (OQ-17)** in `encoder_version`, read before the intermediate file is deleted:
  - `lame`: `<ffmpeg_version>; ` plus the encoder string of the LAME tag in the first frame of the MP3 (for example `LAME3.100`). Fallback: `libmp3lame (version not readable)`.
  - `aac`: `<ffmpeg_version>; aac (native)` (the encoder is part of that ffmpeg build).
  - `opus`: `<ffmpeg_version>; ` plus the vendor string of the `OpusTags` header if it names libopus (for example `libopus 1.5.2`). Fallback: `libopus (version not readable)`.
- **Failures.** If ffmpeg cannot be started, or `ffmpeg -version` fails, and at least one transcode is needed: exit 3 (`error: ffmpeg not found; set FAKEFLAC_FFMPEG or add it to PATH`). If ffmpeg is missing but every entry is cached, the run continues and the report says so. If one call exits non-zero, that entry is a generation failure: it gets no label row, is not cached (it is retried on the next run), and is listed in `report.md` with the setting and the first line of ffmpeg's error output. The run continues.

## 3. Files in `generated/` (OQ-9)

- Flat folder, one name per entry: `<id>.flac`, where `<id>` is 16 lower-case hex digits of `fmix64(fnv1a64(recipe JSON))` (hash functions as in inputs.md section 4). Short names keep paths below the Windows limit and keep corpus names out of `generated/`.
- **Recipe** (JSON object, keys sorted): `source` (`path`, `size`, `mtime` of the source), `setting`, `args`, `decode_args`, `fragment_start_s`, `fragment_s`.
- The lossy intermediate `<id>.<ext>` is always deleted after the decode call.
- With `generate.keep_audio = false` (the default), `<id>.flac` is deleted as soon as its spectra are cached.
- **Inspect misclassifications (`keep_misclassified`):** When `generate.keep_misclassified = true` (default: false), `<id>.flac` is preserved in `generated/` for any track that resulted in a misclassification (`Fake` on a negative, or `Real` on a positive transcode) so that the user can inspect its spectrogram.
- **Sidecar `generated/<id>.json`:** Whenever `<id>.flac` is retained (under `keep_audio = true` or `keep_misclassified = true`), a JSON sidecar `generated/<id>.json` is written alongside it recording the versions and metadata it was produced with (such as `encoder_version`), so that subsequent runs reusing existing audio keep `labels.csv` exactly identical without re-transcoding.
- Deleting `generated/` at any time is safe: generated entries are looked up in the cache by recipe, not by file.
- When a generated entry misses the cache (for example under a new fingerprint), an existing `<id>.flac` is analysed without transcoding. If the audio was deleted, the entry is transcoded again.

## 4. Features cache (OQ-3)

### 4.1 Extraction fingerprint

```
fingerprint JSON = { "extraction_version": fakeflac_audio::EXTRACTION_VERSION,
                     "input": <[input] section of the params file>,
                     "fft":   <[fft] section of the params file> }      (keys sorted)
fingerprint id   = 16 hex digits of fmix64(fnv1a64(fingerprint JSON))
```

- `EXTRACTION_VERSION` is a public integer of `fakeflac-audio` (2 today). It must be bumped in the same PR as any adapter change that alters the `AudioProbe` produced for the same file and parameters (ADR-0036).
- **Decoder version bump:** `EXTRACTION_VERSION` **must also be bumped whenever the `symphonia` crate version is updated**, because decoder changes or bugfixes in symphonia can alter decoded audio samples and resulting spectra.
- `algorithm_version` and every rule parameter (`[bands]`, `[extent]`, `[step]`, `[gaps]`, `[verdict]`, and the optional `[full_band]`, `[lossy]` and `[edge]`) and `[spectrogram]` are **not** part of the fingerprint. Params files that differ only in those values share one cache, whatever their `algorithm_version`.
- Files: `features/<fingerprint id>.meta.json` (the fingerprint JSON) and `features/<fingerprint id>.jsonl` (entries). A params file with a different fingerprint gets its own extraction pass and its own files. The report says which params file used which fingerprint.

### 4.2 Entries

One JSON object per line:

| Field | Content |
|---|---|
| `key` | corpus track: `{ "kind": "corpus", "path", "size", "mtime" }` (path relative to `FAKEFLAC_CORPUS`, `/` separators; size in bytes; mtime in Unix seconds, as `FileEntry`). Generated entry: `{ "kind": "generated", "recipe": {…} }` (section 3) |
| `probe` | `{ "sample_rate", "bit_depth", "channels", "duration_ms", "spectrum" }`, where `spectrum` is `null` (silent track) or `{ "band_hz", "mean_db": [...], "median_db": [...] }` (`BandSpectra`) |
| `error` | instead of `probe`: the `TrackError` code (`corruptStream`, `unsupportedFormat`, `tooShort`). `unreadable` is not cached, because it may be temporary |
| `ffmpeg_version`, `encoder_version` | generated entries only |

- **Lookup:** by key (canonical JSON). When a key appears on several lines, the last one wins. A line that does not parse (for example cut short by a crash) is skipped and counted, and its track is extracted again.
- **Hit:** no decoding and no transcoding. **Miss:** decode (and, for a generated entry, transcode first), then append one line.
- A generated entry whose `ffmpeg_version` differs from the ffmpeg found now is stale and is generated again. If no ffmpeg is found, cached generated entries are used as they are.
- **Exact floats.** Values are written by `serde_json` (shortest representation that round-trips) and read with its `float_roundtrip` feature. A report computed from the cache is identical to one computed from a fresh extraction.
- **Invalidation:** a change in a corpus file's size or mtime changes its key (re-extracted) and the recipes of its transcodes (generated again). A new fingerprint means new files. The tool never deletes files in `features/`; the user may.
- **Statistics** per fingerprint, in the report: cache hits, decoded, transcoded, generation failures, skipped lines.

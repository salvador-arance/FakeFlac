---
id: SPEC-FIXTURES-LOSSY
requirements: [RF-14, RF-15]
status: proposed
adr: ADR-0040
---

# Synthetic lossy test files (ADR-0022, ADR-0040)

> MP3 and M4A test files are **written by small bitstream writers in test support**, from a seed, inside the tests. No encoder runs in CI, no dependency is added and no audio is committed. A writer does not encode audio: it writes the **coded spectrum** directly (quantised MDCT coefficients), so the decoder's output is noise up to a chosen edge and essentially nothing above it, exactly what an honest or a fake lossy file looks like to the rules of `lossy-source.md`. Lossless fixtures stay those of `fixtures.md`.

## 1. Writers

Each writer is deterministic: the same arguments and seed give the same bytes. Coefficients are drawn with a seeded generator (`rand_chacha`, already a dev-dependency).

### 1.1 MP3 (MPEG-1 Layer III)

- **Frames.** 44.1 or 48 kHz, mono, no CRC, no bit reservoir (`main_data_begin` = 0), long blocks only, `scalefac_compress` = 0 (no scale factors), `big_values` = 0, `count1table_select` = 1 (quadruple table B: four bits per quadruple plus one sign bit per non-zero value, ISO/IEC 11172-3 table B.7), `part2_3_length` = the bits actually written. Every granule codes coefficients `0 … k − 1` and leaves the rest zero, with `k` = `round(edge_hz / bin_hz / 4) · 4` (halves away from zero) and `bin_hz` = `sample_rate / 2 / 576` (38.28 Hz at 44.1 kHz, 41.67 Hz at 48 kHz), so the **coded edge** `k · bin_hz` is within 2 bins (77 Hz at 44.1 kHz) of the nominal edge; section 2 lists it per fixture. Each coded coefficient is −1, 0 or +1 with probabilities ¼, ½, ¼.
- **Length.** "30 s" means `round(30 · sample_rate / 1152)` audio frames: 1148 at 44.1 kHz (29.989 s), 1250 at 48 kHz (30.000 s); "2 s" means 77 frames at 44.1 kHz (2.011 s). The tag frame comes on top and is not audio.
- **Level.** One `global_gain` for every granule, chosen so that the decoded RMS of a full-band fixture lies between −30 and −12 dBFS (the test stage picks the value once and records it in the test-support module).
- **Bitrate.** Each frame has the size of its bitrate index (`144 · bitrate / sample_rate` bytes, with the padding bit set as an encoder does); the bits after the coded data are zero (ancillary data). For a VBR fixture the bitrate index changes from frame to frame in a given repeating pattern; it must be large enough for the coded data (128 kbps holds a 17 kHz edge at these densities).
- **Tag frame** (optional): an `Info` or `Xing` frame first (a frame of the same bitrate index as the first audio frame), with the frames field = the number of **audio** frames (the tag frame excluded) and the bytes field = the size of every frame **including the tag frame** (the writer writes no ID3 tag), as LAME writes them (`lossy-source.md` section 2.1), and optionally the LAME extension: a 9-byte encoder field (`LAME3.100` unless a row says otherwise), the VBR-method nibble and the low-pass byte (Hz / 100). Fields the rules do not read are zero.
- **Gated edge** (optional): in a given share of granules the coefficients from a second edge upwards are zero, as LAME leaves sfb21 empty.

### 1.2 M4A (MP4 container)

- **Container.** `ftyp` (`M4A `), `moov` (with `mvhd`, one audio `trak`, its `mdhd` duration, `stsd`, `stts`, `stsc`, `stsz`, `stco`) **before** `mdat`. One sample per AAC frame (1024 samples) or per ALAC frame (4096 samples).
- **AAC-LC.** `mp4a` sample entry with an `esds` holding an AudioSpecificConfig (object type 2, the sample-rate index, 1 channel). Each raw data block is one `single_channel_element`: long window only, `max_sfb` = the first scale-factor band at or above the edge, one section per band with spectral codebook 1, every scale factor equal to `global_gain` (each difference codes as the 1-bit code of index 60), no pulse, TNS or gain control data; coefficients from bin `round(edge_hz / bin_hz)` up (`bin_hz` = 21.53 Hz at 44.1 kHz) are zero, those below are −1, 0 or +1 as in 1.1. **Frame sizes:** with `B = bitrate · 1024 / (8 · sample_rate)` bytes (371.52 at 128 kbps, 743.04 at 256 kbps, 44.1 kHz), frame `n` (from 0) has exactly `floor((n + 1) · B) − floor(n · B)` bytes; the coded data is followed by as many `fill_element`s as needed (one carries at most 269 bytes, so a 256 kbps frame needs two or three), the last one sized so that the frame has exactly that length. "30 s" means 1292 frames (29.999 s, `mdhd` duration 1292 · 1024 at time scale 44100), so `Σ sample sizes = floor(1292 · B)` and the declared bitrate (`lossy-source.md` section 2.2) is **exactly** 128 or 256 kbps (about 128.003 and 256.003 before rounding). The largest frame, 744 bytes, is below AAC-LC's 768-byte limit per mono frame. The Huffman codebook 1 and the scale-factor codebook come from ISO/IEC 14496-3 (tables 4.A.1 and 4.A.2); the test stage verifies them by decoding a fixture with symphonia and checking its spectrum (section 3).
- **ALAC.** `alac` sample entry with the 24-byte ALAC configuration (frame length 4096, bit depth 16, 1 channel) and **uncompressed** frames (escape flag set; samples verbatim), filled with a lossless fixture of `fixtures.md`.
- **HE-AAC header.** The AAC writer with object type 5 in the AudioSpecificConfig (explicit SBR signalling); the frames are LC frames. The adapter rejects it from the header (`lossy-source.md` section 2.2), so the frames are never decoded.

## 2. Fixtures

All 44.1 kHz, mono, 30 s unless the row says otherwise. "Edge" is the nominal frequency above which every coefficient is zero, followed by the **coded edge** (`k · bin_hz`, sections 1.1 and 1.2), which is what the decoder produces. The truth is the v5 result with `detection-params.v5.toml` (`lossy-source.md` sections 4 and 5); cutoff tolerance ± 200 Hz (2 bands) around the nominal edge, and every `s` below uses the coded edge.

| Id | Writer and stream | Edge (coded) | Truth (v5) | Models |
|---|---|---|---|---|
| `mp3_320_lp20k5` | MP3 CBR 320, `Info` + LAME tag, low-pass byte 205 | 20500 (20519) | Real (`s` ≈ −19); cutoff ≈ 20.5 kHz; estimate 320 | An honest LAME 320 |
| `mp3_320_lp17k` | MP3 CBR 320, `Info` + LAME tag, low-pass byte 205 | 17000 (16997) | **Fake** (`s` ≈ 3500); cutoff ≈ 17.0 kHz; estimate 128 | A 320 made from a LAME 128 |
| `mp3_128_lp17k` | MP3 CBR 128, `Info` + LAME tag, low-pass byte 170 | 17000 (16997) | Real (`s` ≈ 3); estimate 128 | An honest LAME 128 |
| `mp3_192_lp17k_notag` | MP3 CBR 192, **no tag frame** | 17000 (16997) | Suspicious, `cutoffBelowDeclared` (`s` ≈ 1600); estimate 128 | A 192 from a 128, and the CBR path without a tag |
| `mp3_320_lp18k5` | MP3 CBR 320, `Info` + LAME tag, low-pass byte 205 | 18500 (18528) | Suspicious, `cutoffBelowDeclared` (`s` ≈ 1970); estimate 192 | A 320 from a 192 |
| `mp3_320_tag16k` | MP3 CBR 320, `Info` + LAME tag, **low-pass byte 160** | 16000 (15925) | Real (`c_exp` = 16000, `s` ≈ 75); estimate 320 | A user-chosen low-pass (Q6) |
| `mp3_vbr_lp19k5` | MP3 VBR: bitrate indexes 224 and 256 alternating; `Xing` + LAME tag, VBR method 4, low-pass byte 195 | 19500 (19447) | Real (`b` = 240, `c_exp` = 19400, `s` ≈ −47); mode `vbr`; estimate 240 | An honest VBR file |
| `mp3_abr_lp18k6` | MP3 ABR: bitrate indexes 160 and 224 alternating; `Xing` + LAME tag, **VBR method 2**, **low-pass byte 0** (no low-pass declared) | 18600 (18528) | Real (`b` = 192, `c_exp` = 18600, `s` ≈ 72); mode `abr`; estimate 192 | The ABR path |
| `mp3_320_48k_lp20k` | MP3 CBR 320 at **48 kHz**, `Info` + LAME tag, low-pass byte 205 | 20000 (20000) | Real (`s` ≈ 500, known limitation); estimate 320 | An MP3 made from Opus |
| `mp3_320_low_hf` | MP3 CBR 320, `Info` + LAME tag | 6000 (5972) | Suspicious, `lowHighFrequencyContent`; no estimate | Little treble |
| `mp3_silence` | MP3 CBR 320, every coefficient zero | — | Suspicious, `lowHighFrequencyContent`; no cutoff; no estimate | A silent file |
| `mp3_short` | `mp3_320_lp20k5`, **2 s** | 20500 (20519) | Error, `tooShort` | |
| `mp3_corrupt` | A text file of 4 KiB of ASCII letters saved as `.mp3` (no `0xFF` byte, so no frame sync and no ID3 header) | — | Error, `corruptStream` | A file that is not MP3 |
| `m4a_aac_128_lp16k` | M4A, AAC-LC, 128 kbps | 16000 (15999) | Real (`c_exp` = 15500, `s` ≈ −500); codec `aac`; estimate 128 | A streaming-site AAC 128 copied without re-encoding |
| `m4a_aac_256_lp15k3` | M4A, AAC-LC, 256 kbps | 15300 (15310) | **Fake** (`s` ≈ 3690); estimate 128 (nearest row 15500) | An AAC 256 made from a 128 kbps source |
| `m4a_alac_full_band` | M4A, ALAC, samples of `full_band` (16-bit) | — | Real (lossless rules); codec `alac`; bit depth 16; no declared bitrate; no estimate | ALAC is lossless |
| `m4a_alac_lp16k` | M4A, ALAC, samples of `lp16k` | — | Fake (lossless rules); cutoff ≈ 16 kHz; no estimate | A transcode stored as ALAC |
| `m4a_he_aac` | M4A, AAC writer with object type 5 | 16000 | Error, `unsupportedFormat` | HE-AAC (Q10) |
| `m4a_corrupt` | `m4a_aac_128_lp16k` truncated in the middle of its `moov` box | — | Error, `corruptStream` | |

Bitrate check: at these densities a granule codes about 1.5 bits per coefficient below the edge, so a 128 kbps mono frame at 44.1 kHz (417 or 418 bytes, about 3340 bits) holds two granules of an edge at 17 kHz (about 2 × 670 bits plus 168 bits of header and side information). Edges were chosen at least 200 Hz away from the midpoint between two table rows, so that the estimate does not depend on where inside the ± 200 Hz tolerance `fc` falls.

## 3. What the test stage confirms first

The verdicts above are derived from the writer definitions, not measured. Before deriving the scenario tests, the test stage checks on each fixture, with the default parameters:

1. symphonia decodes it without error (except the error fixtures) and the decoded sample rate, channels and duration (± 1 frame) are the writer's;
2. the measured cutoff (`fc`) is within ± 200 Hz of the edge, and `D` ≥ `fake_min_step_db` for every fixture with an edge between 11 kHz and Nyquist − 400 Hz;
3. every band level `M(b)` is finite (the decoder's output above the edge is filter-bank leakage, not exact zeros); if a band is exactly zero, the test stage stops and reports it (rules.md defines no floor);
4. `H` ≥ `low_hf_threshold_db` for every fixture with an edge of 16 kHz or more, and below it for `mp3_320_low_hf`.

If any check fails, it stops and the spec returns to G1 (as for `sfb21_gaps` in `fixtures.md`, "Verdicts with v4").

## 4. Unicode names and read-only checks

The decode scenarios save fixtures under Unicode names (RNF-09: `Álbum ″Ñandú″/01 – Canción ‘x’.mp3`) and compare size, modification time and content hash before and after the analysis (RNF-05), as the lossless scenarios do.

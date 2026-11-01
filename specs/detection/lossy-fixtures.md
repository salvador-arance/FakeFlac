---
id: SPEC-FIXTURES-LOSSY
requirements: [RF-14, RF-15, RF-16]
status: approved
adr: [ADR-0040, ADR-0047]
---

# Synthetic lossy test files (ADR-0022, ADR-0040, ADR-0047)

> MP3 and M4A test files are **written by small bitstream writers in test support**, from a seed, inside the tests. No external encoder runs in CI, no dependency is added and no audio is committed. A writer does not encode audio: it writes the **coded spectrum** directly (quantised MDCT coefficients), so the decoder's output is noise up to a chosen edge and essentially nothing above it, exactly what an honest or a fake lossy file looks like to the rules of `lossy-source.md`. Lossless fixtures stay those of `fixtures.md`.
>
> **Opus test files** (RF-14, RF-15, detection v8) are the exception decided by ADR-0047 point 3: they are **encoded** in test support by `rusty-opus`'s own encoder (already a production dependency, so nothing is added) from the seeded synthetic signals, and written to Ogg by a small **test-support Ogg writer** that is not the adapter's reader (sections 1.3 and 1.4). No external tool runs and no audio is committed.

## 1. Writers

Each writer is deterministic: the same arguments and seed give the same bytes. Coefficients are drawn with a seeded generator (`rand_chacha`, already a dev-dependency).

### 1.1 MP3 (MPEG-1 Layer III)

- **Frames.** 44.1 or 48 kHz, mono, no CRC, no bit reservoir (`main_data_begin` = 0), long blocks only, `scalefac_compress` = 0 (no scale factors), `big_values` = 0, `count1table_select` = 1 (quadruple table B: four bits per quadruple plus one sign bit per non-zero value, ISO/IEC 11172-3 table B.7), `part2_3_length` = the bits actually written. Every granule codes coefficients `0 … k − 1` and leaves the rest zero, with `k` = `round(edge_hz / bin_hz / 4) · 4` (halves away from zero) and `bin_hz` = `sample_rate / 2 / 576` (38.28 Hz at 44.1 kHz, 41.67 Hz at 48 kHz), so the **coded edge** `k · bin_hz` is within 2 bins (77 Hz at 44.1 kHz) of the nominal edge; section 2 lists it per fixture. Each coded coefficient is −1, 0 or +1 with probabilities ¼, ½, ¼.
- **Length.** "30 s" means `round(30 · sample_rate / 1152)` audio frames: 1148 at 44.1 kHz (29.989 s), 1250 at 48 kHz (30.000 s); "2 s" means 77 frames at 44.1 kHz (2.011 s) and 83 frames at 48 kHz (1.992 s; added for RF-16). The tag frame comes on top and is not audio.
- **Level.** One `global_gain` for every granule, chosen so that the decoded RMS of a full-band fixture lies between −30 and −12 dBFS (the test stage picks the value once and records it in the test-support module).
- **Bitrate.** Each frame has the size of its bitrate index (`144 · bitrate / sample_rate` bytes, with the padding bit set as an encoder does); the bits after the coded data are zero (ancillary data). For a VBR fixture the bitrate index changes from frame to frame in a given repeating pattern; it must be large enough for the coded data (128 kbps holds a 17 kHz edge at these densities).
- **Tag frame** (optional): an `Info` or `Xing` frame first (a frame of the same bitrate index as the first audio frame), with the frames field = the number of **audio** frames (the tag frame excluded) and the bytes field = the size of every frame **including the tag frame** (the writer writes no ID3 tag), as LAME writes them (`lossy-source.md` section 2.1), and optionally the LAME extension: a 9-byte encoder field (`LAME3.100` unless a row says otherwise), the VBR-method nibble and the low-pass byte (Hz / 100). Fields the rules do not read are zero.
- **Gated edge** (optional): in a given share of granules the coefficients from a second edge upwards are zero, as LAME leaves sfb21 empty.

### 1.2 M4A (MP4 container)

- **Container.** `ftyp` (`M4A `), `moov` (with `mvhd`, one audio `trak`, its `mdhd` duration, `stsd`, `stts`, `stsc`, `stsz`, `stco`) **before** `mdat`. One sample per AAC frame (1024 samples) or per ALAC frame (4096 samples).
- **AAC-LC.** `mp4a` sample entry with an `esds` holding an AudioSpecificConfig (object type 2, the sample-rate index, 1 channel). Each raw data block is one `single_channel_element`: long window only, `max_sfb` = the first scale-factor band at or above the edge, one section per band with spectral codebook 1, every scale factor equal to `global_gain` (each difference codes as the 1-bit code of index 60), no pulse, TNS or gain control data; coefficients from bin `round(edge_hz / bin_hz)` up (`bin_hz` = 21.53 Hz at 44.1 kHz) are zero, those below are −1, 0 or +1 as in 1.1. **Frame sizes:** with `B = bitrate · 1024 / (8 · sample_rate)` bytes (371.52 at 128 kbps, 743.04 at 256 kbps, 44.1 kHz), frame `n` (from 0) has exactly `floor((n + 1) · B) − floor(n · B)` bytes; the coded data is followed by as many `fill_element`s as needed (one carries at most 269 bytes, so a 256 kbps frame needs two or three), the last one sized so that the frame has exactly that length. "30 s" means 1292 frames (29.999 s, `mdhd` duration 1292 · 1024 at time scale 44100), so `Σ sample sizes = floor(1292 · B)` and the declared bitrate (`lossy-source.md` section 2.2) is **exactly** 128 or 256 kbps (about 128.003 and 256.003 before rounding). The largest frame, 744 bytes, is below AAC-LC's 768-byte limit per mono frame. **At 48 kHz** (added for RF-16; sample-rate index of 48000 in the AudioSpecificConfig, `bin_hz` = 23.4375 Hz, the 48 kHz long-window scale-factor band table, which ISO/IEC 14496-3 shares with 44.1 kHz): "30 s" means 1406 frames (29.995 s, `mdhd` duration 1406 · 1024 at time scale 48000); at 256 kbps `B` = 682.67 bytes, `Σ sample sizes = floor(1406 · B)` = 959829 and the declared bitrate is **exactly** 256 kbps (about 255.9999 before rounding); the largest frame, 683 bytes, is below the limit. The Huffman codebook 1 and the scale-factor codebook come from ISO/IEC 14496-3 (tables 4.A.1 and 4.A.2); the test stage verifies them by decoding a fixture with symphonia and checking its spectrum (section 3).
- **ALAC.** `alac` sample entry with the 24-byte ALAC configuration (frame length 4096, bit depth 16, 1 channel) and **uncompressed** frames (escape flag set; samples verbatim), filled with a lossless fixture of `fixtures.md`.
- **HE-AAC header.** The AAC writer with object type 5 in the AudioSpecificConfig (explicit SBR signalling); the frames are LC frames. The adapter rejects it from the header (`lossy-source.md` section 2.2), so the frames are never decoded.

### 1.3 Opus (encoded by `rusty-opus`, ADR-0047 point 3)

- **Where.** `crates/fakeflac-audio/tests/common/` (test support). The encoder is `rusty-opus` `=1.0.2`, the crate the adapter decodes with; no dev-dependency is added.
- **Input signal.** The generators of `fixtures.md` run at **48 kHz** (the encoder is only ever given 48 kHz, so no resampling is involved): `full_band` (white noise, no low-pass) and `lp16k` (the same noise through the brick-wall low-pass at 16 kHz), with the levels of `fixtures.md` ("Levels": −12 dBFS RMS, then scaled down to a −1 dBFS peak, which leaves about −15 dBFS RMS for 30 s of Gaussian noise). Samples are `f32` in `[−1, 1]`, not quantised to 16 bits. "30 s" is N = 1 440 000 samples per channel, "2 s" is N = 96 000. Mono unless a row says stereo; a stereo fixture feeds the generator's output for seed `s` to the left channel and for seed `s + 1` to the right (independent noise).
- **Seeds.** Each Opus fixture uses the seed of its signal in `fixtures.md` (the seed of the 44.1 kHz signal of the same name); test support records them next to the writer. The same seed and settings give the same bytes on the same machine (section 3, check 7; "SIMD" below).
- **Encoder settings.** Application `Audio` (music), sample rate 48000 Hz, the row's channel count, frame size **20 ms** (960 samples), the row's bitrate, **`max_bandwidth`** as the row says (`Fullband`: content to 20 kHz; `Superwideband`: content to 12 kHz), and every other setting at the encoder's default (VBR, the encoder's default complexity). The test stage records the settings used in test support.
- **Pre-skip and framing.** Pre-skip = **120 samples**, a constant in test support. `rusty-opus` 1.0.2 has no public lookahead getter; its encoder delay, measured before G1 by cross-correlating the decode with the input, is 120 samples (2.5 ms) in mono and stereo (`lossy-source.md` section 12.8 F7; libopus's is 312, and with 312 a `rusty-opus` fixture would decode 192 samples early). The test stage checks that the value still holds (section 3, check 5) instead of reading it from the encoder. The writer feeds the N input samples in 960-sample frames and pads the end with zeros until `pre_skip + N` samples have been coded: `ceil((pre_skip + N) / 960)` packets, **1501** for 30 s (1 440 960 samples coded, last granule 1 440 120, end trim 840) and **101** for 2 s (96 960 coded, last granule 96 120, end trim 840).
- **SIMD** (ADR-0047 point 3 as amended, `lossy-source.md` Q42). Measured before G1 (`lossy-source.md` section 12.8 F7): `rusty-opus` 1.0.2 gives byte-identical output for the same ISA level, but its scalar, SSE2 and AVX paths give **different bytes** from the default AVX2 path for mono fixtures (the decoded difference is about 62 dB SNR; `fc`, `D` and `H` unchanged). So the rule is:
  - (a) **Determinism on one machine.** The same recipe and seed give the same bytes within one process and between two runs on the same machine (section 3, check 7; `opus.feature`, "The fixture encoder is deterministic on one machine").
  - (b) **Truth on the scalar path.** Every encoded fixture encoded in a child process of the test binary with the environment variable `RUSTY_OPUS_ISA=scalar` keeps its **truth**, not its bytes: the verdict and reasons of section 2.2, the cutoff within the fixture tolerance (± 200 Hz), the exact sample counts (durationMs, packet count, decoded samples per channel) and the declared bitrate within 1 kbps of its target (`opus.feature`, "Opus fixtures encoded on the scalar path keep their truth"). If `rusty-opus` 1.0.2 does not read that variable, or a truth changes, the test stage stops and reports it (G1).
  - **No test asserts fixture bytes against a stored hash** (or against the bytes of another ISA level), so a machine or CI runner with another SIMD level gets the same verdicts.

### 1.4 Ogg writer (test support, RFC 3533 and RFC 7845)

The writer turns a list of packets and header fields into an Ogg Opus file. It **shares no code with the adapter's Ogg reader** (`lossy-source.md` section 2.4), not even the CRC table, so a reader bug cannot be hidden by a matching writer bug (ADR-0047 point 3; `opus.feature`, "The Ogg writer of the tests is not the adapter's reader").

- **Pages.** Page header: `OggS`, stream structure version 0, header type flags (`0x01` continued packet, `0x02` beginning of stream, `0x04` end of stream), granule position (signed 64-bit, little-endian), serial number `0x5446_4B46` (fixed), page sequence number from 0, CRC-32, number of segments, segment table. Lacing: a packet of `L` bytes takes `floor(L / 255)` values of 255 and one value `L mod 255` (0 when `L` is a multiple of 255). A page holds at most 255 segments; the writer closes a page before it would exceed that.
- **CRC-32.** Polynomial `0x04C11DB7`, initial value 0, no bit reflection, no final XOR, over the whole page with the CRC field set to zero (RFC 3533, `crc_checksum`). Implemented in test support.
- **Header pages.** Page 0: only the OpusHead packet, beginning-of-stream flag, granule 0. OpusHead: `OpusHead`, version 1, channel count, pre-skip (u16 LE), input sample rate (u32 LE; 48000 unless a row says otherwise), output gain (i16 LE, Q7.8 dB; 0 unless a row says otherwise), channel mapping family (0 unless a row says otherwise) and, for a family other than 0, the mapping table (stream count, coupled count, one mapping byte per channel). Page 1: only the OpusTags packet, granule 0: `OpusTags`, vendor length (u32 LE), vendor `fakeflac-test`, user comment count 0.
- **Audio pages** from page 2: **50 packets per page** (1 s of audio at 20 ms), so 30 s is 30 full pages and a last page with 1 packet. The granule position of an audio page is 960 × the number of audio packets completed up to and including that page; the **last page** has the end-of-stream flag and the granule position `pre_skip + N` (the end trim). A page that completes no packet (split mode) has granule −1.
- **Split mode** (fixture `opus_128_fb_split`): the writer laces the audio packets into segments as usual (255-byte segments, then the remainder) and closes an audio page **before the segment that would take its body past 255 bytes**, whatever the packet boundaries. A body is therefore a sum of whole lacing values, at most 255 bytes. A packet left unfinished continues on the next page, which has the continued flag set, and the page that leaves it unfinished ends with a lacing value of 255.
- **Variants.** The fixtures of section 2.2 that are variants are described as changes to the writer's inputs (packets, header fields, page layout) or, for byte-level damage, as edits of the written bytes. After a byte-level edit the writer recomputes the CRC of every page it touched, unless the row says it does not. "Audio page k" counts audio pages from 1 (audio page 1 is page sequence 2). "Audio packet i" counts audio packets from 0.
- **Audio packet bytes.** The writer returns, with the file, the total size of the audio packets (the two header packets and the page headers and segment tables excluded): the value the declared-bitrate step of `opus.feature` compares with.

## 2. Fixtures

### 2.1 MP3 and M4A fixtures

All 44.1 kHz, mono, 30 s unless the row says otherwise. "Edge" is the nominal frequency above which every coefficient is zero, followed by the **coded edge** (`k · bin_hz`, sections 1.1 and 1.2), which is what the decoder produces. The truth is the v5 result with `detection-params.v5.toml` (`lossy-source.md` sections 4 and 5), and, where a row says **v7**, the result with `detection-params.v7.toml` (RF-16, `lossy-source.md` section 11). Every row without a v7 note gives its v5 verdict, reasons and estimate with v7 too: only `mp3_320_48k_lp20k` and `m4a_aac_256_48k_lp20k` are at 48 kHz with an edge inside `[19800, 20200)` Hz, and `mp3_320_48k_short` is an error first. Every row gives its v7 result with `detection-params.v8.toml` too (v8 only adds Opus rows; `opus.feature`, "v8 gives the v7 results on every file that is not Opus"). Cutoff tolerance ± 200 Hz (2 bands) around the nominal edge, and every `s` below uses the coded edge.

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
| `mp3_320_48k_lp20k` | MP3 CBR 320 at **48 kHz**, `Info` + LAME tag, low-pass byte 205 | 20000 (20000) | Real (`s` ≈ 500, known limitation); estimate 320. **v7:** Suspicious, `streamingSampleRate`; no estimate | An MP3 made from Opus |
| `mp3_320_lp20k` | MP3 CBR 320 (44.1 kHz), `Info` + LAME tag, low-pass byte 205 (RF-16) | 20000 (20059) | Real (`s` ≈ 441); estimate 320; the same with v7 | The 44.1 kHz control of `mp3_320_48k_lp20k` |
| `mp3_320_48k_short` | `mp3_320_48k_lp20k`, **2 s** (83 frames) (RF-16) | 20000 (20000) | Error, `tooShort`; the same with v7 | A broken 48 kHz rip is an error, not flagged |
| `mp3_320_low_hf` | MP3 CBR 320, `Info` + LAME tag | 6000 (5972) | Suspicious, `lowHighFrequencyContent`; no estimate | Little treble |
| `mp3_silence` | MP3 CBR 320, every coefficient zero | — | Suspicious, `lowHighFrequencyContent`; no cutoff; no estimate | A silent file |
| `mp3_short` | `mp3_320_lp20k5`, **2 s** | 20500 (20519) | Error, `tooShort` | |
| `mp3_corrupt` | A text file of 4 KiB of ASCII letters saved as `.mp3` (no `0xFF` byte, so no frame sync and no ID3 header) | — | Error, `corruptStream` | A file that is not MP3 |
| `m4a_aac_128_lp16k` | M4A, AAC-LC, 128 kbps | 16000 (15999) | Real (`c_exp` = 15500, `s` ≈ −500); codec `aac`; estimate 128 | A streaming-site AAC 128 copied without re-encoding |
| `m4a_aac_256_lp15k3` | M4A, AAC-LC, 256 kbps | 15300 (15310) | **Fake** (`s` ≈ 3690); estimate 128 (nearest row 15500) | An AAC 256 made from a 128 kbps source |
| `m4a_aac_256_48k_lp20k` | M4A, AAC-LC, 256 kbps at **48 kHz** (RF-16) | 20000 (19992) | Real (`c_exp` = 19000, `s` ≈ −992); estimate 256. **v7:** Suspicious, `streamingSampleRate`; no estimate | An AAC 256 made from Opus |
| `m4a_alac_full_band` | M4A, ALAC, samples of `full_band` (16-bit) | — | Real (lossless rules); codec `alac`; bit depth 16; no declared bitrate; no estimate | ALAC is lossless |
| `m4a_alac_lp16k` | M4A, ALAC, samples of `lp16k` | — | Fake (lossless rules); cutoff ≈ 16 kHz; no estimate | A transcode stored as ALAC |
| `m4a_he_aac` | M4A, AAC writer with object type 5 | 16000 | Error, `unsupportedFormat` | HE-AAC (Q10) |
| `m4a_corrupt` | `m4a_aac_128_lp16k` truncated in the middle of its `moov` box | — | Error, `corruptStream` | |

Bitrate check: at these densities a granule codes about 1.5 bits per coefficient below the edge, so a 128 kbps mono frame at 44.1 kHz (417 or 418 bytes, about 3340 bits) holds two granules of an edge at 17 kHz (about 2 × 670 bits plus 168 bits of header and side information). Edges were chosen at least 200 Hz away from the midpoint between two table rows, so that the estimate does not depend on where inside the ± 200 Hz tolerance `fc` falls.

**Window margin (RF-16).** The three fixtures with a 20000 Hz edge (`mp3_320_48k_lp20k`, coded 20000; `m4a_aac_256_48k_lp20k`, coded 19992; `mp3_320_lp20k`, coded 20059) are judged against the L6a window `[min_cutoff_hz, max_cutoff_hz)` = `[19800, 20200)` of `detection-params.v7.toml` (the 44.1 kHz control only by its rate, which the window never reaches). The ± 200 Hz tolerance touches both bounds, so the margin rests on the grid of `fc`, not on the tolerance: `fs` is a band centre (100 Hz bands, rules.md section 2), so inside the ± 200 Hz band the possible values are 19850, 19950, 20050 and 20150 Hz, all in the window; an `fe`-type value (a multiple of 100) of 19800 would be inside and 20200 outside, but these fixtures have a step of at least 10 dB, so `fc = fs`. The test stage checks that `fc` of the two 48 kHz fixtures is one of the four values (section 3, check 2) and returns to G1 otherwise, instead of a v7 scenario failing; the 44.1 kHz control then differs from `mp3_320_48k_lp20k` only by its sample rate. The same band keeps the v5 truths: for both MP3 fixtures `s` = 20500 − `fc` ≤ 700 < `suspicious_shortfall_hz` (Real with v6), and for the AAC fixture `s` = 19000 − `fc` < 0.

### 2.2 Opus fixtures (RF-14, RF-15, detection v8)

Encoded as in section 1.3 and written as in section 1.4: 48 kHz, mono, 30 s, 20 ms frames, pre-skip 120, standard page layout, unless the row says otherwise. "Edge" is the nominal frequency above which the decoded content stops: the band edge of the row's `max_bandwidth` (20000 Hz for `Fullband`, 12000 Hz for `Superwideband`), or the input's own low-pass when it is lower. The truth is the result with `detection-params.v8.toml` (`lossy-source.md` section 12). Cutoff tolerance ± 200 Hz around the edge. The edges of the encoded fixtures were **measured before G1** with the fixture recipes (`rusty-opus` 1.0.2, `lossy-source.md` section 12.8 F4, F5; re-measured at G1 with the product decoder, whose output limit to [−1, 1] does not act on these signals, F8): `full_band` reports `fc` 20050 Hz at every bitrate from 64 to 320 kbps, mono and stereo (`D` ≈ 26 dB, `H` ≈ 0 dB); mono `lp16k` reports 16050 Hz from 96 kbps (`D` ≈ 33 dB); `Superwideband` reports 12050 Hz (`D` ≈ 26 dB, `H` −62.4 dB). **The lp16k edge survives in mono from 96 kbps but in independent-channel stereo only from 192 kbps** (band folding, `lossy-source.md` section 12.6), so `opus_160_src16k` stays **mono**; a stereo `Fake` fixture would have to be encoded at 192 kbps or more. The test stage still runs the checks of section 3.

**Encoded fixtures**

| Id | Input, encoder | Edge | Truth (v8) | Models |
|---|---|---|---|---|
| `opus_128_fb` | `full_band`, 128 kbps, `Fullband` | 20000 | Real (`c_exp` = 20000, `s` ≈ 0); cutoff ≈ 20.0 kHz; estimate 128; durationMs 30000 | An honest Opus 128 |
| `opus_128_fb_stereo` | `full_band`, **stereo** (seeds `s`, `s + 1`), 128 kbps, `Fullband` | 20000 | Real; 2 channels; estimate 128 | Family 0 stereo |
| `opus_64_fb` | `full_band`, 64 kbps, `Fullband` | 20000 | Real; estimate 64 | A lower table row |
| `opus_160_src16k` | **`lp16k`**, 160 kbps, `Fullband`, **mono** | 16000 (input) | **Fake** (measured `fc` 16050, so `s` = 3950; `D` ≈ 33 dB); cutoff ≈ 16.0 kHz; **no estimate** (`lossy-source.md` section 12.4) | An Opus 160 made from a source low-passed at 16 kHz (an MP3 or AAC 128); mono, because in stereo the edge is hidden below 192 kbps |
| `opus_160_swb` | `full_band`, 160 kbps, **`Superwideband`** | 12000 | Suspicious, `lowHighFrequencyContent` (measured: `fc` 12050, `D` ≈ 26 dB, `H` −62.4 dB, below the −60 dB threshold: L2); cutoff ≈ 12.0 kHz; no estimate | An Opus whose bandwidth the encoder limited |
| `opus_short` | `opus_128_fb`'s settings, **2 s** (101 packets) | 20000 | Error, `tooShort` | |

**Variants of `opus_128_fb`** (the same 1501 packets; only the Ogg layer or the header changes)

| Id | Change | Truth (v8) |
|---|---|---|
| `opus_128_fb_split` | Split mode (section 1.4): page bodies of at most 255 bytes, closed whatever the packet boundaries | As `opus_128_fb` (same decoded samples) |
| `opus_128_fb_preskip` | OpusHead pre-skip **1080** (120 + 960); granule positions unchanged (last granule 1 440 120) | Real; durationMs 29980; the decoded samples of `opus_128_fb` from sample 960 on |
| `opus_128_fb_rate44k` | OpusHead input sample rate 44100 | As `opus_128_fb` (sample rate 48000) |
| `opus_128_fb_gain_m20` | OpusHead output gain **−5120** (−20 dB) | As `opus_128_fb`; every sample × 0.1 |
| `opus_128_fb_gain_m60` | OpusHead output gain **−15360** (−60 dB): the decoded level is about −75 dBFS (the input's RMS, about −15 dBFS, section 1.3, minus 60 dB), below `silence_dbfs` | Suspicious, `lowHighFrequencyContent`; no cutoff; no estimate (a silent track) |
| `opus_not_ogg` | Not a variant: a text file of 4 KiB of ASCII letters saved as `.opus` (no `OggS`) | Error, `corruptStream` (`not an Ogg stream`) |
| `opus_bad_crc` | One bit of the body of audio page 3 flipped; **CRC not recomputed** | Error, `corruptStream` (`Ogg page CRC mismatch`) |
| `opus_page_version` | Stream structure version byte of audio page 2 set to 1 | Error, `corruptStream` (`unsupported Ogg page version`) |
| `opus_truncated_page` | The file cut after half of the body of audio page 5 | Error, `corruptStream` (`truncated Ogg page`) |
| `opus_sequence_gap` | Audio page 4 removed (the later pages keep their sequence numbers) | Error, `corruptStream` (`Ogg page out of sequence`) |
| `opus_no_bos` | Beginning-of-stream flag of page 0 cleared | Error, `corruptStream` (`first Ogg page is not a beginning of stream`) |
| `opus_after_eos` | A copy of the last page appended, with sequence number last + 1 | Error, `corruptStream` (`Ogg page after end of stream`) |
| `opus_continuation` | `opus_128_fb_split` with the continued flag cleared on the first audio page whose body starts inside a packet | Error, `corruptStream` (`Ogg packet continuation mismatch`) |
| `opus_unfinished_packet` | `opus_128_fb_split` cut right after the first audio page that ends inside a packet (a whole page, no end-of-stream flag) | Error, `corruptStream` (`unfinished Ogg packet at end of stream`) |
| `opus_no_head` | OpusHead magic `OpusHeaX` | Error, `corruptStream` (`no OpusHead`) |
| `opus_head_version` | OpusHead version byte `0x10` (major version 1) | Error, `corruptStream` (`invalid OpusHead`) |
| `opus_head_channels` | OpusHead channel count 3 with family 0 | Error, `corruptStream` (`invalid OpusHead`) |
| `opus_head_only` | Only page 0 (the stream ends after OpusHead) | Error, `corruptStream` (`no OpusTags`) |
| `opus_no_tags` | OpusTags magic `OpusTagz` | Error, `corruptStream` (`no OpusTags`) |
| `opus_tags_overrun` | OpusTags vendor length set to the packet's own length (the vendor string would run past the end) | Error, `corruptStream` (`invalid OpusTags`) |
| `opus_tags_shared_page` | Page 1 holds OpusTags **and** audio packet 0 (the audio pages after it shift by one packet) | Error, `corruptStream` (`audio data on the OpusTags page`) |
| `opus_no_audio` | Pages 0 and 1 only, page 1 with the end-of-stream flag | Error, `corruptStream` (`no Opus audio packets`) |
| `opus_granule_decreases` | Granule of audio page 3 set to that of audio page 2 minus 960 | Error, `corruptStream` (`Ogg granule position decreases`) |
| `opus_granule_below_preskip` | Pages 0, 1 and audio page 1 only; audio page 1 with the end-of-stream flag and granule 119 (one below the pre-skip, 120) | Error, `corruptStream` (`granule position below pre-skip`) |
| `opus_granule_beyond_audio` | Granule of the last page set to 1 440 961 (one more than the samples of all packets) | Error, `corruptStream` (`granule position beyond the audio`) |
| `opus_bad_packet` | Audio packet 100 replaced by the two bytes `0xFB 0x00` (TOC code 3 with a frame count of 0, invalid by RFC 6716 section 3.2.5) | Error, `corruptStream` (detail contains `Opus packet 100`) |
| `opus_serial_change` | Serial number of audio page 2 set to the stream's serial + 1 | Error, `unsupportedFormat` (`multiplexed or chained Ogg streams are not supported`) |
| `opus_family1_6ch` | OpusHead channel count 6, family 1, stream count 4, coupled count 2, mapping 0–5 | Error, `unsupportedFormat` (`channel mapping family 1 is not supported`); the packets are never decoded |
| `opus_family255_3ch` | OpusHead channel count 3, family 255, stream count 3, coupled count 0, mapping 0–2 | Error, `unsupportedFormat` (`channel mapping family 255 is not supported`) |

Every error variant breaks exactly one rule of `lossy-source.md` section 2.4.4, and every earlier rule of that list holds for it, so its expected detail does not depend on the order in which the reader runs checks that hold.

## 3. What the test stage confirms first

The MP3 and M4A verdicts above are derived from the writer definitions, not measured; the Opus edges were measured before G1 with the fixture recipes (section 2.2), but the fixtures themselves are written in the test stage. Before deriving the scenario tests, the test stage checks on each fixture, with the default parameters:

1. symphonia decodes it without error (except the error fixtures) and the decoded sample rate, channels and duration (± 1 frame) are the writer's;
2. the measured cutoff (`fc`) is within ± 200 Hz of the edge, and `D` ≥ `fake_min_step_db` for every fixture with an edge between 11 kHz and Nyquist − 400 Hz; for `mp3_320_48k_lp20k` and `m4a_aac_256_48k_lp20k` (RF-16), `fc` is one of 19850, 19950, 20050 or 20150 Hz (section 2, "Window margin");
3. every band level `M(b)` is finite (the decoder's output above the edge is filter-bank leakage, not exact zeros); if a band is exactly zero, the test stage stops and reports it (rules.md defines no floor);
4. `H` ≥ `low_hf_threshold_db` for every fixture with an edge of 16 kHz or more, and below it for `mp3_320_low_hf` and `opus_160_swb`.

For the **Opus fixtures** (section 2.2), check 1 reads "the adapter decodes it with `rusty-opus`" instead of symphonia, with sample rate 48000, the row's channels, the exact durationMs (30000; 29980 for `opus_128_fb_preskip`; 2000 for `opus_short`) and the packet counts of section 1.3 (1501 or 101). Three more checks:

5. the pre-skip of 120 samples matches the encoder's delay (section 1.3): for `opus_128_fb` and `opus_128_fb_stereo`, the decoded samples the adapter keeps (pre-skip and end trim removed, before the mono mix) align with the encoder's input, that is, the peak of their cross-correlation with the input lies at offset 0 within ± 1 sample, per channel (measured before G1: delay 120 samples, `lossy-source.md` section 12.8 F7);
6. the decoded RMS of `opus_128_fb` is within 1 dB of the RMS of the input actually written (about −15 dBFS: the generator's −12 dBFS scaled to a −1 dBFS peak, section 1.3), and the declared bitrate of every encoded fixture except `opus_short` is within 1 kbps of its target (VBR averages over 30 s; `opus_short` is `tooShort` and its 2 s average was not measured; measured before G1 with the fixture recipes: the target plus 0.4–0.8 kbps, `lossy-source.md` section 12.8 F3);
7. each encoded fixture is byte-identical when written twice in one process and when written by a child process of the same test binary with the same environment (determinism on one machine, section 1.3 (a)); the scalar-path truth check (section 1.3 (b)) is a scenario of `opus.feature`. No check compares bytes with a stored hash.

Check 2 for Opus: `fc` within ± 200 Hz of the edge and `D` ≥ `fake_min_step_db` for `opus_128_fb`, `opus_128_fb_stereo`, `opus_64_fb`, `opus_160_src16k` and `opus_160_swb`. Measured before G1 with the same recipes (section 2.2): 20050, 20050, 20050, 16050 and 12050 Hz, `D` ≈ 26, 26, 26, 33 and 26 dB. `opus_160_src16k` is the fixture most at risk: CELT codes the energy of its top band (15.6–20 kHz) as a whole and may fill it with folded energy (it does in stereo below 192 kbps and in mono at 64 kbps); if `fc` is not within ± 200 Hz of 16000 Hz, the test stage stops (G1) instead of adjusting the fixture.

If any check fails, it stops and the spec returns to G1 (as for `sfb21_gaps` in `fixtures.md`, "Verdicts with v4").

## 4. Unicode names and read-only checks

The decode scenarios save fixtures under Unicode names (RNF-09: `Álbum ″Ñandú″/01 – Canción ‘x’.mp3`; for Opus, `01 – Canción ‘x’.opus` and `02 – Ópera «Ça va» 🎵.opus`) and compare size, modification time and content hash before and after the analysis (RNF-05), as the lossless scenarios do.

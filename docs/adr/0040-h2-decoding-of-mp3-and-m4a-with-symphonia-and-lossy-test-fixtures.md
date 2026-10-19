# ADR-0040: H2 decoding of MP3 and M4A with symphonia (features `mp3`, `aac`, `isomp4`, `alac`); stream information read by the adapter; lossy test files written by bitstream writers in test support

- Status: accepted
- Date: 2026-10-19

## Context and rationale

RF-14 (high, H1/H2) asks for MP3, M4A and Opus "decoded without external tools and with the same result contract"; H1 delivered WAV and FLAC. ADR-0030 already chose symphonia for MP3 and M4A (AAC) and a separate decoder for Opus, after a spike. RF-15 (high, H2) needs more than samples from those files: the codec inside an M4A (AAC or ALAC), the declared bitrate, the bitrate mode, and the LAME tag's encoder string and low-pass (`specs/detection/lossy-source.md` section 2).

Forces:
- ADR-0033 lists symphonia with the features `flac`, `wav` and `pcm` only, and says that "adding anything not in this list still needs a new ADR" (risk R11). Enabling codec features adds crates to `Cargo.lock`.
- RNF-06: installer under 20 MB. RNF-03: no network. RNF-05: read-only access.
- CI runs no ffmpeg and no encoder (calibration OQ-4), and ADR-0022 keeps audio out of the repository ("Committing FLAC/MP3 to the repo" was a rejected alternative). Yet the RF-14 and RF-15 scenarios need real MP3 and M4A files with known content, bitrate and tags.
- The test files must have an exactly known spectral edge, bitrate and tag, so that the scenarios can name a verdict and an estimate.

## Decision

1. **symphonia features** (amends the symphonia row of ADR-0033). `crates/fakeflac-audio` enables, on the same `symphonia` 0.6.1 with `default-features = false`, the features `mp3` (MPEG Layer III only; `mp1` and `mp2` stay off), `aac` (AAC-LC), `isomp4` (the MP4 container) and `alac`. ALAC is needed because an M4A may hold ALAC, which is lossless and must be analysed like FLAC (backend.md section 8). The new crates are symphonia's own sub-crates for these formats (expected: `symphonia-bundle-mp3`, `symphonia-codec-aac`, `symphonia-format-isomp4`, `symphonia-codec-alac`), from the same project and under MPL-2.0, which `deny.toml` already allows. No other crate is added. The implementer confirms the feature and crate names against 0.6.1 and records them, with the installer size, in "Notes from implementation" below.
2. **Stream information is read by the adapter itself.** The Xing/Info/VBRI frame and its LAME extension are parsed from the bytes of the first MP3 frame; the M4A codec and object type come from the sample entry and its AudioSpecificConfig; the declared bitrate is computed as `lossy-source.md` section 2 defines. No tag library (`lofty`, `id3`) is added: the fields are a few bytes, and ID3 or iTunes metadata are not needed in H2. The calibration tool already parses the LAME encoder field the same way (`tools/fakeflac-calibrate/src/ffmpeg.rs`).
3. **Not decoded:** HE-AAC (object types 5 and 29) and MP3 layers I/II are `unsupportedFormat` (lossy-source.md Q10).
4. **Opus stays pending.** The scanner keeps skipping `.opus`; its scenarios are written in `specs/pending/opus.feature`, a folder that neither `just trace` nor the test stage reads, so no test is derived from them. After the ADR-0030 spike picks the decoder, a later spec PR moves the file to `specs/features/`.
5. **Lossy test files are written by bitstream writers in test support** (`specs/detection/lossy-fixtures.md`): an MP3 frame writer and an MP4/AAC/ALAC writer that write the **coded spectrum** directly (noise up to a chosen edge, chosen bitrates and tag fields), from a seed, inside the tests. They use only existing dev-dependencies (`rand`, `rand_chacha`). Like the lossless fixtures, they live in `crates/fakeflac-audio/tests/common/` and other crates include the module by path (ADR-0036, "Consequences").
6. **`EXTRACTION_VERSION` stays 2** (`specs/calibration/lossy.md`, C7): the symphonia version does not change, and enabling more formats does not change how FLAC and WAV are decoded.

## Alternatives considered

- **ffmpeg as a sidecar or linked library.** Rejected by ADR-0030 (installer size, external tool).
- **A tag library** (`lofty`) for the LAME/Xing header and MP4 metadata. One more crate, with its own parsing of the same bytes, for fields that are a few bytes long. `docs/architecture/frontend.md` mentions `lofty` for optional artist/album columns; it can be decided then, with its own ADR.
- **Encoder dev-dependencies** for the test files: `mp3lame-encoder` (builds LAME from C source with `cc`, LGPL) and an AAC encoder such as `fdk-aac` (C, Fraunhofer licence). They give real encoder output, but add C builds to every CI job and a licence review for `deny.toml`, and a real encoder applies its own low-pass, so the exact edge of a fixture would depend on the encoder version. Kept as the fallback if the writers prove impractical (the test stage stops and returns to G1; a new ADR would be needed).
- **Committing small encoded fixture files** made once with ffmpeg from the synthetic signals. Simplest to use, but ADR-0022 rejected committing MP3 files, the files would be opaque binaries that nobody can regenerate exactly, and every change of a fixture would be a new binary in the history. Rejected.
- **ffmpeg in CI.** Rejected by calibration OQ-4.
- **Enabling `mp1`/`mp2`.** Rare in music collections; a one-line change later if needed.

## Consequences

- The implementation PR changes `crates/fakeflac-audio/Cargo.toml` (owner approval of the dependency surface, R11) and measures the installer size against RNF-06.
- `AudioFormat::from_extension` maps `mp3` and `m4a`; `opus` stays unmapped until the Opus decision. The protected test `crates/fakeflac-core/tests/detect_rules.rs:869` `supported_extensions` (`from_extension("mp3")` is `None` in H1) changes in the test stage (also listed in ADR-0041).
- symphonia's MP3 seeking is approximate for VBR files without a table of contents. The analysis samples 64 windows spread over the track, so an approximate position does not matter; the time per track is measured (lossy-source.md B11, RNF-01).
- The AAC writer needs ISO/IEC 14496-3's spectral and scale-factor Huffman tables (codebook 1 and the scale-factor book). The test stage verifies them by decoding a fixture with symphonia (lossy-fixtures.md section 3) before deriving the scenario tests.
- The fixtures model the bitstream, not an encoder: LAME's, AAC's and Opus's real behaviour is checked only by the calibration tool on the local corpus (lossy-source.md section 9).
- A future symphonia upgrade still bumps `EXTRACTION_VERSION` (existing rule).

## Notes from implementation

(To be filled in by the implementation PR: confirmed feature and crate names, locked versions, installer size.)

# ADR-0047: Opus decoding with `rusty-opus` (pure Rust, pinned); the Ogg layer is read by the adapter; Opus test files are encoded in test support with the same crate

- Status: accepted
- Date: 2026-11-01 (G1: 2026-11-01, the owner accepted every default of #102)

## Context and rationale

RF-14 lists Opus among the formats decoded "without external tools and with the same result contract". ADR-0030 chose symphonia for WAV, FLAC, MP3 and M4A and left Opus to "a separate decoder chosen in a spike (opus crate/libopus bundled vs a pure-Rust alternative)", because symphonia does not decode Opus. ADR-0040 (point 4) kept `.opus` skipped by the scanner and its scenarios in `specs/pending/opus.feature` until that spike. This ADR records the spike and its choice.

Forces:
- RF-14/RF-15: the decoded samples feed the same spectral pipeline, so they must match the reference decoder (libopus) closely. That holds for quiet content too: a 16-bit output grid would put a quantisation floor into the spectrum.
- RNF-04: a corrupt file is an error track, never a crash. Opus packets are untrusted input.
- RNF-01: a typical track (360 s) must be analysed in under 3 s.
- RNF-06: installer under 20 MB (today 2.7 MB NSIS). RNF-03: no network. RNF-07: Windows x86-64 (MSVC).
- Workspace: `rust-version = "1.85"`, `unsafe_code = "forbid"` (our code), `deny.toml` licence allow-list. ADR-0033 needs an ADR for any new dependency (risk R11).
- Tests: CI runs no ffmpeg and no encoder (calibration OQ-4), and ADR-0022 keeps audio out of the repository. The Opus scenarios need real Opus files with a known edge and bitrate (ADR-0040 point 5 solved this for MP3 and M4A with bitstream writers).

### The spike (2026-11-01)

Run in a throwaway crate under `target/` (not committed), on the development machine (Ryzen 7 5800X, Windows 11, Rust 1.95, release profile of the workspace: `lto`, `codegen-units = 1`, `opt-level = "s"`).

Material: 10 Ogg Opus files encoded by libopus (ffmpeg 8.1) from synthetic 30 s signals. They cover:
- full-band white noise at 64, 128 and 256 kbps;
- 2.5 ms and 60 ms frames;
- pink noise low-passed at 16 kHz from a 44.1 kHz source, at 160 kbps;
- a tonal mono signal at 12 kbps (SILK) and 24 kbps VoIP (hybrid);
- stereo at 32 kbps VoIP (hybrid);
- white noise at −60 dBFS, at 128 kbps.

The reference is libopus's own float decode (`-request_sample_fmt flt`), with pre-skip and end trim. The local corpus holds no `.opus` file.

Each candidate decoded every packet through the same hand-written Ogg demuxer (about 60 lines). The comparison covered sample count, maximum absolute error and SNR against the reference, and time.

Robustness: 72 020 mutated packets per decoder, made from 5 of the files with bit flips, truncation, random bytes and forced code-3 TOCs. Each packet was decoded under `catch_unwind`, counting panics and non-finite samples.

Binary size: a minimal release executable per decoder against an empty one.

| Candidate | Licence | Match with libopus float | 30 s stereo decode | Corrupt packets: panics / NaN | Added size | Notes |
|---|---|---|---|---|---|---|
| **`rusty-opus` 1.0.2** | BSD-3-Clause | 125–139 dB SNR; SILK bit-identical; same length | 44–99 ms | 0 / 0 | +236 KB | MSRV 1.85; zero dependencies; typed errors; semver 1.0; `UNSAFE.md` inventory (unsafe only in SIMD kernels, each with a scalar twin); encoder has `max_bandwidth`; encoder output byte-identical with SIMD and with `RUSTY_OPUS_ISA=scalar` |
| `opus-rs` 0.1.40 | BSD-3-Clause | same as `rusty-opus` | 38–84 ms | 0 / 0 | +234 KB | Upstream of `rusty-opus`; most contributors; 38 releases since 2026-02 (0.1.x API); `&'static str` errors; no public bandwidth limit on the encoder |
| `opus-pure` 0.2.2 | BSD-3-Clause | same | 19–87 ms | 0 / 0 | +223 KB | Fork of `rusty-opus` 0.9.1 with an Ogg layer; MSRV **1.88** (above the workspace's 1.85); 4 releases |
| `opus-decoder` 0.1.1 | MIT OR Apache-2.0 | 16-bit output: 81 dB SNR at normal level, **32 dB** at −60 dBFS; **31.5 dB** (max error 0.22) on the 24 kbps VoIP file | **8.6 s** | not run | not measured | Fixed-point; decoder only |
| `opus` 0.4.0 (`opusic-sys`, libopus in C) | MIT/Apache-2.0, BSD-3-Clause | the reference | not built | not run | not measured | Builds libopus with **cmake** (not installed on the development machine, and needed in every CI job); C code parsing untrusted packets |

Encoder check (for test files): the encoders of `rusty-opus`, `opus-rs` and `opus-pure` produced Ogg Opus files from full-band noise at 128 and 160 kbps, with payloads within 0.5 kbps of the target. libopus decoded all of them with no error. Their band levels matched libopus's own encode of the same signal within 0.5 dB: full band up to about 20 kHz, about −43 dBFS above 19.5 kHz against −29 dBFS at 15–19 kHz. With `max_bandwidth = Superwideband` the content above 12.5 kHz falls to about −48 dBFS, and above 19.5 kHz to −99 dBFS.

## Decision

1. **Decoder: `rusty-opus`, pinned `=1.0.2`**, a normal dependency of `crates/fakeflac-audio` with default features. Every bump needs:
   - a reviewed upstream diff;
   - the Opus scenarios passing;
   - a note in "Notes from implementation" below.

   The adapter always decodes at 48 kHz, which is what Opus streams carry whatever `input_sample_rate` the header records (RFC 7845), into `f32`. It applies the pre-skip, the end trim from the last granule position and the header's output gain.
2. **The Ogg layer is read by the adapter itself**, in a new module of `fakeflac-audio`, as ADR-0040 did for the Xing/LAME and MP4 headers. That covers page parsing with its CRC-32 check, packet reassembly, `OpusHead` and `OpusTags`, and the granule positions that give the duration and the declared bitrate. No `ogg` crate is added: the format is a 27-byte header and a segment table, and the adapter needs the byte counts anyway for the declared bitrate (`lossy-source.md` section 2). A page with a wrong CRC or a broken packet is `Error(corruptStream)`; the spec fixes the exact cases.
3. **Opus test files are encoded in test support with `rusty-opus`'s own encoder** (`crates/fakeflac-audio/tests/common/`), from the seeded synthetic signals, and written to Ogg by a small test-support writer. The writer is not the adapter's reader, so a reader bug is not hidden by a matching writer bug. This is a change to ADR-0040 point 5, for Opus only: no CELT bitstream writer is written. A real encoder is acceptable here because:
   - it adds no dependency (the crate is already a production dependency);
   - its bandwidth is set explicitly (`max_bandwidth`), so a fixture's edge is known;
   - its output does not depend on the CPU's SIMD level. The spike found it byte-identical with and without SIMD, and the test stage confirms it with a test. **Amended by the Opus spec PR (see "Amendment" below): this holds for the spike's stereo case only; what the tests rely on is that every fixture keeps its truth at every SIMD level.**
4. **Scope:** channel mapping family 0 (mono and stereo) is decoded. Other families (multichannel) are `Error(unsupportedFormat)` until a requirement asks for them, and the spec states it. `.opus` is the only Opus extension scanned (`.ogg` and `.oga` may hold Vorbis or FLAC and stay out of scope, as `lossy-source.md` Q14 did for `.mp4`).
5. **`EXTRACTION_VERSION` stays 2**, as in ADR-0040 point 6: adding a format does not change how WAV, FLAC, MP3 or M4A are decoded, and Opus results never existed before. The spec PR confirms or overrides this.

## Alternatives considered

- **libopus in C through `opus` 0.4 / `opusic-sys` (bundled).** This is the reference implementation, but it needs cmake and a C toolchain on every developer machine and in every CI job. It also parses untrusted packets in C, outside the safety the rest of the pipeline has. The pure-Rust candidates match its float output to the level of float rounding (about 126 dB SNR). It stays the fallback if `rusty-opus` is abandoned or a defect appears that upstream does not fix; switching would need a new ADR.
- **`opus-rs`** (upstream of `rusty-opus`). Its decoder is just as good and it has the larger community. Rejected for now:
  - a 0.1.x API that changed 38 times in eight months;
  - untyped errors;
  - no public bandwidth limit on the encoder, so test files would have to filter their content to get an edge;
  - no audited inventory of its `unsafe`.

  Because the two share their API shape, it is the cheapest switch if `rusty-opus` stops being maintained.
- **`opus-pure`.** It has the most defect fixes and a ready Ogg layer, but its MSRV of 1.88 would force the workspace `rust-version` up (its own decision), and it is the youngest (4 releases, one author).
- **`opus-decoder`.** Rejected on measurement:
  - its fixed-point 16-bit output puts a quantisation floor into the spectrum (32 dB SNR on quiet content);
  - it decodes about 170 times slower than the others (8.6 s for 30 s, so about 100 s for a typical track against RNF-01's 3 s);
  - it diverged from libopus on a hybrid VoIP file.
- **The `ogg` crate** for the container. It is well used and has a compatible licence, but it is a second parser for a few dozen bytes per page that the adapter must count anyway.
- **A CELT bitstream writer in test support**, the ADR-0040 way. This means a range coder, PVQ and band energy quantisation written for tests: far more code than the MP3 and AAC writers. It is pointless once a deterministic encoder is in the dependency set.
- **Committing small Opus files made with ffmpeg.** Already rejected by ADR-0022 and ADR-0040.

## Consequences

- `.opus` files can be analysed once the spec PR moves `specs/pending/opus.feature` to `specs/features/`. That PR also:
  - adds the Opus rows of `[lossy.cutoffs]`, which means a new params version and `algorithm_version`;
  - fills in the values marked `<to be specified>`.

  The calibration tool already encodes Opus cascades with ffmpeg's libopus (ADR-0043), so the Opus rows can be calibrated the same way.
- **Supply chain:** one new crate with no transitive dependencies. It is young (first release 2026-07), mostly one maintainer's work, and its README carries an unrelated advertisement. The exact pin and the reviewed diff on every bump are the mitigation, and `cargo deny` keeps checking licence and advisories. Its dev-dependencies, such as its own allocator crate, are not built for FakeFlac. The spike found no network code in its `src/` (RNF-03).
- **Size:** about +240 KB uncompressed in the executable, a few per cent of the 2.7 MB installer (RNF-06 limit: 20 MB).
- **Speed:** a 360 s stereo Opus track decodes in about 0.5–1.2 s on one thread (spike rate: 44–99 ms per 30 s), inside RNF-01's 3 s with the rest of the pipeline. The first implementation may decode the stream sequentially and keep only the sampled windows (ADR-0010). Seeking by granule with the 80 ms pre-roll of RFC 7845 is a lever the RNF-01 rules (performance.md section 7) allow if a budget is missed.
- **Robustness:** all three Rust candidates returned errors, never panics, on 72 020 corrupt packets. `analyze_folder` keeps its `catch_unwind` around each track as the last line of defence (RNF-04).
- Amends ADR-0030 (the decoder is chosen), ADR-0033 (dependency set) and ADR-0040 (point 4: Opus is no longer pending once the spec PR lands; point 5: Opus test files come from the encoder, not a bitstream writer).

## G1 decisions (2026-11-01)

The owner accepted the defaults of the three questions in #102:

1. `rusty-opus` is accepted with its supply-chain profile. The mitigation is the exact pin, a reviewed diff on every bump and `cargo deny`; `opus-rs` is the fallback.
2. Only channel mapping family 0 (mono and stereo) is decoded. Other families are `Error(unsupportedFormat)`, and only `.opus` is scanned.
3. `EXTRACTION_VERSION` stays 2.

## Amendment: measurements before G1 of the Opus spec PR (2026-11-01, accepted at G1 of #103, questions Q30, Q37 and Q42)

This amendment was proposed in the Opus spec PR (#103) and accepted at its G1 (decisions at the end of this section). A throwaway probe (not committed) encoded the fixture recipes with `rusty-opus` 1.0.2 on the development machine (AVX2) and compared them with `RUSTY_OPUS_ISA=scalar`, `sse2` and `avx`. Full figures: `specs/detection/lossy-source.md` section 12.8.

- **Not byte-identical across SIMD levels.** Encodes at one ISA level are byte-identical run to run. The default AVX2 path gives other bytes than the scalar, SSE2 and AVX paths for mono fixtures, both VBR and CBR (11 of 18 recipes). Stereo VBR 160, the spike's case, was identical. The two decodes differ at about 62 dB SNR, because the encoder made different coding decisions, not because of rounding. `fc`, `D` and `H` are the same.
- **Point 3 is amended.** Its last reason now reads: every encoded fixture keeps its **truth** at every SIMD level, meaning:
  - the same verdict, reasons and sample counts;
  - the same cutoff within the fixture tolerance;
  - the same declared bitrate within 1 kbps.

  Its bytes may differ. A test checks this by encoding in a child process with `RUSTY_OPUS_ISA=scalar` (`lossy-fixtures.md` section 1.3). No test compares fixture bytes with a stored hash. The reasons for a real encoder still hold: the truth is what the scenarios assert, and a CI runner without AVX2 would take the scalar or SSE path.
- **Pre-skip.** `rusty-opus` 1.0.2 has no public lookahead getter. Its encoder delay, measured by cross-correlating the decode with the input, is **120 samples** (2.5 ms) in mono and in stereo; libopus's is 312. The Ogg writer in test support therefore writes pre-skip 120 as a constant, and the test stage checks that each fixture decodes aligned with its input. The adapter is unaffected: it applies whatever pre-skip the file's `OpusHead` records.
- **The decoder clamps its output.** `rusty-opus` 1.0.2 limits its `f32` output to [-1, 1] on all three of its output paths (CELT, the CELT sub-frame path and the SILK/hybrid mix); libopus's float decode does not. Loud masters exceed full scale after an Opus round trip: 54 of the 69 corpus sources measured did so at 128 kbps. The clamp's distortion changes the band spectra of those tracks. The first figures of the spec PR were measured with ffmpeg's native, unclamped decoder, and the G1 review re-measured them with the `rusty-opus` decode:
  - Honest 24 and 32 kbps encodes are `Fake` in about 8 % of files (11 of 138).
  - Honest encodes from 48 kbps up are `Fake` in 6 of 552.
  - MP3 128 → Opus 256 is `Fake` in 93 % of files.
- **G1 decisions (owner, 2026-11-01, #103):**
  - The clamp is accepted as product behaviour. The spec states it (`lossy-source.md` section 2.4.3), and its figures are the ones measured with the `rusty-opus` decode.
  - The Opus table starts at 48 kbps (Q30), so lower bitrates are not judged by the shortfall rules.
  - Acceptance O3 allows `Fake` ≤ 2 % (Q37).
  - An unclamped float output, through an upstream request or a separate CELT path, is not pursued now. If it is wanted later, it needs a new ADR and a new `EXTRACTION_VERSION`, because it changes the decoded samples.

## Notes from implementation

(Filled in by the implement stage: confirmed crate version, installer sizes before and after, SIMD/scalar determinism test result.)

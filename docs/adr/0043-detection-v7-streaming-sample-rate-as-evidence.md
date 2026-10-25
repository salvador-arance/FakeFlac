# ADR-0043: Detection v7: a 48 kHz lossy file with a near-full-band edge is `Suspicious` (streaming sample rate as evidence)

- Status: proposed
- Date: 2026-10-25

## Context and rationale

RF-15 (detection v5, ADR-0041) judges a lossy file only by where its spectrum ends: the reported cutoff `fc` against the low-pass `c_exp` that an honest encode at the declared bitrate keeps. `specs/detection/lossy-source.md` section 8 lists **Opus → MP3** as a known limitation. Opus keeps a full-band edge of about 20 kHz at common bitrates, so an MP3 320 made from it has a shortfall `s` of about 500 Hz, below `suspicious_shortfall_hz` (1000 Hz), and rule L6 calls it `Real` with the declared bitrate as estimate.

This is the usual way a streaming-site rip is made. YouTube's best audio stream is Opus, Opus always decodes at 48 kHz, and a ripper that does not resample writes the MP3 or AAC at 48 kHz. A local case (described without names, ADR-0022) shows the gap: nine MP3 CBR 320 files at 48 kHz, most likely rips of an Opus stream, all `Real` with estimate 320 (`fc` = 20 050 Hz, `s` = 450 Hz).

Most music is released at 44.1 kHz, and the calibration data already encodes the same observation (`[labels.upstream_by_rate]` of `calibration.v2.toml`: a 48 kHz rip comes from the Opus stream). The sample rate is a field of every analysed track, so it costs nothing to read. It is not proof: there are legitimate 48 kHz releases, such as video soundtracks, stores that sell 48 kHz files, and the owner's own encodes of 48 kHz masters.

Until now no detection rule has used the sample rate as evidence. This is a new kind of signal, which is why it gets its own ADR (RF-16).

## Decision

1. **Rule L6a** (`lossy-source.md` section 11). After L5 and before L6, a lossy track is `Suspicious` with the new reason **`streamingSampleRate`** and no source-bitrate estimate (null) when all four of these hold:
   - its codec is listed;
   - its decoded sample rate exactly equals one of the listed rates;
   - its declared bitrate is at least a minimum;
   - its reported cutoff `fc` is in a near-full-band window `[min_cutoff_hz, max_cutoff_hz)`.

   It never gives `Fake` and never overrides L0–L5. Compared with v6, the only possible change is `Real → Suspicious` on lossy files. Lossless codecs and Opus files are out of scope.
2. **Data, not code** (ADR-0011, ADR-0025). The thresholds go in an optional `[lossy.streaming]` section of `detection-params.v7.toml`, with `algorithm_version` 7, which becomes the new default. The indicative starting values, not measured, are:
   - `sample_rates_hz = [48000]`
   - `codecs = ["mp3", "aac"]`
   - `min_declared_bitrate_kbps = 192`
   - `min_cutoff_hz = 19500`
   - `max_cutoff_hz = 21000`

   Params files without the section (v5, v6) give exactly their own verdicts. There is no new measurement and no extraction change (`EXTRACTION_VERSION` stays), and the rule stays a pure function in `fakeflac_core::detect`.
3. **Contract.** `commands.md` goes to version 6: the new reason, and `estimatedSourceBitrateKbps` null in this case. `csv_format_version` stays 1.
4. **Acceptance.** K1–K7 in `lossy-source.md` section 11.7. K1 and K2 hold by construction: lossless results are unchanged, and lossy files can only change `Real → Suspicious`. The gates are K3 (generated `opus128_cbr320` caught) and K4 (confirmed 48 kHz fakes caught). K5 and K6 report the false positives per sample rate and where the cutoffs fall.

## Alternatives considered

- **Keep it as a known limitation.** This is the status quo in v5 and v6. The common streaming-rip case stays `Real`, and the owner asked for it to be flagged.
- **MDCT-domain double-compression fingerprints** (H3, `lossy-source.md` section 10). They would catch Opus → MP3 at any sample rate, and close bitrates too, but they need research and a spike before they can be specified. Kept as a candidate for later, not rejected.
- **`Fake` instead of `Suspicious`.** Legitimate 48 kHz releases would be called fake. The sample rate is a hint about where the file came from, not a measurement of lost content.
- **Flag every 48 kHz lossy file**, with no cutoff window. That would flag low-bitrate encodes that already have their own rules, and encodes with no low-pass whose content reaches Nyquist, which is more treble than Opus codes. The window limits the rule to the files L6 would otherwise call `Real`.
- **A tighter upper bound (`max_cutoff_hz` 20300)** would stop flagging honest LAME 320 encodes of 48 kHz masters. It relies on an unverified gap of about 500 Hz between Opus's edge and LAME's 320 kbps low-pass. Left as an open decision at G1 (`lossy-source.md` Q21); K6 reports the figures needed to decide it later.
- **Treat Opus files themselves.** They are natively 48 kHz, so the hint says nothing about them, and they are still `unsupportedFormat` until the decoder spike (ADR-0030).

## Consequences

- A 48 kHz MP3 or AAC rip of an Opus stream at 192 kbps or more becomes `Suspicious` with a reason that names what was seen. The motivating case becomes `Suspicious` with no estimate.
- **False positives are accepted by design.** Genuine 48 kHz releases with a near-full-band edge are flagged as `Suspicious`, and the reason text says why. The owner confirms this at G1 through Q21.
- **What is still not caught:**
  - a rip that the ripper resampled to 44.1 kHz;
  - re-encodes below 192 kbps;
  - low-bitrate Opus sources whose edge falls below `min_cutoff_hz`. These are partly caught already by L3–L5.
- The default `algorithmVersion` becomes 7, so cached app results are recomputed (ADR-0008), and the next free `algorithm_version` becomes 8.
- **Protected tests that change in the test stage** (`lossy-source.md` Q25 (j)):
  - the default-params and `algorithmVersion` checks (v6 → v7, 6 → 7);
  - `crates/fakeflac-core/tests/detect_lossy.rs`, whose 48 kHz "known limitation" test must load a params set without `[lossy.streaming]`;
  - the calibration tests that expect the next free version to be 7.
- `docs/architecture/backend.md` still names an older default version and needs updating with the feature.

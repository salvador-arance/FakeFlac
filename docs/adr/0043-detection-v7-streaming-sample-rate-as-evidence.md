# ADR-0043: Detection v7: a 48 kHz lossy file whose edge is Opus's 20 kHz band edge is `Suspicious` (streaming sample rate as evidence)

- Status: proposed
- Date: 2026-10-25

## Context and rationale

RF-15 (detection v5, ADR-0041) judges a lossy file only by where its spectrum ends: the reported cutoff `fc` against the low-pass `c_exp` that an honest encode at the declared bitrate keeps. `specs/detection/lossy-source.md` section 8 lists **Opus → MP3** as a known limitation. Opus keeps a full-band edge of about 20 kHz at common bitrates, so an MP3 320 made from it has a shortfall `s` of about 450 Hz. That is below `suspicious_shortfall_hz` (1000 Hz), so rule L6 calls it `Real` with the declared bitrate as estimate.

This is the usual way a streaming-site rip is made. YouTube's best audio stream is Opus, Opus always decodes at 48 kHz, and a ripper that does not resample writes the MP3 or AAC at 48 kHz. The owner's corpus holds 152 labelled 48 kHz rips (134 strong and 18 weak labels, all MP3 CBR 320, ADR-0022: aggregates only). Under v6, 149 of them are `Real`: every strong label, with a reported cutoff of 20 050 Hz (77 %) or 20 150 Hz (23 %). The other 3 are weak-labelled files that are `Fake`. The case that started RF-16 is a sample of them: nine such files from one album, all at 20 050 Hz.

Most music is released at 44.1 kHz, and the calibration data already encodes the same observation (`[labels.upstream_by_rate]` of `calibration.v2.toml`: a 48 kHz rip comes from the Opus stream). The sample rate is a field of every analysed track, so it costs nothing to read. It is not proof: there are legitimate 48 kHz releases, such as video soundtracks, stores that sell 48 kHz files, and the owner's own encodes of 48 kHz masters.

Until now no detection rule has used the sample rate as evidence. This is a new kind of signal, which is why it gets its own ADR (RF-16).

**Measured basis** (`lossy-source.md` section 11.3.1). The measurements use FakeFlac's own `fc` (100 Hz bands; a band centre when the step is deep) on three data sets:
- the owner's acceptance run;
- 6 488 controlled cascades from 257 lossless sources (libopus 1.6.1, LAME 3.100 and ffmpeg's native AAC);
- a scan of the owner's library.

The measurements show:
- **Opus re-encoded to MP3 320 reports 19 850–20 150 Hz.** It does so at every libopus bitrate from 24 to 192 kbps (95–100 % of files, almost always 20 050 or 20 150).
- **An honest LAME 320 at 48 kHz reports 20 250–20 450 Hz**, so its cutoff lies above Opus's.
- **Honest LAME 224/256 reports at most 19 750 Hz**, so its cutoff lies below Opus's.

So the evidence is not "near full band" but "the edge sits exactly where Opus puts it". A window of one band on each side of that cluster separates it from both neighbours.

## Decision

1. **Rule L6a** (`lossy-source.md` section 11). After L5 and before L6, a lossy track is `Suspicious` with the new reason **`streamingSampleRate`** and no source-bitrate estimate (null) when all four of these hold:
   - its codec is listed;
   - its decoded sample rate exactly equals one of the listed rates;
   - its declared bitrate is at least a minimum;
   - its reported cutoff `fc` is in a window `[min_cutoff_hz, max_cutoff_hz)` around Opus's 20 kHz band edge.

   It never gives `Fake` and never overrides L0–L5. Compared with v6, the only possible change is `Real → Suspicious` on lossy files. Lossless codecs and Opus files are out of scope.
2. **Data, not code** (ADR-0011, ADR-0025). The thresholds go in an optional `[lossy.streaming]` section of `detection-params.v7.toml`, with `algorithm_version` 7, which becomes the new default. The values are:
   - `sample_rates_hz = [48000]` (indicative)
   - `codecs = ["mp3", "aac"]` (indicative)
   - `min_declared_bitrate_kbps = 192` (indicative)
   - `min_cutoff_hz = 19800` (measured): the midpoint between 19 750, the highest value for LAME 224/256, and 19 850, the lowest common value for Opus.
   - `max_cutoff_hz = 20200` (measured): the midpoint between 20 150, the highest value for Opus, and 20 250, the lowest common value for honest LAME 320 at 48 kHz.

   Both bounds have a margin of ±50 Hz, the most that the 100 Hz grid allows. Params files without the section (v5, v6) give exactly their own verdicts. There is no new measurement and no extraction change (`EXTRACTION_VERSION` stays), and the rule stays a pure function in `fakeflac_core::detect`.
3. **Contract.** `commands.md` goes to version 6: the new reason, and `estimatedSourceBitrateKbps` null in this case. `csv_format_version` stays 1.
4. **Acceptance.** K1–K7 in `lossy-source.md` section 11.7. K1 and K2 hold by construction: lossless results are unchanged, and lossy files can only change `Real → Suspicious`. The gates are:
   - **K3:** generated `opus128_cbr320` caught. Measured with this window: 99.5 % (n = 1 425).
   - **K4:** strong-labelled 48 kHz fakes caught. Measured: 134 of 134.

   K4's labels come from the same sample-rate observation (`upstream=by-rate`), so K4 measures recall only, never false positives. K5 and K6 report the false positives per sample rate and where the cutoffs fall.

## Alternatives considered

- **Keep it as a known limitation.** This is the status quo in v5 and v6. The common streaming-rip case stays `Real`, and the owner asked for it to be flagged.
- **MDCT-domain double-compression fingerprints** (H3, `lossy-source.md` section 10). They would catch Opus → MP3 at any sample rate, and close bitrates too, but they need research and a spike before they can be specified. Kept as a candidate for later, not rejected.
- **`Fake` instead of `Suspicious`.** Legitimate 48 kHz releases would be called fake. The sample rate is a hint about where the file came from, not a measurement of lost content.
- **Flag every 48 kHz lossy file**, with no cutoff window. It would flag honest encodes whose cutoff comes from their own encoder's low-pass (LAME 224/256), and encodes whose content goes past 20.2 kHz, which is more than Opus codes.
- **The first proposed window, [19 500, 21 000).** It catches 100 % of Opus rips, but it also flags:
  - 96–100 % of honest LAME 320 at 48 kHz;
  - 88–91 % of honest LAME 224/256 at 48 kHz.

  Its lower bound admits the encoders' own low-passes, and its upper bound admits content that Opus does not hold. In the owner's library it would flag 506 of the 638 candidate files.
- **[19 500, 20 300).** The upper bound alone still flags 12.5–13.5 % of honest LAME 320 and does nothing for 224/256. It gains at most 0.4 points of recall over 20 200.
- **Treat Opus files themselves.** They are natively 48 kHz, so the hint says nothing about them, and they are still `unsupportedFormat` until the decoder spike (ADR-0030).

## Consequences

- A 48 kHz MP3 or AAC rip of an Opus stream at 192 kbps or more becomes `Suspicious` with a reason that names what was seen. The motivating case becomes `Suspicious` with no estimate.
- **False positives are accepted by design.** About 3–5 % of honest 48 kHz LAME 320 encodes, those whose master itself ends near 20 kHz, cannot be told from an Opus rip and are flagged; the reason text says why. In the owner's library the rule flags 274 of 638 candidate files: the 149 labelled rips plus 125 unlabelled files that only the owner can classify.
- **What is still not caught:**
  - a rip that the ripper resampled to 44.1 kHz, which cannot be told from honest LAME 320 at 44.1 kHz;
  - Opus rips re-encoded to MP3 224/256 or AAC 192, whose cutoff is their own encoder's;
  - re-encodes below 192 kbps.
- **Evidence limits.** The window was read off the owner's corpus and replicated on independent sources, but with one build of each encoder. Other MP3 encoders (FhG), Apple/FDK AAC and the AAC branch on real files (the library has no 48 kHz AAC) are unmeasured.
- The default `algorithmVersion` becomes 7, so cached app results are recomputed (ADR-0008), and the next free `algorithm_version` becomes 8.
- **Protected tests that change in the test stage** (`lossy-source.md` Q25 (j)):
  - the default-params and `algorithmVersion` checks (v6 → v7, 6 → 7) in:
    - `crates/fakeflac-audio/tests/{detection_lossy,detection_lossless,detection_edge_step}.rs`;
    - `crates/fakeflac-core/tests/{detect_rules,params_lossy}.rs`;
    - `src-tauri/tests/run_analysis_lossy_v5.rs`;
  - `crates/fakeflac-core/tests/detect_lossy.rs`, whose `v5()` helper must return the defaults with `lossy.streaming = None`, because the core crate cannot read a file.

  The calibration tests need no change, because they compute the next free version from the files.
- `docs/architecture/backend.md` still names an older default version and needs updating with the feature.

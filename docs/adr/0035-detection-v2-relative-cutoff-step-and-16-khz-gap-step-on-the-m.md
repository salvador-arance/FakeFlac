# ADR-0035: Detection v2: relative cutoff step on the mean spectrum and 16 kHz gap step on the per-frame median spectrum

- Status: accepted
- Date: 2026-10-18

## Context and rationale

Detection v1 (`algorithm_version` 1) misses the most common real case, a FLAC made from a decoded MP3 (RF-04, risk R1). It placed the cutoff where the smoothed mean spectrum fell more than 60 dB below the reference level. A decoded MP3 written at 16 bits still has codec noise and dither above its low-pass. In real files that floor sits only 40–55 dB below the reference level, so v1 treats it as content and reports the cutoff at Nyquist. The synthetic fixtures had digital silence above the low-pass, so tests never saw this.

Evidence (local corpus, ADR-0022; nothing from it is in the repository):

- One album with known truth (16 tracks: 7 transcodes, 9 lossless; checked against a reference tool and by hand on spectrograms). The **Rust v1 code** returns `Real` for all 16, with cutoff = 22,050 Hz.
- 1,865 FLAC tracks analysed with a Python prototype (Hann, N = 4096, 100 Hz bands, mean and per-frame median spectra). Its verdicts flag 289 tracks as lossy. v1 applied to the same spectra gives 82 Fake / 26 Suspicious / **181 Real** for those 289.
- The single most discriminating signal is a level step at ~16 kHz in the **median** spectrum. LAME often leaves scale-factor band 21 (above ~16 kHz at 44.1 and 48 kHz) empty in a frame, even at 320 kbps. The power mean hides this because the frames that do have treble dominate it. The median shows it.

## Decision

1. The audio adapter produces **two band spectra** (bands of `band_hz`, 100 Hz): `M`, the dB of the mean power across frames, and `Q`, the median across frames of the per-frame band dB. Both are computed from `segments × frames_per_segment` sampled frames (ADR-0010 still applies; v2 uses 64 × 4 frames, Hann, N = 4096 at 44.1/48 kHz, 50 % overlap).
2. The **cutoff is found as a relative step** on `M`: the largest drop from the 3 bands below a transition band to the 3 bands above it, minus the slope already present below. Bands where content comes back higher up are excluded. No absolute level is used.
3. A **gap step** `G` at 16 kHz is measured on `Q`, between 15.3–16.0 and 16.0–16.7 kHz, as a median across bands and minus the slope already present below.
4. Verdicts: `Fake` if `G ≥ 6 dB`, or if the cutoff step is ≥ 10 dB within 14.5–20.3 kHz. Weaker evidence gives `Suspicious`, with a new reason `HighBandGaps` (contract change) or the existing `AmbiguousCutoff`. Exact rules: `specs/detection/rules.md`; values: `specs/detection/detection-params.v2.toml`; `algorithm_version` = 2.

Results (the corpus rows use the prototype's spectra; the known-truth album was also run on its real audio with a simulation of the v2 sampling, 16/16 correct):

| Data | v1 | v2 |
|---|---|---|
| Known-truth album, 7 transcodes | 0 Fake, 7 Real | 7 Fake |
| Known-truth album, 9 lossless | 9 Real | 9 Real |
| 289 tracks flagged lossy by the prototype | 82 / 26 / 181 (Fake / Suspicious / Real) | 285 / 2 / 2 |
| 102 tracks the prototype rated doubtful | 1 / 3 / 98 | 0 / 100 / 2 |
| 1,474 tracks the prototype rated OK | 0 / 21 / 1453 | 0 / 4 / 1470 |
| An album that is clearly from MP3, 9 tracks (real audio; Python simulation of both pipelines) | 6 Fake, 1 Suspicious, 2 Real | 9 Fake |

## Alternatives considered

- **Keep v1 and lower `content_drop_db`** (e.g. to 35 dB). Rejected: the floor level depends on the mix level and the bitrate, and a fixed threshold that catches it also catches the natural treble roll-off of dark lossless mixes. The problem is the absolute threshold, not its value.
- **Mean spectrum only (cutoff step without the gap step).** Rejected: 4 of the 7 known transcodes have their clearest evidence in the median gap step. One of them (cutoff step 5 dB) is caught only by it.
- **Find the largest step separately in each frequency range** instead of one global maximum. Rejected after evaluation: it flagged 21 lossless-rated tracks as Suspicious instead of 4. A global maximum at ~21 kHz (an anti-aliasing filter) proves there is content above 20.3 kHz, so a smaller step below it is not a lossy cutoff.
- **Rebound check that includes the first band above the transition band** (as in the prototype). Rejected: on a steep edge that band sits above the mean of the window, so the true edge was skipped and the reported cutoff moved by up to 250 Hz on the synthetic fixtures.
- **Keep the v1 sampling (32 × 2 frames, N = 8192, Blackman-Harris).** Rejected: the median needs more frames, and frames of 186 ms average several MP3 frames together, which blurs the gaps. On the known-truth album, 32 × 2 frames turns one transcode from Fake into Suspicious.

## Consequences

- The `SpectrumAnalyzer` output changes from one average spectrum to the two band spectra; the core rules change accordingly. `SuspicionReason` gains `highBandGaps` (regenerate bindings, `just gen`).
- `algorithm_version` 2 invalidates every cached v1 result (ADR-0008).
- v1 tests (`detect_rules.rs`, `detection_lossless.rs`) derive from the v1 spec and must be rewritten by the test author after G1. `detection-params.v1.toml` stays as a record; the code only loads v2.
- Known limits: the thresholds were calibrated on the same corpus. The only independent check is the 9 lossless tracks of the known-truth album, and the margins are thin: its lossless tracks reach `G` = 3.7 dB and a cutoff step of 6.1 dB on the prototype spectra (up to 3.9 dB and 6.9 dB across the sampling densities tried), against thresholds of 4 and 7. Two of its transcodes (a short intro and one track) move between Fake and Suspicious with the sampling density; with the sampling densities tried, none of the 7 came out Real. Short intros and skits account for a large share of the tracks flagged inside otherwise-real albums, and nobody has verified them. Phase 5 calibration (`just accept`) must check more labelled albums before release.
- The thresholds were evaluated with a Hann window. The window stays a parameter (ADR-0025): a later version may return to a 4-term window if acceptance shows no loss.

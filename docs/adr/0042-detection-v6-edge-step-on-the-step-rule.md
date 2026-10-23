# ADR-0042: Detection v6: the step rule V4 does not apply to a content edge (median step and post-step residual)

- Status: accepted (G1, #60: the direction; the values and Q1–Q7 of the proposal are settled by the follow-up spec PR and the corpus run)
- Date: 2026-10-23

## Context and rationale

W4 (FP4), a purchased lossless track confirmed as a false positive, has been `Fake` since v2 through the step rule V4: `D` = 14.22 dB at `fs` = 19450 Hz, in the lossy range. v4 accepted it as a known limitation (rules.md Q7) because, with the measurements of v2–v4, its step "cannot be told from a cbr256 transcode's", and rules.md section 6 says clearing it "needs a new measurement, not a threshold", naming the continuity of the spectrum across the step as a candidate (RF-04, target T1).

A local analysis of W4's album (aggregates and per-track measurements only, ADR-0022; details in `specs/pending/detection-v6-edge-step.md`) shows what makes the step. A short, narrow-band sound just below 19.45 kHz raises the **mean** spectrum `M` there by 8–12 dB, while the per-frame **median** `Q` is smooth across the step (it falls 0.32 dB). Above the step `M` keeps falling for another 17.5 dB before it reaches the floor near 21 kHz. A low-pass does the opposite: right above it the spectrum is at the floor. The same album bought as 320 kbps MP3 matches the lossless files up to 19.5 kHz, which rules out a lossy source cut at 19.5 kHz. A second track of the album (N1, `Suspicious` by V6, 0.12 dB below V4's threshold) shows the same pattern at 20.1 kHz.

Neither the median step nor the residual separates W4 from transcodes on its own. Many transcodes have a median step ≤ 0, because the quantiser leaves holes below the low-pass, and a few have a large residual. Their conjunction separates W4 and N1 from all 78 transcodes of the album with a low-pass (seven `calibration.v2.toml` settings, 91 transcodes in all).

## Decision (proposed)

1. Two measurements of the existing spectra, defined only when `fs` is: the median step `D_Q` (the `d(i)` formula on `Q` at the band of `fs`) and the post-step residual `R` (the `above` level of `d(i)` minus the median of `M` in an absolute floor window [20900, 22000) Hz).
2. `EdgeStep` = `InRange(fs)` ∧ `D_Q < edge.max_median_step_db` ∧ `R ≥ edge.min_residual_db`. V4 gains "and not `EdgeStep`"; such a track falls through to V5/V6 and is `Suspicious`. `LossyStep`, `FullBand` and every other rule are unchanged, so against v5 the only possible change is `Fake → Suspicious`, and the caught rate is unchanged by construction.
3. Data, not code (ADR-0011): an optional `[edge]` section in `detection-params.v6.toml`, `algorithm_version` 6. Its indicative values are `max_median_step_db` 3 dB and `min_residual_db` 12 dB, decided by `just accept` on the real corpus. Without `[edge]`, files v2–v5 give exactly their own verdicts. No extraction change (`EXTRACTION_VERSION` stays) and no contract change.
4. Acceptance against the v5 column of the same run: W4 not `Fake`; 15/15 confirmed fakes; `primary` 0 `Fake`; only `Fake → Suspicious`; and a gate of at most 0.5 percentage points of `Fake`-rate drop per protected setting.

## Alternatives considered

- **Median step alone** ("a low-pass must also show in `Q`"): many genuine transcodes have `D_Q` ≤ 0 (cbr192 median −2.83 dB on the album), so it would clear transcodes.
- **Residual alone:** two V4-only transcodes of the album reach `R` = 12.05 and 14.24 dB. A threshold above them leaves W4 a 3.2 dB margin and no evidence that the corpus has no others.
- **Ignore an `EdgeStep` everywhere** (V4, V6 and `LossyStep`): it re-enables the full-band guard on V3, and a v2 transcode with `G` = 29.42 dB lost its `Fake`. Rejected.
- **Skip V4 and V6, keep `LossyStep`** (W4 and N1 become `Real`): no low-pass transcode of the album is lost, but a misread low-pass could become `Real`. Left to G1 as Q1 of the proposal; the default is V4 only.
- **A `(U, B)` guard on V4:** already simulated and rejected in rules.md section 6 (it costs up to 19.5 pp of `Fake` rate).

## Consequences

- W4 is expected to become `Suspicious` (`AmbiguousCutoff`) and T1 to pass. N1 and N2 stay `Suspicious`: N2's verdict comes from the gap rule, outside this proposal.
- The calibration tool must report `D_Q`, `R` and `EdgeStep` (a change to `specs/calibration/evaluation.md` in the same spec PR).
- The evidence is one album. The values are indicative and the corpus run decides; if its gates fail, the figures go back to G1 and are not tuned in the implement stage.
- The next free `algorithm_version` becomes 7.

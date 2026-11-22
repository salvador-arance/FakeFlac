# ADR-0052: Detection v9: the reported cutoff of a lossless file is its content extent at a content edge

- Status: proposed (G1 pending, with `specs/detection/rules.md` Q24–Q32 on branch `spec/RF-04`)
- Date: 2026-11-21

## Context and rationale

The owner reported (2026-11-21) a genuine 44.1 kHz FLAC shown as "Cutoff 12.8 kHz · Genuine" while its spectrogram has content up to about 22 kHz. The verdict is right; the cutoff is not (RF-04, RF-06). Since v2 the reported cutoff of every file is the step frequency `fs` when the step `D` reaches `step_suspicious_db` (7 dB), and the content extent `fe` otherwise (rules.md section 2). Here a narrow peak just below 12.75 kHz, present in a few frames only (absent from the per-frame median), makes a 7.4 dB step; the content continues, falling, up to Nyquist (`R` = 28.5 dB above the top floor).

That is the content edge that `EdgeStep` (ADR-0042) recognises for rule V4: a small median step `D_Q`, a large post-step residual `R`, a moderate `D`. `EdgeStep` also requires `fs` in the lossy range, because it was built for V4; for the reported cutoff that range does not matter. On the owner's library (census of 2026-11-15, re-measured with v8 on 2026-11-21, indicative, aggregates only per ADR-0022), 60 lossless tracks report a cutoff below 14.5 kHz (38 `Real`), and the `EdgeStep` conjunction without the range holds for 21 of them (18 `Real`).

The reported cutoff is not read by any lossless rule or evidence item. It is what the user sees and what the outputs carry: the Cutoff column, the detail line and the spectrogram's cutoff line, the census output (`cutoff_hz`, calibration/census.md; rules.md section 9, acceptance R4) and the CSV export (`cutoff_hz` and `cutoff_steepness_db_per_khz`, contracts/csv-export.md; the scenario "A content edge in a real run with Unicode names" of `detection-reported-cutoff.feature`). In v2–v8 the same value (the step cutoff) is also read by the lossy rules (`fc`, rules L3–L6a, lossy-source.md section 3) and by the calibration's source-limited rule (calibration/lossy.md section 4.5). Rules are versioned data (ADR-0011), and the result cache keys results by the whole engine (ADR-0008, ADR-0044).

## Decision

1. A named predicate `ContentEdge` = the `EdgeStep` conjunction without `InRange(fs)` (`D_Q < edge.max_median_step_db`, `R ≥ edge.min_residual_db`, `D < edge.max_step_db`), and `EdgeStep` restated as `ContentEdge ∧ InRange(fs)`, the same predicate as in v6. No new threshold.
2. For **lossless files only**, with the new optional key `edge.content_edge_cutoff = true`: when `D ≥ step_suspicious_db`, `ContentEdge` holds and the verdict was not decided by V6, V7 or V8 (whose reason `ambiguousCutoff` is about that step), the reported cutoff is `fe` with steepness 0, as when there is no step. The v2 rule is named `f_step`.
3. **No verdict, reason, evidence or estimate changes**, for any file: `ContentEdge` and the key are read by no rule. Lossy files keep `fc` = `f_step` exactly as in v8; the calibration's source-limited rule reads `f_step`.
4. Data, not code: `detection-params.v9.toml` = v8 with every value unchanged plus `content_edge_cutoff = true` in `[edge]`, `algorithm_version` 9, the new default. The key is optional (absent = `false`), so v2–v8 files keep their own verdicts, reasons and cutoffs. No extraction change, no contract type change (the contract stays version 9).

## Alternatives considered

- **Apply the content-edge cutoff to every lossless verdict**, `ambiguousCutoff` included: the detail line would read "Full band · Suspicious · Sharp cutoff at a high frequency". Kept as G1 option Q27 (1).
- **No new key and no version bump** (the rule for every params file with `[edge]`): fewer edits, but v6–v8 would no longer reproduce their own cutoffs, one `algorithm_version` would name two outputs, and development builds would keep stale cached cutoffs until cleared. Kept as G1 option Q28 (B).
- **A predicate without the median step** (`R ≥ 15 ∧ D < 16 ∧ fe = nyquist`, about 33 of 60 tracks): covers more tracks, but it is a new, uncalibrated conjunction and no longer contains `EdgeStep`. G1 option Q24 (2).
- **Report `fe` for every step below the lossy range:** a genuine low-pass below 14.5 kHz exists (low-bitrate lossy sources, low-passed masters), and `fe` there would be the extent of the floor, the v1 failure.
- **Apply it to lossy `fc` as well:** changes lossy verdicts and estimates; deferred to a lossy spec with its own acceptance (Q26).
- **Relax V7/V8 with `ContentEdge`:** a verdict change (`Suspicious → Real`) that needs a corpus acceptance; not in v9 (Q25), as Q12 kept V6 for v6.

## Consequences

- The owner's track and the other covered `Real` tracks show "Full band" (or their `fe`) instead of a cutoff inside the treble. 20 of the 38 `Real` tracks below 14.5 kHz stay at `fs` (12 whose median also drops, 8 with `R` below 15, 6 of them from one 96 kHz album): stated limitations (rules.md section 6).
- The default `algorithmVersion` becomes 9: cached app results are recomputed once (ADR-0008, ADR-0044), and the next free `algorithm_version` becomes 10, so `/calibrate` proposals become `detection-params.v10.toml`.
- Approved scenarios and protected tests that name the v8 defaults or `algorithmVersion` 8 change (rules.md Q30), and one row of `fake-evidence.feature` ("gap with an edge step") reports 22050 Hz. `fakeflac_core::detect::cutoff_estimate` keeps the `f_step` rule; the lossless reported cutoff becomes a separate pure core function that also takes the decided rule.
- Acceptance (rules.md section 9) checks the invariant on every fixture and on the corpus (verdict-change table v8 × v9 diagonal) and reports how many cutoffs move on the owner's library.

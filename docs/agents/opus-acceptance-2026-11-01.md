# Opus acceptance results (2026-11-01)

RF-14/RF-15, detection v8, PR #105; criteria: `specs/detection/lossy-source.md` section 12.7.

The owner authorized running the acceptance commands with the existing configured roots. Corpus inputs remain read-only; generation, cache and reports are in `FAKEFLAC_CALIBRATION`. Settings were loaded explicitly as UTF-8. Report-only data below comes from `summary.md` of run `20261101T143541Z`; no corpus names or paths are reproduced.

## Run

```text
just accept --config specs/calibration/calibration.v4.toml --params specs/detection/detection-params.v7.toml --params specs/detection/detection-params.v8.toml
```

- Tool commit: `6510921fcdb4d0b5a08d366f46ab0d332d86dce2`.
- ffmpeg: `8.1-full_build-www.gyan.dev`.
- Exit 0; 18,360 new entries decoded/transcoded, 57,325 cache hits, zero generation failures and zero skipped cache lines.
- Each new Opus entry has 2,040 generated tracks. v7 leaves all 18,360 Opus rows unevaluated, as specified.

## Criteria

| Criterion | Result |
|---|---|
| O1: lossless results unchanged | Pass. The verdict-change matrix is diagonal. Local comparison also confirms equal verdicts, reasons, cutoff and estimate. |
| O2: MP3/AAC results unchanged | Pass. The lossy change matrix is diagonal; equal reasons, cutoff and estimate were also verified locally. |
| O3: honest Opus Fake rate <= 2% per entry | Pass: 0.0-0.7%, excluding source-limited rows. See group table below. |
| O4: caught >= 85%, Fake >= 80% on detectable cascades | Pass: MP3 128 -> Opus 256 caught 92.2%, Fake 90.9%; AAC 128 -> Opus 256 caught 99.2%, Fake 95.0%. |
| O5: other cascades | Reported below; no pass/fail gate. |
| O6: labelled corpus Opus | Not applicable: no labelled corpus track has codec Opus. The `confirmed_fake opus` summary group describes upstream provenance, not the file's codec. |
| O7: synthetic scenarios | Pass: local `just ci` and hosted CI at `6510921`, including all 18 Opus tests and native Tauri tests. |
| O8: release analysis time | Report: synthetic 360 s stereo Opus, one track, 3.9 MB, median/max 630/630 ms per track, zero errors. This is generated audio, not a real-world Opus corpus. |

Across both params files, a local field-by-field comparison confirmed identical non-Opus result fields. No track identities from that comparison are copied here.

## v8 Opus group counts (split all)

These rows are copied from the report's aggregate group table. Rates exclude undetectable/source-limited rows as specified.

| group | Real | Suspicious | Fake | Undetectable | Source-limited | Error | total | Fake rate | caught rate (positives) or FP rate (negatives) | streaming | streaming rate | s median (Hz) | s 5th percentile (Hz) | fc median (Hz) | fc 5th percentile (Hz) | fc 95th percentile (Hz) | median estimate (kbps) | median estimate of Fake (kbps) | estimates at most source bitrate |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| opus64 | 1934 | 19 | 14 | 0 | 73 | 0 | 2040 | 0.7 % | 0.7 % | 0 | 0.0 % | -50.00 | -150.00 | 20050 | 20050 | 20150 | 65.00 |  |  |
| opus96 | 1953 | 7 | 7 | 0 | 73 | 0 | 2040 | 0.4 % | 0.4 % | 0 | 0.0 % | -50.00 | -150.00 | 20050 | 20050 | 20150 | 97.00 |  |  |
| opus128 | 1962 | 1 | 4 | 0 | 73 | 0 | 2040 | 0.2 % | 0.2 % | 0 | 0.0 % | -50.00 | -150.00 | 20050 | 20050 | 20150 | 130.00 |  |  |
| opus160 | 1966 | 1 | 0 | 0 | 73 | 0 | 2040 | 0.0 % | 0.0 % | 0 | 0.0 % | -50.00 | -150.00 | 20050 | 20050 | 20150 | 163.00 |  |  |
| cbr320_opus128 | 1962 | 3 | 2 | 0 | 73 | 0 | 2040 | 0.1 % | 0.1 % | 0 | 0.0 % | -50.00 | -150.00 | 20050 | 20050 | 20150 | 130.00 |  |  |
| cbr128_opus256 | 160 | 26 | 1853 | 1 | 0 | 0 | 2040 | 90.9 % | 92.2 % | 0 | 0.0 % | 3350.00 | -50.00 | 16650 | 16050 | 20050 | 261.00 |  | 0/161 |
| aac128_opus256 | 17 | 84 | 1937 | 2 | 0 | 0 | 2040 | 95.0 % | 99.2 % | 0 | 0.0 % | 2750.00 | 2650.00 | 17250 | 17250 | 17350 | 264.00 |  | 0/18 |
| cbr192_opus256 | 342 | 1680 | 13 | 5 | 0 | 0 | 2040 | 0.6 % | 83.2 % | 0 | 0.0 % | 1350.00 | -150.00 | 18650 | 18550 | 20150 | 262.00 |  | 0/344 |
| cbr128_opus160 | 753 | 43 | 1243 | 1 | 0 | 0 | 2040 | 61.0 % | 63.1 % | 0 | 0.0 % | 3350.00 | -150.00 | 16650 | 16050 | 20150 | 161.00 |  | 26/754 |

For every O3 entry there are 73 source-limited rows. Its reported `s` median/5th percentile is -50/-150 Hz, and `fc` median/5th/95th percentile is 20050/20050/20150 Hz. The Suspicious counts are 19, 7, 1, 1 and 3, over 1967 non-source-limited rows per entry: derived Suspicious rates 1.0%, 0.4%, 0.1%, 0.1% and 0.2%, respectively.

## Existing targets and limitations

The generic lossless acceptance targets remain **T1 PASS (0), T2 FAIL (5), T3 FAIL (8)** with both v7 and v8. Holdout failures are respectively 4 and 3 for T2/T3 in both files. These unchanged reference failures are not Opus regressions and are not marked resolved.

The report warns that its primary false-positive sample is small: 66 tracks from 5 albums, with correlated tracks. Generated Opus validation does not establish real-world Opus performance. Decoder clipping and band folding retain the limitations already accepted in the spec.

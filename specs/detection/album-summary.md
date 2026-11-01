---
id: SPEC-RF-10-album-summary
requirement: RF-10
status: approved
version: 1
---

# Album summary verdict (RF-10)

> Pure domain rule in `fakeflac-core`: given the analyses of the tracks of one folder, produce an `AlbumSummary` with counts and one summary verdict. First feature to run the full agentic cycle (H0 smoke test, step 0.13).

## Inputs and outputs

- Input: the folder path and the list of `TrackAnalysis` whose `info.folder` equals that folder (the direct parent folder of each file; subfolders are separate albums).
- Output: `AlbumSummary { folder, total, real, suspicious, fake, errors, verdict }` (`crates/fakeflac-core/src/domain.rs`).
- Function (in `fakeflac-core`, no I/O): `summarize_album(folder: &Path, tracks: &[TrackAnalysis]) -> Option<AlbumSummary>`.

## Rules

1. `total` is the number of tracks; `real`, `suspicious`, `fake`, `errors` count tracks by verdict. `real + suspicious + fake + errors == total`.
2. **Severity order:** `Fake` > `Suspicious` > `Real`. The summary `verdict` is the most severe verdict among the tracks that were analysed (not `Error`).
3. Tracks with `Verdict::Error` are counted in `errors` but **do not** decide the summary verdict, unless every track is an error: then the summary verdict is `Error`.
4. An empty list returns `None` (no summary for a folder without tracks).
5. The order of the input tracks does not change the result.
6. Tracks whose `info.folder` differs from `folder` are ignored (defensive: they belong to another album).

## Non-goals

- Grouping tracks into folders (done by the caller).
- Any threshold: this rule has no parameters, so it does not touch `DetectionParams`.

## Acceptance

Scenarios in `specs/features/album-summary.feature`, tag `@RF-10`.

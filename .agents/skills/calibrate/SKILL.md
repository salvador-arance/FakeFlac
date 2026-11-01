---
name: calibrate
description: Runs the calibration grid search on the local corpus and shows the summary and the proposed DetectionParams file. Never commits the proposal. Usage - /calibrate [--grid FILE]
---
# /calibrate [--grid FILE]

Spec: `specs/calibration/calibration.md` section 5, `specs/calibration/evaluation.md` section 7, ADR-0036. The tool is `tools/fakeflac-calibrate`. It needs `FAKEFLAC_CORPUS` and `FAKEFLAC_CALIBRATION` (set by the human in `.Codex/settings.local.json`, key `env`; never set them yourself) and ffmpeg for new transcodes.

1. Run `just calibrate` (add `--grid FILE` if the user gave one; the default grid is `specs/calibration/calibrate-grid.v1.toml`).
   - Exit 1 (internal error, a bug in the tool): show the error output, say it is a bug, and stop. Do not retry, do not touch the repository or `FAKEFLAC_CALIBRATION`, and offer to report it. Nothing new was written except cache lines.
   - Exit 2 (input or configuration error) or 3 (ffmpeg): show the error output and stop. Do not retry with other values and do not edit `trusted.txt`, `confirmed.txt` or any file under `FAKEFLAC_CALIBRATION`.
   - Exit 0: the last line of the output gives the report folder `FAKEFLAC_CALIBRATION/reports/<run>/`.
2. Show the user `summary.md` of that folder. You may read `report.md` locally to answer questions, but only text from `summary.md` may be copied into a repository file, a commit or a PR (ADR-0022, calibration.md section 6). Never copy a corpus path, folder name, file name or note from `report.md`, `tracks.csv` or `labels.csv`.
3. Give the proposal's location as `FAKEFLAC_CALIBRATION/reports/<run>/detection-params.vN.toml`. If the output says "no change proposed", say so: the baseline is at least as good, and no file was written. The headline figures of a proposal are the `holdout` ones.
4. **Never** copy the proposal into `specs/`, never commit it and never open a PR. A proposal enters the repository only through a separate spec PR (gate G1), written from `summary.md` by the human's decision.
5. Check that `git status` shows no change in the repository caused by the run. If it does, report it.

---
id: SPEC-CALIBRATION-SAMPLE-CORPUS
requirements: [RP-11, RNF-04, RNF-05, RNF-09]
status: approved
---

# Sample corpus for CI (OQ-4)

> A small corpus built in temporary folders from the synthetic signals of `specs/detection/fixtures.md`, so that the acceptance path runs in CI (`cargo test`) with no ffmpeg and no real music. Scenarios: `specs/features/calibration.feature`. Expected verdicts with `detection-params.v2.toml` (the tool's default, sections 1 to 3) are the v2 truths of `specs/detection/fixtures.md`, which `detection-lossless.feature` checks in "A v2 params file changes only the guarded verdicts"; with `detection-params.v3.toml` and `detection-params.v4.toml`, section 4 ("v3 side by side", "v4 side by side"). The watch id `W1` below belongs to this synthetic corpus only; it is not the real track FP1 of `specs/detection/rules.md`. Lossy rows (RF-15) use a separate **lossy variant** of this corpus (`specs/calibration/lossy.md` section 9); everything below is unchanged by it.

## 1. Fixture transcoder

The tool's library takes the transcoder as a parameter (ADR-0036). The binary always uses ffmpeg. Tests pass a **fixture transcoder**: for a setting with `encoder = "fixture"`, it ignores the source audio and writes the fixture named in `args[0]` as 16-bit FLAC, recording `encoder_version = "fixture"`. The binary rejects `encoder = "fixture"` with exit 2.

| Setting | `args[0]` (fixture written) | `lowpass_hz` | Verdict with v2 |
|---|---|---|---|
| `fx128` | `lp16k_dither` | 16000 | fake |
| `fxv0` | `sfb21_gaps` | 22050 | fake |
| `fx320` | `lp20k_floor` | 20500 | fake |
| `fxweak` | `sfb21_gaps_weak` | 16000 | suspicious (`highBandGaps`) |

Track 5 (`low_hf`, bandwidth extent $\le 16\text{ kHz}$) transcoded to `fx128` (`lowpass_hz = 16000`) has `source.fe <= setting.lowpass_hz`: it is classified as undetectable and excluded from `must_catch` feasibility and cost.

## 2. Files

`FAKEFLAC_CORPUS` and `FAKEFLAC_CALIBRATION` are two fresh temporary folders outside the repository. Names use accents, `Ñ`, `Ø`, a typographic apostrophe `’`, an en dash `–` and a comma (RNF-09; the comma tests CSV quoting). No top-level folder is named like an origin, so the privacy check cannot match an origin name by accident.

| # | Path under `FAKEFLAC_CORPUS` | Fixture | Listed as |
|---|---|---|---|
| 1 | `Compras/Ñandú – Ácido, Vol. 1 (2024)/01 – Intro ’A’.flac` | `full_band` | `tienda` (folder entry) |
| 2 | `Compras/Ñandú – Ácido, Vol. 1 (2024)/02 – Canción.flac` | `hf_bursts` | `tienda`; watch `W1` |
| 3 | `Compras/Ñandú – Ácido, Vol. 1 (2024)/03 – Bonus.flac` | `full_band` | excluded with `!` |
| 4 | `Colección/Álbum Ü/CD1/01 – Uno.flac` | `full_band` | `analisis-limpio` (folder entry) |
| 5 | `Colección/Álbum Ü/CD2/01 – Dos.flac` | `low_hf` | `analisis-limpio` |
| 6 | `Colección/Álbum Ü/folder.jpg` | (bytes of any small file) | ignored: not audio |
| 7 | `Revisión/Ørsted/01 – Tres.wav` | `full_band` (WAV) | `revisado` (folder entry) |
| 8 | `Revisión/Ørsted/02 – Roto.flac` | `corrupt` | `revisado` |
| 9 | `Revisión/Ørsted/03 – Corto.flac` | `short` | `revisado` |
| 10 | `Sueltas/Mixto/05 – Cuatro.flac` | `full_band` | `analisis` (file entry) |
| 11 | `Sueltas/Mixto/06 – Falso.flac` | `lp16k_dither` | `confirmed.txt`: `fake` |

`trusted.txt` (line numbers matter for the error scenarios; `<CORPUS>` is the value of `FAKEFLAC_CORPUS`):

```
1  # Sample trusted list (CI).
2  library_root = <CORPUS>
3
4  Compras\Ñandú – Ácido, Vol. 1 (2024) | tienda | comprado
5  !Compras\Ñandú – Ácido, Vol. 1 (2024)\03 – Bonus.flac | otra edición
6  Colección\Álbum Ü | analisis-limpio
7  Revisión/Ørsted/ | revisado | barras normales y separador final
8  Sueltas\Mixto\05 – Cuatro.flac | analisis
```

`confirmed.txt`:

```
1  library_root = <CORPUS>
2  Sueltas\Mixto\06 – Falso.flac | fake | espectrograma
3  Compras\Ñandú – Ácido, Vol. 1 (2024)\02 – Canción.flac | false-positive | detector antiguo
```

**Config:** `calibration.v1.toml` with `[[generate.settings]]` replaced by the 4 settings of section 1, and `[[targets]]` replaced by `T1` (class `primary`, `Fake`, max 0) and `T2` (setting `fx128`, `Real`, max 0). Scenarios that need it set `split.holdout_fraction` to 0.0 (everything `tune`) or 1.0 (everything `holdout`).

## 3. Expected results with `detection-params.v2.toml`

- **Albums (4):** `Compras/Ñandú – Ácido, Vol. 1 (2024)`, `Colección/Álbum Ü` (CD1 and CD2 merged, OQ-6), `Revisión/Ørsted`, `Sueltas/Mixto`.
- **Sources for transcodes (6):** tracks 1, 2, 4, 5, 7 and 10 (8 and 9 are `Error`, 3 is excluded, 11 is a confirmed fake). Generated entries: 6 × 4 = 24.
- **Labelled rows (33):** 8 negatives (1, 2, 4, 5, 7, 8, 9, 10), 24 generated and 1 confirmed fake.
- **Decodes on a cold run:** 33 (9 corpus files plus 24 generated), the same with one params file or with two that share a fingerprint. On the second run: 0 decoded, 0 transcoded.

**Global confusion matrix** (`all`; evaluation set without track 10, class `reference`):

| truth \ verdict | Real | Suspicious | Fake | Error | total |
|---|---|---|---|---|---|
| real | 4 | 1 | 0 | 2 | 7 |
| fake | 0 | 6 | 19 | 0 | 25 |

**Per setting:** `fx128` 6 Fake · `fxv0` 6 Fake · `fx320` 6 Fake · `fxweak` 6 Suspicious · `confirmed_fake` 1 Fake.

**False positives by origin:** `tienda` 2 tracks, 1 album, 0 Fake, 0 Suspicious · `analisis-limpio` 2 tracks, 1 album, 0 Fake, 1 Suspicious · `revisado` 1 track, 1 album, 0 Fake, 0 Suspicious, 2 Error · `analisis` (reference) 1 track, 0 Fake. **Primary figure:** `0/2 tracks (0.0 %, 95 % CI 0.0–65.8 %), 0/1 albums`, with the small-sample warning.

**Watch list:** `W1` is `Real`. **Targets:** T1 PASS, T2 PASS.

## 4. Candidate parameters

`candidate.toml` = `detection-params.v2.toml` with `algorithm_version` set to the next free version (1 + the highest `algorithm_version` in `specs/detection/`, 9 today; never 3, 4, 5, 6, 7 or 8, which are detection v3, v4, v5, v6, v7 and v8) and `verdict.gap_fake_db = 4.0`. Only rule values change, so it shares v2's fingerprint. With it, the 6 `fxweak` entries (`G` ≈ 5 dB) become `Fake`, and no negative changes (`full_band` and `hf_bursts` have `G < 4` because they are `Real` under v2; `low_hf` is decided by rule V2 first). The fake row becomes 0 / 0 / 25 / 0, and the verdict-change table against v2 has exactly 6 changes, all `Suspicious → Fake` on positives.

### v3 side by side (detection v3, ADR-0038)

With `--params detection-params.v2.toml --params detection-params.v3.toml` (same fingerprint: no extra decode):

- The 6 `fxv0` entries (`sfb21_gaps`: gated content up to Nyquist, so `FullBand`) become `Suspicious` (`highBandGaps`) with v3. Every other labelled row has the same verdict as with v2: `fx128` (`lp16k_dither`), `fx320` (`lp20k_floor`) and the confirmed fake have a stationary floor above their low-pass; `fxweak` is already `Suspicious`; `hf_bursts` (track 2, watch `W1`) is `FullBand` but has no gap or step, so it stays `Real`; `full_band` has `B` < 0.5 dB.
- v3 global fake row: 0 / 12 / 13 / 0 (Real / Suspicious / Fake / Error); the real row is unchanged. The verdict-change table against v2 has exactly 6 changes, all `Fake → Suspicious` on `fxv0` positives.
- In `tracks.csv`, the v2 rows have empty `U`, `B`, `full_band` and full-band margins; every v3 row with spectra has them filled in.

### v4 side by side (detection v4, ADR-0039)

With `--params detection-params.v2.toml --params detection-params.v3.toml --params detection-params.v4.toml` (same fingerprint: no extra decode; v2 is the baseline of every change table). The v4 verdicts below are derived from the fixture definitions in `specs/detection/fixtures.md` ("Verdicts with v4"); the one value they rest on, the cutoff step of `sfb21_gaps` (`D` ≈ 5–7 dB at `fs` ≈ 16 kHz, so at least `max_lossy_step_db` = 3 dB inside the lossy range), is confirmed by the test stage.

- The 6 `fxv0` entries (`sfb21_gaps`) pass the `U` and `B` thresholds, but their deepest step lies in the lossy range, so they are not `FullBand` with v4 and stay `Fake`, as with v2. Every other labelled row has its v2 (and v3) verdict, for the reasons given in the v3 subsection above.
- v4 global fake row: 0 / 6 / 19 / 0 (Real / Suspicious / Fake / Error), the same as v2; the real row is unchanged. The verdict-change table of v4 against v2 has no change; the one of v3 against v2 still has its 6 `Fake → Suspicious` on `fxv0`.
- v4 per-setting `FullBand` count: 0 on every setting (`fxv0` by its lossy step; `fx128`, `fx320` and the confirmed fake by their stationary floor; `fxweak` by `B` ≈ 2.4 dB). `B` statistics are reported as with v3.
- Watch `W1` (`hf_bursts`) is `FullBand` with v3 and v4 (no cutoff step of 3 dB in the lossy range: its gate's edge, 8 kHz, is below the step search range) and `Real` with all three files.
- In `tracks.csv`, `m_full_band_step_db` (evaluation.md section 8) is empty on the v2 and v3 rows; on the v4 rows it is filled where `fs` lies in the lossy range, and it is at most 0 on the `fxv0` rows.

## 5. Grid search

Grid: baseline `detection-params.v2.toml`, `must_catch = ["fx128"]`, default weights, `[values] "verdict.gap_fake_db" = [4.0, 6.0]`.

- With `holdout_fraction = 0.0` (everything `tune`): the baseline costs `0.5 · 1 + 0.1 · 6 = 1.1` (1 Suspicious negative, 6 Suspicious positives; `Error` rows left out), and the point `gap_fake_db = 4.0` costs `0.5`. Proposal: `detection-params.v9.toml` with `algorithm_version = 9` (the next free version, evaluation.md section 7) and `gap_fake_db = 4.0`, the rest of the baseline text unchanged.
- From the v4 baseline with `"full_band.max_lossy_step_db" = [3.0, 10.0, 12.0]` and `holdout_fraction = 0.0`: 12.0 breaks `max_lossy_step_db ≤ step_fake_db` and is skipped. The baseline costs 1.1, as v2 (same verdicts). With 10.0 the `fxv0` step (≈ 6 dB) no longer removes the guard, the 6 `fxv0` entries become `Suspicious`, and the point costs `1.1 + 0.1 · 6 = 1.7`. Output: "no change proposed", no file written.
- With `holdout_fraction = 1.0` (everything `holdout`): there is no tune data, every point ties with the baseline, and the output is "no change proposed" with no file written. This shows that holdout rows are never used to choose.

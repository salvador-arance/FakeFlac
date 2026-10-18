---
id: SPEC-CALIBRATION-SAMPLE-CORPUS
requirements: [RP-11, RNF-04, RNF-05, RNF-09]
status: proposed
---

# Sample corpus for CI (OQ-4)

> A small corpus built in temporary folders from the synthetic signals of `specs/detection/fixtures.md`, so that the acceptance path runs in CI (`cargo test`) with no ffmpeg and no real music. Scenarios: `specs/features/calibration.feature`. Expected verdicts are those that `specs/features/detection-lossless.feature` already requires with DetectionParams v2.

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

`candidate.toml` = `detection-params.v2.toml` with `algorithm_version = 3` and `verdict.gap_fake_db = 4.0`. Only rule values change, so it shares v2's fingerprint. With it, the 6 `fxweak` entries (`G` ≈ 5 dB) become `Fake`, and no negative changes (`full_band` and `hf_bursts` have `G < 4` because they are `Real` under v2; `low_hf` is decided by rule V2 first). The fake row becomes 0 / 0 / 25 / 0, and the verdict-change table against v2 has exactly 6 changes, all `Suspicious → Fake` on positives.

## 5. Grid search

Grid: baseline `detection-params.v2.toml`, `must_catch = ["fx128"]`, default weights, `[values] "verdict.gap_fake_db" = [4.0, 6.0]`.

- With `holdout_fraction = 0.0` (everything `tune`): the baseline costs `0.5 · 1 + 0.1 · 6 = 1.1` (1 Suspicious negative, 6 Suspicious positives; `Error` rows left out), and the point `gap_fake_db = 4.0` costs `0.5`. Proposal: `detection-params.v3.toml` with `algorithm_version = 3` and `gap_fake_db = 4.0`, the rest of the baseline text unchanged.
- With `holdout_fraction = 1.0` (everything `holdout`): there is no tune data, every point ties with the baseline, and the output is "no change proposed" with no file written. This shows that holdout rows are never used to choose.

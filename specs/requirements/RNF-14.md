---
id: RNF-14
title: An update or a recalibration does not decode the library again
priority: high
milestone: H3
status: draft
---

**Re-analysis cost.** After an app update, or a new `DetectionParams` version, that does not change how audio is decoded and turned into spectra, a rerun on an unchanged library decodes no file: every verdict is recomputed from features stored by an earlier run. Today the result cache (RF-11) stores only the final `TrackAnalysis`, keyed by the whole engine: every `DetectionParams` value, `EXTRACTION_VERSION` and the app version (`specs/contracts/result-cache.md` section 4, Q3; ADR-0044 decision 1). So every release and every recalibration decodes the whole library again, once, even when the spectra it produces are identical.

**Why.** Speed on a large library is the product's main advantage (RNF-01: about 170 ms for the typical track, against a 3000 ms budget). A library of tens of thousands of tracks pays a full cold run after each update, which is exactly when the user reruns it. The decoded features rarely change: the extraction sections (`[input]` and `[fft]`) are identical from `detection-params.v2.toml` to `detection-params.v8.toml`, so the seven recalibrations since v2 would all have reused them. The calibration tool already works this way: its feature cache is keyed by an **extraction fingerprint** (`EXTRACTION_VERSION` plus the `[input]` and `[fft]` sections), so params files that differ only in rule values share it (`specs/calibration/calibration.md` OQ-3, `tools/fakeflac-calibrate/src/cache.rs`). The app's cache does not.

**Direction** (not binding; fixed at G1 of the spec and in an ADR):
- **Two levels.** (1) *Features*: what the audio adapter produces (`AudioProbe`, including `BandSpectra`), keyed by the file (canonical path, size, mtime, as in RF-11) and by the extraction fingerprint of the calibration tool. (2) *Decision*: the verdict, reasons, cutoff, estimate and evidence, computed by `fakeflac-core` from the features and the full `DetectionParams` by the running build. The decision may be cached too, but a miss on the decision never causes a decode.
- **Make the split a type.** Split `DetectionParams` into the values the adapter reads and the rule values, so the compiler decides which values key the features, not a list in a spec. The `SpectrumAnalyzer` port then takes only the extraction part.
- **App version.** Q3 put the app version in the engine so that a bug fix shipped without a new `algorithm_version` cannot leave a stale verdict. With two levels the decision is always recomputed by the running build, so a fix in the core reaches every cached track without a decode. The residual risk is an adapter change shipped without an `EXTRACTION_VERSION` bump. The G1 question is whether to close it with an automatic fingerprint (for example a build-time hash of `crates/fakeflac-audio/src/` and of the locked versions of its decoder dependencies) or to keep the app version in the *features* key only for releases that touch the adapter.

Measurable acceptance (to be fixed at G1 of its spec):
- **No decode after a decision-only change.** On an unchanged folder, a rerun after changing only rule values (for example from `detection-params.v7.toml` to `v8`) or only the app version makes no call to the audio decoder (counted with a spy of the `SpectrumAnalyzer` port), and its results and CSV are byte-identical to a cold run with the new params and the new build.
- **Decode after an extraction change.** Changing any extraction value or `EXTRACTION_VERSION` (or the fingerprint chosen at G1) decodes every file again.
- **Equivalence.** For every fixture and every loadable params file (v2–v8), the analysis computed from stored features equals, bit for bit, the analysis computed from a fresh decode (the `float_roundtrip` rule of ADR-0044 extends to the features).
- **Time.** On a folder of at least 1 000 tracks, a rerun after a decision-only change takes at most a share of the cold run that G1 fixes (indicative: 5 %). It is reported by `just perf` beside T1 and T2.
- **Size.** The stored features per track stay within a budget fixed at G1 (indicative: about 220 bands × 2 spectra at 44.1 kHz, about 480 at 96 kHz), and `just perf` reports bytes per track. The cache keeps only the current feature fingerprint and the current decision engine, as it keeps only the current engine today.
- **Unchanged guarantees.** The cache never changes a verdict and never fails a run (RF-11); the cold-run budgets RNF-01 (10 % bound) and RNF-02 hold with the extra write.

Interactions: RF-12 and RF-20 are decision-only and become free on a cached library. RNF-12 and every RF-19 decoder change the extraction (a bump, one decode). RF-21 adds a new kind of feature: the spec decides whether each extractor has its own version, so a later change to one extractor invalidates only its own features. The spectrogram (RF-07) stays uncached.

Source: comparison with comparable tools of 2026-11-20 (session notes, not in the repository): FakeFlac is the fastest of them on a large library today, and the owner wants to keep that advantage in the finished product. This requirement and the result-cache key are where a release can lose it.

Spec: pending. Amends `specs/contracts/result-cache.md` (sections 3, 4 and 8.1 `ResultStore`, Q3) with an ADR that revisits ADR-0044 decision 1. Pointer note in `requirements/RF-11.md`.

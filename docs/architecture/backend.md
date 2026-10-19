# Backend design (Rust)

> Pipeline: file → technical metadata → N sampled windows → decode → windowed real FFT → band spectra (power mean and per-frame median) → cutoff step and gap step (+ full-band guard from v3, ADR-0038) → rules (`DetectionParams`) → `TrackAnalysis`. Formats: WAV, FLAC (H1); MP3, M4A, Opus (H2). All thresholds, the FFT window and the cutoff→bitrate tables are versioned data.

## 1. Analysis pipeline

```
path → metadata (fs) → select N windows → decode with seek (f32 mono)
     → window + real FFT (N=4096 @ 44.1/48 kHz, v2) → 100 Hz bands per frame
     → mean spectrum (dB of mean power) + median spectrum (median of per-frame dB)
     → cutoff step + gap step (ADR-0035) + full-band guard (U, B above 20.7 kHz; ADR-0038, v3)
     → apply rules (DetectionParams) → TrackAnalysis
```

The full-band guard (detection v3, `specs/detection/rules.md`, ADR-0038) is the default: `DetectionParams::default()` equals `detection-params.v3.toml`. Params files without `[full_band]` (such as v2) still load and reproduce the v2 verdicts.

- Stages after the FFT are pure functions over arrays; tested with synthetic signals (low-passed noise at 16/19/20 kHz, full band, silence).
- Mono for analysis (mean of channels). The spectrogram also uses mono by default.
- Noise baseline estimated from the spectrum itself (median of the 21–22 kHz band, or the highest available band) to mitigate R1.

## 2. Domain model (`fakeflac-core`)

See `crates/fakeflac-core/src/domain.rs`: `Verdict { Real, Suspicious, Fake, Error }`, `AudioFormat { Wav, Flac, Mp3, M4a, Opus }`, `TrackInfo`, `CutoffEstimate`, `SuspicionReason`, `TrackError`, `TrackAnalysis` (with `estimated_source_bitrate_kbps`), `AlbumSummary` (summary verdict = the worst of its tracks; exact rule in `specs/detection/`).

## 3. Ports (traits in `core`)

| Port | Responsibility | Adapter |
|---|---|---|
| `FileScanner` | Recursively list supported files and read `TrackInfo` | `fakeflac-fs` |
| `SpectrumAnalyzer` | `analyze(path, params, cancel)` | `fakeflac-audio` |
| `ResultStore` | `get(key)`, `put(analysis)`, `clear()` | `fakeflac-store` |
| `CsvExporter` | `export(&[TrackAnalysis], dest)` | `fakeflac-export` |
| `Clock` / `Cancel` | Injectable time and cancellation token | `core` / `src-tauri` |

## 4. Use cases

- `AnalyzeFolder(root, params, cancel, progress)`: scan → per track (`workers` std scoped threads, ADR-0034): cache lookup → analyse → store → report progress. A `TrackError` becomes `Verdict::Error` and the batch continues (RNF-04).
- `GetSpectrogram(path)`: on demand, with no use case in `core` and no port method: `src-tauri` calls the inherent `SymphoniaAnalyzer::spectrogram(path, &SpectrogramParams)` through `run::get_spectrogram` (the display STFT is not part of the analysis contract, and a port method would break the `SpectrumAnalyzer` fakes of the protected tests); decoded on a blocking thread; not cached on disk in v1 (measured in phase 5; if it misses the 2 s target, add thumbnails via a new ADR).
- `ExportCsv(results, dest)`; `SummarizeByFolder(results)` (RF-10).

## 5. Concurrency and resources

- `cores − 1` worker threads (chosen by the composition root, ADR-0034), each taking the next track from a shared counter. One track per thread, reused window buffers → bounded memory (RNF-02).
- Cancellation: the token is checked before each track; tracks already running finish, so the batch stops in < 500 ms as long as one track takes less than that. Checking it between windows inside the analyzer (the `cancel` argument of `SpectrumAnalyzer::analyze`) is pending until per-track times are measured (RNF-01).
- A panic while analysing one track is caught and becomes an error track (ADR-0013).
- Read-only: files are opened read-only; no port can write audio (RNF-05).

## 6. Errors

`TrackError { Unreadable, CorruptStream, UnsupportedFormat, TooShort }` are serialisable data that reach the table with their reason; never `panic!` on the user-data path (ADR-0013). Contract-level errors (`AppError`) are separate: `NO_AUDIO_FOUND`, `FOLDER_UNREADABLE`, `ANALYSIS_RUNNING`, `NOT_ANALYZED`, `EXPORT_FAILED`, `INTERNAL`.

## 7. FFT window (ADR-0025, ADR-0026)

**Goal:** detect a drop of tens of dB in a high band where real content is 80–130 dB below the bass. The window must not let leakage from loud low frequencies fill the region above the cutoff. Frequency resolution needed is low (the cliff is hundreds of Hz to kHz wide; one bin at 44.1 kHz with N=4096 is 10.8 Hz, and v2 groups bins in 100 Hz bands).

Selection criteria, in order: (1) sidelobe suppression and decay, (2) cutoff-estimate accuracy (Hz) and steepness, (3) unbiased noise-floor estimate (known ENBW, normalised), (4) cost/simplicity (identical cost: the window is computed once).

| Window | Max sidelobe | Sidelobe decay | Noise bandwidth | Notes |
|---|---|---|---|---|
| Rectangular | −13 dB | 6 dB/oct | 1.0 bin | Rejected: heavy leakage |
| Hann | −31 dB | 18 dB/oct | 1.5 bins | Reference: simple, COLA at 50 % |
| Hamming | −43 dB | 6 dB/oct | 1.36 bins | Slow decay |
| Blackman-Harris 4-term | −92 dB | 6 dB/oct | ≈ 2.0 bins | **Initial hypothesis** |
| Nuttall 4-term | −93 dB | 18 dB/oct | ≈ 2.0 bins | Strong candidate |
| Kaiser (β ≈ 9–12) | ≈ −80…−110 dB | 6 dB/oct | ≈ 1.8–2.2 bins | Tunable |
| Flat-top | ≈ −93 dB | — | ≈ 3.8 bins | Control only |

(Theoretical, approximate values.) The wider main lobe of 4-term windows (≈ 22 Hz) is irrelevant at this scale, but **the choice is measured, not assumed.**

Fixed design decisions:
- The window is an enum in `DetectionParams` (`Hann | BlackmanHarris4 | Nuttall4 | Kaiser(beta)`); changing it bumps `algorithm_version` and invalidates the cache.
- Periodic window, `f64` coefficients computed once per (kind, N) and shared across threads.
- Overlap per window: 50 % for Hann, 75 % for 4-term windows.
- Average in **power** (|X|²) across frames for the mean spectrum, convert to dB at the end; never average dB across frames. The median spectrum takes the median of per-frame dB (the median does not depend on the scale). The step rules compare levels between neighbouring bands of the finished spectrum (`specs/detection/rules.md`).
- Normalise by Σw² (noise power) so the noise floor is comparable across windows, and by coherent gain for tones.
- Real FFT (`realfft`). FFT size by constant resolution: N = nearest power of two to `sample_rate / target_bin_hz` so thresholds hold across sample rates (v1: 5.4 Hz → 8192 at 44.1/48 kHz; v2: 10.8 Hz → 4096 at 44.1/48 kHz and 8192 at 88.2/96 kHz, so that one frame spans few MP3 frames; ADR-0035).
- The spectrogram uses its own window (Hann, N=2048, 75 % overlap) because its goal is visual/temporal (`spectrogram_window` parameter).

Selection protocol (phase 5, planned recipe `just bench-windows`, not written yet): synthetic signals with known truth (ideal low-pass noise at 16/19/20 kHz, slopes 24–120 dB/oct, bass tones at −3 dBFS, 16/24-bit with and without dither) + the real corpus (ADR-0022). Metrics: cutoff error (Hz), false cliffs on real tracks, noise-floor bias, resolvable dynamic range next to a strong tone, final classification, time per track. Rule: the window that minimises cutoff error **without worsening classification** wins; ties go to Hann. The result is recorded in a calibration ADR and in `specs/detection/`.

## 8. Formats and decoding (ADR-0030)

| Format | Nature | Decoder | Milestone | Notes |
|---|---|---|---|---|
| WAV | Lossless | `symphonia` (PCM) | H1 | 16/24/32-bit; analysed like FLAC |
| FLAC | Lossless | `symphonia` | H1 | |
| MP3 | Lossy | `symphonia` (MPEG) | H2 | Also read the Xing/LAME header (encoder, lowpass, bitrate) |
| M4A | Lossy (AAC); may hold ALAC (lossless) | `symphonia` (isomp4 + AAC) | H2 | Codec inside the container is detected; ALAC is treated as lossless |
| Opus | Lossy | Separate decoder (ADR-0030) | H2 | symphonia does not decode Opus; spike first |

All formats go through the same `SpectrumAnalyzer` port: the adapter picks a decoder by format and returns `f32` samples; the rest of the pipeline is common (ADR-0031).

## 9. Provenance of lossy files (RF-15)

**Question:** was this 320 kbps MP3 encoded from a good source or from something with a lower bitrate?

**Main signal (H2): spectral cutoff vs declared bitrate.** Encoders apply a bitrate-dependent low-pass (indicative: ≈ 16 kHz at 128 kbps, ≈ 19 kHz at 192, ≈ 20 kHz at 256, ≈ 20.5 kHz at 320 in LAME; AAC and Opus have other tables). A 320 kbps file whose content stops at 16 kHz betrays a ≈ 128 kbps origin. A **cutoff → source-bitrate table per codec and encoder** is kept versioned in `DetectionParams` and calibrated with the R3 corpus.

**Complementary signals:** encoder metadata (LAME/Xing tag, declared lowpass), per-frame bitrate (VBR/ABR) and anomalous padding, energy in the last scalefactor band (sfb21) of MP3.

**Robust signal (H3): double-compression fingerprint** in the MDCT domain (quantisation structure, frame alignment). Detects cases the cutoff cannot see (e.g. a 128 kbps source re-encoded to 320 with a high low-pass).

**Known limits (shown in the help):** a source with little treble, an encoder without low-pass, or an AAC → MP3 chain can produce false negatives; a genuine source with little treble can produce false positives (R1). These cases become "suspicious" with the reason shown.

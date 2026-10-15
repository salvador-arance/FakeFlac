# fakeflac-audio

Decoding, windowed FFT and spectrum analysis (`docs/architecture/backend.md`, pipeline and FFT window sections).
- Every parameter (window kind, overlap, FFT size, thresholds, cutoff→bitrate tables) comes from `DetectionParams`, never from a constant.
- The stages after the FFT (average spectrum, cutoff estimate, rules) are pure functions over arrays and are tested with synthetic signals.
- Average in power, convert to dB at the end. Normalize by the window's noise power.
- Decoders: `symphonia` for WAV/FLAC/MP3/M4A; Opus is a separate decoder (ADR-030).
- Read-only access to audio files.

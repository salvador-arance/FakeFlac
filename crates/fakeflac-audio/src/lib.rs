//! Audio adapter: decoding (symphonia), windowed real FFT and band spectra (mean and median, RF-03).
//! See `docs/architecture/backend.md` (pipeline, FFT window, formats) and `specs/detection/rules.md`.

mod decode;
mod mp3;
mod mp4;
mod opus;
mod spectrogram;
mod spectrum;
pub mod window;

use fakeflac_core::params::SpectrogramParams;
use fakeflac_core::ports::{AudioProbe, SpectrumAnalyzer};
use fakeflac_core::{DetectionParams, Spectrogram, TrackError};
use std::path::Path;

/// Version of the extraction: bump it in the same PR as any adapter change that alters the `AudioProbe`
/// produced for the same file and parameters, and whenever `symphonia` is upgraded (ADR-0036, OQ-3).
pub const EXTRACTION_VERSION: u32 = 2;

/// `SpectrumAnalyzer` for WAV, FLAC, MP3 and M4A (AAC-LC, ALAC) based on symphonia.
#[derive(Debug, Default, Clone, Copy)]
pub struct SymphoniaAnalyzer;

impl SpectrumAnalyzer for SymphoniaAnalyzer {
    fn analyze(&self, path: &Path, params: &DetectionParams) -> Result<AudioProbe, TrackError> {
        let mut stream = decode::MonoStream::open(path)?;
        let mut info = stream.info;
        let declared = stream.declared.clone();
        // rules.md 1.2: a stream that does not declare its length (FLAC STREAMINFO with
        // total_samples = 0) is decoded once to count its frames, then sampled as usual.
        if matches!(info.total_frames, None | Some(0)) {
            let total = stream.for_each_chunk(|_, _| {})?;
            stream = decode::MonoStream::open(path)?;
            info.total_frames = Some(total);
            // The second pass checks against the counted length, so a file cut between passes is
            // a CorruptStream, not a verdict from partial data.
            stream.info.total_frames = Some(total);
        }
        let n = fft_size(info.sample_rate, params.fft.target_bin_hz);
        let mut collector = spectrum::BlockCollector::new(info.total_frames, n, params);
        let frames = stream.for_each_chunk(|offset, samples| collector.push(offset, samples))?;
        Ok(AudioProbe {
            sample_rate: info.sample_rate,
            bit_depth: info.bit_depth,
            channels: info.channels,
            duration_ms: declared
                .duration_ms
                .unwrap_or(frames * 1000 / u64::from(info.sample_rate.max(1))),
            codec: declared.codec,
            declared_bitrate_kbps: declared.declared_bitrate_kbps,
            bitrate_mode: declared.bitrate_mode,
            encoder: declared.encoder,
            encoder_lowpass_hz: declared.encoder_lowpass_hz,
            spectrum: collector.finish(info.sample_rate, params),
        })
    }
}

/// FFT size for a sample rate: the power of two nearest to `sample_rate / target_bin_hz`.
pub fn fft_size(sample_rate: u32, target_bin_hz: f64) -> usize {
    let target = (f64::from(sample_rate) / target_bin_hz).max(2.0);
    let upper = (target.ceil() as usize).next_power_of_two();
    let lower = (upper / 2).max(1);
    if target - lower as f64 <= upper as f64 - target {
        lower
    } else {
        upper
    }
}

impl SymphoniaAnalyzer {
    /// Spectrogram of the whole file for display (RF-07): mono, `params.window`/`fft_size`/`overlap`,
    /// `params.height` frequency rows, at most `params.max_width` time columns, dB clamped to
    /// `[db_floor, db_ceiling]` and mapped to u8. Not cached (backend.md, GetSpectrogram).
    pub fn spectrogram(
        &self,
        path: &Path,
        params: &SpectrogramParams,
    ) -> Result<Spectrogram, TrackError> {
        spectrogram::compute(path, params)
    }
}

/// Per-channel decoded samples used by the spectrum pipeline (test access for RF-14).
/// The test stage chooses this seam per opus.feature; implementation must reuse
/// its actual decoder path, including pre-skip, trim and output gain.
#[doc(hidden)]
pub fn decode_opus_samples_for_test(path: &Path) -> Result<Vec<Vec<f32>>, TrackError> {
    let mut stream = opus::OpusStream::open(path)?;
    let channels = usize::from(stream.channels);
    let mut samples = vec![Vec::new(); channels];
    stream.for_each_interleaved(|_, pcm| {
        for frame in pcm.chunks_exact(channels) {
            for (channel, sample) in frame.iter().enumerate() {
                samples[channel].push(*sample);
            }
        }
    })?;
    Ok(samples)
}

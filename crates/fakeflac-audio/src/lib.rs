//! Audio adapter: decoding (symphonia), windowed real FFT and band spectra (mean and median, RF-03).
//! See `docs/architecture/backend.md` (pipeline, FFT window, formats) and `specs/detection/rules.md`.

mod decode;
mod spectrum;
pub mod window;

use fakeflac_core::ports::{AudioProbe, SpectrumAnalyzer};
use fakeflac_core::{DetectionParams, TrackError};
use std::path::Path;

/// `SpectrumAnalyzer` for WAV and FLAC based on symphonia.
#[derive(Debug, Default, Clone, Copy)]
pub struct SymphoniaAnalyzer;

impl SpectrumAnalyzer for SymphoniaAnalyzer {
    fn analyze(&self, path: &Path, params: &DetectionParams) -> Result<AudioProbe, TrackError> {
        let mut stream = decode::MonoStream::open(path)?;
        let mut info = stream.info;
        // rules.md 1.2: a stream that does not declare its length (FLAC STREAMINFO with
        // total_samples = 0) is decoded once to count its frames, then sampled as usual.
        if matches!(info.total_frames, None | Some(0)) {
            let total = stream.for_each_chunk(|_, _| {})?;
            stream = decode::MonoStream::open(path)?;
            info.total_frames = Some(total);
        }
        let n = fft_size(info.sample_rate, params.fft.target_bin_hz);
        let mut collector = spectrum::BlockCollector::new(info.total_frames, n, params);
        let frames = stream.for_each_chunk(|offset, samples| collector.push(offset, samples))?;
        Ok(AudioProbe {
            sample_rate: info.sample_rate,
            bit_depth: info.bit_depth,
            channels: info.channels,
            duration_ms: frames * 1000 / u64::from(info.sample_rate.max(1)),
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

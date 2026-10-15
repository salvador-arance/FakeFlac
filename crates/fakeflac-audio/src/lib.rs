//! Audio adapter: decoding (symphonia), windowed real FFT and average spectrum (RF-03).
//! See `docs/architecture/backend.md` (pipeline, FFT window, formats).

pub mod window;

use fakeflac_core::ports::{AudioProbe, SpectrumAnalyzer};
use fakeflac_core::{DetectionParams, TrackError};
use std::path::Path;

/// `SpectrumAnalyzer` for WAV and FLAC based on symphonia.
#[derive(Debug, Default, Clone, Copy)]
pub struct SymphoniaAnalyzer;

impl SpectrumAnalyzer for SymphoniaAnalyzer {
    fn analyze(&self, path: &Path, params: &DetectionParams) -> Result<AudioProbe, TrackError> {
        let _ = (path, params);
        todo!("RF-03")
    }
}

/// FFT size for a sample rate: the power of two nearest to `sample_rate / target_bin_hz`.
pub fn fft_size(sample_rate: u32, target_bin_hz: f64) -> usize {
    let _ = (sample_rate, target_bin_hz);
    todo!("RF-03")
}

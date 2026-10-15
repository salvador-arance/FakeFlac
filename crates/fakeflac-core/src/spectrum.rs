//! Average power spectrum produced by the audio adapter (RF-03), consumed by the pure detection rules.

/// Power-averaged spectrum of a track, in dB, one value per FFT bin from 0 Hz to Nyquist.
#[derive(Debug, Clone, PartialEq)]
pub struct AverageSpectrum {
    pub sample_rate: u32,
    /// Width of one bin in Hz (`sample_rate / fft_size`).
    pub bin_hz: f64,
    /// `fft_size / 2 + 1` values in dB (arbitrary reference; rules work on relative levels).
    pub power_db: Vec<f64>,
}

impl AverageSpectrum {
    pub fn nyquist_hz(&self) -> f64 {
        f64::from(self.sample_rate) / 2.0
    }
}

//! Band spectra produced by the audio adapter (RF-03), consumed by the pure detection rules.
//! Spec: `specs/detection/rules.md`, section 1 (items 7 to 9).

/// Mean and median spectra of a track, grouped in bands of `band_hz` (algorithm_version 2).
///
/// Band `b` covers `[b * band_hz, (b + 1) * band_hz)`; its centre is `(b + 0.5) * band_hz`.
/// The last band is the last full band that ends at or below Nyquist, so both vectors hold
/// `floor(nyquist / band_hz)` values. Levels are in dB with an arbitrary reference (the rules
/// work on levels relative to `L_ref`).
#[derive(Debug, Clone, PartialEq)]
pub struct BandSpectra {
    pub sample_rate: u32,
    /// Width of one band in Hz (`fft.band_hz`).
    pub band_hz: f64,
    /// `M(b)`: `10 * log10` of the band power averaged over all kept frames.
    pub mean_db: Vec<f64>,
    /// `Q(b)`: median over all kept frames of `10 * log10` of the band power of each frame.
    pub median_db: Vec<f64>,
}

impl BandSpectra {
    pub fn nyquist_hz(&self) -> f64 {
        f64::from(self.sample_rate) / 2.0
    }

    /// Centre frequency of band `b`.
    pub fn centre_hz(&self, b: usize) -> f64 {
        (b as f64 + 0.5) * self.band_hz
    }
}

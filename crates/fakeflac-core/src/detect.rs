//! Detection rules v1 for lossless files (RF-03, RF-04). Spec: `specs/detection/rules.md`.
//! Pure functions: no I/O, every number comes from `DetectionParams`.

use crate::domain::{CutoffEstimate, SuspicionReason, Verdict};
use crate::params::DetectionParams;
use crate::spectrum::AverageSpectrum;

/// Measurements of section 2 of the spec, relative to the reference level `L_ref`.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurements {
    pub noise_floor_db: f64,
    pub high_frequency_db: f64,
    pub cutoff_hz: f64,
    pub steepness_db_per_khz: f64,
    pub cliff: bool,
}

/// Outcome of the rules for one analysed (decodable, long enough) track.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub verdict: Verdict,
    pub reasons: Vec<SuspicionReason>,
    pub cutoff: Option<CutoffEstimate>,
}

/// Section 2: measurements on the smoothed spectrum.
pub fn measure(spectrum: &AverageSpectrum, params: &DetectionParams) -> Measurements {
    let _ = (spectrum, params);
    todo!("RF-03")
}

/// Sections 1 (silence) and 3: verdict. `None` means every frame was silent.
pub fn detect(spectrum: Option<&AverageSpectrum>, params: &DetectionParams) -> Detection {
    let _ = (spectrum, params);
    todo!("RF-04")
}

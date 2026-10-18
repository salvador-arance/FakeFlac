//! Detection rules v2 for lossless files (RF-03, RF-04). Spec: `specs/detection/rules.md`.
//! Pure functions: no I/O, every number comes from `DetectionParams`.

use crate::domain::{CutoffEstimate, SuspicionReason, Verdict};
use crate::params::DetectionParams;
use crate::spectrum::BandSpectra;

/// Measurements of section 2 of the spec. Levels are relative to `L_ref`; `gap_db` is a
/// difference of levels of the median spectrum.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurements {
    /// `F`: noise floor (reported, not used to decide).
    pub noise_floor_db: f64,
    /// `H`: high-frequency content.
    pub high_frequency_db: f64,
    /// `fe`: content extent in Hz (Nyquist when the content reaches the top).
    pub extent_hz: f64,
    /// `D`: depth of the deepest cutoff step, 0 when there is none.
    pub step_db: f64,
    /// `fs`: centre of the transition band of that step; `None` when `D` is 0.
    pub step_hz: Option<f64>,
    /// `G`: gap step at `gaps.edge_hz`.
    pub gap_db: f64,
}

/// Outcome of the rules for one analysed (decodable, long enough) track.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub verdict: Verdict,
    pub reasons: Vec<SuspicionReason>,
    pub cutoff: Option<CutoffEstimate>,
}

/// Section 2: measurements on the band spectra.
pub fn measure(spectra: &BandSpectra, params: &DetectionParams) -> Measurements {
    let _ = (spectra, params);
    todo!("RF-03: measurements of rules.md section 2")
}

/// Section 2, "Reported cutoff": `fs` and its steepness when `D >= step_suspicious_db`,
/// otherwise `fe` and 0. `noise_floor_db` is `F`.
pub fn cutoff_estimate(m: &Measurements, params: &DetectionParams) -> CutoffEstimate {
    let _ = (m, params);
    todo!("RF-03: reported cutoff of rules.md section 2")
}

/// Section 3, rows V2 to V10, evaluated in order (V1 is decided before the spectra exist).
pub fn decide(m: &Measurements, params: &DetectionParams) -> (Verdict, Vec<SuspicionReason>) {
    let _ = (m, params);
    todo!("RF-04: verdict table of rules.md section 3")
}

/// Sections 1 (silence) and 3: verdict. `None` means every frame was silent.
pub fn detect(spectra: Option<&BandSpectra>, params: &DetectionParams) -> Detection {
    let _ = (spectra, params);
    todo!("RF-03, RF-04: detection rules v2")
}

//! Use case: analyse one track (RF-03, RF-04). Combines the scanner's `FileEntry`,
//! the analyzer port and the pure detection rules into a `TrackAnalysis`.

use crate::domain::TrackAnalysis;
use crate::params::DetectionParams;
use crate::ports::{FileEntry, SpectrumAnalyzer};

pub fn analyze_track(
    analyzer: &dyn SpectrumAnalyzer,
    entry: &FileEntry,
    params: &DetectionParams,
) -> TrackAnalysis {
    let _ = (analyzer, entry, params);
    todo!("RF-04")
}

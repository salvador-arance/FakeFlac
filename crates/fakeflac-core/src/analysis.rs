//! Use case: analyse one track (RF-03, RF-04). Combines the scanner's `FileEntry`,
//! the analyzer port and the pure detection rules into a `TrackAnalysis`.

use crate::detect::detect;
use crate::domain::{TrackAnalysis, TrackError, TrackInfo, Verdict};
use crate::params::DetectionParams;
use crate::ports::{FileEntry, SpectrumAnalyzer};

pub fn analyze_track(
    analyzer: &dyn SpectrumAnalyzer,
    entry: &FileEntry,
    params: &DetectionParams,
) -> TrackAnalysis {
    let mut info = TrackInfo {
        path: entry.path.clone(),
        folder: entry.folder.clone(),
        format: entry.format,
        sample_rate: 0,
        bit_depth: None,
        channels: 0,
        declared_bitrate_kbps: None,
        duration_ms: 0,
        size_bytes: entry.size_bytes,
        modified_unix: entry.modified_unix,
    };
    let error = |info: TrackInfo, error: TrackError| TrackAnalysis {
        info,
        verdict: Verdict::Error,
        cutoff: None,
        estimated_source_bitrate_kbps: None,
        reasons: Vec::new(),
        error: Some(error),
        algorithm_version: params.algorithm_version,
    };

    let probe = match analyzer.analyze(&entry.path, params) {
        Ok(probe) => probe,
        Err(e) => return error(info, e),
    };
    info.sample_rate = probe.sample_rate;
    info.bit_depth = probe.bit_depth;
    info.channels = probe.channels;
    info.duration_ms = probe.duration_ms;

    if (probe.duration_ms as f64) < params.input.min_duration_s * 1000.0 {
        return error(info, TrackError::TooShort);
    }

    let detection = detect(probe.spectrum.as_ref(), params);
    TrackAnalysis {
        info,
        verdict: detection.verdict,
        cutoff: detection.cutoff,
        estimated_source_bitrate_kbps: None,
        reasons: detection.reasons,
        error: None,
        algorithm_version: params.algorithm_version,
    }
}

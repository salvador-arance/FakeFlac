//! Use case: analyse one track (RF-03, RF-04, RF-15). Combines the scanner's `FileEntry`,
//! the analyzer port and the pure detection rules into a `TrackAnalysis`.

use crate::detect::{detect_track, track_guard, StreamInfo};
use crate::domain::{AudioCodec, TrackAnalysis, TrackError, TrackInfo, Verdict};
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
        codec: AudioCodec::from_format(entry.format),
        sample_rate: 0,
        bit_depth: None,
        channels: 0,
        declared_bitrate_kbps: None,
        bitrate_mode: None,
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
    info.codec = probe.codec.or(info.codec);
    info.declared_bitrate_kbps = probe.declared_bitrate_kbps;
    info.bitrate_mode = probe.bitrate_mode;

    // lossy-source.md section 4: L0, then the too-short and no-bitrate guards (L1b).
    if let Some(e) = track_guard(
        info.codec,
        probe.duration_ms,
        info.declared_bitrate_kbps,
        params,
    ) {
        return error(info, e);
    }

    let stream = StreamInfo {
        // The probe sets the codec of every analysed file; PCM rules are the fallback.
        codec: info.codec.unwrap_or(AudioCodec::Pcm),
        sample_rate: probe.sample_rate,
        declared_bitrate_kbps: info.declared_bitrate_kbps,
        encoder_lowpass_hz: probe.encoder_lowpass_hz,
    };
    let detection = detect_track(probe.spectrum.as_ref(), &stream, params);
    if let Some(e) = detection.error {
        return error(info, e);
    }
    TrackAnalysis {
        info,
        verdict: detection.verdict,
        cutoff: detection.cutoff,
        estimated_source_bitrate_kbps: detection.estimated_source_bitrate_kbps,
        reasons: detection.reasons,
        error: None,
        algorithm_version: params.algorithm_version,
    }
}

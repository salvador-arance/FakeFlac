//! Use-case tests of `specs/features/export-csv.feature` (RF-09), section "Use case (fakeflac-core,
//! fake exporter)". No real audio, no file system. Protected after the `test(RF-09)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_core::ports::CsvExporter;
use fakeflac_core::{
    export_csv, AppError, AppErrorCode, AudioCodec, AudioFormat, TrackAnalysis, TrackInfo, Verdict,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
struct FakeExporter {
    calls: Mutex<Vec<(Vec<TrackAnalysis>, PathBuf)>>,
}

impl CsvExporter for FakeExporter {
    fn export(&self, tracks: &[TrackAnalysis], dest: &Path) -> Result<(), AppError> {
        self.calls
            .lock()
            .unwrap()
            .push((tracks.to_vec(), dest.to_path_buf()));
        Ok(())
    }
}

fn track(name: &str) -> TrackAnalysis {
    let path = PathBuf::from(r"C:\Music\Album A").join(name);
    TrackAnalysis {
        info: TrackInfo {
            folder: PathBuf::from(r"C:\Music\Album A"),
            path,
            format: AudioFormat::Flac,
            codec: Some(AudioCodec::Flac),
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            declared_bitrate_kbps: None,
            bitrate_mode: None,
            duration_ms: 30000,
            size_bytes: 1000,
            modified_unix: 1_700_000_000,
        },
        verdict: Verdict::Real,
        cutoff: None,
        estimated_source_bitrate_kbps: None,
        reasons: vec![],
        error: None,
        algorithm_version: 6,
    }
}

// @RF-09 Scenario: Nothing to export
#[test]
fn nothing_to_export_is_not_analyzed_and_the_exporter_is_not_called() {
    let exporter = FakeExporter::default();
    let err = export_csv(&exporter, &[], Path::new(r"C:\Out\results.csv")).unwrap_err();
    assert_eq!(err.code, AppErrorCode::NotAnalyzed);
    assert!(exporter.calls.lock().unwrap().is_empty());
}

// @RF-09 Scenario Outline: The destination must be an absolute .csv path (accepted rows)
#[test]
fn an_absolute_csv_destination_calls_the_exporter_once_with_the_tracks_in_order() {
    for dest in [r"C:\Out\results.csv", r"C:\Out\RESULTS.CSV"] {
        let exporter = FakeExporter::default();
        let tracks = vec![track("b.flac"), track("a.flac")];
        let rows = export_csv(&exporter, &tracks, Path::new(dest)).expect(dest);
        assert_eq!(rows, 2, "{dest}");
        let calls = exporter.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "{dest}");
        assert_eq!(calls[0].0, tracks, "{dest}: tracks in the order given");
        assert_eq!(calls[0].1, PathBuf::from(dest));
    }
}

// @RF-09 Scenario Outline: The destination must be an absolute .csv path (refused rows)
#[test]
fn a_destination_that_is_not_an_absolute_csv_path_is_export_failed() {
    for dest in [
        r"C:\Out\results.txt",
        r"C:\Out\results",
        r"C:\Out\results.csv.flac",
        r"C:\Music\Album A\lp16k.flac",
        r"out\results.csv",
    ] {
        let exporter = FakeExporter::default();
        let tracks = vec![track("a.flac"), track("b.flac")];
        let err = export_csv(&exporter, &tracks, Path::new(dest)).unwrap_err();
        assert_eq!(err.code, AppErrorCode::ExportFailed, "{dest}");
        assert!(exporter.calls.lock().unwrap().is_empty(), "{dest}");
    }
}

// @RF-09 csv-export.md section 5: the exporter's typed failure reaches the caller unchanged
#[test]
fn an_exporter_failure_is_returned_as_it_is() {
    struct Failing;
    impl CsvExporter for Failing {
        fn export(&self, _: &[TrackAnalysis], _: &Path) -> Result<(), AppError> {
            Err(AppError::new(AppErrorCode::ExportFailed, "cannot write"))
        }
    }
    let err = export_csv(&Failing, &[track("a.flac")], Path::new(r"C:\Out\r.csv")).unwrap_err();
    assert_eq!(err.code, AppErrorCode::ExportFailed);
    assert_eq!(err.message, "cannot write");
}

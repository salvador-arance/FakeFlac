//! `ExportCsv` use case (RF-09, `specs/contracts/csv-export.md` section 5).

use crate::domain::TrackAnalysis;
use crate::error::{AppError, AppErrorCode};
use crate::ports::CsvExporter;
use std::path::Path;

/// Exports `tracks` to `dest` through `exporter` and returns the number of rows written.
/// `NOT_ANALYZED` when `tracks` is empty; `EXPORT_FAILED` when `dest` is not absolute or does not
/// end in `.csv` (case-insensitive). The exporter is not called in those cases.
pub fn export_csv(
    exporter: &dyn CsvExporter,
    tracks: &[TrackAnalysis],
    dest: &Path,
) -> Result<u32, AppError> {
    if tracks.is_empty() {
        return Err(AppError::new(
            AppErrorCode::NotAnalyzed,
            "there are no results to export",
        ));
    }
    if !dest.is_absolute() {
        return Err(AppError::new(
            AppErrorCode::ExportFailed,
            format!("cannot write {}: the path is not absolute", dest.display()),
        ));
    }
    let is_csv = dest
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"));
    if !is_csv {
        return Err(AppError::new(
            AppErrorCode::ExportFailed,
            format!(
                "cannot write {}: the file name must end in .csv",
                dest.display()
            ),
        ));
    }
    exporter.export(tracks, dest)?;
    Ok(u32::try_from(tracks.len()).unwrap_or(u32::MAX))
}

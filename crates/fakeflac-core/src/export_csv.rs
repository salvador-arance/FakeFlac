//! `ExportCsv` use case (RF-09, `specs/contracts/csv-export.md` section 5).

use crate::domain::TrackAnalysis;
use crate::error::AppError;
use crate::ports::CsvExporter;
use std::path::Path;

/// Exports `tracks` to `dest` through `exporter` and returns the number of rows written.
/// `NOT_ANALYZED` when `tracks` is empty; `EXPORT_FAILED` when `dest` is not absolute or does not
/// end in `.csv` (case-insensitive). The exporter is not called in those cases.
pub fn export_csv(
    _exporter: &dyn CsvExporter,
    _tracks: &[TrackAnalysis],
    _dest: &Path,
) -> Result<u32, AppError> {
    todo!("RF-09: ExportCsv use case")
}

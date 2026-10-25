//! See docs/architecture/backend.md. Adapter crate: implements ports defined in fakeflac-core.
//! CSV export (RF-09, `specs/contracts/csv-export.md`).

use fakeflac_core::ports::CsvExporter;
use fakeflac_core::{AppError, TrackAnalysis};
use std::path::Path;

/// The whole CSV document (header and one record per track, in the order given), UTF-8 without
/// BOM, CRLF line ends (csv-export.md sections 1 to 4). Pure: no locale, no I/O.
pub fn build_document(_tracks: &[TrackAnalysis]) -> String {
    todo!("RF-09: CSV document")
}

/// Proposed file name of the save dialog (csv-export.md section 5.1). Pure: reads no file system.
pub fn default_file_name(_root: &Path) -> String {
    todo!("RF-09: default file name")
}

/// Writes the document to the destination through a temporary file (csv-export.md section 6).
#[derive(Debug, Default, Clone, Copy)]
pub struct CsvFileExporter;

impl CsvExporter for CsvFileExporter {
    fn export(&self, _tracks: &[TrackAnalysis], _dest: &Path) -> Result<(), AppError> {
        todo!("RF-09: safe file write")
    }
}

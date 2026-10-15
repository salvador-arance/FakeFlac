//! File-system adapter: recursive scan of supported files (RF-02, RF-14). Read-only (RNF-05).
//! See `docs/architecture/backend.md` and `specs/contracts/commands.md` ("Supported files in H1").

use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_core::AppError;
use std::path::Path;

/// `FileScanner` based on `walkdir` (ADR-0033).
#[derive(Debug, Default, Clone, Copy)]
pub struct WalkDirScanner;

impl FileScanner for WalkDirScanner {
    fn scan(&self, root: &Path) -> Result<Vec<FileEntry>, AppError> {
        let _ = root;
        todo!("RF-02: WalkDirScanner::scan")
    }
}

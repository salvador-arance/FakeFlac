//! File-system adapter: recursive scan of supported files (RF-02, RF-14). Read-only (RNF-05).
//! See `docs/architecture/backend.md` and `specs/contracts/commands.md` ("Supported files in H1").

use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_core::{AppError, AppErrorCode, AudioFormat};
use std::path::Path;
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};

/// `FileScanner` based on `walkdir` (ADR-0033).
#[derive(Debug, Default, Clone, Copy)]
pub struct WalkDirScanner;

impl FileScanner for WalkDirScanner {
    fn scan(&self, root: &Path) -> Result<Vec<FileEntry>, AppError> {
        if !root.is_dir() {
            return Err(AppError::new(
                AppErrorCode::FolderUnreadable,
                format!("cannot read folder {}", root.display()),
            ));
        }
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_hidden(e));
        Ok(walker
            .filter_map(Result::ok)
            .filter_map(|e| to_entry(&e))
            .collect())
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

/// Builds a `FileEntry` for a supported file; `None` for anything else.
fn to_entry(entry: &DirEntry) -> Option<FileEntry> {
    if !entry.file_type().is_file() {
        return None;
    }
    let path = entry.path();
    let format = AudioFormat::from_extension(path.extension()?.to_str()?)?;
    let metadata = entry.metadata().ok()?;
    let modified_unix = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    Some(FileEntry {
        path: path.to_path_buf(),
        folder: path.parent()?.to_path_buf(),
        format,
        size_bytes: metadata.len(),
        modified_unix,
    })
}

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
        check_root(root)?;
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

/// Pre-run checks of `root` (`specs/contracts/commands.md`, `start_analysis` errors).
fn check_root(root: &Path) -> Result<(), AppError> {
    // First, before touching the disk: a relative path would resolve against the process
    // working directory, not the user's.
    if !root.is_absolute() {
        return Err(AppError::new(
            AppErrorCode::FolderUnreadable,
            format!("not an absolute path: {}", root.display()),
        ));
    }
    if root.is_file() {
        return Err(AppError::new(
            AppErrorCode::NotAFolder,
            format!("not a folder: {}", root.display()),
        ));
    }
    // `is_dir` alone is not enough: a folder can exist and still deny listing, which would
    // otherwise end as an empty scan (`NO_AUDIO_FOUND`).
    if !root.is_dir() || std::fs::read_dir(root).is_err() {
        return Err(AppError::new(
            AppErrorCode::FolderUnreadable,
            format!("cannot read folder {}", root.display()),
        ));
    }
    Ok(())
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
        .map_or(0, |t| match t.duration_since(UNIX_EPOCH) {
            Ok(after) => after.as_secs() as i64,
            Err(before) => -(before.duration().as_secs() as i64),
        });
    Some(FileEntry {
        path: path.to_path_buf(),
        folder: path.parent()?.to_path_buf(),
        format,
        size_bytes: metadata.len(),
        modified_unix,
    })
}

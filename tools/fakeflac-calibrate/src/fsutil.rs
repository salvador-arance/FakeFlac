//! Small file-system helpers: atomic writes inside `FAKEFLAC_CALIBRATION`, path display for ffmpeg.

use std::path::{Path, PathBuf};

/// Writes `bytes` to a temporary name in the same folder and renames it over `path`
/// (calibration.md section 3).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Drops the Windows verbatim prefix (`\\?\C:\…` -> `C:\…`) that `canonicalize` adds, for programs
/// that may not accept it. Other paths are returned unchanged.
pub fn plain_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

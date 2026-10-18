//! Environment and containment (calibration.md section 3).

use crate::error::ToolError;
use std::path::{Path, PathBuf};

/// The three variables of calibration.md section 3, plus the repository root. The library never reads
/// the process environment by itself: the binary builds this with [`Env::from_process`].
#[derive(Debug, Clone, Default)]
pub struct Env {
    /// `FAKEFLAC_CORPUS`
    pub corpus: Option<PathBuf>,
    /// `FAKEFLAC_CALIBRATION`
    pub calibration: Option<PathBuf>,
    /// `FAKEFLAC_FFMPEG`
    pub ffmpeg: Option<PathBuf>,
    /// Output of `git rev-parse --show-toplevel`; `None` skips the "not inside the repository" checks.
    pub repo_root: Option<PathBuf>,
}

/// The two validated roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub corpus: PathBuf,
    pub calibration: PathBuf,
}

impl Env {
    /// Reads the variables and runs `git rev-parse --show-toplevel` from the current folder.
    pub fn from_process() -> Env {
        todo!("RP-11: calibration.md section 3")
    }

    /// The checks of calibration.md section 3, before any file is read or written: variables set
    /// (all missing ones named, exit 2), absolute and existing paths, `FAKEFLAC_CALIBRATION` not inside
    /// the repository nor inside `FAKEFLAC_CORPUS`, `FAKEFLAC_CORPUS` not inside `FAKEFLAC_CALIBRATION`.
    pub fn check(&self) -> Result<Roots, ToolError> {
        todo!("RP-11: calibration.md section 3")
    }
}

/// Whether `inner` lies inside `outer` (canonical paths).
pub fn is_inside(_inner: &Path, _outer: &Path) -> bool {
    todo!("RP-11: calibration.md section 3")
}

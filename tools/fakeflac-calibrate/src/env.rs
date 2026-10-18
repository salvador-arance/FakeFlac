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

/// The two validated roots (canonical paths).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub corpus: PathBuf,
    pub calibration: PathBuf,
}

fn variable(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

impl Env {
    /// Reads the variables and runs `git rev-parse --show-toplevel` from the current folder.
    pub fn from_process() -> Env {
        let repo_root = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .stdin(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        Env {
            corpus: variable("FAKEFLAC_CORPUS"),
            calibration: variable("FAKEFLAC_CALIBRATION"),
            ffmpeg: variable("FAKEFLAC_FFMPEG"),
            repo_root,
        }
    }

    /// The checks of calibration.md section 3, before any file is read or written: variables set
    /// (all missing ones named, exit 2), absolute and existing paths, `FAKEFLAC_CALIBRATION` not inside
    /// the repository nor inside `FAKEFLAC_CORPUS`, `FAKEFLAC_CORPUS` not inside `FAKEFLAC_CALIBRATION`.
    pub fn check(&self) -> Result<Roots, ToolError> {
        let mut errors = Vec::new();
        for (name, value) in [
            ("FAKEFLAC_CORPUS", &self.corpus),
            ("FAKEFLAC_CALIBRATION", &self.calibration),
        ] {
            if value.is_none() {
                errors.push(format!(
                    "error: {name} is not set (define it in .claude/settings.local.json, key \"env\")"
                ));
            }
        }
        let (Some(corpus), Some(calibration)) = (&self.corpus, &self.calibration) else {
            return Err(ToolError::input_lines(errors));
        };

        let mut canonical = Vec::new();
        for (name, path) in [
            ("FAKEFLAC_CORPUS", corpus),
            ("FAKEFLAC_CALIBRATION", calibration),
        ] {
            if !path.is_absolute() {
                errors.push(format!("error: {name} must be an absolute path"));
            } else if !path.is_dir() {
                errors.push(format!("error: {name} is not an existing folder"));
            } else {
                match std::fs::canonicalize(path) {
                    Ok(c) => canonical.push(c),
                    Err(_) => errors.push(format!("error: {name} cannot be read")),
                }
            }
        }
        if !errors.is_empty() {
            return Err(ToolError::input_lines(errors));
        }
        let (corpus, calibration) = (canonical[0].clone(), canonical[1].clone());

        if let Some(repo) = &self.repo_root {
            if is_inside(&corpus, repo) {
                errors.push("error: FAKEFLAC_CORPUS is inside the repository".to_string());
            }
            if is_inside(&calibration, repo) {
                errors.push("error: FAKEFLAC_CALIBRATION is inside the repository".to_string());
            }
        }
        if is_inside(&calibration, &corpus) {
            errors.push("error: FAKEFLAC_CALIBRATION is inside FAKEFLAC_CORPUS".to_string());
        }
        if is_inside(&corpus, &calibration) {
            errors.push("error: FAKEFLAC_CORPUS is inside FAKEFLAC_CALIBRATION".to_string());
        }
        if !errors.is_empty() {
            return Err(ToolError::input_lines(errors));
        }
        Ok(Roots {
            corpus,
            calibration,
        })
    }
}

/// Whether `inner` lies inside `outer` (canonical paths; the same folder counts as inside).
pub fn is_inside(inner: &Path, outer: &Path) -> bool {
    let inner = std::fs::canonicalize(inner).unwrap_or_else(|_| inner.to_path_buf());
    let outer = std::fs::canonicalize(outer).unwrap_or_else(|_| outer.to_path_buf());
    inner.starts_with(outer)
}

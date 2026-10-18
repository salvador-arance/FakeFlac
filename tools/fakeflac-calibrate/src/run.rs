//! Commands of the tool (calibration.md section 2) as a library entry point.

use crate::env::Env;
use crate::error::ToolError;
use crate::report::{CacheStats, Evaluation, GridOutcome};
use crate::transcode::Transcoder;
use std::path::PathBuf;

/// `accept [--params FILE]… [--config FILE] [--no-generate]`
#[derive(Debug, Clone)]
pub struct AcceptArgs {
    /// Empty means `specs/detection/detection-params.v2.toml`. The first file is the baseline.
    pub params: Vec<PathBuf>,
    pub config: PathBuf,
    pub no_generate: bool,
}

/// `calibrate [--grid FILE] [--config FILE]`
#[derive(Debug, Clone)]
pub struct CalibrateArgs {
    pub grid: PathBuf,
    pub config: PathBuf,
}

/// `privacy-check [--base REF]`
#[derive(Debug, Clone)]
pub struct PrivacyArgs {
    pub base: String,
}

#[derive(Debug, Clone)]
pub enum Command {
    Accept(AcceptArgs),
    Calibrate(CalibrateArgs),
    PrivacyCheck(PrivacyArgs),
}

/// Everything a run produces besides the files it writes.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub exit_code: i32,
    pub stdout: String,
    /// One error per line (`trusted.txt:6: …`, `error: FAKEFLAC_CORPUS is not set …`).
    pub stderr: String,
    /// `FAKEFLAC_CALIBRATION/reports/<run>/`, when a report was written.
    pub report_dir: Option<PathBuf>,
    /// One per extraction fingerprint used by the run.
    pub cache: Vec<CacheStats>,
    /// One per params file, in order (the baseline of `calibrate` first).
    pub evaluations: Vec<Evaluation>,
    /// `calibrate` only.
    pub grid: Option<GridOutcome>,
}

/// Runs one command. Never panics on bad input: errors become `exit_code` 1, 2 or 3 with their
/// message in `stderr`. The transcoder is only asked for its version or to transcode when a
/// transcode is needed.
pub fn run(_command: &Command, _env: &Env, _transcoder: &dyn Transcoder) -> Outcome {
    todo!("RP-11: calibration.md sections 2 to 4")
}

/// Parses the command line (`accept`, `calibrate`, `privacy-check` and their flags).
pub fn parse_args(_args: &[String]) -> Result<Command, ToolError> {
    todo!("RP-11: calibration.md section 2")
}

/// The binary: arguments from the process, `Env::from_process()`, `FfmpegTranscoder`, prints
/// `stdout`/`stderr` of the outcome, returns the exit code.
pub fn cli_main() -> i32 {
    todo!("RP-11: calibration.md section 2")
}

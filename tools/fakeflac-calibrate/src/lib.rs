//! Calibration and acceptance tool (RP-11, ADR-0036). Dev-only: never shipped in the installer.
//! Specs: `specs/calibration/` (calibration.md, inputs.md, generation.md, evaluation.md, sample-corpus.md).
//!
//! The library holds everything testable without ffmpeg: the binary only wires `FfmpegTranscoder`,
//! the process environment and the command line to [`run::run`]. Tests pass a fixture transcoder.
//! Stage `test`: signatures only, bodies are `todo!()` until the `implement` stage.

pub mod config;
pub mod env;
pub mod error;
pub mod ffmpeg;
pub mod grid;
pub mod hash;
pub mod report;
pub mod run;
pub mod split;
pub mod transcode;

pub use config::Config;
pub use env::Env;
pub use error::{ExitKind, ToolError};
pub use run::{run, AcceptArgs, CalibrateArgs, Command, Outcome, PrivacyArgs};
pub use transcode::{TranscodeError, TranscodeOutput, TranscodeRequest, Transcoder};

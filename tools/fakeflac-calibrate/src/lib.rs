//! Calibration and acceptance tool (RP-11, ADR-0036). Dev-only: never shipped in the installer.
//! Specs: `specs/calibration/` (calibration.md, inputs.md, generation.md, evaluation.md, sample-corpus.md).
//!
//! The library holds everything testable without ffmpeg: the binary only wires `FfmpegTranscoder`,
//! the process environment and the command line to [`run::run`]. Tests pass a fixture transcoder.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod cache;
pub mod config;
pub mod env;
pub mod error;
pub mod evaluate;
pub mod extract;
pub mod ffmpeg;
pub mod fmt;
pub mod fsutil;
pub mod grid;
pub mod hash;
pub mod lists;
pub mod model;
pub mod privacy;
pub mod render;
pub mod report;
pub mod run;
pub mod split;
pub mod transcode;

pub use config::Config;
pub use env::Env;
pub use error::{ExitKind, ToolError};
pub use run::{run, AcceptArgs, CalibrateArgs, Command, Outcome, PrivacyArgs};
pub use transcode::{TranscodeError, TranscodeOutput, TranscodeRequest, Transcoder};

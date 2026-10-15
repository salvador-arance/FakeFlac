//! FakeFlac domain core (hexagonal architecture, ADR-001).
//!
//! This crate holds the domain model, use cases and ports. It performs no I/O
//! and depends on no other workspace crate (see `docs/architecture/dependencies.md`).

pub mod analysis;
pub mod analyze_folder;
pub mod detect;
pub mod domain;
pub mod error;
pub mod params;
pub mod ports;
pub mod spectrum;
pub mod summary;

pub use analyze_folder::{
    analyze_folder, ActiveRun, AnalysisSlot, AnalysisSummary, CancelToken, ProgressEvent,
};
pub use domain::{
    AlbumSummary, AudioFormat, CutoffEstimate, SuspicionReason, TrackAnalysis, TrackError,
    TrackInfo, Verdict,
};
pub use error::{AppError, AppErrorCode};
pub use params::{DetectionParams, WindowKind};
pub use spectrum::AverageSpectrum;
pub use summary::summarize_album;

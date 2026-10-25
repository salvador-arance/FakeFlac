//! FakeFlac domain core (hexagonal architecture, ADR-001).
//!
//! This crate holds the domain model, use cases and ports. It performs no I/O
//! and depends on no other workspace crate (see `docs/architecture/dependencies.md`).

pub mod analysis;
pub mod analyze_folder;
pub mod detect;
pub mod domain;
pub mod error;
pub mod export_csv;
pub mod params;
pub mod ports;
pub mod spectrogram;
pub mod spectrum;
pub mod summary;

pub use analyze_folder::{
    analyze_folder, ActiveRun, AnalysisSlot, AnalysisSummary, CancelToken, ProgressEvent,
};
pub use domain::{
    AlbumSummary, AudioCodec, AudioFormat, BitrateMode, CutoffEstimate, SuspicionReason,
    TrackAnalysis, TrackError, TrackInfo, Verdict,
};
pub use error::{AppError, AppErrorCode};
pub use export_csv::export_csv;
pub use params::{DetectionParams, WindowKind};
pub use spectrogram::Spectrogram;
pub use spectrum::BandSpectra;
pub use summary::summarize_album;

//! FakeFlac domain core (hexagonal architecture, ADR-001).
//!
//! This crate holds the domain model, use cases and ports. It performs no I/O
//! and depends on no other workspace crate (see `docs/architecture/dependencies.md`).

pub mod domain;
pub mod ports;
pub mod summary;

pub use domain::{
    AlbumSummary, AudioFormat, CutoffEstimate, SuspicionReason, TrackAnalysis, TrackError,
    TrackInfo, Verdict,
};
pub use summary::summarize_album;

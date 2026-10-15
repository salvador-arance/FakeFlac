//! Album summary verdict (RF-10). Spec: `specs/detection/album-summary.md`.

use crate::domain::{AlbumSummary, TrackAnalysis};
use std::path::Path;

/// Summarises the analyses of the tracks of one folder.
///
/// Returns `None` when no track belongs to `folder`.
pub fn summarize_album(folder: &Path, tracks: &[TrackAnalysis]) -> Option<AlbumSummary> {
    let _ = (folder, tracks);
    todo!("RF-10: implement in the `implement` stage")
}

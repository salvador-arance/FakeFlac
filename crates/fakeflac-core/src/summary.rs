//! Album summary verdict (RF-10). Spec: `specs/detection/album-summary.md`.

use crate::domain::{AlbumSummary, TrackAnalysis, Verdict};
use std::path::Path;

/// Summarises the analyses of the tracks of one folder.
///
/// The summary verdict is the most severe verdict among analysed tracks
/// (`Fake` > `Suspicious` > `Real`). Error tracks are counted but only decide
/// the verdict when every track is an error. Tracks of other folders are ignored.
/// Returns `None` when no track belongs to `folder`.
pub fn summarize_album(folder: &Path, tracks: &[TrackAnalysis]) -> Option<AlbumSummary> {
    let mut tally = AlbumTally::default();
    for track in tracks.iter().filter(|t| t.info.folder == folder) {
        tally.add(track.verdict);
    }
    if tally.total == 0 {
        return None;
    }
    Some(tally.summary(folder))
}

/// Running counts of one album: the rule of `summarize_album`, one track at a time. The
/// `analyze_folder` use case keeps one per album so each `TrackDone` costs O(1) (RF-10).
#[derive(Debug, Clone, Default)]
pub(crate) struct AlbumTally {
    total: u32,
    real: u32,
    suspicious: u32,
    fake: u32,
    errors: u32,
    /// Most severe analysed verdict so far (rule 2); `None` while every track is an error.
    worst: Option<Verdict>,
}

impl AlbumTally {
    /// Counts one track of the album.
    pub(crate) fn add(&mut self, verdict: Verdict) {
        self.total += 1;
        match verdict {
            Verdict::Real => self.real += 1,
            Verdict::Suspicious => self.suspicious += 1,
            Verdict::Fake => self.fake += 1,
            Verdict::Error => {
                self.errors += 1;
                return;
            }
        }
        if self.worst.is_none_or(|w| severity(verdict) > severity(w)) {
            self.worst = Some(verdict);
        }
    }

    /// The summary of the tracks counted so far (rule 3: `Error` when every one is an error).
    pub(crate) fn summary(&self, folder: &Path) -> AlbumSummary {
        AlbumSummary {
            folder: folder.to_path_buf(),
            total: self.total,
            real: self.real,
            suspicious: self.suspicious,
            fake: self.fake,
            errors: self.errors,
            verdict: self.worst.unwrap_or(Verdict::Error),
        }
    }
}

/// Severity order of analysed verdicts (rule 2). `Error` is handled separately.
fn severity(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Real => 0,
        Verdict::Suspicious => 1,
        Verdict::Fake => 2,
        Verdict::Error => 0,
    }
}

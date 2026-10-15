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
    let mut summary = AlbumSummary {
        folder: folder.to_path_buf(),
        total: 0,
        real: 0,
        suspicious: 0,
        fake: 0,
        errors: 0,
        verdict: Verdict::Error,
    };
    let mut worst: Option<Verdict> = None;

    for track in tracks.iter().filter(|t| t.info.folder == folder) {
        summary.total += 1;
        match track.verdict {
            Verdict::Real => summary.real += 1,
            Verdict::Suspicious => summary.suspicious += 1,
            Verdict::Fake => summary.fake += 1,
            Verdict::Error => {
                summary.errors += 1;
                continue;
            }
        }
        if worst.is_none_or(|w| severity(track.verdict) > severity(w)) {
            worst = Some(track.verdict);
        }
    }

    if summary.total == 0 {
        return None;
    }
    summary.verdict = worst.unwrap_or(Verdict::Error);
    Some(summary)
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

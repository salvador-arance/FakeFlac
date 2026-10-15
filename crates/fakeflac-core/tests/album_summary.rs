//! Tests derived from `specs/features/album-summary.feature` (@RF-10).
//! Spec: `specs/detection/album-summary.md`. Protected after the `test(RF-10)` commit.

use fakeflac_core::{
    summarize_album, AlbumSummary, AudioFormat, TrackAnalysis, TrackError, TrackInfo, Verdict,
};
use std::path::{Path, PathBuf};

fn track(folder: &str, name: &str, verdict: Verdict) -> TrackAnalysis {
    let folder = PathBuf::from(folder);
    TrackAnalysis {
        info: TrackInfo {
            path: folder.join(name),
            folder,
            format: AudioFormat::Flac,
            sample_rate: 44_100,
            bit_depth: Some(16),
            channels: 2,
            declared_bitrate_kbps: None,
            duration_ms: 180_000,
            size_bytes: 30_000_000,
            modified_unix: 0,
        },
        verdict,
        cutoff: None,
        estimated_source_bitrate_kbps: None,
        reasons: Vec::new(),
        error: (verdict == Verdict::Error).then(|| TrackError::CorruptStream("test".into())),
        algorithm_version: 1,
    }
}

fn album(folder: &str, verdicts: &[Verdict]) -> Vec<TrackAnalysis> {
    verdicts
        .iter()
        .enumerate()
        .map(|(i, v)| track(folder, &format!("{:02} - Track.flac", i + 1), *v))
        .collect()
}

fn summary(folder: &str, tracks: &[TrackAnalysis]) -> AlbumSummary {
    match summarize_album(Path::new(folder), tracks) {
        Some(s) => s,
        None => panic!("expected a summary for {folder}"),
    }
}

/// (total, real, suspicious, fake, errors)
fn counts(s: &AlbumSummary) -> (u32, u32, u32, u32, u32) {
    (s.total, s.real, s.suspicious, s.fake, s.errors)
}

use Verdict::{Error, Fake, Real, Suspicious};

// Scenario: All tracks are real
#[test]
fn all_tracks_real() {
    let s = summary("Album A", &album("Album A", &[Real, Real, Real]));
    assert_eq!(s.verdict, Real);
    assert_eq!(counts(&s), (3, 3, 0, 0, 0));
}

// Scenario: One fake track makes the album fake
#[test]
fn one_fake_makes_album_fake() {
    let s = summary("Album A", &album("Album A", &[Real, Fake, Real]));
    assert_eq!(s.verdict, Fake);
    assert_eq!(counts(&s), (3, 2, 0, 1, 0));
}

// Scenario: Suspicious without fakes makes the album suspicious
#[test]
fn suspicious_without_fakes() {
    let s = summary("Album A", &album("Album A", &[Real, Suspicious, Real]));
    assert_eq!(s.verdict, Suspicious);
}

// Scenario: Fake wins over suspicious
#[test]
fn fake_wins_over_suspicious() {
    let s = summary(
        "Album A",
        &album("Album A", &[Suspicious, Fake, Suspicious]),
    );
    assert_eq!(s.verdict, Fake);
}

// Scenario: Errors are counted but do not decide the verdict
#[test]
fn errors_counted_but_do_not_decide() {
    let s = summary("Album A", &album("Album A", &[Real, Error, Real]));
    assert_eq!(s.verdict, Real);
    assert_eq!(counts(&s), (3, 2, 0, 0, 1));
}

// Scenario: Only errors
#[test]
fn only_errors() {
    let s = summary("Album A", &album("Album A", &[Error, Error]));
    assert_eq!(s.verdict, Error);
    assert_eq!(counts(&s), (2, 0, 0, 0, 2));
}

// Scenario: Empty folder
#[test]
fn empty_folder_has_no_summary() {
    assert_eq!(summarize_album(Path::new("Album A"), &[]), None);
}

// Scenario: Order does not matter
#[test]
fn order_does_not_matter() {
    let a = album("Album A", &[Fake, Real, Suspicious]);
    let b = vec![a[2].clone(), a[0].clone(), a[1].clone()];
    assert_eq!(summary("Album A", &a), summary("Album A", &b));
}

// Scenario: Tracks from another folder are ignored
#[test]
fn tracks_from_another_folder_are_ignored() {
    let mut tracks = album("Album A", &[Real, Real]);
    tracks.push(track("Album B", "01 - Other.flac", Fake));
    let s = summary("Album A", &tracks);
    assert_eq!(s.verdict, Real);
    assert_eq!(counts(&s), (2, 2, 0, 0, 0));
}

// Rule 4 + rule 6: only tracks from other folders behaves like an empty folder
#[test]
fn only_foreign_tracks_has_no_summary() {
    let tracks = album("Album B", &[Fake]);
    assert_eq!(summarize_album(Path::new("Album A"), &tracks), None);
}

// Scenario: Unicode folder names
#[test]
fn unicode_folder_names() {
    let folder = "Álbum ″Ñandú″";
    let s = summary(folder, &album(folder, &[Real, Suspicious]));
    assert_eq!(s.folder, PathBuf::from(folder));
    assert_eq!(s.verdict, Suspicious);
}

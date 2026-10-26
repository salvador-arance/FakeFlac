//! Wiring of rule L6a (detection v7, RF-16) to the real adapters (RNF-09, RNF-05), derived from
//! `specs/features/detection-streaming-source.feature`, "A 48 kHz rip with a Unicode name is
//! flagged in a real run and left unchanged". A real temp folder with synthetic lossy fixtures
//! written by the bitstream writers of `specs/detection/lossy-fixtures.md` (included by path from
//! `crates/fakeflac-audio/tests/common/lossy.rs`, ADR-0036); the default parameters (v7).
//!
//! Names chosen by the test author: `dto::SuspicionReason::StreamingSampleRate`, serialised
//! `streamingSampleRate` (contract version 6, `specs/contracts/commands.md`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../crates/fakeflac-audio/tests/common/lossy.rs"]
mod lossy;

use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, ProgressEvent, SuspicionReason, Verdict};
use fakeflac_lib::run::run_analysis;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ALBUM: &str = "Álbum ″Ñandú″";
const RIP: &str = "01 – Canción ‘x’.mp3";
const CONTROL: &str = "02 – Ópera «Ça va» 🎵.mp3";

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("run_analysis_streaming_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

/// Writes the fixture `name` of lossy-fixtures.md section 2 as `dir/file`.
fn put(dir: &Path, name: &str, file: &str) {
    let bytes = lossy::fixture_bytes(name).unwrap_or_else(|| panic!("unknown fixture {name}"));
    std::fs::write(dir.join(file), bytes).unwrap();
}

fn run(root: &Path) -> Vec<ProgressEvent> {
    let events = Mutex::new(Vec::new());
    run_analysis(&AnalysisSlot::new(), root, &|e| {
        events.lock().unwrap().push(e);
        true
    })
    .expect("run ok");
    events.into_inner().unwrap()
}

fn dones(events: &[ProgressEvent]) -> Vec<(&dto::TrackAnalysis, &dto::AlbumUpdate)> {
    events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone {
                analysis, album, ..
            } => Some((analysis, album)),
            _ => None,
        })
        .collect()
}

/// The trackDone analysis whose path's last component is exactly `file`.
fn done<'a>(
    all: &[(&'a dto::TrackAnalysis, &dto::AlbumUpdate)],
    file: &str,
) -> &'a dto::TrackAnalysis {
    all.iter()
        .map(|(a, _)| *a)
        .find(|a| Path::new(&a.info.path).file_name().and_then(|n| n.to_str()) == Some(file))
        .unwrap_or_else(|| panic!("no trackDone for {file}"))
}

/// Size, modification time and content hash of `dir/file`.
fn snapshot(dir: &Path, file: &str) -> (u64, std::time::SystemTime, u64) {
    let p = dir.join(file);
    let m = std::fs::metadata(&p).unwrap();
    let mut hasher = DefaultHasher::new();
    std::fs::read(&p).unwrap().hash(&mut hasher);
    (m.len(), m.modified().unwrap(), hasher.finish())
}

// @RF-16 @RNF-09 @RNF-05 Scenario: A 48 kHz rip with a Unicode name is flagged in a real run and
// left unchanged
#[test]
fn rf16_a_48k_rip_with_a_unicode_name_is_flagged_in_a_real_run_and_left_unchanged() {
    let root = temp_dir("unicode").join(ALBUM);
    std::fs::create_dir_all(&root).unwrap();
    put(&root, "mp3_320_48k_lp20k", RIP);
    put(&root, "mp3_320_lp20k", CONTROL);
    let before = [snapshot(&root, RIP), snapshot(&root, CONTROL)];

    let events = run(&root);
    let all = dones(&events);
    assert_eq!(all.len(), 2);

    // The 48 kHz rip: suspicious, ["streamingSampleRate"], 48000, 320, no estimate, version 7.
    let rip = done(&all, RIP);
    assert_eq!(rip.verdict, Verdict::Suspicious, "{rip:#?}");
    assert_eq!(rip.reasons, vec![SuspicionReason::StreamingSampleRate]);
    assert_eq!(rip.info.sample_rate, 48_000);
    assert_eq!(rip.info.declared_bitrate_kbps, Some(320));
    assert_eq!(rip.estimated_source_bitrate_kbps, None);
    assert_eq!(rip.algorithm_version, 7);
    assert_eq!(
        serde_json::to_value(&rip.reasons).unwrap(),
        serde_json::json!(["streamingSampleRate"]),
        "the contract identifier"
    );

    // The 44.1 kHz control: real, no reasons, 44100, estimate 320.
    let control = done(&all, CONTROL);
    assert_eq!(control.verdict, Verdict::Real, "{control:#?}");
    assert!(control.reasons.is_empty());
    assert_eq!(control.info.sample_rate, 44_100);
    assert_eq!(control.estimated_source_bitrate_kbps, Some(320));

    // Both paths in the events equal the names on disk code point for code point.
    let mut on_disk: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    let mut in_events: Vec<String> = all
        .iter()
        .map(|(a, _)| {
            let path = Path::new(&a.info.path);
            assert_eq!(
                path.parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                ALBUM
            );
            path.file_name().unwrap().to_str().unwrap().to_string()
        })
        .collect();
    in_events.sort();
    assert_eq!(in_events, on_disk);
    assert_eq!(
        in_events
            .iter()
            .map(|n| n.chars().collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        [RIP, CONTROL]
            .map(|n| n.chars().collect::<Vec<_>>())
            .into_iter()
            .collect::<Vec<_>>()
    );

    // The album update after both tracks: suspicious.
    let last = all
        .iter()
        .map(|(_, a)| *a)
        .max_by_key(|a| a.summary.total)
        .unwrap();
    assert_eq!(last.name, ALBUM);
    assert_eq!(last.summary.total, 2);
    assert_eq!(last.summary.verdict, Verdict::Suspicious);

    // Size, modification time and content hash unchanged.
    assert_eq!([snapshot(&root, RIP), snapshot(&root, CONTROL)], before);
}

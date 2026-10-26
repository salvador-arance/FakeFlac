//! CSV record of a track flagged by rule L6a (RF-16, RF-09, RNF-09):
//! `specs/features/detection-streaming-source.feature`, "The CSV record of a flagged track"; format in
//! `specs/contracts/csv-export.md` section 3 (a new contract identifier, `csv_format_version` 1).
//! Stubbed `TrackAnalysis` values only, no audio.
//!
//! Names chosen by the test author: `SuspicionReason::StreamingSampleRate` (the contract's
//! `streamingSampleRate`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{col, cutoff, read_csv, track, HEADER};
use fakeflac_core::{
    AudioCodec, AudioFormat, BitrateMode, SuspicionReason, TrackAnalysis, Verdict,
};
use fakeflac_export::build_document;

/// The stubbed TrackAnalysis of the scenario.
fn flagged() -> TrackAnalysis {
    let mut t = track("C:\\Music\\Álbum ″Ñandú″\\01 – Canción ‘x’.mp3");
    assert_eq!(
        t.info.folder.to_string_lossy(),
        "C:\\Music\\Álbum ″Ñandú″",
        "folder"
    );
    t.info.format = AudioFormat::Mp3;
    t.info.codec = Some(AudioCodec::Mp3);
    t.info.sample_rate = 48_000;
    t.info.bit_depth = None;
    t.info.channels = 2;
    t.info.duration_ms = 240_000;
    t.info.size_bytes = 9_600_000;
    t.info.declared_bitrate_kbps = Some(320);
    t.info.bitrate_mode = Some(BitrateMode::Cbr);
    t.estimated_source_bitrate_kbps = None;
    t.cutoff = cutoff(20050.0, 85.0, -95.0);
    t.verdict = Verdict::Suspicious;
    t.reasons = vec![SuspicionReason::StreamingSampleRate];
    t.error = None;
    t.algorithm_version = 7;
    t
}

// @RF-16 @RF-09 @RNF-09 Scenario: The CSV record of a flagged track
#[test]
fn rf16_the_csv_record_of_a_flagged_track() {
    let doc = build_document(&[flagged()]);
    let lines: Vec<&str> = doc.split("\r\n").collect();
    assert_eq!(lines[0], HEADER);
    assert_eq!(
        lines[1],
        "suspicious,01 – Canción ‘x’.mp3,C:\\Music\\Álbum ″Ñandú″,mp3,48000,,240000,20050,320,cbr,,streamingSampleRate,,,mp3,2,9600000,85.0,-95.0,7,C:\\Music\\Álbum ″Ñandú″\\01 – Canción ‘x’.mp3"
    );
    // The same through the RFC 4180 reader: the reasons column holds the contract identifier and
    // the estimate cell is empty (csv-export.md section 3).
    let records = read_csv(doc.as_bytes());
    assert_eq!(records.len(), 2);
    assert_eq!(records[1][col("reasons")], "streamingSampleRate");
    assert_eq!(records[1][col("estimated_source_bitrate_kbps")], "");
    assert_eq!(records[1][col("verdict")], "suspicious");
}

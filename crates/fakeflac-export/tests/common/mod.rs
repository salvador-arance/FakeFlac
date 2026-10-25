//! Test support for RF-09: a small RFC 4180 reader (the `csv` crate is not a dependency, Q12)
//! and builders of stubbed `TrackAnalysis` values. Shared with `src-tauri/tests/export_csv.rs`.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use fakeflac_core::{
    AudioCodec, AudioFormat, BitrateMode, CutoffEstimate, SuspicionReason, TrackAnalysis,
    TrackError, TrackInfo, Verdict,
};
use std::path::PathBuf;

pub const HEADER: &str = "verdict,file,folder,format,sample_rate_hz,bit_depth,duration_ms,cutoff_hz,declared_bitrate_kbps,bitrate_mode,estimated_source_bitrate_kbps,reasons,error,error_detail,codec,channels,size_bytes,cutoff_steepness_db_per_khz,noise_floor_db,algorithm_version,path";

pub const COLUMNS: [&str; 21] = [
    "verdict",
    "file",
    "folder",
    "format",
    "sample_rate_hz",
    "bit_depth",
    "duration_ms",
    "cutoff_hz",
    "declared_bitrate_kbps",
    "bitrate_mode",
    "estimated_source_bitrate_kbps",
    "reasons",
    "error",
    "error_detail",
    "codec",
    "channels",
    "size_bytes",
    "cutoff_steepness_db_per_khz",
    "noise_floor_db",
    "algorithm_version",
    "path",
];

/// Index of a column by its header name.
pub fn col(name: &str) -> usize {
    COLUMNS
        .iter()
        .position(|c| *c == name)
        .unwrap_or_else(|| panic!("no column {name}"))
}

/// Reads a document as UTF-8 with an RFC 4180 reader: records end with CRLF (a bare LF or CR
/// outside quotes is a failure), fields are quoted only with `"` and `""` stands for a quote.
/// The last record must end with CRLF and no empty line may follow it.
pub fn read_csv(bytes: &[u8]) -> Vec<Vec<String>> {
    let text = std::str::from_utf8(bytes).expect("valid UTF-8");
    let chars: Vec<char> = text.chars().collect();
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut i = 0;
    let mut at_record_start = true;
    while i < chars.len() {
        let c = chars[i];
        at_record_start = false;
        if c == '"' && field.is_empty() {
            i += 1;
            loop {
                let q = *chars.get(i).expect("unterminated quoted field");
                if q == '"' {
                    if chars.get(i + 1) == Some(&'"') {
                        field.push('"');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                field.push(q);
                i += 1;
            }
            let next = chars.get(i).copied();
            assert!(
                matches!(next, Some(',') | Some('\r')),
                "garbage after a closing quote: {next:?}"
            );
            continue;
        }
        match c {
            ',' => {
                record.push(std::mem::take(&mut field));
                i += 1;
            }
            '\r' => {
                assert_eq!(
                    chars.get(i + 1),
                    Some(&'\n'),
                    "CR without LF outside quotes"
                );
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
                i += 2;
                at_record_start = true;
            }
            '\n' => panic!("LF without CR outside quotes"),
            '"' => panic!("quote inside an unquoted field"),
            _ => {
                field.push(c);
                i += 1;
            }
        }
    }
    assert!(
        at_record_start && field.is_empty() && record.is_empty(),
        "the last record must end with CRLF"
    );
    records
}

/// Splits `C:\a\b.flac` at the last backslash (the paths of the scenarios are Windows paths).
pub fn parent_of(path: &str) -> String {
    path.rsplit_once('\\')
        .map_or(String::new(), |(p, _)| p.to_string())
}

/// A stubbed lossless track: real, 44.1 kHz, 16 bits, no cutoff, algorithm version 6.
pub fn track(path: &str) -> TrackAnalysis {
    TrackAnalysis {
        info: TrackInfo {
            path: PathBuf::from(path),
            folder: PathBuf::from(parent_of(path)),
            format: AudioFormat::Flac,
            codec: Some(AudioCodec::Flac),
            sample_rate: 44100,
            bit_depth: Some(16),
            channels: 2,
            declared_bitrate_kbps: None,
            bitrate_mode: None,
            duration_ms: 30000,
            size_bytes: 5_000_000,
            modified_unix: 1_700_000_000,
        },
        verdict: Verdict::Real,
        cutoff: None,
        estimated_source_bitrate_kbps: None,
        reasons: vec![],
        error: None,
        algorithm_version: 6,
    }
}

pub fn cutoff(
    frequency_hz: f32,
    steepness_db_per_khz: f32,
    noise_floor_db: f32,
) -> Option<CutoffEstimate> {
    Some(CutoffEstimate {
        frequency_hz,
        steepness_db_per_khz,
        noise_floor_db,
    })
}

/// The four stubbed tracks of the scenario "The document for four stubbed tracks".
pub fn four_tracks() -> Vec<TrackAnalysis> {
    let mut a = track(r#"C:\Music\Album A\02 Live, "Encore".mp3"#);
    a.info.format = AudioFormat::Mp3;
    a.info.codec = Some(AudioCodec::Mp3);
    a.info.bit_depth = None;
    a.info.duration_ms = 180000;
    a.info.size_bytes = 7_200_000;
    a.info.declared_bitrate_kbps = Some(245);
    a.info.bitrate_mode = Some(BitrateMode::Vbr);
    a.verdict = Verdict::Suspicious;
    a.cutoff = cutoff(18500.0, 12.5, -88.0);
    a.estimated_source_bitrate_kbps = Some(192);
    a.reasons = vec![SuspicionReason::CutoffBelowDeclared];

    let mut b = track(r"C:\Music\Album A\corrupt.flac");
    b.info.sample_rate = 0;
    b.info.bit_depth = None;
    b.info.channels = 0;
    b.info.duration_ms = 0;
    b.info.size_bytes = 1024;
    b.verdict = Verdict::Error;
    b.error = Some(TrackError::CorruptStream(
        "bad frame header, sync lost".into(),
    ));

    let mut c = track(r"C:\Music\Album A\lp16k.flac");
    c.verdict = Verdict::Fake;
    c.cutoff = cutoff(16000.4, 42.06, -101.94);

    let mut d = track("C:\\Music\\Álbum ″Ñandú″\\01 – Canción ‘x’.m4a");
    d.info.format = AudioFormat::M4a;
    d.info.codec = Some(AudioCodec::Alac);
    d.info.sample_rate = 48000;
    d.info.bit_depth = Some(24);
    d.info.duration_ms = 245000;
    d.info.size_bytes = 30_000_000;
    d.cutoff = cutoff(24000.0, 0.0, -120.0);

    vec![a, b, c, d]
}

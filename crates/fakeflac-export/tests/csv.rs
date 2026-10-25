//! CSV export (RF-09, RNF-04, RNF-05, RNF-09): `specs/features/export-csv.feature` sections
//! "Document", "Default file name" and the file writer; format in `specs/contracts/csv-export.md`.
//! Stubbed `TrackAnalysis` values only, no audio. Files live under `target/tmp`.
//! Protected after the `test(RF-09)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{col, cutoff, four_tracks, parent_of, read_csv, track, COLUMNS, HEADER};
use fakeflac_core::{
    export_csv, AppErrorCode, AudioCodec, AudioFormat, BitrateMode, SuspicionReason, TrackAnalysis,
    TrackError, Verdict,
};
use fakeflac_export::{build_document, default_file_name, CsvFileExporter};
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("export_csv_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn doc(tracks: &[TrackAnalysis]) -> String {
    build_document(tracks)
}

/// The cell `column` of the only record of the document of `t`.
fn cell(t: &TrackAnalysis, column: &str) -> String {
    let records = read_csv(doc(std::slice::from_ref(t)).as_bytes());
    assert_eq!(records.len(), 2);
    records[1][col(column)].clone()
}

fn record(fields: [&str; 21]) -> String {
    fields.join(",")
}

// ---------------------------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------------------------

// @RF-09 Scenario: The header row
#[test]
fn the_header_row_and_21_fields_per_record() {
    let d = doc(&[track(r"C:\Music\Album A\a.flac")]);
    let records = read_csv(d.as_bytes());
    assert_eq!(records[0].join(","), HEADER);
    assert!(d.starts_with(&format!("{HEADER}\r\n")));
    assert!(records.iter().all(|r| r.len() == 21));
    assert_eq!(COLUMNS.len(), 21);
}

// @RF-09 @RNF-04 @RNF-09 Scenario: The document for four stubbed tracks
#[test]
fn the_document_for_four_stubbed_tracks_is_exact() {
    let expected = [
        HEADER,
        r#"suspicious,"02 Live, ""Encore"".mp3",C:\Music\Album A,mp3,44100,,180000,18500,245,vbr,192,cutoffBelowDeclared,,,mp3,2,7200000,12.5,-88.0,6,"C:\Music\Album A\02 Live, ""Encore"".mp3""#,
        r#"error,corrupt.flac,C:\Music\Album A,flac,,,,,,,,,corruptStream,"bad frame header, sync lost",flac,,1024,,,6,C:\Music\Album A\corrupt.flac"#,
        r"fake,lp16k.flac,C:\Music\Album A,flac,44100,16,30000,16000,,,,,,,flac,2,5000000,42.1,-101.9,6,C:\Music\Album A\lp16k.flac",
        "real,01 – Canción ‘x’.m4a,C:\\Music\\Álbum ″Ñandú″,m4a,48000,24,245000,24000,,,,,,,alac,2,30000000,0.0,-120.0,6,C:\\Music\\Álbum ″Ñandú″\\01 – Canción ‘x’.m4a",
    ]
    .join("\r\n")
        + "\r\n";
    assert_eq!(doc(&four_tracks()), expected);
}

// @RF-09 Scenario Outline: A field is quoted only when it must be (RFC 4180)
#[test]
fn a_field_is_quoted_only_when_it_must_be() {
    // (value, exact bytes of the field, quoted)
    let rows: [(&str, &str, bool); 14] = [
        ("plain text", "plain text", false),
        ("a, b", "\"a, b\"", true),
        ("say \"hi\"", "\"say \"\"hi\"\"\"", true),
        ("\"", "\"\"\"\"", true),
        ("line\nbreak", "\"line\nbreak\"", true),
        ("line\rbreak", "\"line\rbreak\"", true),
        ("line\r\nbreak", "\"line\r\nbreak\"", true),
        (" leading space", " leading space", false),
        ("trailing space ", "trailing space ", false),
        ("tab\tinside", "tab\tinside", false),
        ("semi;colon", "semi;colon", false),
        ("it's", "it's", false),
        ("=1+1", "=1+1", false),
        ("Ñandú ″x″", "Ñandú ″x″", false),
    ];
    for (value, field, quoted) in rows {
        let mut t = track(r"C:\M\f.flac");
        t.info.sample_rate = 0;
        t.info.bit_depth = None;
        t.info.channels = 0;
        t.info.duration_ms = 0;
        t.info.size_bytes = 100;
        t.verdict = Verdict::Error;
        t.error = Some(TrackError::CorruptStream(value.to_string()));
        let expected_record = record([
            "error",
            "f.flac",
            r"C:\M",
            "flac",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "corruptStream",
            field,
            "flac",
            "",
            "100",
            "",
            "",
            "6",
            r"C:\M\f.flac",
        ]);
        assert_eq!(
            doc(&[t.clone()]),
            format!("{HEADER}\r\n{expected_record}\r\n"),
            "value {value:?}"
        );
        assert_eq!(field.starts_with('"') && field.ends_with('"'), quoted);
        assert_eq!(cell(&t, "error_detail"), value);
    }
    // "none": an absent detail is an empty, unquoted field
    let mut t = track(r"C:\M\f.flac");
    t.verdict = Verdict::Error;
    t.error = Some(TrackError::TooShort);
    let d = doc(&[t.clone()]);
    assert!(d.contains(",tooShort,,flac,"), "{d}");
    assert_eq!(cell(&t, "error_detail"), "");
}

// @RF-09 Scenario: File names are quoted by the same rule
#[test]
fn file_names_are_quoted_by_the_same_rule() {
    let t = track(r"C:\Music\Rock, Pop\Live, 1999.flac");
    let d = doc(&[t]);
    let line = d.split("\r\n").nth(1).unwrap();
    assert!(
        line.starts_with(r#"real,"Live, 1999.flac","C:\Music\Rock, Pop",flac,"#),
        "{line}"
    );
    assert!(
        line.ends_with(r#","C:\Music\Rock, Pop\Live, 1999.flac""#),
        "{line}"
    );
}

// @RF-09 Scenario Outline: Numbers do not depend on the locale
#[test]
fn numbers_do_not_depend_on_the_locale() {
    let with_cutoff = |f: f32, s: f32, n: f32| {
        let mut t = track(r"C:\M\a.flac");
        t.cutoff = cutoff(f, s, n);
        t
    };
    assert_eq!(
        cell(&with_cutoff(16000.4, 40.0, -90.0), "cutoff_hz"),
        "16000"
    );
    assert_eq!(
        cell(&with_cutoff(16000.5, 40.0, -90.0), "cutoff_hz"),
        "16001"
    );
    assert_eq!(
        cell(
            &with_cutoff(16000.0, 42.06, -90.0),
            "cutoff_steepness_db_per_khz"
        ),
        "42.1"
    );
    assert_eq!(
        cell(
            &with_cutoff(16000.0, -0.04, -90.0),
            "cutoff_steepness_db_per_khz"
        ),
        "0.0"
    );
    assert_eq!(
        cell(&with_cutoff(16000.0, 40.0, -101.94), "noise_floor_db"),
        "-101.9"
    );
    assert_eq!(
        cell(&with_cutoff(16000.0, 40.0, -120.0), "noise_floor_db"),
        "-120.0"
    );
    let mut t = track(r"C:\M\a.flac");
    t.info.size_bytes = 1_234_567;
    t.info.duration_ms = 3_723_456;
    t.info.sample_rate = 192_000;
    assert_eq!(cell(&t, "size_bytes"), "1234567");
    assert_eq!(cell(&t, "duration_ms"), "3723456");
    assert_eq!(cell(&t, "sample_rate_hz"), "192000");
}

// @RF-09 Scenario: Values are contract identifiers, not interface texts
#[test]
fn values_are_contract_identifiers() {
    let mut t = track(r"C:\M\a.m4a");
    t.info.format = AudioFormat::M4a;
    t.info.codec = Some(AudioCodec::Aac);
    t.info.bitrate_mode = Some(BitrateMode::Cbr);
    t.verdict = Verdict::Real;
    assert_eq!(cell(&t, "verdict"), "real");
    assert_eq!(cell(&t, "format"), "m4a");
    assert_eq!(cell(&t, "codec"), "aac");
    assert_eq!(cell(&t, "bitrate_mode"), "cbr");
    assert_eq!(cell(&t, "reasons"), "");
}

// @RF-09 Scenario Outline: Reasons and errors
#[test]
#[allow(clippy::type_complexity)]
fn reasons_and_errors() {
    use SuspicionReason::*;
    let rows: Vec<(
        Verdict,
        Vec<SuspicionReason>,
        Option<TrackError>,
        &str,
        &str,
        &str,
    )> = vec![
        (
            Verdict::Suspicious,
            vec![HighBandGaps, AmbiguousCutoff],
            None,
            "highBandGaps;ambiguousCutoff",
            "",
            "",
        ),
        (
            Verdict::Suspicious,
            vec![LowHighFrequencyContent],
            None,
            "lowHighFrequencyContent",
            "",
            "",
        ),
        (Verdict::Fake, vec![], None, "", "", ""),
        (
            Verdict::Error,
            vec![],
            Some(TrackError::TooShort),
            "",
            "tooShort",
            "",
        ),
        (
            Verdict::Error,
            vec![],
            Some(TrackError::UnsupportedFormat("HE-AAC".into())),
            "",
            "unsupportedFormat",
            "HE-AAC",
        ),
        (
            Verdict::Error,
            vec![],
            Some(TrackError::Unreadable("permission denied".into())),
            "",
            "unreadable",
            "permission denied",
        ),
    ];
    for (verdict, reasons, error, reasons_cell, error_cell, detail_cell) in rows {
        let mut t = track(r"C:\M\a.flac");
        t.verdict = verdict;
        t.reasons = reasons;
        t.error = error;
        assert_eq!(cell(&t, "reasons"), reasons_cell);
        assert_eq!(cell(&t, "error"), error_cell);
        assert_eq!(cell(&t, "error_detail"), detail_cell);
    }
}

// @RF-09 @RNF-04 Scenario: Unknown values of an error track are empty cells
#[test]
fn unknown_values_of_an_error_track_are_empty_cells() {
    let mut t = track(r"C:\M\x.m4a");
    t.info.format = AudioFormat::M4a;
    t.info.codec = None;
    t.info.sample_rate = 0;
    t.info.channels = 0;
    t.info.duration_ms = 0;
    t.info.bit_depth = None;
    t.info.size_bytes = 2048;
    t.verdict = Verdict::Error;
    t.error = Some(TrackError::CorruptStream("bad".into()));
    for c in [
        "codec",
        "sample_rate_hz",
        "bit_depth",
        "channels",
        "duration_ms",
        "declared_bitrate_kbps",
        "bitrate_mode",
        "cutoff_hz",
        "cutoff_steepness_db_per_khz",
        "noise_floor_db",
        "estimated_source_bitrate_kbps",
    ] {
        assert_eq!(cell(&t, c), "", "{c}");
    }
    assert_eq!(cell(&t, "size_bytes"), "2048");
    assert_eq!(cell(&t, "format"), "m4a");
}

// @RF-09 @RNF-04 Scenario: An error track keeps the values learnt before the error
#[test]
fn an_error_track_keeps_the_values_learnt_before_the_error() {
    let mut t = track(r"C:\M\short.mp3");
    t.info.format = AudioFormat::Mp3;
    t.info.codec = Some(AudioCodec::Mp3);
    t.info.sample_rate = 44100;
    t.info.channels = 1;
    t.info.duration_ms = 2011;
    t.info.bit_depth = None;
    t.info.declared_bitrate_kbps = Some(320);
    t.info.bitrate_mode = Some(BitrateMode::Cbr);
    t.verdict = Verdict::Error;
    t.error = Some(TrackError::TooShort);
    assert_eq!(cell(&t, "codec"), "mp3");
    assert_eq!(cell(&t, "sample_rate_hz"), "44100");
    assert_eq!(cell(&t, "channels"), "1");
    assert_eq!(cell(&t, "duration_ms"), "2011");
    assert_eq!(cell(&t, "declared_bitrate_kbps"), "320");
    assert_eq!(cell(&t, "bitrate_mode"), "cbr");
    for c in [
        "cutoff_hz",
        "cutoff_steepness_db_per_khz",
        "noise_floor_db",
        "estimated_source_bitrate_kbps",
        "reasons",
        "error_detail",
    ] {
        assert_eq!(cell(&t, c), "", "{c}");
    }
    assert_eq!(cell(&t, "verdict"), "error");
    assert_eq!(cell(&t, "error"), "tooShort");
}

// @RF-09 Scenario: A non-finite cutoff is written as no cutoff
#[test]
fn a_non_finite_cutoff_is_written_as_no_cutoff() {
    let mut t = track(r"C:\M\a.flac");
    t.cutoff = cutoff(16000.0, 40.0, f32::NAN);
    for c in ["cutoff_hz", "cutoff_steepness_db_per_khz", "noise_floor_db"] {
        assert_eq!(cell(&t, c), "", "{c}");
    }
}

// @RF-09 @RNF-09 Scenario: UTF-8 without BOM and CRLF line ends
#[test]
fn utf8_without_bom_and_crlf_line_ends() {
    let t = track("C:\\Music\\Álbum ″Ñandú″\\01 – Canción ‘x’ 🎵.flac");
    let d = doc(&[t.clone(), t]);
    let bytes = d.as_bytes();
    assert_eq!(&bytes[..3], &[0x76, 0x65, 0x72]);
    assert_ne!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert!(std::str::from_utf8(bytes).is_ok());
    let find = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
    assert!(find(&[0xE2, 0x80, 0xB3]), "U+2033 encoded as E2 80 B3");
    assert!(
        find(&[0xF0, 0x9F, 0x8E, 0xB5]),
        "U+1F3B5 encoded as F0 9F 8E B5"
    );
    assert!(bytes.ends_with(&[0x0D, 0x0A]));
    assert!(!bytes.ends_with(&[0x0D, 0x0A, 0x0D, 0x0A]));
    for (i, b) in bytes.iter().enumerate() {
        if *b == 0x0A {
            assert!(i > 0 && bytes[i - 1] == 0x0D, "LF without CR at {i}");
        }
    }
    assert_eq!(read_csv(bytes).len(), 3);
}

// ---------------------------------------------------------------------------------------------
// Default file name
// ---------------------------------------------------------------------------------------------

// @RF-09 @RNF-09 Scenario Outline: Default file name
#[test]
fn default_file_name_of_every_root() {
    let rows = [
        (r"C:\Music\Album A", "FakeFlac - Album A.csv"),
        (r"C:\Music\Album A\", "FakeFlac - Album A.csv"),
        (r"C:\Music\Album A\\", "FakeFlac - Album A.csv"),
        ("C:/Music/Album A/", "FakeFlac - Album A.csv"),
        ("C:\\Music\\Álbum ″Ñandú″", "FakeFlac - Álbum ″Ñandú″.csv"),
        ("C:\\Music\\02 «Ça va» 🎵", "FakeFlac - 02 «Ça va» 🎵.csv"),
        (r"C:\Music\Rock, Pop", "FakeFlac - Rock, Pop.csv"),
        (r"D:\", "FakeFlac.csv"),
        ("D:", "FakeFlac.csv"),
        (r"\\nas\music", "FakeFlac.csv"),
        (r"\\nas\music\", "FakeFlac.csv"),
        (r"\\nas\music\Album A", "FakeFlac - Album A.csv"),
        (r"\\?\C:\Music\Album A", "FakeFlac - Album A.csv"),
        (r"\\?\UNC\nas\music", "FakeFlac.csv"),
    ];
    for (root, name) in rows {
        assert_eq!(default_file_name(Path::new(root)), name, "root {root}");
    }
    // a root ending in ".." has no name of its own (csv-export.md section 5.1)
    assert_eq!(default_file_name(Path::new(r"C:\Music\..")), "FakeFlac.csv");
}

// @RF-09 Scenario: The default file name does not touch the file system
#[test]
fn default_file_name_does_not_touch_the_file_system() {
    let root = temp_dir("default_name").join("Gone");
    assert!(!root.exists());
    assert_eq!(default_file_name(&root), "FakeFlac - Gone.csv");
    assert!(!root.exists());
    assert_eq!(
        std::fs::read_dir(root.parent().unwrap()).unwrap().count(),
        0
    );
}

// ---------------------------------------------------------------------------------------------
// File writer
// ---------------------------------------------------------------------------------------------

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn two_tracks() -> Vec<TrackAnalysis> {
    vec![
        track(r"C:\Music\Album A\a.flac"),
        track(r"C:\Music\Album A\b.flac"),
    ]
}

// @RF-09 Scenario: Replacing an existing file
#[test]
fn replacing_an_existing_file_leaves_exactly_the_new_document() {
    let dir = temp_dir("replace");
    let dest = dir.join("results.csv");
    std::fs::write(&dest, vec![b'x'; 100_000]).unwrap();
    let tracks = two_tracks();
    let rows = export_csv(&CsvFileExporter, &tracks, &dest).expect("export");
    assert_eq!(rows, 2);
    assert_eq!(
        std::fs::read(&dest).unwrap(),
        build_document(&tracks).into_bytes()
    );
    assert_eq!(names(&dir), vec!["results.csv"]);
}

// @RF-09 Scenario: The temporary file never overwrites an existing file
#[test]
fn the_temporary_file_never_overwrites_an_existing_file() {
    let dir = temp_dir("tmp_clash");
    let tmp = dir.join(format!("results.csv.{}-0.tmp", std::process::id()));
    std::fs::write(&tmp, b"known content").unwrap();
    let tracks = two_tracks();
    let rows = export_csv(&CsvFileExporter, &tracks, &dir.join("results.csv")).expect("export");
    assert_eq!(rows, 2);
    assert_eq!(
        std::fs::read(dir.join("results.csv")).unwrap(),
        build_document(&tracks).into_bytes()
    );
    assert_eq!(std::fs::read(&tmp).unwrap(), b"known content");
    assert_eq!(names(&dir).len(), 2);
}

// @RF-09 Scenario: All temporary names taken
#[test]
fn all_temporary_names_taken_is_export_failed() {
    let dir = temp_dir("tmp_all_taken");
    let pid = std::process::id();
    for n in 0..16 {
        std::fs::write(
            dir.join(format!("results.csv.{pid}-{n}.tmp")),
            format!("known {n}"),
        )
        .unwrap();
    }
    let err = export_csv(&CsvFileExporter, &two_tracks(), &dir.join("results.csv")).unwrap_err();
    assert_eq!(err.code, AppErrorCode::ExportFailed);
    assert!(!dir.join("results.csv").exists());
    for n in 0..16 {
        let content =
            std::fs::read_to_string(dir.join(format!("results.csv.{pid}-{n}.tmp"))).unwrap();
        assert_eq!(content, format!("known {n}"));
    }
    assert_eq!(names(&dir).len(), 16);
}

// @RF-09 Scenario Outline: A file that cannot be written is a typed error (missing folder)
#[test]
fn a_missing_destination_folder_is_export_failed_and_nothing_is_created() {
    let dir = temp_dir("missing_folder");
    let err = export_csv(
        &CsvFileExporter,
        &two_tracks(),
        &dir.join("missing").join("results.csv"),
    )
    .unwrap_err();
    assert_eq!(err.code, AppErrorCode::ExportFailed);
    assert!(!dir.join("missing").exists());
    assert!(names(&dir).is_empty());
}

// @RF-09 Scenario Outline: A file that cannot be written is a typed error (read-only file)
#[test]
fn a_read_only_destination_is_kept_and_no_temporary_file_is_left() {
    let dir = temp_dir("read_only");
    let dest = dir.join("results.csv");
    std::fs::write(&dest, b"known content").unwrap();
    let mut perms = std::fs::metadata(&dest).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&dest, perms).unwrap();

    let result = export_csv(&CsvFileExporter, &two_tracks(), &dest);

    let readonly_after = std::fs::metadata(&dest).unwrap().permissions().readonly();
    let mut perms = std::fs::metadata(&dest).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&dest, perms).unwrap();

    assert_eq!(result.unwrap_err().code, AppErrorCode::ExportFailed);
    assert!(readonly_after, "the read-only attribute is kept");
    assert_eq!(std::fs::read(&dest).unwrap(), b"known content");
    assert_eq!(names(&dir), vec!["results.csv"]);
}

// @RF-09 Scenario Outline: A file that cannot be written is a typed error (a folder in the way)
#[test]
fn a_folder_named_like_the_destination_is_unchanged_and_no_temporary_file_is_left() {
    let dir = temp_dir("folder_in_the_way");
    let dest = dir.join("results.csv");
    std::fs::create_dir(&dest).unwrap();
    std::fs::write(dest.join("inner.txt"), b"inner").unwrap();
    let err = export_csv(&CsvFileExporter, &two_tracks(), &dest).unwrap_err();
    assert_eq!(err.code, AppErrorCode::ExportFailed);
    assert!(dest.is_dir());
    assert_eq!(names(&dest), vec!["inner.txt"]);
    assert_eq!(names(&dir), vec!["results.csv"]);
}

// @RF-09 Scenario: Large result sets
#[test]
fn ten_thousand_tracks_read_back_in_the_order_given() {
    let dir = temp_dir("large");
    let tracks: Vec<TrackAnalysis> = (0..10_000)
        .map(|i| {
            let mut t = track(&format!(r"C:\Music\Big\file-{i:05}.flac"));
            t.reasons = vec![SuspicionReason::GradualRolloff];
            t
        })
        .collect();
    let dest = dir.join("big.csv");
    let rows = export_csv(&CsvFileExporter, &tracks, &dest).expect("export");
    assert_eq!(rows, 10_000);
    let records = read_csv(&std::fs::read(&dest).unwrap());
    assert_eq!(records.len(), 10_001);
    assert!(records.iter().all(|r| r.len() == 21));
    for (i, r) in records[1..].iter().enumerate() {
        assert_eq!(r[col("path")], format!(r"C:\Music\Big\file-{i:05}.flac"));
        assert_eq!(r[col("folder")], parent_of(&r[col("path")]));
    }
}

// Test support check: the small RFC 4180 reader reads quoted fields, doubled quotes and breaks.
#[test]
fn the_test_reader_reads_rfc4180() {
    let r = read_csv(b"a,\"b,\"\"c\"\"\",\"x\r\ny\"\r\n,,\r\n");
    assert_eq!(r, vec![vec!["a", "b,\"c\"", "x\r\ny"], vec!["", "", ""]]);
}

//! Tests derived from `specs/features/calibration.feature` (@RP-11): `trusted.txt` and `confirmed.txt`
//! (`specs/calibration/inputs.md` sections 1 and 2) on the sample corpus. Protected after the
//! `test(RP-11)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;

fn stderr_lines(o: &fakeflac_calibrate::Outcome) -> Vec<&str> {
    o.stderr.lines().collect()
}

/// Runs `accept` with line `n` (1-based) of the sample `trusted.txt` replaced and checks that the run
/// stops with exit 2, the single expected error line, and nothing new in FAKEFLAC_CALIBRATION.
fn trusted_line_error(test: &str, n: usize, line: &str, expected: &str) {
    let sample = Sample::new(test);
    let mut lines = sample.trusted_lines();
    lines[n - 1] = line.to_string();
    sample.write_trusted(&lines);
    let before = names(&sample.calibration);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(stderr_lines(&o), vec![expected], "error output");
    assert_eq!(names(&sample.calibration), before, "nothing is written");
}

const ORIGINS: &str =
    "cd-log, accuraterip, bandcamp, qobuz, tienda, revisado, usuario, analisis-limpio, analisis";

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 1
#[test]
fn rp11_trusted_error_unknown_origin() {
    trusted_line_error(
        "trusted-unknown-origin",
        6,
        "Colección\\Álbum Ü | comprado",
        &format!("trusted.txt:6: unknown origin 'comprado' (expected one of: {ORIGINS})"),
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 2
#[test]
fn rp11_trusted_error_path_does_not_exist() {
    trusted_line_error(
        "trusted-missing-path",
        6,
        "Colección\\No existe | revisado",
        "trusted.txt:6: path does not exist: Colección\\No existe",
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 3
#[test]
fn rp11_trusted_error_missing_origin() {
    trusted_line_error(
        "trusted-missing-origin",
        6,
        "Colección\\Álbum Ü",
        "trusted.txt:6: missing origin",
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 4
#[test]
fn rp11_trusted_error_not_a_lossless_file() {
    trusted_line_error(
        "trusted-not-lossless",
        6,
        "Colección\\Álbum Ü\\folder.jpg | revisado",
        "trusted.txt:6: not a supported audio file (FLAC, WAV, MP3, M4A or Opus)",
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 5
#[test]
fn rp11_trusted_error_dot_dot() {
    trusted_line_error(
        "trusted-dotdot",
        6,
        "..\\fuera | revisado",
        "trusted.txt:6: '..' is not allowed",
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 6
#[test]
fn rp11_trusted_error_already_listed() {
    trusted_line_error(
        "trusted-already-listed",
        6,
        "Compras\\Ñandú – Ácido, Vol. 1 (2024) | tienda",
        "trusted.txt:6: already listed at line 4",
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 7
#[test]
fn rp11_trusted_error_exclusion_outside_listed_folders() {
    trusted_line_error(
        "trusted-exclusion-outside",
        6,
        "!Colección\\Álbum Ü\\CD1 | x",
        "trusted.txt:6: exclusion is not inside any listed folder",
    );
}

// Scenario Outline: Errors in trusted.txt stop the run with the line number (@RP-11), row 8
#[test]
fn rp11_trusted_error_library_root_set_twice() {
    trusted_line_error(
        "trusted-root-twice",
        6,
        "library_root = C:\\otra",
        "trusted.txt:6: library_root is already set at line 2",
    );
}

// Scenario: A folder with no lossless audio is an error (@RP-11)
#[test]
fn rp11_folder_with_no_lossless_audio_is_an_error() {
    let sample = Sample::new("empty-folder");
    std::fs::create_dir_all(sample.corpus.join("Vacío")).unwrap();
    std::fs::write(sample.corpus.join("Vacío/folder.jpg"), b"not audio").unwrap();
    let mut lines = sample.trusted_lines();
    lines[5] = "Vacío | revisado".to_string();
    sample.write_trusted(&lines);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec!["trusted.txt:6: no FLAC or WAV files under this folder"]
    );
}

// Scenario: Every error is reported, in line order (@RP-11)
#[test]
fn rp11_every_error_is_reported_in_line_order() {
    let sample = Sample::new("every-error");
    let mut lines = sample.trusted_lines();
    lines[5] = "Colección\\Álbum Ü | comprado".to_string();
    lines[7] = "Sueltas\\Mixto\\05 – Cuatro.flac".to_string();
    sample.write_trusted(&lines);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    let out = stderr_lines(&o);
    assert_eq!(out.len(), 2, "{out:?}");
    assert!(out[0].starts_with("trusted.txt:6:"), "{}", out[0]);
    assert!(out[1].starts_with("trusted.txt:8:"), "{}", out[1]);
}

// Scenario: library_root does not match FAKEFLAC_CORPUS (@RP-11)
#[test]
fn rp11_library_root_does_not_match_the_corpus() {
    let sample = Sample::new("root-mismatch");
    let mut lines = sample.trusted_lines();
    lines[1] = format!("library_root = {}", sample.scratch.display());
    sample.write_trusted(&lines);
    let before = names(&sample.calibration);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec!["trusted.txt:2: library_root does not match FAKEFLAC_CORPUS"]
    );
    assert_eq!(names(&sample.calibration), before, "nothing is written");
}

// Scenario: library_root with a trailing separator matches (@RP-11)
#[test]
fn rp11_library_root_with_trailing_separator_matches() {
    let sample = Sample::new("root-trailing-separator");
    let mut lines = sample.trusted_lines();
    lines[1] = format!(
        "library_root = {}{}",
        sample.corpus.display(),
        std::path::MAIN_SEPARATOR
    );
    sample.write_trusted(&lines);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
}

// Scenario: Empty trusted list (@RP-11)
#[test]
fn rp11_empty_trusted_list() {
    let sample = Sample::new("empty-trusted");
    let lines: Vec<String> = sample.trusted_lines().into_iter().take(2).collect();
    sample.write_trusted(&lines);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(stderr_lines(&o), vec!["trusted.txt: no tracks listed"]);
}

// Scenario: The most specific line wins (@RP-11)
#[test]
fn rp11_the_most_specific_line_wins() {
    let sample = Sample::new("most-specific");
    let mut lines = sample.trusted_lines();
    lines.push("Colección\\Álbum Ü\\CD2\\01 – Dos.flac | revisado".to_string());
    sample.write_trusted(&lines);
    let o = sample.accept_v2();
    assert_ok(&o);
    let labels = Table::read(&sample.calibration.join("labels.csv"));
    let dos = labels.find("path", TRACK_5).expect("row of CD2 track");
    assert_eq!(labels.get(dos, "trust_origin"), "revisado");
    assert_eq!(labels.get(dos, "class"), "reviewed");
    let uno = labels.find("path", TRACK_4).expect("row of CD1 track");
    assert_eq!(labels.get(uno, "trust_origin"), "analisis-limpio");
    assert!(
        labels.find("path", TRACK_3).is_none(),
        "the excluded Bonus track has no row"
    );
}

/// Line 2 of the sample `confirmed.txt` replaced: exit 2 with the single expected error line.
fn confirmed_line_error(test: &str, line: &str, expected: &str) {
    let sample = Sample::new(test);
    let mut lines = sample.confirmed_lines();
    lines[1] = line.to_string();
    sample.write_confirmed(&lines);
    let o = sample.accept_v2();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(stderr_lines(&o), vec![expected], "error output");
}

// Scenario Outline: Errors in confirmed.txt (@RP-11), row 1
#[test]
fn rp11_confirmed_error_also_trusted() {
    confirmed_line_error(
        "confirmed-also-trusted",
        "Revisión\\Ørsted\\01 – Tres.wav | fake",
        "confirmed.txt:2: also trusted (trusted.txt line 7)",
    );
}

// Scenario Outline: Errors in confirmed.txt (@RP-11), row 2
#[test]
fn rp11_confirmed_error_not_a_trusted_track() {
    confirmed_line_error(
        "confirmed-not-trusted",
        "Sueltas\\Mixto\\06 – Falso.flac | false-positive",
        "confirmed.txt:2: not a trusted track",
    );
}

// Scenario Outline: Errors in confirmed.txt (@RP-11), row 3
#[test]
fn rp11_confirmed_error_must_be_a_file() {
    confirmed_line_error(
        "confirmed-must-be-file",
        "Sueltas\\Mixto | false-positive",
        "confirmed.txt:2: must be a file",
    );
}

// Scenario Outline: Errors in confirmed.txt (@RP-11), row 4
#[test]
fn rp11_confirmed_error_unknown_kind() {
    confirmed_line_error(
        "confirmed-unknown-kind",
        "Sueltas\\Mixto\\06 – Falso.flac | dudoso",
        "confirmed.txt:2: unknown kind 'dudoso' (expected fake, real or false-positive)",
    );
}

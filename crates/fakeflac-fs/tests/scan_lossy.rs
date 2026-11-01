//! Tests of `WalkDirScanner` for the lossy formats (RF-02, RF-14), derived from
//! `specs/features/decode-lossy.feature`, scenario "MP3, M4A and Opus files are found". Files hold dummy bytes: the scanner does not decode.

#![allow(clippy::expect_used)]

use fakeflac_core::ports::FileScanner;
use fakeflac_core::AudioFormat;
use fakeflac_fs::WalkDirScanner;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_root(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("scan-lossy-{test}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    root
}

fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(p, bytes).expect("write");
}

// Scenario: MP3, M4A and Opus files are found (@RF-02 @RF-14)
#[test]
fn rf14_mp3_m4a_and_opus_files_are_found() {
    let root = temp_root("library");
    write(&root, "a.flac", b"1");
    write(&root, "b.mp3", b"22");
    write(&root, "c.M4A", b"333");
    write(&root, "d.opus", b"4");
    write(&root, "e.ogg", b"5");
    write(&root, "f.mp4", b"6");
    write(&root, "notes.txt", b"7");
    write(&root, "CD2/g.MP3", b"8888");

    let mut found = WalkDirScanner.scan(&root).expect("scan");
    found.sort_by(|a, b| a.path.cmp(&b.path));
    let listed: Vec<(PathBuf, AudioFormat)> = found
        .iter()
        .map(|e| {
            (
                e.path.strip_prefix(&root).expect("under root").to_owned(),
                e.format,
            )
        })
        .collect();
    let mut expected = vec![
        (PathBuf::from("a.flac"), AudioFormat::Flac),
        (PathBuf::from("b.mp3"), AudioFormat::Mp3),
        (PathBuf::from("c.M4A"), AudioFormat::M4a),
        (PathBuf::from("d.opus"), AudioFormat::Opus),
        (PathBuf::from("CD2").join("g.MP3"), AudioFormat::Mp3),
    ];
    expected.sort_by_key(|a| root.join(&a.0));
    assert_eq!(listed, expected, "5 files are scanned: {found:?}");
    let g = found
        .iter()
        .find(|e| e.path.ends_with("g.MP3"))
        .expect("g.MP3");
    assert_eq!(g.folder, root.join("CD2"));
    assert_eq!(g.size_bytes, 4);
}

// Scenario: A folder with only other Ogg files has no supported audio (@RF-02 @RF-14), scanner side:
// the scanner returns nothing and the use case reports NO_AUDIO_FOUND (`src-tauri/tests/run_analysis_lossy.rs`).
#[test]
fn rf14_a_folder_with_only_other_ogg_files_has_no_supported_audio() {
    let root = temp_root("opus-only");
    write(&root, "a.ogg", b"x");
    write(&root, "b.oga", b"x");
    assert!(WalkDirScanner.scan(&root).expect("scan").is_empty());
}

// @RF-02 @RF-14 opus.feature: Opus files are found; other Ogg extensions are not.
#[test]
fn rf14_opus_case_insensitive_recursive_scan() {
    let root = temp_root("opus-case-recursive");
    for name in [
        "a.flac",
        "b.opus",
        "c.OPUS",
        "d.ogg",
        "e.oga",
        "notes.txt",
        "CD2/f.Opus",
    ] {
        write(&root, name, b"fixture");
    }
    let files = WalkDirScanner.scan(&root).expect("scan");
    let mut names: Vec<_> = files
        .iter()
        .map(|f| f.path.strip_prefix(&root).expect("relative").to_path_buf())
        .collect();
    names.sort();
    let mut expected = vec![
        PathBuf::from("a.flac"),
        PathBuf::from("b.opus"),
        PathBuf::from("c.OPUS"),
        PathBuf::from("CD2").join("f.Opus"),
    ];
    expected.sort();
    assert_eq!(names, expected);
    assert_eq!(
        files
            .iter()
            .filter(|f| f.format == AudioFormat::Opus)
            .count(),
        3
    );
}

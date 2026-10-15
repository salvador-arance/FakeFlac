//! Tests of `WalkDirScanner` on real temporary folders (RF-02, RF-14, RNF-05, RNF-09).
//! Files hold dummy bytes: the scanner does not decode. Protected after the `test(RF-02,RF-05)` commit.

#![allow(clippy::expect_used)]

use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_core::{AppErrorCode, AudioFormat};
use fakeflac_fs::WalkDirScanner;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Fresh empty folder under the cargo target tmp dir, one per test.
fn temp_root(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("scan-{test}"));
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

fn sorted(mut v: Vec<FileEntry>) -> Vec<FileEntry> {
    v.sort_by(|a, b| a.path.cmp(&b.path));
    v
}

// Scenario: Supported files are found recursively (@RF-02 @RF-14)
#[test]
fn rf02_supported_files_found_recursively() {
    let root = temp_root("recursive");
    write(&root, "a.flac", b"12345");
    write(&root, "b.WAV", b"1234567");
    write(&root, "notes.txt", b"text");
    write(&root, "cover.jpg", b"jpg");
    write(&root, "CD2/c.flac", b"123");

    let found = sorted(WalkDirScanner.scan(&root).expect("scan"));
    assert_eq!(found.len(), 3, "{found:?}");
    let by_name = |n: &str| {
        found
            .iter()
            .find(|e| e.path.file_name().is_some_and(|f| f == n))
            .unwrap_or_else(|| panic!("missing {n}"))
    };
    let a = by_name("a.flac");
    let b = by_name("b.WAV");
    let c = by_name("c.flac");
    assert_eq!(a.format, AudioFormat::Flac);
    assert_eq!(b.format, AudioFormat::Wav);
    assert_eq!(c.format, AudioFormat::Flac);
    assert_eq!(a.path, root.join("a.flac"));
    assert_eq!(c.path, root.join("CD2").join("c.flac"));
    assert_eq!(a.folder, root);
    assert_eq!(c.folder, root.join("CD2"));
    assert_eq!(a.size_bytes, 5);
    assert_eq!(b.size_bytes, 7);
    assert_eq!(c.size_bytes, 3);
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs(),
    )
    .expect("fits");
    for e in &found {
        assert!(
            (now - e.modified_unix).abs() < 120,
            "modified_unix {} not near {now}",
            e.modified_unix
        );
    }
}

// Scenario: Hidden files and folders are skipped (@RF-02)
#[test]
fn rf02_hidden_files_and_folders_are_skipped() {
    let root = temp_root("hidden");
    write(&root, "a.flac", b"x");
    write(&root, ".hidden.flac", b"x");
    write(&root, ".trash/b.flac", b"x");

    let found = WalkDirScanner.scan(&root).expect("scan");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, root.join("a.flac"));
}

// Scenario: Folder without supported audio (@RF-02): the scanner returns empty, the use case errors
#[test]
fn rf02_folder_without_supported_audio_is_empty() {
    let root = temp_root("empty");
    write(&root, "notes.txt", b"x");
    let found = WalkDirScanner.scan(&root).expect("scan");
    assert!(found.is_empty());
}

#[test]
fn rf02_missing_root_is_folder_unreadable() {
    let root = temp_root("missing").join("does-not-exist");
    let err = WalkDirScanner.scan(&root).expect_err("error");
    assert_eq!(err.code, AppErrorCode::FolderUnreadable);
}

// Scenario: Analysing a file instead of a folder (@RF-01), contract v2
#[test]
fn rf01_root_that_is_a_file_is_not_a_folder() {
    let root = temp_root("file-root");
    write(&root, "a.flac", b"x");
    let err = WalkDirScanner
        .scan(&root.join("a.flac"))
        .expect_err("error");
    assert_eq!(err.code, AppErrorCode::NotAFolder);
}

// Scenario: Relative paths are rejected (@RF-01), contract v2. `tests` exists relative to the
// test process (cargo runs it from the package folder) and must still be rejected.
#[test]
fn rf01_relative_root_is_folder_unreadable() {
    assert!(
        Path::new("tests").is_dir(),
        "precondition: relative folder exists"
    );
    let err = WalkDirScanner.scan(Path::new("tests")).expect_err("error");
    assert_eq!(err.code, AppErrorCode::FolderUnreadable);
}

// Scenario: Unicode names (@RNF-09)
#[test]
fn rnf09_unicode_names() {
    let root = temp_root("unicode");
    let folder = "\u{c1}lbum \u{2033}\u{d1}andú\u{2033}";
    let name = "01 \u{2013} Canci\u{f3}n \u{2018}x\u{2019}.flac";
    write(&root, &format!("{folder}/{name}"), b"x");

    let found = WalkDirScanner.scan(&root).expect("scan");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].path.file_name().and_then(|f| f.to_str()),
        Some(name)
    );
    assert_eq!(
        found[0].folder.file_name().and_then(|f| f.to_str()),
        Some(folder)
    );
}

// Scenario: Analysed files are never modified (@RNF-05), scanner part
#[test]
fn rnf05_scan_does_not_modify_files() {
    let root = temp_root("readonly");
    write(&root, "a.flac", b"content-a");
    write(&root, "sub/b.wav", b"content-b");
    let snapshot = |p: &Path| {
        let m = fs::metadata(p).expect("meta");
        (
            m.len(),
            m.modified().expect("mtime"),
            fs::read(p).expect("read"),
        )
    };
    let paths = [root.join("a.flac"), root.join("sub").join("b.wav")];
    let before: Vec<_> = paths.iter().map(|p| snapshot(p)).collect();

    let found = WalkDirScanner.scan(&root).expect("scan");
    assert_eq!(found.len(), 2);

    let after: Vec<_> = paths.iter().map(|p| snapshot(p)).collect();
    assert_eq!(before, after);
}

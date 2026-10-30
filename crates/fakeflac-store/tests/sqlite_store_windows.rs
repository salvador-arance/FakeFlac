//! SQLite adapter, the Windows-only scenarios of section "Storage" of
//! `specs/features/result-cache.feature` (@RF-11 @RNF-09): path spellings that `canonicalize`
//! resolves to one key, Unicode names compared code point for code point, and a name that is not
//! valid Unicode (`specs/contracts/result-cache.md` sections 2.3 and 3). CI runs them on
//! `windows-latest`. Protected after the `test(RF-11)` commit.

#![cfg(windows)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::cache::DecoderSpy;
use common::{count, put_file, run_store, stored_paths, tag, DirScanner, TempDir};
use fakeflac_core::ports::FileScanner;
use fakeflac_core::{CacheStatus, DetectionParams};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

fn p() -> DetectionParams {
    DetectionParams::default()
}

/// `path` without the verbatim prefix `\\?\` (a plain `C:\...` path).
fn plain(path: &Path) -> PathBuf {
    let s = path.as_os_str().to_string_lossy().into_owned();
    PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s))
}

// @RF-11 @RNF-09 Scenario: Windows spellings of the same path share one key
#[test]
fn rf11_windows_spellings_of_the_same_path_share_one_key() {
    let test_name = format!("spellings-{}", std::process::id());
    // <tmp>: the canonical temp folder without "\\?\" (long names, even if temp_dir() is 8.3).
    let tmp = plain(&std::fs::canonicalize(std::env::temp_dir()).unwrap())
        .join("fakeflac-tests")
        .join(&test_name);
    // <raw>: the same folder spelled from temp_dir() as the OS gives it.
    let raw = std::env::temp_dir().join("fakeflac-tests").join(&test_name);
    let _ = std::fs::remove_dir_all(&tmp);
    let guard = TempDir(tmp.clone());
    put_file(&tmp.join("Album A"), "Track.flac", 400, 5);
    let file = tmp.join("cache").join(fakeflac_store::FILE_NAME);

    let spy1 = DecoderSpy::new();
    run_store(&file, tag(), &p(), &spy1, &tmp.join("Album A"));
    assert_eq!(spy1.count(), 1);

    let mut verbatim = OsString::from(r"\\?\");
    verbatim.push(tmp.join("Album A").as_os_str());
    for (run, root) in [
        ("run 2 (ALBUM A)", tmp.join("ALBUM A")),
        (r"run 3 (\\?\)", PathBuf::from(verbatim)),
        ("run 4 (<raw>)", raw.join("Album A")),
    ] {
        let spy = DecoderSpy::new();
        let r = run_store(&file, tag(), &p(), &spy, &root);
        assert_eq!(spy.count(), 0, "{run}: 0 analyze calls");
        let scan = DirScanner.scan(&root).unwrap();
        assert_eq!(
            r.done()[0].1.info.path,
            scan[0].path,
            "{run}: the path of this run's scan"
        );
    }
    assert_eq!(count(&file, "track_result"), 1);
    drop(guard);
}

// @RF-11 @RNF-09 Scenario: Unicode names are keys code point for code point
#[test]
fn rf11_unicode_names_are_keys_code_point_for_code_point() {
    let t = TempDir::new("unicode");
    let album = t.path().join("Álbum ″Ñandú″");
    let nfc = "02 Canci\u{00F3}n.flac";
    let nfd = "02 Cancio\u{0301}n.flac";
    assert_ne!(nfc, nfd);
    put_file(&album, "01 – Canción ‘x’ 🎵.flac", 300, 1);
    put_file(&album, nfc, 301, 2);
    put_file(&album, nfd, 302, 3);
    let file = t.cache_file();
    let scan = DirScanner.scan(&album).unwrap();
    assert_eq!(
        scan.len(),
        3,
        "NTFS keeps the NFC and NFD names as two files"
    );

    let spy1 = DecoderSpy::new();
    run_store(&file, tag(), &p(), &spy1, &album);
    assert_eq!(spy1.count(), 3);

    let spy2 = DecoderSpy::new();
    let run2 = run_store(&file, tag(), &p(), &spy2, &album);
    assert_eq!(spy2.count(), 0);
    for (i, a) in run2.done() {
        assert_eq!(
            a.info.path.as_os_str(),
            scan[i as usize].path.as_os_str(),
            "code point for code point"
        );
    }
    assert_eq!(count(&file, "track_result"), 3);
}

// @RF-11 @RNF-09 Scenario: A name that is not valid Unicode is analysed every time
#[test]
fn rf11_a_name_that_is_not_valid_unicode_is_analysed_every_time() {
    let t = TempDir::new("non-unicode");
    let album = t.path().join("Album A");
    put_file(&album, "a.flac", 300, 1);
    let wide: Vec<u16> = [u16::from(b'b'), 0xD800]
        .into_iter()
        .chain(".flac".encode_utf16())
        .collect();
    let odd = album.join(OsString::from_wide(&wide));
    assert!(odd.to_str().is_none(), "precondition: not valid Unicode");
    std::fs::write(&odd, vec![9u8; 333]).unwrap();
    let file = t.cache_file();

    let spy1 = DecoderSpy::new();
    let run1 = run_store(&file, tag(), &p(), &spy1, &album);
    assert_eq!(spy1.count(), 2);
    assert_eq!(run1.summary.cache, CacheStatus::Ok);

    let spy2 = DecoderSpy::new();
    let run2 = run_store(&file, tag(), &p(), &spy2, &album);
    assert_eq!(spy2.calls(), vec![odd], "only the non-Unicode file");
    assert_eq!(run2.summary.cache, CacheStatus::Ok);
    assert_eq!(run2.summary.from_cache, 1);
    let rows = stored_paths(&file);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].ends_with("a.flac"), "{}", rows[0]);
}

//! SQLite adapter, section "Storage" of `specs/features/result-cache.feature` (@RF-11): the real
//! `SqliteResultStore` in a temporary folder with `analyze_folder_cached`, a scanner over real
//! files and the decoder spy (acceptance C1, C2, C3 and C4 of `specs/contracts/result-cache.md`
//! section 11). "Run 1" and "run 2" each open a new store on the cache file with the same
//! `EngineTag` and `DetectionParams`, unless the scenario says otherwise.
//!
//! Readings chosen where the scenarios are open (reported in the test-stage report):
//! - "a test folder with N files" holds `.flac` files of a few hundred bytes of any content;
//! - "changed in mtime" moves the modification time 10 s later, so that it is a new second
//!   whatever the file system's resolution.
//!
//! Protected after the `test(RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::cache::{file_name_of, DecoderSpy};
use common::{
    count, mtime, open, put_file, read_only, run_store, set_mtime, stored_paths, tag, TempDir,
};
use fakeflac_core::ports::FileEntry;
use fakeflac_core::{
    AudioCodec, AudioFormat, BitrateMode, CacheStatus, CutoffEstimate, DetectionParams,
    ResultStore, SuspicionReason, TrackAnalysis, TrackError, TrackInfo, Verdict,
};
use fakeflac_store::EngineTag;
use std::path::Path;
use std::time::Duration;

fn p() -> DetectionParams {
    DetectionParams::default()
}

// @RF-11 @RNF-01 Scenario: A rerun on an unchanged folder decodes no audio (acceptance C1)
#[test]
fn rf11_a_rerun_on_an_unchanged_folder_decodes_no_audio() {
    let t = TempDir::new("c1");
    let album = t.path().join("Album A");
    for (i, n) in ["a", "b", "c", "fake_d", "susp_e", "f"].iter().enumerate() {
        put_file(&album, &format!("{n}.flac"), 300 + i, i as u8);
    }
    let file = t.cache_file();

    let spy1 = DecoderSpy::new();
    let run1 = run_store(&file, tag(), &p(), &spy1, &album);
    assert_eq!(spy1.count(), 6, "run 1 analyses every file");

    let spy2 = DecoderSpy::new();
    let run2 = run_store(&file, tag(), &p(), &spy2, &album);
    assert_eq!(spy2.count(), 0, "run 2 makes 0 analyze calls");
    assert_eq!(run2.summary.from_cache, 6);
    assert_eq!(run2.summary.cache, CacheStatus::Ok);
    assert_eq!(run2.done(), run1.done(), "each trackDone equals run 1's");
}

// @RF-11 Scenario Outline: Only the changed file is analysed again (acceptance C3)
#[test]
fn rf11_only_the_changed_file_is_analysed_again() {
    type Change = fn(&Path);
    let rows: [(&str, Change); 3] = [
        ("b.flac gets one more byte", |b| {
            let mut bytes = std::fs::read(b).unwrap();
            bytes.push(7);
            std::fs::write(b, bytes).unwrap();
        }),
        (
            "b.flac has its modification time moved 1 second later",
            |b| set_mtime(b, mtime(b) + Duration::from_secs(1)),
        ),
        (
            "b.flac is replaced by another file with a different size",
            |b| {
                std::fs::remove_file(b).unwrap();
                std::fs::write(b, vec![1u8; 777]).unwrap();
            },
        ),
    ];
    for (i, (row, change)) in rows.into_iter().enumerate() {
        let t = TempDir::new(&format!("c3-{i}"));
        let album = t.path().join("Album A");
        for n in ["a.flac", "b.flac", "c.flac"] {
            put_file(&album, n, 400, 3);
        }
        let file = t.cache_file();
        run_store(&file, tag(), &p(), &DecoderSpy::new(), &album);

        change(&album.join("b.flac"));
        let spy = DecoderSpy::new();
        let run2 = run_store(&file, tag(), &p(), &spy, &album);
        assert_eq!(spy.names(), ["b.flac"], "{row}");
        assert_eq!(run2.summary.from_cache, 2, "{row}");
    }
}

// @RF-11 Scenario: A file changed with the same size and mtime is served from the cache
// (accepted risk, ADR-0008)
#[test]
fn rf11_a_file_changed_with_the_same_size_and_mtime_is_served_from_the_cache() {
    let t = TempDir::new("same-size-mtime");
    let album = t.path().join("Album A");
    let a = put_file(&album, "a.flac", 1000, 1);
    let original = mtime(&a);
    let file = t.cache_file();
    run_store(&file, tag(), &p(), &DecoderSpy::new(), &album);

    std::fs::write(&a, vec![0xEEu8; 1000]).unwrap();
    set_mtime(&a, original);
    assert_eq!(std::fs::metadata(&a).unwrap().len(), 1000);
    let spy = DecoderSpy::new();
    run_store(&file, tag(), &p(), &spy, &album);
    assert_eq!(spy.count(), 0);
}

// @RF-11 Scenario Outline: A new engine analyses everything again and keeps only itself
// (acceptance C4)
#[test]
fn rf11_a_new_engine_analyses_everything_again_and_keeps_only_itself() {
    let base = p();
    assert!(
        base.lossy.as_ref().is_some_and(|l| l.streaming.is_some()),
        "precondition: the default params have lossy.streaming"
    );
    let mut other_version = base.clone();
    other_version.algorithm_version += 1;
    let mut other_threshold = base.clone();
    other_threshold.extent.content_drop_db += 1.0;
    let mut no_streaming = base.clone();
    no_streaming.lossy.as_mut().unwrap().streaming = None;
    let other_extraction = EngineTag {
        extraction_version: tag().extraction_version + 1,
        ..tag()
    };
    let other_app = EngineTag {
        app_version: "0.0.2".into(),
        ..tag()
    };
    let rows: Vec<(&str, DetectionParams, EngineTag)> = vec![
        ("params of another algorithm_version", other_version, tag()),
        (
            "params equal to run 1's except one threshold, same algorithm_version",
            other_threshold,
            tag(),
        ),
        (
            "params equal to run 1's but with lossy.streaming removed",
            no_streaming,
            tag(),
        ),
        (
            "an EngineTag with another extraction_version",
            base.clone(),
            other_extraction,
        ),
        (
            "an EngineTag with another app_version",
            base.clone(),
            other_app,
        ),
    ];
    for (i, (row, params2, tag2)) in rows.into_iter().enumerate() {
        let t = TempDir::new(&format!("c4-{i}"));
        let album = t.path().join("Album A");
        for n in ["a.flac", "b.flac", "c.flac"] {
            put_file(&album, n, 500, 9);
        }
        let file = t.cache_file();
        run_store(&file, tag(), &base, &DecoderSpy::new(), &album);

        let spy = DecoderSpy::new();
        let run2 = run_store(&file, tag2, &params2, &spy, &album);
        assert_eq!(spy.count(), 3, "{row}: analyze calls");
        assert_eq!(run2.summary.from_cache, 0, "{row}");
        assert_eq!(count(&file, "engine"), 1, "{row}: engine rows");
        assert_eq!(count(&file, "track_result"), 3, "{row}: track_result rows");
    }
}

// @RF-11 Scenario: The same file through another root is a hit with the new paths
#[test]
fn rf11_the_same_file_through_another_root_is_a_hit_with_the_new_paths() {
    let t = TempDir::new("another-root");
    let library = t.path().join("Library");
    let album = library.join("Album A");
    put_file(&album, "a.flac", 300, 1);
    put_file(&album, "b.flac", 301, 2);
    let file = t.cache_file();
    run_store(&file, tag(), &p(), &DecoderSpy::new(), &library);

    let spy = DecoderSpy::new();
    let run2 = run_store(&file, tag(), &p(), &spy, &album);
    assert_eq!(spy.count(), 0);
    let scan: Vec<FileEntry> = {
        use fakeflac_core::ports::FileScanner;
        common::DirScanner.scan(&album).unwrap()
    };
    let done = run2.done();
    assert_eq!(done.len(), 2);
    for (i, a) in &done {
        let e = &scan[*i as usize];
        assert_eq!(a.info.path, e.path);
        assert_eq!(a.info.folder, e.folder);
    }
    for e in &run2.events {
        if let fakeflac_core::ProgressEvent::TrackDone { album, .. } = e {
            assert!(
                album.name.starts_with("Album A"),
                "album name {}",
                album.name.display()
            );
        }
    }
}

// --- Round trip -------------------------------------------------------------------------------

fn info(e: &FileEntry, codec: Option<AudioCodec>) -> TrackInfo {
    TrackInfo {
        path: e.path.clone(),
        folder: e.folder.clone(),
        format: e.format,
        codec,
        sample_rate: 44_100,
        bit_depth: Some(16),
        channels: 2,
        declared_bitrate_kbps: None,
        bitrate_mode: None,
        duration_ms: 181_000,
        size_bytes: e.size_bytes,
        modified_unix: e.modified_unix,
    }
}

fn cutoff(f: f32, s: f32, n: f32) -> Option<CutoffEstimate> {
    Some(CutoffEstimate {
        frequency_hz: f,
        steepness_db_per_khz: s,
        noise_floor_db: n,
    })
}

/// Stubbed analyses: every verdict, every reason, every codec (and an unprobed M4A), every
/// bitrate mode, the errors tooShort and unsupportedFormat, and the floats of result-cache.md 2.1.
fn stubbed(e: &FileEntry, i: usize) -> TrackAnalysis {
    use SuspicionReason as R;
    let codecs = [
        AudioCodec::Pcm,
        AudioCodec::Flac,
        AudioCodec::Alac,
        AudioCodec::Mp3,
        AudioCodec::Aac,
        AudioCodec::Opus,
    ];
    let base = TrackAnalysis {
        info: info(e, Some(codecs[i % codecs.len()])),
        verdict: Verdict::Real,
        cutoff: None,
        estimated_source_bitrate_kbps: None,
        reasons: vec![],
        error: None,
        algorithm_version: 7,
    };
    match i {
        0 => TrackAnalysis {
            cutoff: cutoff(16_000.4, 42.06, -101.94),
            ..base
        },
        1 => TrackAnalysis {
            verdict: Verdict::Fake,
            cutoff: cutoff(19_999.998, 1e-7, -90.0),
            ..base
        },
        2 => TrackAnalysis {
            verdict: Verdict::Suspicious,
            cutoff: cutoff(20_500.0, 0.1, -60.123_456),
            reasons: vec![
                R::LowHighFrequencyContent,
                R::GradualRolloff,
                R::AmbiguousCutoff,
                R::HighBandGaps,
            ],
            ..base
        },
        3 => {
            let mut info = base.info.clone();
            info.declared_bitrate_kbps = Some(320);
            info.bitrate_mode = Some(BitrateMode::Cbr);
            info.bit_depth = None;
            TrackAnalysis {
                info,
                verdict: Verdict::Suspicious,
                cutoff: cutoff(16_000.4, 42.06, -101.94),
                estimated_source_bitrate_kbps: Some(128),
                reasons: vec![R::CutoffBelowDeclared],
                ..base
            }
        }
        4 => {
            let mut info = base.info.clone();
            info.declared_bitrate_kbps = Some(256);
            info.sample_rate = 48_000;
            TrackAnalysis {
                info,
                verdict: Verdict::Suspicious,
                cutoff: cutoff(19_999.998, 80.0, -95.5),
                reasons: vec![R::StreamingSampleRate],
                ..base
            }
        }
        5 => {
            let mut info = base.info.clone();
            info.bitrate_mode = Some(BitrateMode::Vbr);
            info.declared_bitrate_kbps = Some(245);
            TrackAnalysis { info, ..base }
        }
        6 => {
            let mut info = base.info.clone();
            info.bitrate_mode = Some(BitrateMode::Abr);
            info.declared_bitrate_kbps = Some(192);
            TrackAnalysis {
                info,
                verdict: Verdict::Fake,
                cutoff: cutoff(1e-7, 16_000.4, -101.94),
                estimated_source_bitrate_kbps: Some(96),
                ..base
            }
        }
        7 => TrackAnalysis {
            verdict: Verdict::Error,
            error: Some(TrackError::TooShort),
            ..base
        },
        _ => {
            let mut info = base.info.clone();
            info.codec = None;
            info.sample_rate = 0;
            info.bit_depth = None;
            info.channels = 0;
            info.duration_ms = 0;
            TrackAnalysis {
                info,
                verdict: Verdict::Error,
                error: Some(TrackError::UnsupportedFormat("codec «x» 🎵".into())),
                ..base
            }
        }
    }
}

fn f32_bits(a: &TrackAnalysis) -> Option<[u32; 3]> {
    a.cutoff.as_ref().map(|c| {
        [
            c.frequency_hz.to_bits(),
            c.steepness_db_per_khz.to_bits(),
            c.noise_floor_db.to_bits(),
        ]
    })
}

// @RF-11 Scenario: Stored analyses round-trip exactly (acceptance C2)
#[test]
fn rf11_stored_analyses_round_trip_exactly() {
    let t = TempDir::new("round-trip");
    let folder = t.path().join("Album A");
    let formats = [
        ("flac", AudioFormat::Flac),
        ("wav", AudioFormat::Wav),
        ("mp3", AudioFormat::Mp3),
        ("m4a", AudioFormat::M4a),
    ];
    let store = open(&t.cache_file());
    let params = p();
    let mut stored = Vec::new();
    for i in 0..9 {
        let (ext, format) = formats[i % formats.len()];
        let path = put_file(
            &folder,
            &format!("{i:02} – Canción.{ext}"),
            200 + i,
            i as u8,
        );
        let meta = std::fs::metadata(&path).unwrap();
        let e = FileEntry {
            folder: folder.clone(),
            path,
            format,
            size_bytes: meta.len(),
            modified_unix: meta
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        let a = stubbed(&e, i);
        store.put(&e, &params, &a).expect("put");
        stored.push((e, a));
    }
    // Every verdict, reason and codec is covered.
    let all = |f: &dyn Fn(&TrackAnalysis) -> bool| stored.iter().any(|(_, a)| f(a));
    for v in [
        Verdict::Real,
        Verdict::Suspicious,
        Verdict::Fake,
        Verdict::Error,
    ] {
        assert!(all(&|a| a.verdict == v), "{v:?} covered");
    }
    for c in [
        AudioCodec::Pcm,
        AudioCodec::Flac,
        AudioCodec::Alac,
        AudioCodec::Mp3,
        AudioCodec::Aac,
        AudioCodec::Opus,
    ] {
        assert!(all(&|a| a.info.codec == Some(c)), "{c:?} covered");
    }
    let mut reasons: Vec<SuspicionReason> = Vec::new();
    for r in stored.iter().flat_map(|(_, a)| a.reasons.iter().copied()) {
        if !reasons.contains(&r) {
            reasons.push(r);
        }
    }
    assert_eq!(reasons.len(), 6, "every reason covered");

    for (e, a) in &stored {
        let back = store.get(e, &params).expect("get").expect("a hit");
        assert_eq!(&back, a, "{}", e.path.display());
        assert_eq!(f32_bits(&back), f32_bits(a), "bit for bit");
    }
    drop(store);
    // Read back again through a store opened anew.
    let store = open(&t.cache_file());
    for (e, a) in &stored {
        let back = store.get(e, &params).expect("get").expect("a hit");
        assert_eq!(&back, a);
        assert_eq!(f32_bits(&back), f32_bits(a));
    }
}

// @RF-11 Scenario: A row that does not parse is a miss and is replaced
#[test]
fn rf11_a_row_that_does_not_parse_is_a_miss_and_is_replaced() {
    let t = TempDir::new("unparsable");
    let album = t.path().join("Album A");
    put_file(&album, "a.flac", 300, 1);
    let file = t.cache_file();
    run_store(&file, tag(), &p(), &DecoderSpy::new(), &album);
    assert_eq!(count(&file, "track_result"), 1);
    {
        let c = rusqlite::Connection::open(&file).unwrap();
        c.execute("UPDATE track_result SET analysis_json = '{not json'", [])
            .unwrap();
    }
    let spy = DecoderSpy::new();
    let run2 = run_store(&file, tag(), &p(), &spy, &album);
    assert_eq!(spy.count(), 1);
    assert_eq!(run2.summary.cache, CacheStatus::Ok);
    let json: String = read_only(&file)
        .query_row("SELECT analysis_json FROM track_result", [], |r| r.get(0))
        .unwrap();
    let parsed: TrackAnalysis = serde_json::from_str(&json).expect("the row parses again");
    assert_eq!(file_name_of(&parsed.info.path), "a.flac");
}

// @RF-11 Scenario: One row per file
#[test]
fn rf11_one_row_per_file() {
    let t = TempDir::new("one-row");
    let album = t.path().join("Album A");
    let a = put_file(&album, "a.flac", 300, 1);
    let file = t.cache_file();
    let spy = DecoderSpy::new();
    run_store(&file, tag(), &p(), &spy, &album);
    let mut bytes = std::fs::read(&a).unwrap();
    bytes.push(1);
    std::fs::write(&a, bytes).unwrap();
    run_store(&file, tag(), &p(), &spy, &album);
    set_mtime(&a, mtime(&a) + Duration::from_secs(10));
    run_store(&file, tag(), &p(), &spy, &album);
    assert_eq!(spy.count(), 3, "every change is analysed");
    assert_eq!(count(&file, "track_result"), 1);
    assert_eq!(stored_paths(&file).len(), 1);
}

// result-cache.md 2.3: an entry whose path cannot be canonicalised (the file disappeared) is a
// miss and is not stored, and is not a cache failure (scenario "Which results are stored" at the
// adapter, and "An entry without an mtime" for the adapter's own keyless entries).
#[test]
fn rf11_an_entry_that_cannot_be_canonicalised_is_a_miss_and_is_not_stored() {
    let t = TempDir::new("gone");
    let store = open(&t.cache_file());
    let e = FileEntry {
        path: t.path().join("Album A").join("gone.flac"),
        folder: t.path().join("Album A"),
        format: AudioFormat::Flac,
        size_bytes: 1000,
        modified_unix: 1_700_000_000,
    };
    let a = stubbed(&e, 0);
    assert_eq!(store.get(&e, &p()), Ok(None));
    assert_eq!(store.put(&e, &p(), &a), Ok(()));
    drop(store);
    assert_eq!(count(&t.cache_file(), "track_result"), 0);
}

// result-cache.md 6.3 step 3 (and acceptance C3 at the adapter): a row is a hit only if the
// entry's size and mtime match; the adapter checks it itself, not only the core.
#[test]
fn rf11_get_is_a_hit_only_when_size_and_mtime_match() {
    let t = TempDir::new("get-size-mtime");
    let folder = t.path().join("Album A");
    let path = put_file(&folder, "a.flac", 300, 1);
    let e = FileEntry {
        path,
        folder,
        format: AudioFormat::Flac,
        size_bytes: 300,
        modified_unix: 1_700_000_000,
    };
    let store = open(&t.cache_file());
    let a = stubbed(&e, 0);
    store.put(&e, &p(), &a).expect("put");
    assert_eq!(store.get(&e, &p()), Ok(Some(a)), "the same entry is a hit");
    let bigger = FileEntry {
        size_bytes: 301,
        ..e.clone()
    };
    assert_eq!(store.get(&bigger, &p()), Ok(None), "another size is a miss");
    let later = FileEntry {
        modified_unix: 1_700_000_001,
        ..e.clone()
    };
    assert_eq!(store.get(&later, &p()), Ok(None), "another mtime is a miss");
    let mut other = p();
    other.algorithm_version += 1;
    assert_eq!(store.get(&e, &other), Ok(None), "another engine is a miss");
}

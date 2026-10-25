//! Contract and real-run layer of `specs/features/album-summary-app.feature` (@RF-10): the
//! `trackDone.album` data of real runs (real adapters on generated test folders), the DTO of
//! folders whose names are not valid Unicode, and the CSV that stays one record per track.
//! The JSON of `trackDone` is in `contract.rs`. Audio files are generated inside the test
//! (target/tmp), never taken from the corpus. Protected after the `test(RF-10)` commit.
//!
//! Readings chosen where the scenarios are open (as in `export_csv.rs`; the `src-tauri` crate has
//! no FLAC encoder): "full_band.wav" is 16-bit WAV noise (real), "corrupt.flac" is random bytes
//! (error) and "lp16k.mp3" is the lossy fixture `mp3_320_lp17k` of `lossy-fixtures.md` (fake) in
//! the place of the lossless `lp16k`. The tests check these verdicts first.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../crates/fakeflac-audio/tests/common/lossy.rs"]
mod lossy;

#[path = "../../crates/fakeflac-export/tests/common/mod.rs"]
mod csv_support;

use csv_support::{read_csv, HEADER};
use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, ProgressEvent, Verdict};
use fakeflac_lib::run::{begin_export, export_csv, run_analysis, run_analysis_kept, ExportState};
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("album_summary_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn put_real(dir: &Path, file: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(dir.join(file), spec).expect("wav");
    let mut rng = ChaCha8Rng::seed_from_u64(1);
    for _ in 0..(44_100 * 6) {
        let s: f64 = rng.random::<f64>() * 2.0 - 1.0;
        w.write_sample((s * 10_000.0) as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

fn put_corrupt(dir: &Path, file: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(99);
    let bytes: Vec<u8> = (0..4096).map(|_| rng.random::<u8>()).collect();
    std::fs::write(dir.join(file), bytes).unwrap();
}

fn put_fake(dir: &Path, file: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(file),
        lossy::fixture_bytes("mp3_320_lp17k").unwrap(),
    )
    .unwrap();
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

/// `(analysis, album)` of each trackDone, in channel order.
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

fn verdict_of(events: &[ProgressEvent], file: &str) -> Verdict {
    dones(events)
        .iter()
        .find(|(a, _)| a.info.path.ends_with(file))
        .unwrap_or_else(|| panic!("no trackDone for {file}"))
        .0
        .verdict
}

// @RF-10 @RNF-04 Scenario: A real run with a corrupt file
#[test]
fn a_real_run_with_a_corrupt_file() {
    let root = temp_dir("mixed").join("Mixed");
    put_real(&root, "full_band.wav");
    put_corrupt(&root, "corrupt.flac");
    put_fake(&root, "lp16k.mp3");
    let events = run(&root);

    assert_eq!(verdict_of(&events, "full_band.wav"), Verdict::Real);
    assert_eq!(verdict_of(&events, "corrupt.flac"), Verdict::Error);
    assert_eq!(verdict_of(&events, "lp16k.mp3"), Verdict::Fake);

    let all = dones(&events);
    assert_eq!(all.len(), 3);
    for (_, a) in &all {
        assert_eq!((a.index, a.name.as_str(), a.files), (0, "Mixed", 3));
    }
    let last = all
        .iter()
        .map(|(_, a)| *a)
        .max_by_key(|a| a.summary.total)
        .unwrap();
    let s = &last.summary;
    assert_eq!(
        (s.total, s.real, s.fake, s.errors, s.verdict),
        (3, 1, 1, 1, Verdict::Fake)
    );
}

// @RF-10 @RNF-09 Scenario: A real run in a folder with a Unicode name
#[test]
fn a_real_run_in_a_folder_with_a_unicode_name() {
    let root = temp_dir("unicode").join("Álbum ″Ñandú″");
    let disc2 = root.join("Disco 2 «Ça va» 🎵");
    put_real(&root, "full_band.wav");
    put_fake(&disc2, "lp16k.mp3");
    let events = run(&root);

    let all = dones(&events);
    assert_eq!(all.len(), 2);
    let mut last: Vec<&dto::AlbumUpdate> = Vec::new();
    for index in 0..2 {
        last.push(
            all.iter()
                .map(|(_, a)| *a)
                .filter(|a| a.index == index)
                .max_by_key(|a| a.summary.total)
                .expect("album update"),
        );
    }
    assert_eq!(last[0].name, "Álbum ″Ñandú″");
    assert_eq!(last[1].name, "Álbum ″Ñandú″\\Disco 2 «Ça va» 🎵");
    for (analysis, album) in &all {
        assert_eq!(album.summary.folder, analysis.info.folder);
    }
    assert_eq!(last[0].summary.verdict, Verdict::Real);
    assert_eq!(last[1].summary.verdict, Verdict::Fake);
}

// @RF-10 Scenario: The CSV export does not include album summaries
#[test]
fn the_csv_export_does_not_include_album_summaries() {
    let root = temp_dir("csv").join("Lib");
    put_real(&root.join("Album 1"), "full_band.wav");
    put_fake(&root.join("Album 2"), "lp16k.mp3");
    let (slot, state) = (AnalysisSlot::new(), ExportState::new());
    run_analysis_kept(&slot, &state, &root, &|_| true).expect("run ok");

    let dest = temp_dir("csv_out").join("results.csv");
    let (snapshot, guard) = begin_export(&slot, &state).expect("begin export");
    let exported = export_csv(&snapshot, &dest).expect("export");
    drop(guard);
    assert_eq!(exported.rows, 2);

    let records = read_csv(&std::fs::read(&dest).unwrap());
    assert_eq!(records[0].join(","), HEADER);
    assert_eq!(records.len(), 3, "the header and one record per track");
    for r in &records {
        assert_eq!(r.len(), records[0].len());
    }
    assert!(!records[0]
        .iter()
        .any(|c| c.to_lowercase().contains("album")));
}

#[cfg(windows)]
mod non_unicode {
    use fakeflac_core::ports::{AudioProbe, FileEntry, FileScanner, SpectrumAnalyzer};
    use fakeflac_core::{
        analyze_folder, AppError, AudioCodec, AudioFormat, BandSpectra, CancelToken,
        DetectionParams, TrackError,
    };
    use fakeflac_lib::dto::{self, Verdict};
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    struct Scanner(Vec<FileEntry>);

    impl FileScanner for Scanner {
        fn scan(&self, _root: &Path) -> Result<Vec<FileEntry>, AppError> {
            Ok(self.0.clone())
        }
    }

    /// The file in the folder ending with U+D800 is real (flat), the one with U+DC00 is fake.
    struct Analyzer;

    impl SpectrumAnalyzer for Analyzer {
        fn analyze(&self, path: &Path, _p: &DetectionParams) -> Result<AudioProbe, TrackError> {
            let fake = path.parent().is_some_and(|f| {
                matches!(
                    std::os::windows::ffi::OsStrExt::encode_wide(f.as_os_str()).last(),
                    Some(0xDC00)
                )
            });
            let band_hz = 100.0;
            let levels: Vec<f64> = (0..220)
                .map(|b| {
                    let f = (f64::from(b) + 0.5) * band_hz;
                    if fake && f >= 16_000.0 {
                        -100.0
                    } else {
                        0.0
                    }
                })
                .collect();
            Ok(AudioProbe {
                sample_rate: 44_100,
                bit_depth: Some(16),
                channels: 2,
                duration_ms: 180_000,
                codec: Some(AudioCodec::Flac),
                declared_bitrate_kbps: None,
                bitrate_mode: None,
                encoder: None,
                encoder_lowpass_hz: None,
                spectrum: Some(BandSpectra {
                    sample_rate: 44_100,
                    band_hz,
                    mean_db: levels.clone(),
                    median_db: levels,
                }),
            })
        }
    }

    fn folder_with(last_unit: u16) -> PathBuf {
        let name: Vec<u16> = "Disc ".encode_utf16().chain([last_unit]).collect();
        PathBuf::from(r"C:\Music\Lib").join(OsString::from_wide(&name))
    }

    // @RF-10 @RNF-09 Scenario: Folders whose names are not valid Unicode stay separate albums
    #[test]
    fn folders_whose_names_are_not_valid_unicode_stay_separate_albums() {
        let entries: Vec<FileEntry> = [0xD800, 0xDC00]
            .into_iter()
            .map(|unit| {
                let folder = folder_with(unit);
                FileEntry {
                    path: folder.join("01.flac"),
                    folder,
                    format: AudioFormat::Flac,
                    size_bytes: 1000,
                    modified_unix: 1_700_000_000,
                }
            })
            .collect();
        let events = Mutex::new(Vec::new());
        let sink = |e: fakeflac_core::ProgressEvent| {
            events.lock().unwrap().push(dto::ProgressEvent::from(&e));
        };
        analyze_folder(
            &Scanner(entries),
            &Analyzer,
            &DetectionParams::default(),
            Path::new(r"C:\Music\Lib"),
            1,
            &CancelToken::new(),
            &sink,
        )
        .expect("run");
        let events = events.into_inner().unwrap();
        let albums: Vec<&dto::AlbumUpdate> =
            super::dones(&events).into_iter().map(|(_, a)| a).collect();

        let mut indexes: Vec<u32> = albums.iter().map(|a| a.index).collect();
        indexes.sort_unstable();
        indexes.dedup();
        assert_eq!(indexes, [0, 1]);
        for a in &albums {
            assert_eq!(a.name, "Lib\\Disc \u{FFFD}");
            assert_eq!(a.summary.folder, "C:\\Music\\Lib\\Disc \u{FFFD}");
            assert_eq!((a.files, a.summary.total), (1, 1));
        }
        let mut verdicts: Vec<_> = albums
            .iter()
            .map(|a| (a.index, format!("{:?}", a.summary.verdict)))
            .collect();
        verdicts.sort();
        let names: Vec<_> = verdicts.into_iter().map(|(_, v)| v).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            [
                format!("{:?}", Verdict::Fake),
                format!("{:?}", Verdict::Real)
            ]
        );
    }
}

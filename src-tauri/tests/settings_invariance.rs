//! Language is not an analysis input (@RF-13, @RNF-04, @RNF-09): acceptance L7 of
//! `specs/ui/language-settings.md` and `specs/contracts/settings.md` section 2 ("Language does
//! not enter DetectionParams, the cache engine identity, analysis DTOs or the CSV serializer";
//! "Settings commands do not acquire the analysis slot"). Scenario "Language changes do not change
//! exported results or detection identity" of `specs/features/language-settings.feature`, backend
//! side, and the backend part of "Settings can change while progress and a spectrogram are
//! pending".
//!
//! Readings chosen where the spec is open (reported in the test-stage report):
//! - The cache engine identity is private to `run`; it is observed through the cache itself: a run
//!   after saving Spanish is served from the entries written before it.
//! - The mixed result set (real, suspicious, fake, corrupt track with Unicode names) is recorded
//!   as core events, as `export_csv.rs` does; the cached run uses generated WAV noise.
//!
//! Interface used: `fakeflac_lib::settings::SettingsState` (see `settings_store.rs`).
//! Everything lives under `CARGO_TARGET_TMPDIR`. Protected after the `test(RF-13)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../crates/fakeflac-export/tests/common/mod.rs"]
mod csv_support;

use fakeflac_core as core;
use fakeflac_core::AnalysisSlot;
use fakeflac_lib::dto::{self, Language, Settings, SettingsLoadStatus};
use fakeflac_lib::run::{begin_export, export_csv, run_analysis_cached, ExportState};
use fakeflac_lib::settings::SettingsState;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("settings invariance")
        .join(format!("Ñandú {name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create dir");
    dir
}

fn settings_owner(dir: &Path) -> SettingsState {
    SettingsState::new(Some(dir.join("io.github.fakeflac.app")), || {
        Some(vec!["en-US".to_string()])
    })
}

const ROOT: &str = r"C:\Music";

/// Real, suspicious, fake and corrupt tracks, plus a corrupt track named "Canción – 日本語.flac".
fn mixed_tracks() -> Vec<core::TrackAnalysis> {
    let mut tracks = csv_support::four_tracks();
    let mut corrupt = csv_support::track(r"C:\Music\Álbum 日本語\Canción – 日本語.flac");
    corrupt.verdict = core::Verdict::Error;
    corrupt.info.sample_rate = 0;
    corrupt.info.bit_depth = None;
    corrupt.info.channels = 0;
    corrupt.error = Some(core::TrackError::CorruptStream(
        "invalid frame header at byte 4096".into(),
    ));
    tracks.push(corrupt);
    let verdicts: Vec<core::Verdict> = tracks.iter().map(|t| t.verdict).collect();
    for v in [
        core::Verdict::Real,
        core::Verdict::Suspicious,
        core::Verdict::Fake,
        core::Verdict::Error,
    ] {
        assert!(verdicts.contains(&v), "the set has a {v:?} track");
    }
    tracks
}

fn keep(state: &ExportState, tracks: &[core::TrackAnalysis]) {
    let root = Path::new(ROOT);
    state.record(
        root,
        &core::ProgressEvent::Scanned {
            total: tracks.len() as u32,
        },
    );
    for (i, analysis) in tracks.iter().enumerate() {
        let album = core::AlbumUpdate {
            index: 0,
            name: PathBuf::from("Music"),
            files: tracks.len() as u32,
            summary: core::AlbumSummary {
                folder: analysis.info.folder.clone(),
                total: 1,
                real: 0,
                suspicious: 0,
                fake: 0,
                errors: 0,
                verdict: analysis.verdict,
            },
        };
        state.record(
            root,
            &core::ProgressEvent::TrackDone {
                index: i as u32,
                analysis: analysis.clone(),
                album,
            },
        );
    }
}

fn export_bytes(slot: &AnalysisSlot, state: &ExportState, dest: &Path) -> Vec<u8> {
    let (snapshot, guard) = begin_export(slot, state).expect("results to export");
    let result = export_csv(&snapshot, dest).expect("export");
    drop(guard);
    assert_eq!(result.rows, 5);
    std::fs::read(dest).unwrap()
}

// @RF-13 Scenario: Language changes do not change exported results or detection identity
#[test]
fn rf13_the_csv_bytes_and_dtos_are_identical_in_english_and_spanish() {
    let dir = temp_dir("csv");
    let settings = settings_owner(&dir);
    let slot = AnalysisSlot::new();
    let state = ExportState::new();
    let tracks = mixed_tracks();
    keep(&state, &tracks);
    let dtos_en: Vec<serde_json::Value> = tracks
        .iter()
        .map(|t| serde_json::to_value(dto::TrackAnalysis::from(t)).unwrap())
        .collect();

    assert_eq!(settings.get().settings.language, Language::En);
    let english = export_bytes(&slot, &state, &dir.join("english.csv"));

    settings
        .set(Settings {
            language: Language::Es,
        })
        .expect("save Spanish");
    assert_eq!(settings.get().load_status, SettingsLoadStatus::Loaded);
    let spanish = export_bytes(&slot, &state, &dir.join("spanish.csv"));

    assert_eq!(english, spanish, "the CSV bytes are identical");
    let text = String::from_utf8(spanish).unwrap();
    assert!(
        text.contains("Canción – 日本語.flac"),
        "names keep their bytes"
    );
    assert!(
        text.contains("corruptStream"),
        "CSV values stay fixed English"
    );
    let dtos_es: Vec<serde_json::Value> = tracks
        .iter()
        .map(|t| serde_json::to_value(dto::TrackAnalysis::from(t)).unwrap())
        .collect();
    assert_eq!(dtos_en, dtos_es);
}

fn write_noise_wav(path: &Path, seed: u64) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("wav");
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..(44_100 * 6) {
        let s: f64 = rng.random::<f64>() * 2.0 - 1.0;
        w.write_sample((s * 10_000.0) as i16).expect("sample");
    }
    w.finalize().expect("finalize");
}

fn cached_run(
    slot: &AnalysisSlot,
    cache: &Path,
    root: &Path,
) -> (dto::AnalysisSummary, Vec<dto::TrackAnalysis>) {
    let export = ExportState::new();
    let events = Mutex::new(Vec::new());
    let summary = run_analysis_cached(slot, &export, Some(cache), root, &|e| {
        events.lock().unwrap().push(e);
        true
    })
    .expect("run");
    let mut done: Vec<(u32, dto::TrackAnalysis)> = events
        .into_inner()
        .unwrap()
        .into_iter()
        .filter_map(|e| match e {
            dto::ProgressEvent::TrackDone {
                index, analysis, ..
            } => Some((index, analysis)),
            _ => None,
        })
        .collect();
    done.sort_by_key(|(i, _)| *i);
    (summary, done.into_iter().map(|(_, a)| a).collect())
}

// @RF-13 "the analysis DTOs and cache engine identity are identical"
#[test]
fn rf13_saving_spanish_keeps_the_cache_engine_identity_and_the_analysis_dtos() {
    let dir = temp_dir("cache");
    let root = dir.join("Álbum 日本語");
    std::fs::create_dir_all(&root).unwrap();
    write_noise_wav(&root.join("01 – Canción.wav"), 1);
    write_noise_wav(&root.join("02 noise.wav"), 2);
    let cache = dir.join("cache").join("results-cache.sqlite3");
    let slot = AnalysisSlot::new();
    let settings = settings_owner(&dir);

    let (first, _) = cached_run(&slot, &cache, &root);
    assert_eq!(first.total, 2);
    let (english, english_tracks) = cached_run(&slot, &cache, &root);
    assert_eq!(
        english.from_cache, 2,
        "the second English run uses the cache"
    );

    settings
        .set(Settings {
            language: Language::Es,
        })
        .expect("save Spanish");
    let (spanish, spanish_tracks) = cached_run(&slot, &cache, &root);
    assert_eq!(
        spanish.from_cache, 2,
        "the same engine identity: the entries written before Spanish still match"
    );
    assert_eq!(spanish.cache, english.cache);
    assert_eq!(spanish_tracks, english_tracks, "identical analysis DTOs");
}

// Section 2: "Settings commands do not acquire the analysis slot, reject because an analysis is
// running, create a run, cancel a run ...". Scenario "Settings can change while progress and a
// spectrogram are pending" (backend side).
#[test]
fn rf13_settings_can_be_saved_while_an_analysis_holds_the_slot() {
    let dir = temp_dir("slot");
    let settings = settings_owner(&dir);
    let slot = AnalysisSlot::new();
    let run = slot.begin().expect("a run holds the slot");
    assert_eq!(settings.get().settings.language, Language::En);
    let saved = settings
        .set(Settings {
            language: Language::Es,
        })
        .expect("saving does not need the slot");
    assert_eq!(saved.settings.language, Language::Es);
    assert!(slot.begin().is_err(), "the run still holds the slot");
    assert!(!run.token().is_cancelled(), "the run is not cancelled");
    drop(run);
    assert!(slot.begin().is_ok());
}

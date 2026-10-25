//! Use-case tests of the core layer of `specs/features/album-summary-app.feature` (@RF-10):
//! `trackDone.album` with fakes of the ports. Windows paths (`C:\...`, `\\?\...`, `\\nas\...`) are
//! only parsed as such on Windows, so the whole file is `#[cfg(windows)]` (commands.md, name rule);
//! CI runs it on `windows-latest`. No real audio. Protected after the `test(RF-10)` commit.

#![cfg(windows)]
#![allow(clippy::expect_used)]

use fakeflac_core::ports::{AudioProbe, FileEntry, FileScanner, SpectrumAnalyzer};
use fakeflac_core::{
    analyze_folder, summarize_album, AlbumUpdate, AnalysisSummary, AppError, AudioCodec,
    AudioFormat, BandSpectra, CancelToken, DetectionParams, ProgressEvent, TrackAnalysis,
    TrackError, Verdict,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Real,
    Susp,
    Fake,
    Error,
}

struct FakeScanner(Vec<FileEntry>);

impl FileScanner for FakeScanner {
    fn scan(&self, _root: &Path) -> Result<Vec<FileEntry>, AppError> {
        Ok(self.0.clone())
    }
}

fn entry(path: PathBuf) -> FileEntry {
    let is_mp3 = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp3"));
    FileEntry {
        folder: path.parent().map(Path::to_path_buf).unwrap_or_default(),
        path,
        format: if is_mp3 {
            AudioFormat::Mp3
        } else {
            AudioFormat::Flac
        },
        size_bytes: 1000,
        modified_unix: 1_700_000_000,
    }
}

fn spectrum(db: impl Fn(f64) -> f64) -> BandSpectra {
    let band_hz = 100.0;
    let levels: Vec<f64> = (0..220)
        .map(|b| db((f64::from(b) + 0.5) * band_hz))
        .collect();
    BandSpectra {
        sample_rate: 44_100,
        band_hz,
        mean_db: levels.clone(),
        median_db: levels,
    }
}

/// Gives each file the verdict named; optionally makes the files finish in a fixed order
/// (`finish_order`: the files in the order their analysis returns).
struct FakeAnalyzer {
    kinds: HashMap<PathBuf, Kind>,
    finish_order: Vec<PathBuf>,
    turn: (Mutex<usize>, Condvar),
}

impl FakeAnalyzer {
    fn new(kinds: HashMap<PathBuf, Kind>, finish_order: Vec<PathBuf>) -> Self {
        FakeAnalyzer {
            kinds,
            finish_order,
            turn: (Mutex::new(0), Condvar::new()),
        }
    }

    fn wait_turn(&self, path: &Path) -> Option<()> {
        let rank = self.finish_order.iter().position(|p| p == path)?;
        let (lock, cv) = &self.turn;
        let mut turn = lock.lock().expect("lock");
        while *turn != rank {
            let (t, timeout) = cv
                .wait_timeout(turn, Duration::from_secs(10))
                .expect("wait");
            turn = t;
            assert!(!timeout.timed_out(), "the fake analyzer waited too long");
        }
        Some(())
    }

    fn done_turn(&self) {
        let (lock, cv) = &self.turn;
        *lock.lock().expect("lock") += 1;
        cv.notify_all();
    }
}

impl SpectrumAnalyzer for FakeAnalyzer {
    fn analyze(&self, path: &Path, _p: &DetectionParams) -> Result<AudioProbe, TrackError> {
        let ordered = self.wait_turn(path).is_some();
        let kind = self.kinds.get(path).copied().unwrap_or(Kind::Real);
        let result = match kind {
            Kind::Error => Err(TrackError::CorruptStream("bad frame".into())),
            _ => {
                let s = match kind {
                    Kind::Fake => spectrum(|f| if f < 16_000.0 { 0.0 } else { -100.0 }),
                    Kind::Susp => spectrum(|f| if f < 20_500.0 { 0.0 } else { -100.0 }),
                    _ => spectrum(|_| 0.0),
                };
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
                    spectrum: Some(s),
                })
            }
        };
        if ordered {
            self.done_turn();
        }
        result
    }
}

struct Run {
    result: Result<AnalysisSummary, AppError>,
    events: Vec<ProgressEvent>,
}

/// Runs the use case on `root` with the files `(relative path, kind)`.
fn run_with(
    root: &str,
    files: &[(&str, Kind)],
    finish_order: &[&str],
    workers: usize,
    cancel_at_done: Option<usize>,
) -> Run {
    let root_path = PathBuf::from(root);
    let abs = |rel: &str| root_path.join(rel);
    let scanner = FakeScanner(files.iter().map(|(f, _)| entry(abs(f))).collect());
    let kinds = files.iter().map(|(f, k)| (abs(f), *k)).collect();
    let analyzer = FakeAnalyzer::new(kinds, finish_order.iter().map(|f| abs(f)).collect());
    let cancel = CancelToken::new();
    let events = Mutex::new(Vec::new());
    let sink = |e: ProgressEvent| {
        let mut events = events.lock().expect("lock");
        events.push(e);
        if let Some(n) = cancel_at_done {
            let dones = events
                .iter()
                .filter(|e| matches!(e, ProgressEvent::TrackDone { .. }))
                .count();
            if dones == n {
                cancel.cancel();
            }
        }
    };
    let result = analyze_folder(
        &scanner,
        &analyzer,
        &DetectionParams::default(),
        &root_path,
        workers,
        &cancel,
        &sink,
    );
    Run {
        result,
        events: events.into_inner().expect("events"),
    }
}

fn run(root: &str, files: &[(&str, Kind)]) -> Run {
    run_with(root, files, &[], 1, None)
}

/// `(track index, analysis, album update)` of each trackDone, in channel order.
fn dones(events: &[ProgressEvent]) -> Vec<(u32, &TrackAnalysis, &AlbumUpdate)> {
    events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackDone {
                index,
                analysis,
                album,
            } => Some((*index, analysis, album)),
            _ => None,
        })
        .collect()
}

fn albums(events: &[ProgressEvent]) -> Vec<&AlbumUpdate> {
    dones(events).into_iter().map(|(_, _, a)| a).collect()
}

/// The last album update (channel order) of each album index, sorted by index.
fn last_by_index(events: &[ProgressEvent]) -> Vec<&AlbumUpdate> {
    let mut last: HashMap<u32, &AlbumUpdate> = HashMap::new();
    for a in albums(events) {
        last.insert(a.index, a);
    }
    let mut v: Vec<_> = last.into_values().collect();
    v.sort_by_key(|a| a.index);
    v
}

fn last_of_folder<'a>(events: &'a [ProgressEvent], folder: &str) -> &'a AlbumUpdate {
    albums(events)
        .into_iter()
        .rfind(|a| a.summary.folder == Path::new(folder))
        .unwrap_or_else(|| panic!("no album update for {folder}"))
}

fn name(a: &AlbumUpdate) -> String {
    a.name.to_string_lossy().into_owned()
}

fn finished(r: &Run) -> bool {
    r.result.is_ok() && matches!(r.events.last(), Some(ProgressEvent::Finished { .. }))
}

use Kind::{Error, Fake, Real, Susp};

// @RF-10 Scenario: Every trackDone carries the summary of its album
#[test]
fn every_track_done_carries_the_summary_of_its_album() {
    let r = run(
        r"C:\Music\Album A",
        &[("01.flac", Real), ("02.flac", Fake), ("03.flac", Real)],
    );
    let all = dones(&r.events);
    assert_eq!(all.len(), 3);
    for (_, _, a) in &all {
        assert_eq!((a.index, name(a).as_str(), a.files), (0, "Album A", 3));
    }
    let (_, last_analysis, last) = all[2];
    let s = &last.summary;
    assert_eq!(
        (s.total, s.real, s.suspicious, s.fake, s.errors, s.verdict),
        (3, 2, 0, 1, 0, Verdict::Fake)
    );
    assert_eq!(s.folder, PathBuf::from(r"C:\Music\Album A"));
    assert_eq!(s.folder, last_analysis.info.folder);
}

// @RF-10 Scenario: A folder tree, the root's own tracks and a folder without files of its own
#[test]
fn folder_tree_root_tracks_and_folder_without_own_files() {
    let r = run(
        r"C:\Music\Artist",
        &[
            ("intro.flac", Real),
            (r"Album 1\01.flac", Real),
            (r"Album 1\02.flac", Susp),
            (r"Album 1\CD2\01.flac", Fake),
            (r"Album 2\01.mp3", Real),
            (r"Box\Disc 1\01.flac", Real),
        ],
    );
    let last = last_by_index(&r.events);
    let got: Vec<_> = last
        .iter()
        .map(|a| {
            (
                a.index,
                name(a),
                a.summary.folder.to_string_lossy().into_owned(),
                a.files,
                a.summary.verdict,
            )
        })
        .collect();
    let expected: Vec<(u32, String, String, u32, Verdict)> = [
        (0, "Artist", r"C:\Music\Artist", 1, Verdict::Real),
        (
            1,
            r"Artist\Album 1",
            r"C:\Music\Artist\Album 1",
            2,
            Verdict::Suspicious,
        ),
        (
            2,
            r"Artist\Album 1\CD2",
            r"C:\Music\Artist\Album 1\CD2",
            1,
            Verdict::Fake,
        ),
        (
            3,
            r"Artist\Album 2",
            r"C:\Music\Artist\Album 2",
            1,
            Verdict::Real,
        ),
        (
            4,
            r"Artist\Box\Disc 1",
            r"C:\Music\Artist\Box\Disc 1",
            1,
            Verdict::Real,
        ),
    ]
    .into_iter()
    .map(|(i, n, f, c, v)| (i, n.to_string(), f.to_string(), c, v))
    .collect();
    assert_eq!(got, expected);
    assert!(albums(&r.events)
        .iter()
        .all(|a| a.summary.folder != Path::new(r"C:\Music\Artist\Box")));
    assert_eq!(
        last_of_folder(&r.events, r"C:\Music\Artist\Album 1")
            .summary
            .verdict,
        Verdict::Suspicious
    );
}

// @RF-10 Scenario: The album index does not depend on which track finishes first
#[test]
fn album_index_does_not_depend_on_which_track_finishes_first() {
    let r = run_with(
        r"C:\Music\Lib",
        &[
            (r"B\01.flac", Real),
            (r"A\01.flac", Real),
            (r"A\02.flac", Real),
        ],
        &[r"B\01.flac", r"A\02.flac", r"A\01.flac"],
        3,
        None,
    );
    for a in albums(&r.events) {
        let expected = if a.summary.folder == Path::new(r"C:\Music\Lib\A") {
            0
        } else {
            1
        };
        assert_eq!(a.index, expected, "{}", a.summary.folder.display());
    }
    assert_eq!(albums(&r.events).len(), 3);
}

// @RF-10 Scenario: Folder order is component by component and case-insensitive
#[test]
fn folder_order_is_component_by_component_and_case_insensitive() {
    let r = run(
        r"C:\Music\Lib",
        &[
            (r"album (deluxe)\01.flac", Real),
            (r"Album\02.flac", Real),
            (r"Album\CD2\01.flac", Real),
            (r"ALBUM B\01.flac", Real),
        ],
    );
    let got: Vec<_> = last_by_index(&r.events)
        .iter()
        .map(|a| (a.index, name(a)))
        .collect();
    let expected: Vec<_> = [
        (0, r"Lib\Album"),
        (1, r"Lib\Album\CD2"),
        (2, r"Lib\album (deluxe)"),
        (3, r"Lib\ALBUM B"),
    ]
    .into_iter()
    .map(|(i, n)| (i, n.to_string()))
    .collect();
    assert_eq!(got, expected);
}

// @RF-10 Scenario: Summaries grow one track at a time in event order
#[test]
fn summaries_grow_one_track_at_a_time_in_event_order() {
    let r = run(
        r"C:\Music\Album A",
        &[
            ("01.flac", Real),
            ("02.flac", Real),
            ("03.flac", Fake),
            ("04.flac", Real),
        ],
    );
    let a = albums(&r.events);
    let totals: Vec<_> = a.iter().map(|a| a.summary.total).collect();
    let verdicts: Vec<_> = a.iter().map(|a| a.summary.verdict).collect();
    assert_eq!(totals, [1, 2, 3, 4]);
    assert_eq!(
        verdicts,
        [Verdict::Real, Verdict::Real, Verdict::Fake, Verdict::Fake]
    );
}

fn mixed_verdict(i: usize) -> Kind {
    [Real, Real, Susp, Fake, Error][i % 5]
}

fn order_holds(workers: usize) {
    let mut files: Vec<(String, Kind)> = (0..50)
        .map(|i| (format!("{i:02}.flac"), mixed_verdict(i)))
        .collect();
    files.extend((0..30).map(|i| (format!(r"CD2\{i:02}.flac"), mixed_verdict(i + 2))));
    let refs: Vec<(&str, Kind)> = files.iter().map(|(f, k)| (f.as_str(), *k)).collect();
    let r = run_with(r"C:\Music\Big", &refs, &[], workers, None);
    assert!(finished(&r));

    let mut sent: HashMap<u32, Vec<TrackAnalysis>> = HashMap::new();
    for (_, analysis, album) in dones(&r.events) {
        let tracks = sent.entry(album.index).or_default();
        tracks.push(analysis.clone());
        assert_eq!(
            album.summary.total as usize,
            tracks.len(),
            "the n-th trackDone of an album carries total n"
        );
        let expected = summarize_album(&album.summary.folder, tracks).expect("summary");
        assert_eq!(album.summary, expected);
    }
    let last = last_by_index(&r.events);
    assert_eq!(last.len(), 2);
    assert_eq!(
        (name(last[0]).as_str(), last[0].summary.total, last[0].files),
        ("Big", 50, 50)
    );
    assert_eq!(
        (name(last[1]).as_str(), last[1].summary.total, last[1].files),
        (r"Big\CD2", 30, 30)
    );
}

// @RF-10 Scenario Outline: The order holds with parallel workers (workers = 1)
#[test]
fn order_holds_with_1_worker() {
    order_holds(1);
}

// @RF-10 Scenario Outline: The order holds with parallel workers (workers = 4)
#[test]
fn order_holds_with_4_workers() {
    order_holds(4);
}

// @RF-10 Scenario Outline: The order holds with parallel workers (workers = 8)
#[test]
fn order_holds_with_8_workers() {
    order_holds(8);
}

// @RF-10 @RNF-04 Scenario: A folder whose files all fail has an error summary
#[test]
fn folder_whose_files_all_fail_has_an_error_summary() {
    let r = run(
        r"C:\Music\Lib",
        &[
            (r"Broken\01.flac", Error),
            (r"Broken\02.flac", Error),
            (r"Good\01.flac", Real),
        ],
    );
    let broken = last_of_folder(&r.events, r"C:\Music\Lib\Broken");
    assert_eq!(name(broken), r"Lib\Broken");
    assert_eq!(
        (
            broken.summary.total,
            broken.summary.errors,
            broken.summary.verdict
        ),
        (2, 2, Verdict::Error)
    );
    let good = last_of_folder(&r.events, r"C:\Music\Lib\Good");
    assert_eq!(good.summary.verdict, Verdict::Real);
    assert!(finished(&r));
}

// @RF-10 @RNF-04 Scenario: An album can leave the error verdict when a readable track arrives
#[test]
fn album_can_leave_the_error_verdict_when_a_readable_track_arrives() {
    let r = run(
        r"C:\Music\Album A",
        &[("01.flac", Error), ("02.flac", Fake)],
    );
    let a = albums(&r.events);
    let verdicts: Vec<_> = a.iter().map(|a| a.summary.verdict).collect();
    assert_eq!(verdicts, [Verdict::Error, Verdict::Fake]);
    let s = &a[1].summary;
    assert_eq!((s.total, s.fake, s.errors), (2, 1, 1));
}

// @RF-10 Scenario: A finished run completes every album
#[test]
fn a_finished_run_completes_every_album() {
    let r = run_with(
        r"C:\Music\Artist",
        &[
            ("intro.flac", Real),
            (r"Album 1\01.flac", Real),
            (r"Album 1\02.flac", Susp),
            (r"Album 1\03.flac", Error),
            (r"Album 1\CD2\01.flac", Fake),
            (r"Album 2\01.mp3", Real),
            (r"Album 2\02.mp3", Fake),
        ],
        &[],
        4,
        None,
    );
    assert!(finished(&r));
    let last = last_by_index(&r.events);
    assert_eq!(last.len(), 4);
    for a in &last {
        assert_eq!(a.summary.total, a.files, "{}", name(a));
    }
    let sum: u32 = last.iter().map(|a| a.summary.total).sum();
    assert_eq!(sum, r.result.expect("summary").done);
}

// @RF-10 Scenario: A cancelled run leaves partial albums
#[test]
fn a_cancelled_run_leaves_partial_albums() {
    let mut files: Vec<(String, Kind)> = (1..=12)
        .map(|i| (format!(r"Album 1\{i:02}.flac"), Real))
        .collect();
    files.extend((1..=12).map(|i| (format!(r"Album 2\{i:02}.flac"), Real)));
    let refs: Vec<(&str, Kind)> = files.iter().map(|(f, k)| (f.as_str(), *k)).collect();
    let r = run_with(r"C:\Music\Big", &refs, &[], 1, Some(3));
    assert_eq!(dones(&r.events).len(), 3);
    assert!(matches!(
        r.events.last(),
        Some(ProgressEvent::Cancelled { .. })
    ));
    let a = albums(&r.events);
    assert_eq!(a.len(), 3);
    let totals: Vec<_> = a.iter().map(|a| a.summary.total).collect();
    assert_eq!(totals, [1, 2, 3]);
    for u in a {
        assert_eq!(u.index, 0);
        assert_eq!(u.summary.folder, PathBuf::from(r"C:\Music\Big\Album 1"));
        assert_eq!(u.files, 12);
    }
}

// @RF-10 Scenario: Other events carry no album data
#[test]
fn other_events_carry_no_album_data() {
    let r = run(r"C:\Music\Album A", &[("01.flac", Real), ("02.flac", Real)]);
    let summary = r.result.expect("summary");
    assert_eq!(
        (
            summary.total,
            summary.done,
            summary.real,
            summary.suspicious,
            summary.fake,
            summary.errors,
            summary.cancelled
        ),
        (2, 2, 2, 0, 0, 0, false)
    );
    assert_eq!(summary.root, PathBuf::from(r"C:\Music\Album A"));
    assert!(matches!(
        r.events.first(),
        Some(ProgressEvent::Scanned { total: 2 })
    ));
    let started: Vec<_> = r
        .events
        .iter()
        .filter_map(|e| match e {
            ProgressEvent::TrackStarted { index, path } => Some((*index, path.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        started,
        [
            (0, PathBuf::from(r"C:\Music\Album A\01.flac")),
            (1, PathBuf::from(r"C:\Music\Album A\02.flac"))
        ]
    );
    match r.events.last() {
        Some(ProgressEvent::Finished { summary: s }) => assert_eq!(*s, summary),
        other => panic!("expected finished, got {other:?}"),
    }
}

fn album_name_case(root: &str, folder: &str, expected: &str) {
    let file = Path::new(folder).join("01.flac");
    let scanner = FakeScanner(vec![entry(file.clone())]);
    let analyzer = FakeAnalyzer::new(HashMap::new(), Vec::new());
    let events = Mutex::new(Vec::new());
    let sink = |e: ProgressEvent| events.lock().expect("lock").push(e);
    analyze_folder(
        &scanner,
        &analyzer,
        &DetectionParams::default(),
        Path::new(root),
        1,
        &CancelToken::new(),
        &sink,
    )
    .expect("run");
    let events = events.into_inner().expect("events");
    let a = albums(&events);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].name, PathBuf::from(expected));
}

macro_rules! name_cases {
    ($($test:ident: $root:expr, $folder:expr => $name:expr;)*) => {
        $(
            // @RF-10 Scenario Outline: Album name
            #[test]
            fn $test() {
                album_name_case($root, $folder, $name);
            }
        )*
    };
}

name_cases! {
    album_name_root_own_tracks: r"C:\Music\Artist", r"C:\Music\Artist" => "Artist";
    album_name_subfolder: r"C:\Music\Artist", r"C:\Music\Artist\Album 1" => r"Artist\Album 1";
    album_name_trailing_separator: r"C:\Music\Artist\", r"C:\Music\Artist\Album 1\CD2" => r"Artist\Album 1\CD2";
    album_name_unicode: r"C:\Music\Álbum ″Ñandú″", r"C:\Music\Álbum ″Ñandú″" => r"Álbum ″Ñandú″";
    album_name_verbatim: r"\\?\C:\Music\Album A", r"\\?\C:\Music\Album A" => "Album A";
    album_name_verbatim_drive_root: r"\\?\C:\", r"\\?\C:\" => r"\\?\C:\";
    album_name_verbatim_drive_subfolder: r"\\?\C:\", r"\\?\C:\Album A" => r"\\?\C:\Album A";
    album_name_unc_subfolder: r"\\nas\music\Rock", r"\\nas\music\Rock\Album B" => r"Rock\Album B";
    album_name_drive_root: r"D:\", r"D:\" => r"D:\";
    album_name_drive_root_subfolder: r"D:\", r"D:\Album A" => r"D:\Album A";
    album_name_unc_share_root: r"\\nas\music", r"\\nas\music" => r"\\nas\music";
    album_name_unc_share_subfolder: r"\\nas\music", r"\\nas\music\Album B" => r"\\nas\music\Album B";
}

// @RF-10 @RNF-09 Scenario: Unicode folder names
#[test]
fn unicode_folder_names() {
    let root = r"C:\Music\Álbum ″Ñandú″";
    let r = run(
        root,
        &[
            ("01 – Canción ‘x’.flac", Real),
            (r"Disco 2 «Ça va» 🎵\01.flac", Susp),
        ],
    );
    let last = last_by_index(&r.events);
    assert_eq!(last.len(), 2);
    assert_eq!(name(last[0]), "Álbum ″Ñandú″");
    assert_eq!(name(last[1]), r"Álbum ″Ñandú″\Disco 2 «Ça va» 🎵");
    for (_, analysis, album) in dones(&r.events) {
        assert_eq!(album.summary.folder, analysis.info.folder);
    }
    assert_eq!(last[0].summary.folder, PathBuf::from(root));
    assert_eq!(
        last[1].summary.folder,
        PathBuf::from(root).join("Disco 2 «Ça va» 🎵")
    );
}

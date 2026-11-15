//! Tests derived from `specs/features/census.feature` (@RP-12, with @RNF-04, @RNF-05 and @RNF-09 where
//! they apply) and the acceptance criteria C1 to C11 of `specs/calibration/census.md` section 10: the
//! library census on the census sample folder of section 8 (synthetic fixtures in a temporary folder
//! outside the repository, never real audio), the report builder on analyses built in the test, the
//! refused runs, the refusal of a folder that contains the report folder (C11, against a stand-in
//! repository root in the temporary folder, never the repository's own `target/`, with its cases
//! (a) a `..` segment, (b) a different letter case (Windows only), (c) a repository root that does
//! not exist (fail-open) and (d) a `target` that is a junction (Windows only, skipped with the
//! reason printed when the junction cannot be created)) and the wiring of the recipe. Protected
//! after the `test(RP-12)` commits.
//!
//! # Entry points the implementation must provide (census.md section 9)
//!
//! In `tools/fakeflac-calibrate`, nothing else changes:
//!
//! - `fakeflac_calibrate::run::CensusArgs { pub folder: PathBuf, pub params: Option<PathBuf> }` (exactly
//!   these two public fields) and the variant `fakeflac_calibrate::run::Command::Census(CensusArgs)`.
//! - `fakeflac_calibrate::run::parse_args`: `["census", FOLDER]` and `["census", FOLDER, "--params",
//!   FILE]` (`--params FILE` before or after `FOLDER`) give `Command::Census`, with `FOLDER` and `FILE`
//!   as given. Every usage error of census.md section 2 gives `Err(ToolError)` of kind `Input` whose
//!   `lines` are exactly `["usage: fakeflac-calibrate census FOLDER [--params FILE]"]`; the binary
//!   prints them to stderr and exits 2.
//! - `fakeflac_calibrate::run::run(&Command::Census(args), &env, transcoder)`: the census of section 3.
//!   It reads only `env.repo_root` (the repository root that `Env::from_process` finds with git; `None`
//!   is "not run inside a repository", exit 2), never `env.corpus` or `env.calibration` (D2), and never
//!   calls the transcoder. It writes `<env.repo_root>/target/census/<run>/summary.md` and `tracks.csv`
//!   and nothing else, and returns an `Outcome` with `exit_code` 0, `stdout` = the text of `summary.md`
//!   followed by the line `elapsed: <seconds, one decimal> s`, an empty `stderr` and `report_dir:
//!   Some(<that run folder>)`. A refused run returns its exit code and the exact stderr lines of the
//!   section 2 table, an empty `stdout` and `report_dir: None`, and writes nothing. The binary prints
//!   `stdout` and `stderr` of the outcome unchanged (as `accept` does).
//! - A new module `fakeflac_calibrate::census` with the pure report builder:
//!
//!   ```ignore
//!   /// One track of a run: its analysis and the `album.index` of its `TrackDone`.
//!   pub struct CensusTrack { pub analysis: TrackAnalysis, pub album: u32 }
//!   /// The header data of `summary.md` (section 4.1): `params_id` is `None` for the defaults and the
//!   /// `id16` of the params file's bytes with `--params`.
//!   pub struct CensusHeader { pub run: String, pub algorithm_version: u32, pub params_id: Option<String> }
//!   /// `summary.md` (section 4) from the tracks of a run (any order) and the summary of the last
//!   /// `TrackDone` of every album index.
//!   pub fn summary_md(header: &CensusHeader, tracks: &[CensusTrack],
//!                     albums: &BTreeMap<u32, AlbumSummary>) -> String;
//!   ```
//!
//! - In `justfile`: the opt-in recipe `census *args` (positional arguments, a Node script like
//!   `accept`) that runs `fakeflac-calibrate census`.
//!
//! # How `summary.md` is compared
//!
//! Section 4 ("Blocks and blank lines") makes `summary.md` a sequence of blocks (each header line of
//! 4.1, each table, the one-Fake line of 4.4, and `errors: none` or the error table of 4.7) with one
//! blank line after every block, the last one included, and no blank line inside a block. Blank lines
//! are layout: the tests compare the non-blank lines, a table line cell by cell with the cells trimmed
//! (column padding is layout) and every other line exactly. One test checks the blank lines
//! themselves (`rp12_c1_one_blank_line_follows_every_block_of_the_summary`).
//!
//! # How `tracks.csv` is checked
//!
//! Section 5 ("Empty means not measured"): an error track writes an empty field, never `0`, in
//! `sample_rate`, `bit_depth`, `channels`, `declared_bitrate_kbps` and `duration_ms` where the value
//! was not measured (the unprobed placeholders 0 / no value, or a 0 reported by the probe), and keeps
//! the real values of a probed file. The rule is stated for error tracks only, so a track that is not
//! an error writes its values as they are, `0` included. Checked on the sample folder (C2) and on the
//! pure `tracks_csv` with given analyses.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::census::{summary_md, tracks_csv, CensusHeader, CensusTrack};
use fakeflac_calibrate::error::ExitKind;
use fakeflac_calibrate::hash::id16;
use fakeflac_calibrate::run::{parse_args, run, CensusArgs, Command, Outcome, DEFAULT_PARAMS};
use fakeflac_calibrate::Env;
use fakeflac_core::{
    summarize_album, AlbumSummary, AudioCodec, AudioFormat, EvidenceKind, FakeEvidence,
    SuspicionReason, TrackAnalysis, TrackError, TrackInfo, Verdict,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

// ---------------------------------------------------------------------------------------------
// Fixed texts of census.md
// ---------------------------------------------------------------------------------------------

const ROOT_NAME: &str = "Biblioteca ″Ñandú″ & co";
const USAGE: &str = "usage: fakeflac-calibrate census FOLDER [--params FILE]";
const NOT_A_FOLDER: &str = "error: census: the folder does not exist or is not a folder";
const NO_AUDIO: &str = "error: census: no supported audio files in the folder";
const NO_REPOSITORY: &str =
    "error: census: run it from the repository (the report goes under target/)";
const CONTAINS_REPORT: &str =
    "error: census: the folder contains the report folder (target/census/ of the repository)";

const TITLE: &str = "FakeFlac library census (specs/calibration/census.md)";
const H_CODEC: &str = "| codec | tracks | real | suspicious | fake | error |";
const H_EVIDENCE: &str = "| group | evidence | tracks |";
const H_ALBUMS: &str = "| stratum | albums | fake tracks | lossless | lossy |";
const H_MARGIN: &str = "| margin dB | whole | mixed | isolated | total |";
const H_REASONS: &str = "| reason | lossless | lossy |";
const H_ERRORS: &str = "| codec | unreadable | corruptStream | unsupportedFormat | tooShort |";
const ONE_FAKE: &str = "albums with exactly one Fake among 2 or more analysed tracks: ";
const NO_ERRORS: &str = "errors: none";
const BANDS: [&str; 5] = ["< 1", "1 – 3", "3 – 6", "6 – 12", "≥ 12"];
const REASON_ROWS: [&str; 7] = [
    "lowHighFrequencyContent",
    "gradualRolloff",
    "ambiguousCutoff",
    "highBandGaps",
    "cutoffBelowDeclared",
    "streamingSampleRate",
    "none",
];
const CSV_HEADER: &str = "path,album_index,album_stratum,codec,sample_rate,bit_depth,channels,declared_bitrate_kbps,duration_ms,size_bytes,verdict,reasons,evidence,gap_step_margin_db,cutoff_step_margin_db,shortfall_margin_hz,cutoff_hz,error";

// ---------------------------------------------------------------------------------------------
// The census sample folder (census.md section 8)
// ---------------------------------------------------------------------------------------------

/// One row of the section 8 table: path under the root, fixture, container, and the expected
/// `tracks.csv` values with the defaults (verdict, evidence, reasons, error, album stratum).
struct SampleFile {
    rel: &'static str,
    fixture: &'static str,
    container: Container,
    verdict: &'static str,
    evidence: &'static str,
    reasons: &'static str,
    error: &'static str,
    stratum: &'static str,
}

const fn file(
    rel: &'static str,
    fixture: &'static str,
    verdict: &'static str,
    evidence: &'static str,
    stratum: &'static str,
) -> SampleFile {
    SampleFile {
        rel,
        fixture,
        container: Container::Flac,
        verdict,
        evidence,
        reasons: "",
        error: "",
        stratum,
    }
}

/// Tracks 1 to 15 of census.md section 8, in the table's order.
const SAMPLE: [SampleFile; 15] = [
    file(
        "Álbum A – todo falso/01 – Uno.flac",
        "shelf19k5",
        "fake",
        "cutoffStep",
        "whole",
    ),
    file(
        "Álbum A – todo falso/02 – Dos.flac",
        "sfb21_gaps",
        "fake",
        "gapStep",
        "whole",
    ),
    file(
        "Álbum A – todo falso/03 – Tres.flac",
        "shelf19k5",
        "fake",
        "cutoffStep",
        "whole",
    ),
    file(
        "Álbum A – todo falso/04 – Cuatro.flac",
        "sfb21_gaps",
        "fake",
        "gapStep",
        "whole",
    ),
    file(
        "Álbum B, aislado/01 – Canción ‘x’.flac",
        "full_band",
        "real",
        "",
        "isolated",
    ),
    file(
        "Álbum B, aislado/02 – Canción ‘x’.flac",
        "full_band",
        "real",
        "",
        "isolated",
    ),
    file(
        "Álbum B, aislado/03 – Canción ‘x’.flac",
        "full_band",
        "real",
        "",
        "isolated",
    ),
    file(
        "Álbum B, aislado/04 – Canción ‘x’.flac",
        "full_band",
        "real",
        "",
        "isolated",
    ),
    file(
        "Álbum B, aislado/05 – Falsa.flac",
        "shelf19k5",
        "fake",
        "cutoffStep",
        "isolated",
    ),
    file(
        "Álbum C/CD1/01 – Uno.flac",
        "full_band",
        "real",
        "",
        "mixed",
    ),
    file(
        "Álbum C/CD1/02 – Dos.flac",
        "sfb21_gaps",
        "fake",
        "gapStep",
        "mixed",
    ),
    SampleFile {
        reasons: "lowHighFrequencyContent",
        ..file(
            "Álbum C/CD2/01 – Suave.flac",
            "low_hf",
            "suspicious",
            "",
            "",
        )
    },
    SampleFile {
        error: "corruptStream",
        ..file("Álbum C/CD2/02 – Roto.flac", "corrupt", "error", "", "")
    },
    SampleFile {
        error: "tooShort",
        ..file("Álbum C/CD2/03 – Corto.flac", "short", "error", "", "")
    },
    SampleFile {
        container: Container::Wav,
        ..file("Sueltas/01 – Onda.wav", "full_band", "real", "", "")
    },
];

/// Track 9: the row whose path holds a comma.
const TRACK_9: &str = "Álbum B, aislado/05 – Falsa.flac";
/// Track 13: the corrupt one.
const TRACK_13: &str = "Álbum C/CD2/02 – Roto.flac";
/// Track 14: the short one (probed, then `tooShort`).
const TRACK_14: &str = "Álbum C/CD2/03 – Corto.flac";
/// The five columns of `tracks.csv` that hold values of the probe (census.md section 5).
const PROBE_COLUMNS: [&str; 5] = [
    "sample_rate",
    "bit_depth",
    "channels",
    "declared_bitrate_kbps",
    "duration_ms",
];
/// The file that is not audio.
const NOTES: &str = "Sueltas/notas.txt";

/// The census sample folder in `<temp>/fakeflac-calibrate-tests/<pid>-<name>/`, outside the
/// repository: `root` is the folder `Biblioteca ″Ñandú″ & co`; `base` also holds the stand-in
/// repository roots and the params files of the tests.
struct CensusSample {
    base: PathBuf,
    root: PathBuf,
}

impl CensusSample {
    fn new(name: &str) -> CensusSample {
        let base = scratch_dir(name);
        let root = base.join(ROOT_NAME);
        for f in &SAMPLE {
            place(f.fixture, f.container, &root.join(f.rel));
        }
        std::fs::write(root.join(NOTES), "notas de la carpeta\n").unwrap();
        CensusSample { base, root }
    }

    /// A fresh, empty stand-in for the repository root (`Env.repo_root`): the census writes its
    /// run folder under `<it>/target/census/`.
    fn repo(&self, name: &str) -> PathBuf {
        let repo = self.base.join(name);
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        repo
    }

    /// Sum of the sizes of the 15 tracks (not `notas.txt`).
    fn size_bytes(&self) -> u64 {
        SAMPLE
            .iter()
            .map(|f| std::fs::metadata(self.root.join(f.rel)).unwrap().len())
            .sum()
    }
}

/// Every entry under `dir`: files with size, modification time and content hash; folders `None`.
type FolderState = BTreeMap<String, Option<(u64, SystemTime, String)>>;

fn folder_state(dir: &Path) -> FolderState {
    snapshot(dir)
        .into_keys()
        .map(|rel| {
            let path = dir.join(&rel);
            let meta = std::fs::metadata(&path).unwrap();
            let value = (!meta.is_dir()).then(|| {
                (
                    meta.len(),
                    meta.modified().unwrap(),
                    id16(&std::fs::read(&path).unwrap()),
                )
            });
            (rel, value)
        })
        .collect()
}

/// The census through the library, with only the repository root set (D2: no corpus variables).
fn census(folder: &Path, params: Option<&Path>, repo: Option<&Path>) -> Outcome {
    run(
        &Command::Census(CensusArgs {
            folder: folder.to_path_buf(),
            params: params.map(Path::to_path_buf),
        }),
        &Env {
            corpus: None,
            calibration: None,
            ffmpeg: None,
            repo_root: repo.map(Path::to_path_buf),
        },
        &NoTranscoder,
    )
}

/// The run folder of an outcome, checked to be `<repo>/target/census/<its name>`.
fn run_dir(outcome: &Outcome, repo: &Path) -> PathBuf {
    let dir = outcome
        .report_dir
        .clone()
        .unwrap_or_else(|| panic!("no run folder; stderr: {}", outcome.stderr));
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let expected = repo.join("target").join("census").join(&name);
    assert!(expected.is_dir(), "{} is not a folder", expected.display());
    assert_eq!(
        std::fs::canonicalize(&dir).unwrap(),
        std::fs::canonicalize(&expected).unwrap(),
        "the run folder is <repository root>/target/census/<run>"
    );
    expected
}

fn run_name(dir: &Path) -> String {
    dir.file_name().unwrap().to_string_lossy().into_owned()
}

/// `YYYYMMDDTHHMMSSZ` of a Unix time in seconds (UTC, proleptic Gregorian).
fn stamp(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn is_stamp(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 16
        && b[8] == b'T'
        && b[15] == b'Z'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 8 || i == 15 || c.is_ascii_digit())
}

/// The census of the sample folder with the defaults, shared by the tests that only read it. The
/// sample's state is recorded before the run (C4).
struct SharedCensus {
    sample: CensusSample,
    repo: PathBuf,
    before: FolderState,
    started: i64,
    finished: i64,
    outcome: Outcome,
}

fn shared() -> &'static SharedCensus {
    static CELL: OnceLock<SharedCensus> = OnceLock::new();
    CELL.get_or_init(|| {
        let sample = CensusSample::new("census-shared");
        let repo = sample.repo("repo");
        let before = folder_state(&sample.root);
        let started = now_secs();
        let outcome = census(&sample.root, None, Some(&repo));
        let finished = now_secs();
        SharedCensus {
            sample,
            repo,
            before,
            started,
            finished,
            outcome,
        }
    })
}

impl SharedCensus {
    fn dir(&self) -> PathBuf {
        assert_eq!(self.outcome.exit_code, 0, "stderr: {}", self.outcome.stderr);
        run_dir(&self.outcome, &self.repo)
    }

    fn summary(&self) -> String {
        read(&self.dir().join("summary.md"))
    }
}

// ---------------------------------------------------------------------------------------------
// Reading summary.md
// ---------------------------------------------------------------------------------------------

fn cells(line: &str) -> Vec<String> {
    let inner = line.trim();
    let inner = inner.strip_prefix('|').unwrap_or(inner);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_table_line(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// A line as compared: a table line by its trimmed cells, any other line without trailing spaces.
fn norm(line: &str) -> String {
    if is_table_line(line) {
        format!("| {} |", cells(line).join(" | "))
    } else {
        line.trim_end().to_string()
    }
}

/// The non-blank lines of a text, normalised.
fn content_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(norm)
        .collect()
}

fn is_separator(line: &str) -> bool {
    is_table_line(line)
        && cells(line)
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-'))
}

/// The rows (trimmed cells) of the one table whose header is `header`.
fn table(text: &str, header: &str) -> Vec<Vec<String>> {
    let lines = content_lines(text);
    let header = norm(header);
    let at: Vec<usize> = (0..lines.len()).filter(|i| lines[*i] == header).collect();
    assert_eq!(at.len(), 1, "one table {header} in:\n{text}");
    let at = at[0];
    let width = cells(&header).len();
    assert!(
        lines.get(at + 1).is_some_and(|l| is_separator(l)),
        "{header} is followed by its separator line:\n{text}"
    );
    assert_eq!(
        cells(&lines[at + 1]).len(),
        width,
        "separator width of {header}"
    );
    let mut rows = Vec::new();
    let mut i = at + 2;
    while i < lines.len()
        && is_table_line(&lines[i])
        && !lines.get(i + 1).is_some_and(|l| is_separator(l))
    {
        let row = cells(&lines[i]);
        assert_eq!(row.len(), width, "row {} of {header}", lines[i]);
        rows.push(row);
        i += 1;
    }
    rows
}

fn has_table(text: &str, header: &str) -> bool {
    content_lines(text).contains(&norm(header))
}

/// The numbers of the row whose first cells are `key`.
fn row(rows: &[Vec<String>], key: &[&str]) -> Vec<u32> {
    let found = rows
        .iter()
        .find(|r| r.len() >= key.len() && r.iter().zip(key).all(|(c, k)| c == k))
        .unwrap_or_else(|| panic!("no row {key:?} in {rows:?}"));
    found[key.len()..]
        .iter()
        .map(|c| {
            c.parse::<u32>()
                .unwrap_or_else(|_| panic!("{c} is not a count"))
        })
        .collect()
}

/// The first `n` cells of every row, joined with " | ": the row keys in order.
fn keys(rows: &[Vec<String>], n: usize) -> Vec<String> {
    rows.iter().map(|r| r[..n].join(" | ")).collect()
}

fn one_fake_count(text: &str) -> u32 {
    let lines: Vec<String> = content_lines(text)
        .into_iter()
        .filter(|l| l.starts_with(ONE_FAKE))
        .collect();
    assert_eq!(lines.len(), 1, "one line {ONE_FAKE}… in:\n{text}");
    lines[0][ONE_FAKE.len()..].parse().unwrap()
}

/// Margin table as band -> [whole, mixed, isolated, total], with every band and `none`, in order.
fn margins(text: &str) -> BTreeMap<String, Vec<u32>> {
    let rows = table(text, H_MARGIN);
    let mut expected: Vec<String> = BANDS.iter().map(|b| (*b).to_string()).collect();
    expected.push("none".to_string());
    assert_eq!(keys(&rows, 1), expected, "margin rows in order");
    rows.iter()
        .map(|r| (r[0].clone(), row(&rows, &[r[0].as_str()])))
        .collect()
}

/// `stdout` is the text of `summary.md` followed by one line `elapsed: <seconds, one decimal> s`.
fn assert_stdout(stdout: &str, summary: &str) {
    let rest = stdout
        .strip_prefix(summary)
        .unwrap_or_else(|| panic!("stdout does not start with summary.md:\n{stdout}"));
    let rest = if summary.ends_with('\n') {
        rest
    } else {
        rest.strip_prefix('\n')
            .unwrap_or_else(|| panic!("no line break before elapsed: {rest:?}"))
    };
    let line = rest.strip_suffix('\n').unwrap_or(rest);
    assert!(!line.contains('\n'), "one line after summary.md: {rest:?}");
    let seconds = line
        .strip_prefix("elapsed: ")
        .and_then(|s| s.strip_suffix(" s"))
        .unwrap_or_else(|| panic!("not an elapsed line: {line:?}"));
    let (whole, decimal) = seconds
        .split_once('.')
        .unwrap_or_else(|| panic!("one decimal: {line:?}"));
    assert!(
        !whole.is_empty()
            && whole.chars().all(|c| c.is_ascii_digit())
            && decimal.len() == 1
            && decimal.chars().all(|c| c.is_ascii_digit()),
        "seconds with one decimal: {line:?}"
    );
}

/// The lines of `summary.md` that two runs share: all but `run:`.
fn without_run(text: &str) -> Vec<String> {
    content_lines(text)
        .into_iter()
        .filter(|l| !l.starts_with("run: "))
        .collect()
}

/// The tables of `summary.md`: all but the `run:` and `detection:` lines.
fn tables_only(text: &str) -> Vec<String> {
    without_run(text)
        .into_iter()
        .filter(|l| !l.starts_with("detection: "))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The run on the sample folder
// ---------------------------------------------------------------------------------------------

enum Expected {
    Line(String),
    /// `run: <the run folder's name>`.
    Run,
    /// A band row of the margin table: its numbers depend on the measured margins (section 8).
    Band(&'static str),
}

/// `summary.md` of the sample with the defaults, line by line (census.md section 8).
fn expected_sample(size_mb: &str) -> Vec<Expected> {
    let line = |s: &str| Expected::Line(norm(s));
    let mut v = vec![
        line(TITLE),
        Expected::Run,
        line("detection: algorithm_version 8, params default"),
        line(&format!("tracks 15 | albums 5 | {size_mb} MB")),
        line(H_CODEC),
        line("|---|---|---|---|---|---|"),
        line("| flac | 14 | 5 | 1 | 6 | 2 |"),
        line("| pcm | 1 | 1 | 0 | 0 | 0 |"),
        line("| total | 15 | 6 | 1 | 6 | 2 |"),
        line(H_EVIDENCE),
        line("|---|---|---|"),
        line("| lossless | gapStep | 3 |"),
        line("| lossless | cutoffStep | 3 |"),
        line("| lossless | gapStep + cutoffStep | 0 |"),
        line("| lossless | other | 0 |"),
        line("| lossy | shortfall + cutoffStep | 0 |"),
        line("| lossy | other | 0 |"),
        line(H_ALBUMS),
        line("|---|---|---|---|---|"),
        line("| whole | 1 | 4 | 4 | 0 |"),
        line("| mixed | 1 | 1 | 1 | 0 |"),
        line("| isolated | 1 | 1 | 1 | 0 |"),
        line("| total | 3 | 6 | 6 | 0 |"),
        line(&format!("{ONE_FAKE}2")),
        line(H_MARGIN),
        line("|---|---|---|---|---|"),
    ];
    v.extend(BANDS.iter().map(|b| Expected::Band(b)));
    v.push(line("| none | 0 | 0 | 0 | 0 |"));
    v.push(line(H_REASONS));
    v.push(line("|---|---|---|"));
    v.push(line("| lowHighFrequencyContent | 1 | 0 |"));
    for r in &REASON_ROWS[1..] {
        v.push(line(&format!("| {r} | 0 | 0 |")));
    }
    v.push(line(H_ERRORS));
    v.push(line("|---|---|---|---|---|"));
    v.push(line("| flac | 0 | 1 | 0 | 1 |"));
    v
}

/// Scenario: A census of the sample folder reports its verdicts by codec (@RP-12 @RNF-04).
/// Acceptance C1 (sample) and C9 (tracks 13 and 14 are errors and the run exits 0).
#[test]
fn rp12_c1_census_of_the_sample_reports_its_verdicts_by_codec() {
    let s = shared();
    assert_eq!(s.outcome.exit_code, 0, "stderr: {}", s.outcome.stderr);
    assert_eq!(s.outcome.stderr, "", "no error output");
    let dir = s.dir();
    let summary = s.summary();
    assert!(!summary.contains('\r'), "LF line ends");
    assert!(!summary.starts_with('\u{feff}'), "no byte-order mark");
    assert_stdout(&s.outcome.stdout, &summary);

    let size_mb = format!("{:.1}", s.sample.size_bytes() as f64 / 1_000_000.0);
    let expected = expected_sample(&size_mb);
    let lines = content_lines(&summary);
    assert_eq!(
        lines.len(),
        expected.len(),
        "summary.md has the lines of section 4 and no other:\n{summary}"
    );
    let mut columns = [0u32; 4];
    for (got, want) in lines.iter().zip(&expected) {
        match want {
            Expected::Line(l) => assert_eq!(got, l, "in:\n{summary}"),
            Expected::Run => assert_eq!(got, &format!("run: {}", run_name(&dir))),
            Expected::Band(band) => {
                let c = cells(got);
                assert_eq!(c[0], *band, "band rows in order:\n{summary}");
                let n: Vec<u32> = c[1..].iter().map(|v| v.parse().unwrap()).collect();
                assert_eq!(n.len(), 4, "{got}");
                assert_eq!(n[0] + n[1] + n[2], n[3], "row total of {got}");
                for (sum, v) in columns.iter_mut().zip(&n) {
                    *sum += v;
                }
            }
        }
    }
    assert_eq!(
        columns,
        [4, 1, 1, 6],
        "margin column totals whole, mixed, isolated, total"
    );
}

/// Scenario: The evidence of the Fake verdicts is counted by its kinds (@RP-12). Acceptance C1.
#[test]
fn rp12_c1_evidence_of_the_fake_verdicts_is_counted_by_its_kinds() {
    let summary = shared().summary();
    let rows = table(&summary, H_EVIDENCE);
    assert_eq!(
        rows,
        [
            ["lossless", "gapStep", "3"],
            ["lossless", "cutoffStep", "3"],
            ["lossless", "gapStep + cutoffStep", "0"],
            ["lossless", "other", "0"],
            ["lossy", "shortfall + cutoffStep", "0"],
            ["lossy", "other", "0"],
        ]
        .map(|r| r.map(str::to_string).to_vec())
    );
}

/// Scenario: Albums with a Fake are grouped by the share of their analysed tracks that is Fake
/// (@RP-12). Acceptance C1 (sections 4.4 and 4.5 of the sample).
#[test]
fn rp12_c1_albums_with_a_fake_are_grouped_by_their_share_of_fakes() {
    let summary = shared().summary();
    let rows = table(&summary, H_ALBUMS);
    assert_eq!(keys(&rows, 1), ["whole", "mixed", "isolated", "total"]);
    assert_eq!(row(&rows, &["whole"]), [1, 4, 4, 0]);
    assert_eq!(row(&rows, &["mixed"]), [1, 1, 1, 0]);
    assert_eq!(row(&rows, &["isolated"]), [1, 1, 1, 0]);
    assert_eq!(row(&rows, &["total"]), [3, 6, 6, 0]);
    assert_eq!(one_fake_count(&summary), 2);

    let m = margins(&summary);
    let mut columns = [0u32; 4];
    for band in BANDS {
        for (sum, v) in columns.iter_mut().zip(&m[band]) {
            *sum += v;
        }
    }
    assert_eq!(columns, [4, 1, 1, 6], "column totals of the margin table");
    assert_eq!(m["none"], [0, 0, 0, 0]);
}

/// Scenario: The reasons of the Suspicious verdicts are counted (@RP-12). Acceptance C1.
#[test]
fn rp12_c1_reasons_of_the_suspicious_verdicts_are_counted() {
    let summary = shared().summary();
    let rows = table(&summary, H_REASONS);
    assert_eq!(keys(&rows, 1), REASON_ROWS);
    assert_eq!(row(&rows, &["lowHighFrequencyContent"]), [1, 0]);
    for r in &REASON_ROWS[1..] {
        assert_eq!(row(&rows, &[r]), [0, 0], "{r}");
    }
}

/// Scenario: A census of the sample folder reports its verdicts by codec (@RP-12 @RNF-04), the error
/// table. Acceptance C1 and C9.
#[test]
fn rp12_c9_error_tracks_are_counted_and_never_stop_the_run() {
    let s = shared();
    assert_eq!(s.outcome.exit_code, 0, "stderr: {}", s.outcome.stderr);
    let summary = s.summary();
    let rows = table(&summary, H_ERRORS);
    assert_eq!(keys(&rows, 1), ["flac"]);
    assert_eq!(row(&rows, &["flac"]), [0, 1, 0, 1]);
    assert!(
        !content_lines(&summary).contains(&NO_ERRORS.to_string()),
        "no 'errors: none' line when there are errors"
    );
}

/// Scenario: The per-track file is sorted, quoted and relative to the folder (@RP-12 @RNF-09).
/// Acceptance C2.
#[test]
fn rp12_c2_per_track_file_is_sorted_quoted_and_relative() {
    let s = shared();
    let path = s.dir().join("tracks.csv");
    let text = read(&path);
    assert!(!text.starts_with('\u{feff}'), "no byte-order mark");
    assert!(!text.contains('\r'), "LF line ends");
    assert_eq!(text.lines().next(), Some(CSV_HEADER), "header of section 5");

    let csv = Table::read(&path);
    assert_eq!(csv.rows.len(), 15, "one row per track");
    let paths: Vec<&str> = csv.rows.iter().map(|r| csv.get(r, "path")).collect();
    let mut expected: Vec<&str> = SAMPLE.iter().map(|f| f.rel).collect();
    expected.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    assert_eq!(paths, expected, "sorted by the bytes of path, with '/'");

    // Track 9: its path holds a comma, so it is quoted.
    let quoted = format!("\"{TRACK_9}\",");
    assert!(
        text.lines().any(|l| l.starts_with(&quoted)),
        "the path of track 9 is quoted:\n{text}"
    );
    let r9 = csv.find("path", TRACK_9).unwrap();
    assert_eq!(csv.get(r9, "verdict"), "fake");
    assert_eq!(csv.get(r9, "evidence"), "cutoffStep");
    assert_eq!(csv.get(r9, "album_stratum"), "isolated");
    assert!(!csv.get(r9, "cutoff_step_margin_db").is_empty());
    assert_eq!(csv.get(r9, "gap_step_margin_db"), "");
    assert_eq!(csv.get(r9, "shortfall_margin_hz"), "");

    // Track 13: an error, with empty measurements.
    let r13 = csv.find("path", TRACK_13).unwrap();
    assert_eq!(csv.get(r13, "verdict"), "error");
    assert_eq!(csv.get(r13, "error"), "corruptStream");
    for column in PROBE_COLUMNS.into_iter().chain([
        "reasons",
        "evidence",
        "gap_step_margin_db",
        "cutoff_step_margin_db",
        "shortfall_margin_hz",
        "cutoff_hz",
    ]) {
        assert_eq!(csv.get(r13, column), "", "{column} of the corrupt track");
    }

    // No field holds the root's absolute path.
    let mut absolute = vec![s.sample.root.to_string_lossy().into_owned()];
    absolute.push(absolute[0].replace('\\', "/"));
    if let Ok(c) = std::fs::canonicalize(&s.sample.root) {
        absolute.push(c.to_string_lossy().into_owned());
    }
    for r in &csv.rows {
        for field in r {
            for a in &absolute {
                assert!(!field.contains(a.as_str()), "absolute path in {field}");
            }
        }
    }
}

/// Scenario: The per-track file is sorted, quoted and relative to the folder (@RP-12 @RNF-09):
/// every row has the verdict, evidence, reasons, error and album stratum of census.md section 8, the
/// margins are written in Rust's shortest `f32` form and the album index follows the folder.
/// Acceptance C2.
#[test]
fn rp12_c2_sample_rows_have_the_section_8_verdicts() {
    let s = shared();
    let csv = Table::read(&s.dir().join("tracks.csv"));
    for f in &SAMPLE {
        let r = csv
            .find("path", f.rel)
            .unwrap_or_else(|| panic!("no row {}", f.rel));
        let codec = if f.container == Container::Wav {
            "pcm"
        } else {
            "flac"
        };
        assert_eq!(csv.get(r, "codec"), codec, "{}", f.rel);
        assert_eq!(csv.get(r, "verdict"), f.verdict, "{}", f.rel);
        assert_eq!(csv.get(r, "evidence"), f.evidence, "{}", f.rel);
        assert_eq!(csv.get(r, "reasons"), f.reasons, "{}", f.rel);
        assert_eq!(csv.get(r, "error"), f.error, "{}", f.rel);
        assert_eq!(csv.get(r, "album_stratum"), f.stratum, "{}", f.rel);
        let gap = csv.get(r, "gap_step_margin_db");
        let step = csv.get(r, "cutoff_step_margin_db");
        assert_eq!(!gap.is_empty(), f.evidence == "gapStep", "{}", f.rel);
        assert_eq!(!step.is_empty(), f.evidence == "cutoffStep", "{}", f.rel);
        assert_eq!(csv.get(r, "shortfall_margin_hz"), "", "{}", f.rel);
        for column in [
            "gap_step_margin_db",
            "cutoff_step_margin_db",
            "shortfall_margin_hz",
            "cutoff_hz",
        ] {
            let v = csv.get(r, column);
            if !v.is_empty() {
                let parsed: f32 = v
                    .parse()
                    .unwrap_or_else(|_| panic!("{column} {v} of {}", f.rel));
                assert_eq!(format!("{parsed}"), v, "shortest f32 form of {column}");
            }
        }
    }
    // The album index is the same for the tracks of one folder and differs between folders.
    let mut by_folder: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &csv.rows {
        let path = csv.get(r, "path");
        let folder = path.rsplit_once('/').unwrap().0.to_string();
        by_folder
            .entry(folder)
            .or_default()
            .insert(csv.get(r, "album_index").to_string());
    }
    assert_eq!(by_folder.len(), 5, "five albums");
    let indexes: BTreeSet<&String> = by_folder.values().flatten().collect();
    assert!(by_folder.values().all(|i| i.len() == 1), "{by_folder:?}");
    assert_eq!(indexes.len(), 5, "one index per album: {by_folder:?}");
}

/// Scenario: An empty field of the per-track file means the value was not measured (@RP-12 @RNF-04).
/// Acceptance C2 (census.md section 5, "Empty means not measured"): the corrupt track 13 could not be
/// probed, so its five probe fields are empty; the short track 14 was probed and then failed
/// `tooShort`, so it keeps its real sample rate, channels and duration.
#[test]
fn rp12_c2_an_empty_field_of_the_per_track_file_means_not_measured() {
    let s = shared();
    let csv = Table::read(&s.dir().join("tracks.csv"));

    let r13 = csv.find("path", TRACK_13).unwrap();
    for column in PROBE_COLUMNS {
        assert_eq!(
            csv.get(r13, column),
            "",
            "{column} of the corrupt track is not measured"
        );
    }

    let r14 = csv.find("path", TRACK_14).unwrap();
    assert_eq!(csv.get(r14, "verdict"), "error");
    assert_eq!(csv.get(r14, "error"), "tooShort");
    for column in ["sample_rate", "channels", "duration_ms"] {
        let v = csv.get(r14, column);
        assert!(
            !v.is_empty() && v != "0",
            "{column} of the short track is its real value, not {v:?}"
        );
    }
}

/// The blocks of `summary.md` (census.md section 4, "Blocks and blank lines"): the text is the blocks,
/// each followed by one blank line, the last one included, with no blank line inside a block. Returns
/// the blocks, each as its lines.
fn blocks(text: &str) -> Vec<Vec<&str>> {
    assert!(!text.is_empty(), "an empty summary.md");
    let body = text.strip_suffix("\n\n").unwrap_or_else(|| {
        panic!("summary.md ends with its last line and one blank line: {text:?}")
    });
    body.split("\n\n")
        .map(|block| {
            let lines: Vec<&str> = block.split('\n').collect();
            assert!(
                lines.iter().all(|l| !l.trim().is_empty()),
                "one blank line between two blocks and none inside a block: {block:?} in\n{text}"
            );
            lines
        })
        .collect()
}

/// The blocks of section 4 in order: the four header lines, the tables of 4.2 to 4.4, the one-Fake
/// line, the tables of 4.5 and 4.6, and the errors block.
fn assert_blocks(text: &str, errors_table: bool) {
    let b = blocks(text);
    assert_eq!(b.len(), 11, "the eleven blocks of section 4:\n{text}");
    for (i, prefix) in [TITLE, "run: ", "detection: ", "tracks "]
        .iter()
        .enumerate()
    {
        assert_eq!(
            b[i].len(),
            1,
            "header line {i} is a block of its own:\n{text}"
        );
        assert!(b[i][0].starts_with(prefix), "{:?}", b[i]);
    }
    let is_table = |block: &[&str], header: &str| {
        block.len() >= 3
            && norm(block[0]) == norm(header)
            && is_separator(block[1])
            && block.iter().all(|l| is_table_line(l))
    };
    for (i, header) in [(4, H_CODEC), (5, H_EVIDENCE), (6, H_ALBUMS)] {
        assert!(is_table(&b[i], header), "block {i} is {header}:\n{text}");
    }
    assert_eq!(b[7].len(), 1, "the one-Fake line is a block:\n{text}");
    assert!(b[7][0].starts_with(ONE_FAKE), "{:?}", b[7]);
    assert!(is_table(&b[8], H_MARGIN), "block 8 is {H_MARGIN}:\n{text}");
    assert!(
        is_table(&b[9], H_REASONS),
        "block 9 is {H_REASONS}:\n{text}"
    );
    if errors_table {
        assert!(is_table(&b[10], H_ERRORS), "the error table:\n{text}");
    } else {
        assert_eq!(b[10], [NO_ERRORS], "the line 'errors: none':\n{text}");
    }
}

/// Section 4, "Blocks and blank lines": one blank line follows every block of `summary.md`, the last
/// one included, and there is no blank line inside a block; stdout is `summary.md` and then the
/// `elapsed:` line. On the sample folder (with its error table) and on given analyses (with `errors:
/// none`). Scenario: A census of the sample folder reports its verdicts by codec (@RP-12 @RNF-04).
/// Acceptance C1 and C5.
#[test]
fn rp12_c1_one_blank_line_follows_every_block_of_the_summary() {
    let s = shared();
    let summary = s.summary();
    assert_blocks(&summary, true);
    assert_stdout(&s.outcome.stdout, &summary);
    assert!(
        s.outcome.stdout.starts_with(&format!("{summary}elapsed: ")),
        "the elapsed: line follows the closing blank line of summary.md"
    );

    assert_blocks(&report(&[real(1), real(2)]), false);
    let mut error = track(3, Some(AudioCodec::Flac), Verdict::Error);
    error.analysis.error = Some(TrackError::TooShort);
    assert_blocks(&report(&[real(1), error]), true);
}

/// The names that must never appear in `summary.md` or stdout: the root's name, every folder and file
/// name and stem of the sample, and every component of 4 or more characters of the root's path.
fn private_names(sample: &CensusSample) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.insert(ROOT_NAME.to_string());
    for rel in SAMPLE.iter().map(|f| f.rel).chain([NOTES]) {
        let mut parts: Vec<&str> = rel.split('/').collect();
        let file = parts.pop().unwrap();
        names.insert(file.to_string());
        names.insert(file.rsplit_once('.').unwrap().0.to_string());
        for folder in parts {
            names.insert(folder.to_string());
        }
    }
    let mut roots = vec![sample.root.clone()];
    if let Ok(c) = std::fs::canonicalize(&sample.root) {
        roots.push(c);
    }
    for root in roots {
        for c in root.components() {
            if let std::path::Component::Normal(part) = c {
                let part = part.to_string_lossy().into_owned();
                if part.chars().count() >= 4 {
                    names.insert(part);
                }
            }
        }
    }
    names
}

/// Scenario: The summary names no folder and no file (@RP-12 @RNF-09). Acceptance C3.
#[test]
fn rp12_c3_summary_names_no_folder_and_no_file() {
    let s = shared();
    let summary = s.summary();
    let names = private_names(&s.sample);
    assert!(names.contains(ROOT_NAME));
    assert!(names.contains("Álbum B, aislado"));
    assert!(names.contains("02 – Roto"));
    for name in &names {
        assert!(!summary.contains(name.as_str()), "summary.md holds {name}");
        assert!(
            !s.outcome.stdout.contains(name.as_str()),
            "stdout holds {name}"
        );
    }
}

/// Scenario: The census leaves the folder untouched (@RP-12 @RNF-05). Acceptance C4.
#[test]
fn rp12_c4_census_leaves_the_folder_untouched() {
    let s = shared();
    let dir = s.dir();
    assert_eq!(
        folder_state(&s.sample.root),
        s.before,
        "same files, sizes, modification times and content hashes; no new file"
    );
    let run = run_name(&dir);
    assert_eq!(
        names(&s.repo),
        [
            "target".to_string(),
            "target/census".to_string(),
            format!("target/census/{run}"),
            format!("target/census/{run}/summary.md"),
            format!("target/census/{run}/tracks.csv"),
        ],
        "the only folder created is target/census/<run>/ with summary.md and tracks.csv"
    );
    // The run is the UTC start time of the run.
    assert!(is_stamp(&run), "{run} is YYYYMMDDTHHMMSSZ");
    assert!(
        stamp(s.started - 1) <= run && run <= stamp(s.finished + 1),
        "{run} is the start time of the run"
    );
}

/// Scenario: Two censuses of an unchanged folder are identical (@RP-12), and Scenario: Two runs
/// started in the same second get different folders (@RP-12). Acceptance C6.
#[test]
fn rp12_c6_two_censuses_of_an_unchanged_folder_are_identical() {
    let sample = CensusSample::new("census-twice");
    let repo = sample.repo("repo");
    let first = census(&sample.root, None, Some(&repo));
    assert_eq!(first.exit_code, 0, "stderr: {}", first.stderr);
    let first_dir = run_dir(&first, &repo);

    // A run of the same second already holds `<run>`: the second census gets `<run>-2`.
    let census_dir = repo.join("target").join("census");
    let now = now_secs();
    let mut created: BTreeSet<String> = BTreeSet::new();
    for secs in now - 2..=now + 120 {
        let name = stamp(secs);
        if std::fs::create_dir(census_dir.join(&name)).is_ok() {
            created.insert(name);
        }
    }
    let mut taken = created.clone();
    taken.insert(run_name(&first_dir));
    let second = census(&sample.root, None, Some(&repo));
    assert_eq!(second.exit_code, 0, "stderr: {}", second.stderr);
    let second_dir = run_dir(&second, &repo);
    let second_name = run_name(&second_dir);
    let base = second_name
        .strip_suffix("-2")
        .unwrap_or_else(|| panic!("{second_name} ends with -2"));
    assert!(
        taken.contains(base),
        "{second_name} is <run>-2 of a taken <run>"
    );
    for name in &created {
        assert!(
            names(&census_dir.join(name)).is_empty(),
            "{name} is left as it was"
        );
    }

    assert_eq!(
        std::fs::read(first_dir.join("tracks.csv")).unwrap(),
        std::fs::read(second_dir.join("tracks.csv")).unwrap(),
        "byte-identical tracks.csv"
    );
    let a = read(&first_dir.join("summary.md"));
    let b = read(&second_dir.join("summary.md"));
    assert_eq!(without_run(&a), without_run(&b), "equal but for run:");
    assert!(content_lines(&a).contains(&format!("run: {}", run_name(&first_dir))));
    assert!(content_lines(&b).contains(&format!("run: {second_name}")));
    let _ = std::fs::remove_dir_all(&sample.base);
}

// ---------------------------------------------------------------------------------------------
// Other params (C7) and refused params
// ---------------------------------------------------------------------------------------------

fn shipped_params() -> PathBuf {
    repo_root().join(DEFAULT_PARAMS)
}

/// Scenario: A copy of the shipped params is reported as custom and changes nothing (@RP-12).
/// Acceptance C7.
#[test]
fn rp12_c7_a_copy_of_the_shipped_params_is_custom_and_changes_nothing() {
    let s = shared();
    let default_summary = s.summary();
    let params = s.sample.base.join("Parámetros “copia” del envío.toml");
    std::fs::copy(shipped_params(), &params).unwrap();
    let id = id16(&std::fs::read(&params).unwrap());
    assert_eq!(id.len(), 16);
    let repo = s.sample.repo("repo-params-copy");
    let o = census(&s.sample.root, Some(&params), Some(&repo));
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
    let summary = read(&run_dir(&o, &repo).join("summary.md"));
    assert!(
        content_lines(&summary).contains(&format!(
            "detection: algorithm_version 8, params custom {id}"
        )),
        "header names the custom params by hash:\n{summary}"
    );
    assert_eq!(
        tables_only(&summary),
        tables_only(&default_summary),
        "every table equals the default run's"
    );
    for name in [
        "Parámetros “copia” del envío.toml",
        "Parámetros “copia” del envío",
        "detection-params",
    ] {
        assert!(!summary.contains(name), "summary.md names {name}");
        assert!(!o.stdout.contains(name), "stdout names {name}");
    }
}

/// Scenario: Another params file changes the census (@RP-12). Acceptance C7.
#[test]
fn rp12_c7_another_params_file_changes_the_census() {
    let s = shared();
    let text = read(&shipped_params());
    assert!(text.contains("gap_fake_db = 6.0"), "v8 sets gap_fake_db");
    let params = s.sample.base.join("gap_fake_db 100.toml");
    std::fs::write(
        &params,
        text.replacen("gap_fake_db = 6.0", "gap_fake_db = 100.0", 1),
    )
    .unwrap();
    let repo = s.sample.repo("repo-gap-100");
    let o = census(&s.sample.root, Some(&params), Some(&repo));
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
    let dir = run_dir(&o, &repo);
    let csv = Table::read(&dir.join("tracks.csv"));
    // Tracks 2, 4 and 11 are the ones made from sfb21_gaps.
    for f in SAMPLE.iter().filter(|f| f.fixture == "sfb21_gaps") {
        let r = csv.find("path", f.rel).unwrap();
        assert_ne!(csv.get(r, "verdict"), "fake", "{} is not Fake", f.rel);
    }
    let summary = read(&dir.join("summary.md"));
    let rows = table(&summary, H_EVIDENCE);
    assert_eq!(row(&rows, &["lossless", "gapStep"]), [0]);
}

/// Scenario: An invalid params file is refused as accept refuses it (@RP-12). Acceptance C8 (the
/// params row of the section 2 table): a file that does not validate and a file that cannot be read.
#[test]
fn rp12_c8_an_invalid_params_file_is_refused_as_accept_refuses_it() {
    let s = shared();
    let text = read(&shipped_params());
    let invalid = s.sample.base.join("inválido.toml");
    // Below gap_suspicious_db: DetectionParams::validate rejects it.
    std::fs::write(
        &invalid,
        text.replacen("gap_fake_db = 6.0", "gap_fake_db = 1.0", 1),
    )
    .unwrap();
    let missing = s.sample.base.join("no existe.toml");
    let accept_sample = Sample::new("census-accept-params");
    for params in [invalid, missing] {
        let accept = accept_sample.accept(std::slice::from_ref(&params));
        assert_eq!(accept.exit_code, 2, "accept refuses {}", params.display());
        assert!(!accept.stderr.is_empty());

        let repo = s.sample.repo("repo-invalid-params");
        let o = census(&s.sample.root, Some(&params), Some(&repo));
        assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
        assert_eq!(o.stderr, accept.stderr, "the message accept gives");
        assert_eq!(o.stdout, "");
        assert!(o.report_dir.is_none());
        assert!(names(&repo).is_empty(), "nothing under target/census/");
    }
}

/// Section 2: outside a repository the census exits 2 with its message and writes nothing.
/// Scenario: A folder that cannot be censused is refused without naming it (@RP-12), the
/// repository row of the exit-code table.
#[test]
fn rp12_c8_outside_a_repository_the_census_is_refused() {
    let s = shared();
    let o = census(&s.sample.root, None, None);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(o.stderr.lines().collect::<Vec<_>>(), [NO_REPOSITORY]);
    assert_eq!(o.stdout, "");
    assert!(o.report_dir.is_none());
    assert_eq!(folder_state(&s.sample.root), s.before, "nothing written");
}

// ---------------------------------------------------------------------------------------------
// Command line and binary (C8)
// ---------------------------------------------------------------------------------------------

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

/// Section 2: `census FOLDER [--params FILE]`, `--params` before or after `FOLDER`; every other
/// command line is a usage error with the single usage line (exit 2).
#[test]
fn rp12_c8_command_line_of_the_census() {
    let parsed = |v: &[&str]| match parse_args(&args(v)) {
        Ok(Command::Census(a)) => a,
        Ok(other) => panic!("{v:?} is not a census: {other:?}"),
        Err(e) => panic!("{v:?} refused: {e:?}"),
    };
    let a = parsed(&["census", "Mi música, 1"]);
    assert_eq!(a.folder, PathBuf::from("Mi música, 1"));
    assert_eq!(a.params, None);
    let a = parsed(&["census", "Mi música, 1", "--params", "p v9.toml"]);
    assert_eq!(a.folder, PathBuf::from("Mi música, 1"));
    assert_eq!(a.params, Some(PathBuf::from("p v9.toml")));
    let a = parsed(&["census", "--params", "p v9.toml", "Mi música, 1"]);
    assert_eq!(a.folder, PathBuf::from("Mi música, 1"));
    assert_eq!(a.params, Some(PathBuf::from("p v9.toml")));

    for bad in [
        &["census"][..],
        &["census", "a", "b"],
        &["census", "a", "--params"],
        &["census", "--params", "p.toml"],
        &["census", "a", "--params", "p.toml", "--params", "q.toml"],
        &["census", "a", "--verbose"],
    ] {
        let e = parse_args(&args(bad)).expect_err("usage error");
        assert_eq!(e.kind, ExitKind::Input, "{bad:?}");
        assert_eq!(e.lines, [USAGE], "{bad:?}");
    }
}

/// The binary from the repository root, without the calibration variables (D2). Every binary run of
/// this file holds `BINARY`, so a check of `target/census/` sees only its own run.
fn binary(args: &[OsString]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_fakeflac-calibrate"))
        .args(args)
        .current_dir(repo_root())
        .stdin(std::process::Stdio::null())
        .env_remove("FAKEFLAC_CORPUS")
        .env_remove("FAKEFLAC_CALIBRATION")
        .env_remove("FAKEFLAC_FFMPEG")
        .output()
        .unwrap()
}

static BINARY: Mutex<()> = Mutex::new(());

fn lock_binary() -> std::sync::MutexGuard<'static, ()> {
    BINARY.lock().unwrap_or_else(|e| e.into_inner())
}

/// The entries of the repository's `target/census/` (none when it does not exist).
fn repo_census() -> BTreeSet<String> {
    std::fs::read_dir(repo_root().join("target").join("census"))
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A small folder with one track, for the command-line cases.
fn small_folder(name: &str) -> (PathBuf, PathBuf) {
    let base = scratch_dir(name);
    let folder = base.join("Música ‘prueba’, 1");
    place(
        "full_band",
        Container::Flac,
        &folder.join("01 – Pista.flac"),
    );
    (base, folder)
}

/// Runs the binary with `census` and `rest`, expects exit 2, `line` as the whole stderr, an empty
/// stdout and no new entry under `target/census/`, and that stderr names none of `hidden`.
fn refused(rest: &[OsString], line: &str, hidden: &[&str]) {
    let _guard = lock_binary();
    let before = repo_census();
    let mut all = vec![OsString::from("census")];
    all.extend_from_slice(rest);
    let out = binary(&all);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(2), "{rest:?}: stderr {stderr}");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        [line],
        "stderr of {rest:?}"
    );
    assert_eq!(stdout, "", "stdout of {rest:?}");
    assert_eq!(repo_census(), before, "no folder under target/census/");
    for h in hidden {
        assert!(!stderr.contains(h), "stderr names {h}: {stderr}");
    }
}

/// Scenario Outline: A wrong command line prints the usage and writes nothing (@RP-12), row "no
/// argument". Acceptance C8.
#[test]
fn rp12_c8_usage_no_argument() {
    refused(&[], USAGE, &[]);
}

/// Scenario Outline: A wrong command line prints the usage and writes nothing (@RP-12), row "two
/// folders". Acceptance C8.
#[test]
fn rp12_c8_usage_two_folders() {
    let (base, folder) = small_folder("census-two-folders");
    let other = base.join("Otra carpeta");
    std::fs::create_dir_all(&other).unwrap();
    refused(&[folder.into(), other.into()], USAGE, &[]);
    let _ = std::fs::remove_dir_all(&base);
}

/// Scenario Outline: A wrong command line prints the usage and writes nothing (@RP-12), row "a
/// folder and --params without a value". Acceptance C8.
#[test]
fn rp12_c8_usage_params_without_a_value() {
    let (base, folder) = small_folder("census-params-no-value");
    refused(&[folder.into(), "--params".into()], USAGE, &[]);
    let _ = std::fs::remove_dir_all(&base);
}

/// Scenario Outline: A wrong command line prints the usage and writes nothing (@RP-12), row "a
/// folder and --params twice". Acceptance C8.
#[test]
fn rp12_c8_usage_params_twice() {
    let (base, folder) = small_folder("census-params-twice");
    let params = shipped_params();
    refused(
        &[
            folder.into(),
            "--params".into(),
            params.clone().into(),
            "--params".into(),
            params.into(),
        ],
        USAGE,
        &[],
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// Scenario Outline: A wrong command line prints the usage and writes nothing (@RP-12), row "a
/// folder and an unknown option". Acceptance C8.
#[test]
fn rp12_c8_usage_unknown_option() {
    let (base, folder) = small_folder("census-unknown-option");
    refused(&[folder.into(), "--verbose".into()], USAGE, &[]);
    let _ = std::fs::remove_dir_all(&base);
}

/// Scenario Outline: A folder that cannot be censused is refused without naming it (@RP-12), row "a
/// folder that does not exist". Acceptance C8.
#[test]
fn rp12_c8_a_folder_that_does_not_exist() {
    let base = scratch_dir("census-no-folder");
    let folder = base.join("No existe ‘ñ’, nunca");
    let path = folder.to_string_lossy().into_owned();
    refused(
        &[folder.into()],
        NOT_A_FOLDER,
        &["No existe ‘ñ’, nunca", &path],
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// Scenario Outline: A folder that cannot be censused is refused without naming it (@RP-12), row "a
/// file instead of a folder". Acceptance C8.
#[test]
fn rp12_c8_a_file_instead_of_a_folder() {
    let (base, folder) = small_folder("census-a-file");
    let file = folder.join("01 – Pista.flac");
    let before = folder_state(&folder);
    let path = file.to_string_lossy().into_owned();
    refused(&[file.into()], NOT_A_FOLDER, &["01 – Pista", &path]);
    assert_eq!(folder_state(&folder), before);
    let _ = std::fs::remove_dir_all(&base);
}

/// Scenario Outline: A folder that cannot be censused is refused without naming it (@RP-12), row "a
/// folder that holds only notas.txt". Acceptance C8.
#[test]
fn rp12_c8_a_folder_that_holds_only_notes() {
    let base = scratch_dir("census-only-notes");
    let folder = base.join("Sólo notas & más");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("notas.txt"), "notas\n").unwrap();
    let before = folder_state(&folder);
    let path = folder.to_string_lossy().into_owned();
    refused(
        &[folder.clone().into()],
        NO_AUDIO,
        &["Sólo notas & más", &path],
    );
    assert_eq!(folder_state(&folder), before, "nothing written");
    let _ = std::fs::remove_dir_all(&base);
}

/// Removes a run folder of the repository's `target/census/` when dropped.
struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Scenario: A census of the sample folder reports its verdicts by codec (@RP-12 @RNF-04), through
/// the binary run from the repository root: exit 0, the run folder under the repository's
/// `target/census/`, stdout is its `summary.md` followed by the `elapsed:` line. Acceptance C1.
#[test]
fn rp12_c1_the_binary_censuses_the_sample_from_the_repository() {
    let s = shared();
    let library = s.summary();
    let _guard = lock_binary();
    let before = repo_census();
    let out = binary(&["census".into(), s.sample.root.clone().into()]);
    let added: Vec<String> = repo_census().difference(&before).cloned().collect();
    let _cleanup: Vec<RemoveOnDrop> = added
        .iter()
        .map(|n| RemoveOnDrop(repo_root().join("target").join("census").join(n)))
        .collect();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "stderr: {stderr}");
    assert_eq!(stderr, "");
    assert_eq!(added.len(), 1, "one run folder: {added:?}");
    let dir = repo_root().join("target").join("census").join(&added[0]);
    assert_eq!(
        names(&dir),
        ["summary.md".to_string(), "tracks.csv".to_string()]
    );
    let summary = read(&dir.join("summary.md"));
    assert_stdout(&stdout, &summary);
    assert_eq!(
        without_run(&summary),
        without_run(&library),
        "the same census as the library's"
    );
    assert_eq!(
        std::fs::read(dir.join("tracks.csv")).unwrap(),
        std::fs::read(s.dir().join("tracks.csv")).unwrap(),
        "the same tracks.csv as the library's"
    );
}

// ---------------------------------------------------------------------------------------------
// The folder must not contain the report folder (C11)
// ---------------------------------------------------------------------------------------------

/// The setting of C11: a fresh census sample in its own `<base>` (outside the repository), and an
/// empty stand-in repository root `<base>/repo` beside the sample folder. `<base>` is removed when
/// the guard is dropped, also when the test fails. The census of these tests gets the stand-in as
/// `Env.repo_root`, so nothing is ever written under the repository's own `target/`.
fn guard_setting(name: &str) -> (CensusSample, PathBuf, RemoveOnDrop) {
    let sample = CensusSample::new(name);
    let repo = sample.repo("repo");
    let cleanup = RemoveOnDrop(sample.base.clone());
    (sample, repo, cleanup)
}

/// A `FOLDER` refused by the report-folder check (census.md section 2): exit 2, exactly the stderr
/// line of the exit-code table (it names no path), an empty stdout, no run folder, and the same
/// files and folders under `<base>`, with the same sizes, modification times and content hashes,
/// before and after the call (no `target/census/<run>/`, and no `target/census/` where there was
/// none).
fn assert_contains_the_report_folder(base: &Path, repo: &Path, folder: &Path) {
    let before = folder_state(base);
    let o = census(folder, None, Some(repo));
    assert_eq!(
        o.exit_code, 2,
        "the folder contains the report folder: exit 2; stderr: {}",
        o.stderr
    );
    assert_eq!(
        o.stderr.lines().collect::<Vec<_>>(),
        [CONTAINS_REPORT],
        "the exact stderr of the report-folder check"
    );
    assert_eq!(o.stdout, "", "stdout is empty");
    assert!(o.report_dir.is_none(), "no run folder");
    assert_eq!(
        folder_state(base),
        before,
        "nothing is created, changed or removed under the temporary folder"
    );
}

/// A `FOLDER` that the report-folder check does not refuse: stderr does not hold its line, the run
/// exits 0 and creates exactly one run folder, `<repo>/target/census/<run>/` with `summary.md` and
/// `tracks.csv`. Returns that run folder.
fn assert_not_refused_by_the_check(repo: &Path, folder: &Path) -> PathBuf {
    let o = census(folder, None, Some(repo));
    assert!(
        !o.stderr.contains(CONTAINS_REPORT),
        "not refused by the report-folder check: {}",
        o.stderr
    );
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
    let dir = run_dir(&o, repo);
    let runs: Vec<String> = std::fs::read_dir(repo.join("target").join("census"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        runs,
        [run_name(&dir)],
        "one run folder under target/census/"
    );
    assert_eq!(
        names(&dir),
        ["summary.md".to_string(), "tracks.csv".to_string()]
    );
    dir
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "the temporary folder that holds the sample folder and the
/// stand-in root": `<base>` is an ancestor of the stand-in root and also holds the sample's audio.
/// Acceptance C11.
#[test]
fn rp12_c11_an_ancestor_of_the_repository_root_is_refused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-ancestor");
    assert!(repo.starts_with(&sample.base) && sample.root.starts_with(&sample.base));
    assert!(!repo.join("target").exists(), "no target/ before the call");
    assert_contains_the_report_folder(&sample.base, &repo, &sample.base);
    assert!(!repo.join("target").exists(), "no target/ after the call");
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "the stand-in root": `FOLDER` is the repository root itself (as
/// `just census .` from the repository root). Acceptance C11.
#[test]
fn rp12_c11_the_repository_root_is_refused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-root");
    assert!(!repo.join("target").exists(), "no target/ before the call");
    assert_contains_the_report_folder(&sample.base, &repo, &repo);
    assert!(!repo.join("target").exists(), "no target/ after the call");
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "the stand-in root's empty target folder": `FOLDER` is
/// `<repository root>/target`, created empty by the test, before any census has created
/// `target/census/` (the check holds on the parts of the report folder that do not exist yet).
/// Acceptance C11.
#[test]
fn rp12_c11_the_target_folder_of_the_repository_is_refused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-target");
    let target = repo.join("target");
    std::fs::create_dir(&target).unwrap();
    assert_contains_the_report_folder(&sample.base, &repo, &target);
    assert!(
        !target.join("census").exists(),
        "no target/census/ where there was none"
    );
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), the case "`FOLDER` is `target/census/`" of the exit-code table of
/// census.md section 2 (the report folder itself), created empty by the test rather than by a
/// run. Acceptance C11.
#[test]
fn rp12_c11_the_report_folder_itself_is_refused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-report");
    let report = repo.join("target").join("census");
    std::fs::create_dir_all(&report).unwrap();
    assert_contains_the_report_folder(&sample.base, &repo, &report);
    assert!(names(&report).is_empty(), "target/census/ stays empty");
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "the sample folder": a folder outside the stand-in root is not
/// refused by the check; the census exits 0 as in C1. Acceptance C11.
#[test]
fn rp12_c11_the_sample_folder_beside_the_repository_is_censused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-sample");
    let before = folder_state(&sample.root);
    let dir = assert_not_refused_by_the_check(&repo, &sample.root);
    assert_eq!(
        without_run(&read(&dir.join("summary.md"))),
        without_run(&shared().summary()),
        "the census of C1"
    );
    assert_eq!(
        folder_state(&sample.root),
        before,
        "the sample is untouched"
    );
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "a fixtures folder inside the stand-in root holding one WAV
/// track": a folder inside the repository, beside `target/`, does not contain `target/census/` and
/// is censused. Acceptance C11.
#[test]
fn rp12_c11_a_fixtures_folder_inside_the_repository_is_censused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-fixtures");
    let fixtures = repo.join("fixtures");
    std::fs::create_dir(&fixtures).unwrap();
    std::fs::copy(
        sample.root.join("Sueltas/01 – Onda.wav"),
        fixtures.join("01 – Onda.wav"),
    )
    .unwrap();
    let before = folder_state(&fixtures);
    let dir = assert_not_refused_by_the_check(&repo, &fixtures);
    let csv = Table::read(&dir.join("tracks.csv"));
    assert_eq!(csv.rows.len(), 1, "one track");
    let r = csv.find("path", "01 – Onda.wav").unwrap();
    assert_eq!(csv.get(r, "codec"), "pcm");
    assert_eq!(csv.get(r, "verdict"), "real");
    assert_eq!(
        folder_state(&fixtures),
        before,
        "the fixtures folder is untouched"
    );
    assert_eq!(
        names(&repo)
            .into_iter()
            .filter(|n| !n.starts_with("target"))
            .collect::<Vec<_>>(),
        ["fixtures".to_string(), "fixtures/01 – Onda.wav".to_string()],
        "nothing written beside target/"
    );
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "the stand-in root spelled with a \"..\" segment": `FOLDER` is
/// `<stand-in>/../<stand-in name>`, which resolves to the stand-in root, so the comparison is on
/// the canonical forms, not on the spellings. Acceptance C11 (a).
#[test]
fn rp12_c11_the_repository_root_spelled_with_a_dot_dot_segment_is_refused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-dotdot");
    let spelled = repo.join("..").join(repo.file_name().unwrap());
    assert!(
        spelled
            .components()
            .any(|c| c == std::path::Component::ParentDir),
        "the spelling keeps its .. segment: {}",
        spelled.display()
    );
    assert!(spelled.is_dir(), "the spelling names the stand-in root");
    assert!(!repo.join("target").exists(), "no target/ before the call");
    assert_contains_the_report_folder(&sample.base, &repo, &spelled);
    assert!(!repo.join("target").exists(), "no target/ after the call");
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05 @RNF-07), Examples: Windows, row "the stand-in root spelled in a
/// different letter case": `FOLDER` is the stand-in root with its last component upper-cased, the
/// same folder on Windows, so it is refused the same way. Not run on other platforms. Acceptance
/// C11 (b).
#[cfg(windows)]
#[test]
fn rp12_c11_the_repository_root_in_a_different_letter_case_is_refused() {
    let (sample, repo, _cleanup) = guard_setting("census-guard-case");
    let upper = repo.file_name().unwrap().to_str().unwrap().to_uppercase();
    let spelled = repo.with_file_name(&upper);
    assert_ne!(
        spelled.as_os_str(),
        repo.as_os_str(),
        "a different spelling of the stand-in root"
    );
    assert!(
        spelled.is_dir(),
        "the temporary folder is case-insensitive, so {} names the stand-in root",
        spelled.display()
    );
    assert!(!repo.join("target").exists(), "no target/ before the call");
    assert_contains_the_report_folder(&sample.base, &repo, &spelled);
    assert!(!repo.join("target").exists(), "no target/ after the call");
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05), row "the sample folder" with "a stand-in root that does not exist":
/// the repository root cannot be put in canonical form, so the check does not refuse the run
/// (fail-open, census.md section 2). The census exits 0 with the `summary.md` of C1 (every line
/// but `run:`), and the tool creates exactly one run folder `<that root>/target/census/<run>/`
/// with `summary.md` and `tracks.csv`, together with the missing root, `target/` and `census/`.
/// Acceptance C11 (c).
#[test]
fn rp12_c11_a_repository_root_that_does_not_exist_does_not_refuse_the_run() {
    let (sample, _repo, _cleanup) = guard_setting("census-guard-missing-root");
    let missing = sample.base.join("missing-repo");
    assert!(!missing.exists(), "the stand-in root is not created");
    let before = folder_state(&sample.root);
    let dir = assert_not_refused_by_the_check(&missing, &sample.root);
    assert_eq!(
        without_run(&read(&dir.join("summary.md"))),
        without_run(&shared().summary()),
        "the census of C1"
    );
    let run = run_name(&dir);
    assert_eq!(
        names(&missing),
        [
            "target".to_string(),
            "target/census".to_string(),
            format!("target/census/{run}"),
            format!("target/census/{run}/summary.md"),
            format!("target/census/{run}/tracks.csv"),
        ],
        "only the run folder and its missing parents are created"
    );
    assert_eq!(
        folder_state(&sample.root),
        before,
        "the sample is untouched"
    );
}

/// The setting of C11 (d): the setting of `guard_setting` (in `<base>`, outside the repository)
/// with `<stand-in>/target` a junction to the empty folder `<base>/redirected`. When dropped, it
/// removes the junction itself first (`remove_dir` on the link, never a recursive removal through
/// it), then `<base>`.
#[cfg(windows)]
struct JunctionSetting {
    sample: CensusSample,
    repo: PathBuf,
    redirected: PathBuf,
}

#[cfg(windows)]
impl Drop for JunctionSetting {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(self.repo.join("target"));
        let _ = std::fs::remove_dir_all(&self.sample.base);
    }
}

/// The setting of C11 (d), or `None` (with the reason printed: the case is skipped, as census.md
/// C11 (d) says) when the junction cannot be created. The junction is made with `mklink /J`
/// through `cmd`, no new dependency.
#[cfg(windows)]
fn junction_setting(name: &str) -> Option<JunctionSetting> {
    use std::os::windows::process::CommandExt;
    let sample = CensusSample::new(name);
    let repo = sample.repo("repo");
    let redirected = sample.base.join("redirected");
    std::fs::create_dir(&redirected).unwrap();
    let link = repo.join("target");
    let setting = JunctionSetting {
        sample,
        repo,
        redirected,
    };
    let quoted = |p: &Path| format!("\"{}\"", p.display());
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .raw_arg(quoted(&link))
        .raw_arg(quoted(&setting.redirected))
        .output();
    let why = match made {
        Err(e) => Some(format!("cmd could not be started: {e}")),
        Ok(out) if !out.status.success() => Some(format!(
            "mklink /J failed ({}): {} {}",
            out.status,
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Ok(_) => {
            let is_link = std::fs::symlink_metadata(&link)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            let resolves = std::fs::canonicalize(&link).ok()
                == std::fs::canonicalize(&setting.redirected).ok();
            (!is_link || !resolves).then(|| {
                format!(
                    "mklink /J did not make {} a junction to {}",
                    link.display(),
                    setting.redirected.display()
                )
            })
        }
    };
    match why {
        Some(why) => {
            eprintln!("skipped: census.md C11 (d) needs a junction for target/: {why}");
            None
        }
        None => Some(setting),
    }
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05 @RNF-07), Examples: Windows, row "the folder \"redirected\" beside the
/// stand-in root" with "the stand-in, whose \"target\" is a junction to the empty folder
/// \"redirected\"": the report folder is the canonical `<base>/redirected/census`, so its redirect
/// target is refused although it lies outside the stand-in root. Acceptance C11 (d).
#[cfg(windows)]
#[test]
fn rp12_c11_the_redirect_target_of_a_junction_target_is_refused() {
    let Some(s) = junction_setting("census-guard-junction-target") else {
        return;
    };
    assert_contains_the_report_folder(&s.sample.base, &s.repo, &s.redirected);
    assert!(names(&s.redirected).is_empty(), "redirected/ stays empty");
}

/// Scenario Outline: A folder that contains the report folder is refused before anything is
/// written (@RP-12 @RNF-05 @RNF-07), Examples: Windows, row "the stand-in root" with "the
/// stand-in, whose \"target\" is a junction to the empty folder \"redirected\"": the redirected
/// report folder `<base>/redirected/census` is not inside the stand-in root, so the check does not
/// refuse it; the scan does not follow the junction and the stand-in root holds no audio, so the
/// census exits 2 with the line of a folder without audio and writes nothing (no
/// `<base>/redirected/census/`). Acceptance C11 (d).
#[cfg(windows)]
#[test]
fn rp12_c11_the_repository_root_with_a_junction_target_is_not_refused_by_the_check() {
    let Some(s) = junction_setting("census-guard-junction-root") else {
        return;
    };
    let before = folder_state(&s.sample.base);
    let o = census(&s.repo, None, Some(&s.repo));
    assert!(
        !o.stderr.contains(CONTAINS_REPORT),
        "not refused by the report-folder check: {}",
        o.stderr
    );
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        o.stderr.lines().collect::<Vec<_>>(),
        [NO_AUDIO],
        "the exact stderr of a folder without audio"
    );
    assert_eq!(o.stdout, "", "stdout is empty");
    assert!(o.report_dir.is_none(), "no run folder");
    assert_eq!(
        folder_state(&s.sample.base),
        before,
        "nothing is created, changed or removed under the temporary folder"
    );
    assert!(
        !s.redirected.join("census").exists(),
        "no redirected/census/"
    );
}

// ---------------------------------------------------------------------------------------------
// The report builder on given analyses (C5)
// ---------------------------------------------------------------------------------------------

static NEXT_TRACK: AtomicU32 = AtomicU32::new(1);

/// A track of album `album` (folder `Carpeta secreta <album>`), 1 000 000 bytes.
fn track(album: u32, codec: Option<AudioCodec>, verdict: Verdict) -> CensusTrack {
    let n = NEXT_TRACK.fetch_add(1, Ordering::SeqCst);
    let format = match codec {
        Some(AudioCodec::Pcm) => AudioFormat::Wav,
        Some(AudioCodec::Flac) => AudioFormat::Flac,
        Some(AudioCodec::Mp3) => AudioFormat::Mp3,
        Some(AudioCodec::Opus) => AudioFormat::Opus,
        Some(AudioCodec::Alac | AudioCodec::Aac) | None => AudioFormat::M4a,
    };
    let lossy = codec.is_some_and(AudioCodec::is_lossy);
    let folder = PathBuf::from(format!("Carpeta secreta {album}"));
    let info = TrackInfo {
        path: folder.join(format!("Pista secreta {n}.x")),
        folder,
        format,
        codec,
        sample_rate: 44_100,
        bit_depth: (!lossy).then_some(16),
        channels: 2,
        declared_bitrate_kbps: lossy.then_some(320),
        bitrate_mode: None,
        duration_ms: 180_000,
        size_bytes: 1_000_000,
        modified_unix: 0,
    };
    CensusTrack {
        analysis: TrackAnalysis {
            info,
            verdict,
            cutoff: None,
            estimated_source_bitrate_kbps: None,
            reasons: Vec::new(),
            evidence: Vec::new(),
            error: (verdict == Verdict::Error)
                .then(|| TrackError::CorruptStream("synthetic".to_string())),
            algorithm_version: 8,
        },
        album,
    }
}

/// An evidence item with the given margin (the value and threshold follow the defaults).
fn ev(kind: EvidenceKind, margin: f32) -> FakeEvidence {
    let threshold = match kind {
        EvidenceKind::GapStep => 6.0,
        EvidenceKind::CutoffStep => 10.0,
        EvidenceKind::Shortfall => 2000.0,
    };
    FakeEvidence {
        kind,
        value: threshold + margin,
        threshold,
        margin,
    }
}

fn fake(album: u32, codec: AudioCodec, evidence: Vec<FakeEvidence>) -> CensusTrack {
    let mut t = track(album, Some(codec), Verdict::Fake);
    t.analysis.evidence = evidence;
    t
}

fn lossless_fake(album: u32, evidence: Vec<FakeEvidence>) -> CensusTrack {
    fake(album, AudioCodec::Flac, evidence)
}

fn real(album: u32) -> CensusTrack {
    track(album, Some(AudioCodec::Flac), Verdict::Real)
}

/// The album summaries of the tracks, as the use case would send them with the last `TrackDone`.
fn albums_of(tracks: &[CensusTrack]) -> BTreeMap<u32, AlbumSummary> {
    let analyses: Vec<TrackAnalysis> = tracks.iter().map(|t| t.analysis.clone()).collect();
    tracks
        .iter()
        .map(|t| {
            let summary = summarize_album(&t.analysis.info.folder, &analyses).unwrap();
            (t.album, summary)
        })
        .collect()
}

fn header(params_id: Option<&str>) -> CensusHeader {
    CensusHeader {
        run: "20261115T120000Z".to_string(),
        algorithm_version: 8,
        params_id: params_id.map(str::to_string),
    }
}

fn report(tracks: &[CensusTrack]) -> String {
    summary_md(&header(None), tracks, &albums_of(tracks))
}

/// Scenario Outline: A lossless Fake is banded by its largest margin (@RP-12), every example row.
/// Acceptance C5 (margins 0.0 to 40.0 dB, two per band).
#[test]
fn rp12_c5_a_lossless_fake_is_banded_by_its_largest_margin() {
    let examples: [(f32, &str); 10] = [
        (0.0, "< 1"),
        (0.99, "< 1"),
        (1.0, "1 – 3"),
        (2.99, "1 – 3"),
        (3.0, "3 – 6"),
        (5.99, "3 – 6"),
        (6.0, "6 – 12"),
        (11.99, "6 – 12"),
        (12.0, "≥ 12"),
        (40.0, "≥ 12"),
    ];
    // One track at a time: an album of one Fake (whole).
    for (i, (margin, band)) in examples.iter().enumerate() {
        let kind = if i % 2 == 0 {
            EvidenceKind::CutoffStep
        } else {
            EvidenceKind::GapStep
        };
        let text = report(&[lossless_fake(1, vec![ev(kind, *margin)])]);
        for (b, counts) in margins(&text) {
            let want = if b == *band {
                vec![1, 0, 0, 1]
            } else {
                vec![0, 0, 0, 0]
            };
            assert_eq!(counts, want, "margin {margin} in band {b}:\n{text}");
        }
    }
    // All ten in one report: two per band.
    let tracks: Vec<CensusTrack> = examples
        .iter()
        .enumerate()
        .map(|(i, (m, _))| lossless_fake(i as u32, vec![ev(EvidenceKind::CutoffStep, *m)]))
        .collect();
    let text = report(&tracks);
    let m = margins(&text);
    for band in BANDS {
        assert_eq!(m[band], [2, 0, 0, 2], "{band}:\n{text}");
    }
    assert_eq!(m["none"], [0, 0, 0, 0]);
}

/// Scenario: A track with both kinds is banded by the larger margin (@RP-12). Acceptance C5.
#[test]
fn rp12_c5_a_track_with_both_kinds_is_banded_by_the_larger_margin() {
    let text = report(&[lossless_fake(
        1,
        vec![
            ev(EvidenceKind::GapStep, 2.0),
            ev(EvidenceKind::CutoffStep, 7.5),
        ],
    )]);
    let m = margins(&text);
    assert_eq!(m["6 – 12"], [1, 0, 0, 1], "{text}");
    assert_eq!(m["1 – 3"], [0, 0, 0, 0], "{text}");
    let rows = table(&text, H_EVIDENCE);
    assert_eq!(row(&rows, &["lossless", "gapStep + cutoffStep"]), [1]);
    assert_eq!(row(&rows, &["lossless", "gapStep"]), [0]);
    assert_eq!(row(&rows, &["lossless", "cutoffStep"]), [0]);

    // The set of kinds counts, not their order (section 4.3).
    let text = report(&[lossless_fake(
        1,
        vec![
            ev(EvidenceKind::CutoffStep, 7.5),
            ev(EvidenceKind::GapStep, 2.0),
        ],
    )]);
    let rows = table(&text, H_EVIDENCE);
    assert_eq!(row(&rows, &["lossless", "gapStep + cutoffStep"]), [1]);
    assert_eq!(margins(&text)["6 – 12"], [1, 0, 0, 1]);
}

/// An album of `f` lossless fakes (cutoffStep margin 4 dB) and `n - f` real tracks.
fn album(index: u32, f: u32, n: u32) -> Vec<CensusTrack> {
    let mut tracks: Vec<CensusTrack> = (0..f)
        .map(|_| lossless_fake(index, vec![ev(EvidenceKind::CutoffStep, 4.0)]))
        .collect();
    tracks.extend((f..n).map(|_| real(index)));
    tracks
}

/// Scenario Outline: The stratum of an album compares integers exactly (@RP-12), every example row,
/// one album at a time and all six together. Acceptance C5.
#[test]
fn rp12_c5_the_stratum_of_an_album_compares_integers_exactly() {
    let examples: [(u32, u32, &str); 6] = [
        (3, 4, "whole"),
        (1, 1, "whole"),
        (2, 3, "mixed"),
        (1, 4, "mixed"),
        (1, 5, "isolated"),
        (2, 9, "isolated"),
    ];
    for (f, n, stratum) in examples {
        let text = report(&album(1, f, n));
        let rows = table(&text, H_ALBUMS);
        for s in ["whole", "mixed", "isolated"] {
            let want = if s == stratum {
                vec![1, f, f, 0]
            } else {
                vec![0, 0, 0, 0]
            };
            assert_eq!(row(&rows, &[s]), want, "({f}, {n}) in {s}:\n{text}");
        }
        assert_eq!(row(&rows, &["total"]), [1, f, f, 0]);
        let column = ["whole", "mixed", "isolated"]
            .iter()
            .position(|s| *s == stratum)
            .unwrap();
        let mut want = vec![0, 0, 0, f];
        want[column] = f;
        assert_eq!(margins(&text)["3 – 6"], want, "({f}, {n}):\n{text}");
    }

    let tracks: Vec<CensusTrack> = examples
        .iter()
        .enumerate()
        .flat_map(|(i, (f, n, _))| album(i as u32, *f, *n))
        .collect();
    let text = report(&tracks);
    let rows = table(&text, H_ALBUMS);
    assert_eq!(row(&rows, &["whole"]), [2, 4, 4, 0]);
    assert_eq!(row(&rows, &["mixed"]), [2, 3, 3, 0]);
    assert_eq!(row(&rows, &["isolated"]), [2, 3, 3, 0]);
    assert_eq!(row(&rows, &["total"]), [6, 10, 10, 0]);
    assert_eq!(one_fake_count(&text), 2, "(1, 4) and (1, 5); not (1, 1)");
    assert_eq!(margins(&text)["3 – 6"], [4, 3, 3, 10]);
}

/// Scenario: Error tracks are left out of an album's share (@RP-12 @RNF-04). Acceptance C5.
#[test]
fn rp12_c5_error_tracks_are_left_out_of_an_albums_share() {
    let mut tracks = vec![lossless_fake(1, vec![ev(EvidenceKind::GapStep, 5.0)])];
    tracks.extend((0..3).map(|_| track(1, Some(AudioCodec::Flac), Verdict::Error)));
    let text = report(&tracks);
    let rows = table(&text, H_ALBUMS);
    assert_eq!(row(&rows, &["whole"]), [1, 1, 1, 0], "{text}");
    assert_eq!(row(&rows, &["mixed"]), [0, 0, 0, 0]);
    assert_eq!(row(&rows, &["isolated"]), [0, 0, 0, 0]);
    assert_eq!(one_fake_count(&text), 0, "n = 1");
}

/// Scenario: Lossy fakes are counted but not banded (@RP-12). Acceptance C5.
#[test]
fn rp12_c5_lossy_fakes_are_counted_but_not_banded() {
    let tracks: Vec<CensusTrack> = [AudioCodec::Mp3, AudioCodec::Aac, AudioCodec::Opus]
        .into_iter()
        .map(|codec| {
            fake(
                1,
                codec,
                vec![
                    ev(EvidenceKind::Shortfall, 1500.0),
                    ev(EvidenceKind::CutoffStep, 4.0),
                ],
            )
        })
        .collect();
    let text = report(&tracks);
    let evidence = table(&text, H_EVIDENCE);
    assert_eq!(row(&evidence, &["lossy", "shortfall + cutoffStep"]), [3]);
    assert_eq!(row(&evidence, &["lossy", "other"]), [0]);
    for e in ["gapStep", "cutoffStep", "gapStep + cutoffStep", "other"] {
        assert_eq!(row(&evidence, &["lossless", e]), [0], "{e}");
    }
    let albums = table(&text, H_ALBUMS);
    assert_eq!(row(&albums, &["whole"]), [1, 3, 0, 3], "{text}");
    assert_eq!(row(&albums, &["total"]), [1, 3, 0, 3]);
    for (band, counts) in margins(&text) {
        assert_eq!(counts, [0, 0, 0, 0], "no lossy fake in {band}");
    }
    let codecs = table(&text, H_CODEC);
    assert_eq!(keys(&codecs, 1), ["mp3", "aac", "opus", "total"]);
    for c in ["mp3", "aac", "opus"] {
        assert_eq!(row(&codecs, &[c]), [1, 0, 0, 1, 0], "{c}");
    }
    assert_eq!(row(&codecs, &["total"]), [3, 0, 0, 3, 0]);
}

/// Scenario: Evidence that the rules do not allow is shown, not hidden (@RP-12). Acceptance C5.
#[test]
fn rp12_c5_evidence_that_the_rules_do_not_allow_is_shown() {
    let text = report(&[lossless_fake(1, Vec::new())]);
    let evidence = table(&text, H_EVIDENCE);
    assert_eq!(row(&evidence, &["lossless", "other"]), [1], "{text}");
    let m = margins(&text);
    assert_eq!(m["none"], [1, 0, 0, 1], "{text}");
    for band in BANDS {
        assert_eq!(m[band], [0, 0, 0, 0], "{band}");
    }
}

/// Scenario: A Suspicious track is counted once per reason (@RP-12). Acceptance C5.
#[test]
fn rp12_c5_a_suspicious_track_is_counted_once_per_reason() {
    let mut lossy = track(1, Some(AudioCodec::Mp3), Verdict::Suspicious);
    lossy.analysis.reasons = vec![
        SuspicionReason::CutoffBelowDeclared,
        SuspicionReason::GradualRolloff,
    ];
    let lossless = track(2, Some(AudioCodec::Flac), Verdict::Suspicious);
    let text = report(&[lossy, lossless]);
    let rows = table(&text, H_REASONS);
    assert_eq!(keys(&rows, 1), REASON_ROWS);
    assert_eq!(row(&rows, &["cutoffBelowDeclared"]), [0, 1]);
    assert_eq!(row(&rows, &["gradualRolloff"]), [0, 1]);
    assert_eq!(row(&rows, &["none"]), [1, 0]);
    for r in [
        "lowHighFrequencyContent",
        "ambiguousCutoff",
        "highBandGaps",
        "streamingSampleRate",
    ] {
        assert_eq!(row(&rows, &[r]), [0, 0], "{r}");
    }
}

/// Scenario: A track without a codec is counted as unknown (@RP-12 @RNF-04). Acceptance C5.
#[test]
fn rp12_c5_a_track_without_a_codec_is_counted_as_unknown() {
    let mut m4a = track(3, None, Verdict::Error);
    m4a.analysis.error = Some(TrackError::UnsupportedFormat("synthetic".to_string()));
    let tracks = vec![
        m4a,
        track(1, Some(AudioCodec::Flac), Verdict::Real),
        track(2, Some(AudioCodec::Alac), Verdict::Real),
    ];
    let text = report(&tracks);
    let codecs = table(&text, H_CODEC);
    assert_eq!(keys(&codecs, 1), ["flac", "alac", "unknown", "total"]);
    assert_eq!(row(&codecs, &["unknown"]), [1, 0, 0, 0, 1]);
    assert_eq!(row(&codecs, &["total"]), [3, 2, 0, 0, 1]);
    let errors = table(&text, H_ERRORS);
    assert_eq!(keys(&errors, 1), ["unknown"]);
    assert_eq!(row(&errors, &["unknown"]), [0, 0, 1, 0]);
}

/// Scenario: A folder without errors says so (@RP-12). Acceptance C5.
#[test]
fn rp12_c5_a_folder_without_errors_says_so() {
    let text = report(&[real(1), real(2)]);
    assert!(
        content_lines(&text).contains(&NO_ERRORS.to_string()),
        "the line 'errors: none':\n{text}"
    );
    assert!(!has_table(&text, H_ERRORS), "no error table:\n{text}");
}

/// Section 4: the header, every row of every table with 0 where nothing is counted, in the order of
/// the spec, no path or name of the tracks, and nothing that depends on the order of the tracks
/// (section 7). Acceptance C5.
#[test]
fn rp12_c5_the_report_lists_every_row_in_order_and_names_nothing() {
    let mut tracks = vec![real(1), real(1)];
    tracks[1].analysis.info.size_bytes = 2_260_000;
    let albums = albums_of(&tracks);
    let text = summary_md(&header(Some("0123456789abcdef")), &tracks, &albums);
    let lines = content_lines(&text);
    assert_eq!(lines[0], TITLE);
    assert_eq!(lines[1], "run: 20261115T120000Z");
    assert_eq!(
        lines[2],
        "detection: algorithm_version 8, params custom 0123456789abcdef"
    );
    assert_eq!(lines[3], "tracks 2 | albums 1 | 3.3 MB");
    let codecs = table(&text, H_CODEC);
    assert_eq!(keys(&codecs, 1), ["flac", "total"]);
    assert_eq!(row(&codecs, &["flac"]), [2, 2, 0, 0, 0]);
    let evidence = table(&text, H_EVIDENCE);
    assert_eq!(
        keys(&evidence, 2),
        [
            "lossless | gapStep",
            "lossless | cutoffStep",
            "lossless | gapStep + cutoffStep",
            "lossless | other",
            "lossy | shortfall + cutoffStep",
            "lossy | other",
        ]
    );
    assert!(evidence.iter().all(|r| r[2] == "0"));
    let albums_rows = table(&text, H_ALBUMS);
    assert_eq!(
        keys(&albums_rows, 1),
        ["whole", "mixed", "isolated", "total"]
    );
    assert!(albums_rows.iter().all(|r| r[1..].iter().all(|c| c == "0")));
    assert_eq!(one_fake_count(&text), 0);
    assert!(margins(&text).values().all(|v| v == &[0, 0, 0, 0]));
    let reasons = table(&text, H_REASONS);
    assert_eq!(keys(&reasons, 1), REASON_ROWS);
    assert!(lines.contains(&NO_ERRORS.to_string()));
    // The blocks come in the order of section 4.
    let at = |l: &str| lines.iter().position(|x| x == &norm(l)).unwrap();
    let order = [
        at(H_CODEC),
        at(H_EVIDENCE),
        at(H_ALBUMS),
        at(H_MARGIN),
        at(H_REASONS),
        at(NO_ERRORS),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{text}");
    assert!(!text.contains("secreta"), "no folder or file name:\n{text}");

    tracks.reverse();
    assert_eq!(
        summary_md(&header(Some("0123456789abcdef")), &tracks, &albums),
        text,
        "the order of the tracks does not matter"
    );
}

// ---------------------------------------------------------------------------------------------
// tracks.csv from given analyses (section 5, "Empty means not measured")
// ---------------------------------------------------------------------------------------------

/// The five probe values of a track, in the order of `PROBE_COLUMNS`.
fn set_probe(
    t: &mut CensusTrack,
    sample_rate: u32,
    bit_depth: Option<u8>,
    channels: u8,
    declared_bitrate_kbps: Option<u32>,
    duration_ms: u64,
) {
    let info = &mut t.analysis.info;
    info.sample_rate = sample_rate;
    info.bit_depth = bit_depth;
    info.channels = channels;
    info.declared_bitrate_kbps = declared_bitrate_kbps;
    info.duration_ms = duration_ms;
}

/// The five probe fields of the single row of `tracks.csv` for one track, by column name.
fn probe_fields(t: CensusTrack) -> BTreeMap<&'static str, String> {
    let tracks = [t];
    let text = tracks_csv(&tracks, &albums_of(&tracks), Path::new(""));
    let mut rows = parse_csv(&text);
    assert_eq!(rows.len(), 2, "the header and one row:\n{text}");
    let header = rows.remove(0);
    let csv = Table { header, rows };
    assert_eq!(csv.header.join(","), CSV_HEADER, "header of section 5");
    let r = &csv.rows[0];
    PROBE_COLUMNS
        .iter()
        .map(|c| (*c, csv.get(r, c).to_string()))
        .collect()
}

fn expect(values: [&str; 5]) -> BTreeMap<&'static str, String> {
    PROBE_COLUMNS
        .iter()
        .zip(values)
        .map(|(c, v)| (*c, v.to_string()))
        .collect()
}

/// A flac error track of the given kind with the given probe values.
fn error_track(
    error: TrackError,
    sample_rate: u32,
    bit_depth: Option<u8>,
    channels: u8,
    declared_bitrate_kbps: Option<u32>,
    duration_ms: u64,
) -> CensusTrack {
    let mut t = track(1, Some(AudioCodec::Flac), Verdict::Error);
    t.analysis.error = Some(error);
    set_probe(
        &mut t,
        sample_rate,
        bit_depth,
        channels,
        declared_bitrate_kbps,
        duration_ms,
    );
    t
}

/// Section 5, "Empty means not measured": an error track whose file could not be probed holds only
/// the unprobed placeholders (0 for `sample_rate`, `channels` and `duration_ms`, no value for
/// `bit_depth` and `declared_bitrate_kbps`), and the five fields are empty. Scenario: An empty field of
/// the per-track file means the value was not measured (@RP-12 @RNF-04). Acceptance C2.
#[test]
fn rp12_c2_an_unprobed_error_track_has_five_empty_probe_fields() {
    for error in [
        TrackError::CorruptStream("synthetic".to_string()),
        TrackError::Unreadable("synthetic".to_string()),
        TrackError::UnsupportedFormat("synthetic".to_string()),
    ] {
        let fields = probe_fields(error_track(error.clone(), 0, None, 0, None, 0));
        assert_eq!(fields, expect(["", "", "", "", ""]), "{error:?}");
    }
}

/// Section 5: a track that was probed keeps its real values in the five fields even when it fails
/// afterwards (`tooShort`). Scenario: An empty field of the per-track file means the value was not
/// measured (@RP-12 @RNF-04). Acceptance C2.
#[test]
fn rp12_c2_a_probed_error_track_keeps_its_real_probe_values() {
    let fields = probe_fields(error_track(
        TrackError::TooShort,
        48_000,
        Some(24),
        2,
        Some(1411),
        1_250,
    ));
    assert_eq!(fields, expect(["48000", "24", "2", "1411", "1250"]));
    // A lossless probe without a declared bitrate: that field has no value, the others are kept.
    let fields = probe_fields(error_track(
        TrackError::TooShort,
        44_100,
        Some(16),
        1,
        None,
        900,
    ));
    assert_eq!(fields, expect(["44100", "16", "1", "", "900"]));
}

/// Section 5: an error track whose probe reported 0 for one of the five fields has that field empty
/// (0 is not a valid measurement of these fields) and keeps the others. Scenario: An empty field of
/// the per-track file means the value was not measured (@RP-12 @RNF-04). Acceptance C2.
#[test]
fn rp12_c2_an_error_track_with_a_zero_from_the_probe_has_that_field_empty() {
    /// `v`, or 0 of its type when field `i` is the one reported as 0.
    fn pick<T: From<u8>>(zero: usize, i: usize, v: T) -> T {
        if i == zero {
            T::from(0)
        } else {
            v
        }
    }
    let real = ["48000", "24", "2", "1411", "1250"];
    for zero in 0..PROBE_COLUMNS.len() {
        let t = error_track(
            TrackError::TooShort,
            pick(zero, 0, 48_000),
            Some(pick(zero, 1, 24)),
            pick(zero, 2, 2),
            Some(pick(zero, 3, 1411)),
            pick(zero, 4, 1_250),
        );
        let mut want = real;
        want[zero] = "";
        assert_eq!(
            probe_fields(t),
            expect(want),
            "{} reported as 0",
            PROBE_COLUMNS[zero]
        );
    }
}

/// Section 5 states the empty-field rule for error tracks only: a track that is not an error writes
/// its values as they are, so a 0 there is written as `0`. Scenario: An empty field of the per-track
/// file means the value was not measured (@RP-12 @RNF-04). Acceptance C2.
#[test]
fn rp12_c2_a_track_that_is_not_an_error_writes_a_zero_as_zero() {
    for verdict in [Verdict::Real, Verdict::Suspicious, Verdict::Fake] {
        let mut t = track(1, Some(AudioCodec::Flac), verdict);
        set_probe(&mut t, 0, Some(0), 0, Some(0), 0);
        assert_eq!(
            probe_fields(t),
            expect(["0", "0", "0", "0", "0"]),
            "{verdict:?}"
        );
    }
}

/// Section 5: a track that is not an error keeps its real values in the five fields (a lossless track
/// without a declared bitrate and a lossy one without a bit depth have those fields empty: no value).
/// Scenario: An empty field of the per-track file means the value was not measured (@RP-12 @RNF-04).
/// Acceptance C2.
#[test]
fn rp12_c2_a_track_that_is_not_an_error_keeps_its_real_probe_values() {
    let mut t = real(1);
    set_probe(&mut t, 96_000, Some(24), 2, None, 241_337);
    assert_eq!(probe_fields(t), expect(["96000", "24", "2", "", "241337"]));
    let mut t = track(1, Some(AudioCodec::Mp3), Verdict::Suspicious);
    set_probe(&mut t, 44_100, None, 2, Some(320), 180_000);
    assert_eq!(probe_fields(t), expect(["44100", "", "2", "320", "180000"]));
}

// ---------------------------------------------------------------------------------------------
// Wiring (C10)
// ---------------------------------------------------------------------------------------------

struct Recipe {
    name: String,
    deps: String,
    body: String,
}

/// The recipes of a justfile: header line `name args: deps`, body = the indented lines after it.
fn recipes(text: &str) -> Vec<Recipe> {
    let mut out = Vec::new();
    let mut current: Option<Recipe> = None;
    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(r) = current.as_mut() {
                r.body.push_str(line);
                r.body.push('\n');
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(r) = current.take() {
            out.push(r);
        }
        if line.starts_with('#') || line.starts_with('[') || line.contains(":=") {
            continue;
        }
        if let Some((head, deps)) = line.split_once(':') {
            current = Some(Recipe {
                name: head.split_whitespace().next().unwrap_or("").to_string(),
                deps: deps.to_string(),
                body: String::new(),
            });
        }
    }
    out.extend(current);
    out
}

/// `word` appears in `text` with no letter, digit or `_` on either side.
fn has_word(text: &str, word: &str) -> bool {
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    text.match_indices(word).any(|(i, _)| {
        !is_word(text[..i].chars().next_back()) && !is_word(text[i + word.len()..].chars().next())
    })
}

/// Scenario: The census is opt-in and never runs in CI (@RP-12). Acceptance C10.
#[test]
fn rp12_c10_the_census_is_opt_in_and_never_runs_in_ci() {
    let justfile = read(&repo_root().join("justfile"));
    let all = recipes(&justfile);
    // The parser finds the recipes this check is about.
    for name in ["ci", "check", "check-fast", "test", "check-app", "accept"] {
        assert!(all.iter().any(|r| r.name == name), "recipe {name} found");
    }
    let census = all
        .iter()
        .find(|r| r.name == "census")
        .expect("the justfile has a census recipe");
    assert!(
        census.body.contains("fakeflac-calibrate"),
        "{}",
        census.body
    );
    assert!(census.body.contains("census"), "{}", census.body);
    for name in ["ci", "check", "check-fast", "test", "check-app"] {
        let r = all.iter().find(|r| r.name == name).unwrap();
        assert!(
            !has_word(&r.deps, "census") && !has_word(&r.body, "census"),
            "recipe {name} names census"
        );
    }
    for workflow in ["ci.yml", "release.yml"] {
        let text = read(&repo_root().join(".github").join("workflows").join(workflow));
        assert!(!text.contains("just census"), "{workflow} runs the census");
    }
}

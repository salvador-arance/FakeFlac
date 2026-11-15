//! Library census (RP-12, `specs/calibration/census.md`): the detector's verdicts on a whole folder,
//! in aggregates. The run calls the app's own use case (`fakeflac_core::analyze_folder` with the app's
//! adapters); the report and the per-track file are built by pure functions from the tracks and albums
//! of the run. No ground truth and no pass or fail: it describes verdicts, it does not judge them.

use crate::cache::{codec_name, error_code};
use crate::env::Env;
use crate::error::ToolError;
use crate::evaluate::{reason_name, verdict_name};
use crate::extract::worker_threads;
use crate::fsutil::write_atomic;
use crate::run::{load_params, now_secs, run_stamp, CensusArgs, Outcome};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::{
    analyze_folder, AlbumSummary, AppErrorCode, AudioCodec, CancelToken, DetectionParams,
    EvidenceKind, ProgressEvent, SuspicionReason, TrackAnalysis, TrackError, Verdict,
};
use fakeflac_fs::WalkDirScanner;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

// ---------------------------------------------------------------------------------------------
// Report constants of census.md (not detection parameters: they only group the report)
// ---------------------------------------------------------------------------------------------

/// Album strata (section 4.4, CQ6): `whole` when `WHOLE_NUM·f ≥ WHOLE_DEN·n` (at least 75 % Fake),
/// `isolated` when `ISOLATED_NUM·f < ISOLATED_DEN·n` (fewer than 25 % Fake). Compared in integers.
const WHOLE_NUM: u64 = 4;
const WHOLE_DEN: u64 = 3;
const ISOLATED_NUM: u64 = 4;
const ISOLATED_DEN: u64 = 1;

/// Margin bands of the lossless `Fake` verdicts (section 4.5, CQ7): the lower limits in dB of the
/// bands after the first; a band includes its lower limit and excludes its upper one.
const MARGIN_BAND_LOWER_DB: [f64; 4] = [1.0, 3.0, 6.0, 12.0];
/// The labels of the five bands, in order.
const MARGIN_BAND_LABELS: [&str; 5] = ["< 1", "1 – 3", "3 – 6", "6 – 12", "≥ 12"];

/// Bytes per MB in the header (section 4.1).
const BYTES_PER_MB: f64 = 1_000_000.0;

const TITLE: &str = "FakeFlac library census (specs/calibration/census.md)";

/// The exact messages of census.md section 2.
pub const USAGE: &str = "usage: fakeflac-calibrate census FOLDER [--params FILE]";
const NOT_A_FOLDER: &str = "error: census: the folder does not exist or is not a folder";
const NO_AUDIO: &str = "error: census: no supported audio files in the folder";
const NO_REPOSITORY: &str =
    "error: census: run it from the repository (the report goes under target/)";
const INTERNAL: &str = "error: census: internal error";

/// The header of `tracks.csv` (section 5).
const CSV_HEADER: [&str; 18] = [
    "path",
    "album_index",
    "album_stratum",
    "codec",
    "sample_rate",
    "bit_depth",
    "channels",
    "declared_bitrate_kbps",
    "duration_ms",
    "size_bytes",
    "verdict",
    "reasons",
    "evidence",
    "gap_step_margin_db",
    "cutoff_step_margin_db",
    "shortfall_margin_hz",
    "cutoff_hz",
    "error",
];

// ---------------------------------------------------------------------------------------------
// Data of a run
// ---------------------------------------------------------------------------------------------

/// One track of a run: its analysis and the `album.index` of its `TrackDone`.
#[derive(Debug, Clone, PartialEq)]
pub struct CensusTrack {
    pub analysis: TrackAnalysis,
    pub album: u32,
}

/// The header data of `summary.md` (section 4.1): `params_id` is `None` for the defaults and the
/// `id16` of the params file's bytes with `--params`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CensusHeader {
    pub run: String,
    pub algorithm_version: u32,
    pub params_id: Option<String>,
}

/// The stratum of an album with at least one `Fake` (section 4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stratum {
    Whole,
    Mixed,
    Isolated,
}

impl Stratum {
    const ALL: [Stratum; 3] = [Stratum::Whole, Stratum::Mixed, Stratum::Isolated];

    pub fn name(self) -> &'static str {
        match self {
            Stratum::Whole => "whole",
            Stratum::Mixed => "mixed",
            Stratum::Isolated => "isolated",
        }
    }

    fn column(self) -> usize {
        match self {
            Stratum::Whole => 0,
            Stratum::Mixed => 1,
            Stratum::Isolated => 2,
        }
    }
}

/// The stratum of an album from its `fake` count `f` and its analysed tracks `n` (errors left out);
/// `None` when it has no `Fake`.
pub fn stratum_of(f: u32, n: u32) -> Option<Stratum> {
    if f == 0 {
        return None;
    }
    let (f, n) = (u64::from(f), u64::from(n));
    Some(if WHOLE_NUM * f >= WHOLE_DEN * n {
        Stratum::Whole
    } else if ISOLATED_NUM * f < ISOLATED_DEN * n {
        Stratum::Isolated
    } else {
        Stratum::Mixed
    })
}

/// `(f, n)` of an album summary.
fn album_counts(s: &AlbumSummary) -> (u32, u32) {
    (s.fake, s.real + s.suspicious + s.fake)
}

/// The lossless / lossy group of a track (section 3); `None` without a codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Lossless,
    Lossy,
}

fn group_of(codec: Option<AudioCodec>) -> Option<Group> {
    codec.map(|c| {
        if c.is_lossy() {
            Group::Lossy
        } else {
            Group::Lossless
        }
    })
}

/// The codec rows of section 4.2, in order (`None` is `unknown`).
const CODEC_ROWS: [Option<AudioCodec>; 7] = [
    Some(AudioCodec::Flac),
    Some(AudioCodec::Pcm),
    Some(AudioCodec::Alac),
    Some(AudioCodec::Mp3),
    Some(AudioCodec::Aac),
    Some(AudioCodec::Opus),
    None,
];

fn codec_label(codec: Option<AudioCodec>) -> &'static str {
    codec.map_or("unknown", codec_name)
}

const REASON_ROWS: [SuspicionReason; 6] = [
    SuspicionReason::LowHighFrequencyContent,
    SuspicionReason::GradualRolloff,
    SuspicionReason::AmbiguousCutoff,
    SuspicionReason::HighBandGaps,
    SuspicionReason::CutoffBelowDeclared,
    SuspicionReason::StreamingSampleRate,
];

fn evidence_name(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::GapStep => "gapStep",
        EvidenceKind::CutoffStep => "cutoffStep",
        EvidenceKind::Shortfall => "shortfall",
    }
}

/// The album strata of a run: from the given album summaries, or, for an index without one (never
/// the case in a run), from the tracks of that index.
fn strata(
    tracks: &[CensusTrack],
    albums: &BTreeMap<u32, AlbumSummary>,
) -> BTreeMap<u32, (u32, u32)> {
    let mut counts: BTreeMap<u32, (u32, u32)> = albums
        .iter()
        .map(|(index, s)| (*index, album_counts(s)))
        .collect();
    let mut missing: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
    for t in tracks.iter().filter(|t| !albums.contains_key(&t.album)) {
        let c = missing.entry(t.album).or_default();
        match t.analysis.verdict {
            Verdict::Fake => {
                c.0 += 1;
                c.1 += 1;
            }
            Verdict::Real | Verdict::Suspicious => c.1 += 1,
            Verdict::Error => {}
        }
    }
    counts.extend(missing);
    counts
}

/// The largest margin among the `gapStep` and `cutoffStep` items, widened to `f64` (section 4.5).
fn lossless_margin(analysis: &TrackAnalysis) -> Option<f64> {
    analysis
        .evidence
        .iter()
        .filter(|e| matches!(e.kind, EvidenceKind::GapStep | EvidenceKind::CutoffStep))
        .map(|e| f64::from(e.margin))
        .reduce(f64::max)
}

/// The band index of a margin: the number of lower limits it reaches.
fn margin_band(margin: f64) -> usize {
    MARGIN_BAND_LOWER_DB
        .iter()
        .filter(|lower| margin >= **lower)
        .count()
}

// ---------------------------------------------------------------------------------------------
// summary.md (section 4)
// ---------------------------------------------------------------------------------------------

fn table_header(out: &mut String, columns: &[&str]) {
    let _ = writeln!(out, "| {} |", columns.join(" | "));
    let _ = writeln!(out, "|{}", "---|".repeat(columns.len()));
}

fn table_row(out: &mut String, key: &[&str], values: &[u32]) {
    let mut cells: Vec<String> = key.iter().map(|k| (*k).to_string()).collect();
    cells.extend(values.iter().map(u32::to_string));
    let _ = writeln!(out, "| {} |", cells.join(" | "));
}

/// `summary.md` (section 4) from the tracks of a run (any order) and the summary of the last
/// `TrackDone` of every album index. Counts and fixed words only: no path or name.
pub fn summary_md(
    header: &CensusHeader,
    tracks: &[CensusTrack],
    albums: &BTreeMap<u32, AlbumSummary>,
) -> String {
    let mut out = String::new();
    let strata_counts = strata(tracks, albums);
    let stratum_of_album = |album: u32| {
        strata_counts
            .get(&album)
            .and_then(|(f, n)| stratum_of(*f, *n))
    };

    // 4.1 Header.
    let album_indexes: BTreeSet<u32> = tracks.iter().map(|t| t.album).collect();
    let size: u64 = tracks.iter().map(|t| t.analysis.info.size_bytes).sum();
    let params = match &header.params_id {
        None => "default".to_string(),
        Some(id) => format!("custom {id}"),
    };
    let _ = writeln!(out, "{TITLE}\n");
    let _ = writeln!(out, "run: {}\n", header.run);
    let _ = writeln!(
        out,
        "detection: algorithm_version {}, params {params}\n",
        header.algorithm_version
    );
    let _ = writeln!(
        out,
        "tracks {} | albums {} | {:.1} MB\n",
        tracks.len(),
        album_indexes.len(),
        size as f64 / BYTES_PER_MB
    );

    // 4.2 Verdicts by codec: tracks, real, suspicious, fake, error.
    let verdict_column = |v: Verdict| match v {
        Verdict::Real => 1,
        Verdict::Suspicious => 2,
        Verdict::Fake => 3,
        Verdict::Error => 4,
    };
    let mut by_codec: Vec<[u32; 5]> = vec![[0; 5]; CODEC_ROWS.len()];
    let mut total = [0u32; 5];
    for t in tracks {
        let row = CODEC_ROWS
            .iter()
            .position(|c| *c == t.analysis.info.codec)
            .unwrap_or(CODEC_ROWS.len() - 1);
        for counts in [&mut by_codec[row], &mut total] {
            counts[0] += 1;
            counts[verdict_column(t.analysis.verdict)] += 1;
        }
    }
    table_header(
        &mut out,
        &["codec", "tracks", "real", "suspicious", "fake", "error"],
    );
    for (codec, counts) in CODEC_ROWS.iter().zip(&by_codec) {
        if counts[0] > 0 {
            table_row(&mut out, &[codec_label(*codec)], counts);
        }
    }
    table_row(&mut out, &["total"], &total);
    out.push('\n');

    // 4.3 Evidence of the Fake verdicts: lossless gap, cutoff, both, other; lossy pair, other.
    let mut evidence = [0u32; 6];
    for t in tracks
        .iter()
        .filter(|t| t.analysis.verdict == Verdict::Fake)
    {
        let kinds: BTreeSet<&str> = t
            .analysis
            .evidence
            .iter()
            .map(|e| evidence_name(e.kind))
            .collect();
        let has = |k: EvidenceKind| kinds.contains(evidence_name(k));
        let (gap, step, short) = (
            has(EvidenceKind::GapStep),
            has(EvidenceKind::CutoffStep),
            has(EvidenceKind::Shortfall),
        );
        match group_of(t.analysis.info.codec) {
            Some(Group::Lossless) => {
                let row = match (gap, step, short) {
                    (true, false, false) => 0,
                    (false, true, false) => 1,
                    (true, true, false) => 2,
                    _ => 3,
                };
                evidence[row] += 1;
            }
            Some(Group::Lossy) => {
                let row = if !gap && step && short { 4 } else { 5 };
                evidence[row] += 1;
            }
            None => {}
        }
    }
    table_header(&mut out, &["group", "evidence", "tracks"]);
    for (key, count) in [
        ["lossless", "gapStep"],
        ["lossless", "cutoffStep"],
        ["lossless", "gapStep + cutoffStep"],
        ["lossless", "other"],
        ["lossy", "shortfall + cutoffStep"],
        ["lossy", "other"],
    ]
    .iter()
    .zip(evidence)
    {
        table_row(&mut out, key, &[count]);
    }
    out.push('\n');

    // 4.4 Albums with a Fake: albums, fake tracks, lossless, lossy per stratum.
    let mut by_stratum = [[0u32; 4]; 3];
    let mut one_fake = 0u32;
    for (f, n) in strata_counts.values() {
        let Some(s) = stratum_of(*f, *n) else {
            continue;
        };
        let row = &mut by_stratum[s.column()];
        row[0] += 1;
        row[1] += f;
        if *f == 1 && *n >= 2 {
            one_fake += 1;
        }
    }
    // 4.5 Margins of the lossless fakes: band (and `none`) × whole, mixed, isolated, total.
    let mut margins = [[0u32; 4]; MARGIN_BAND_LABELS.len() + 1];
    for t in tracks
        .iter()
        .filter(|t| t.analysis.verdict == Verdict::Fake)
    {
        let stratum = stratum_of_album(t.album);
        let group = group_of(t.analysis.info.codec);
        if let Some(s) = stratum {
            match group {
                Some(Group::Lossless) => by_stratum[s.column()][2] += 1,
                Some(Group::Lossy) => by_stratum[s.column()][3] += 1,
                None => {}
            }
        }
        if group != Some(Group::Lossless) {
            continue;
        }
        let band = lossless_margin(&t.analysis).map_or(MARGIN_BAND_LABELS.len(), margin_band);
        if let Some(s) = stratum {
            margins[band][s.column()] += 1;
        }
        margins[band][3] += 1;
    }
    table_header(
        &mut out,
        &["stratum", "albums", "fake tracks", "lossless", "lossy"],
    );
    let mut albums_total = [0u32; 4];
    for s in Stratum::ALL {
        let row = by_stratum[s.column()];
        for (sum, v) in albums_total.iter_mut().zip(row) {
            *sum += v;
        }
        table_row(&mut out, &[s.name()], &row);
    }
    table_row(&mut out, &["total"], &albums_total);
    out.push('\n');
    let _ = writeln!(
        out,
        "albums with exactly one Fake among 2 or more analysed tracks: {one_fake}\n"
    );

    table_header(
        &mut out,
        &["margin dB", "whole", "mixed", "isolated", "total"],
    );
    for (label, counts) in MARGIN_BAND_LABELS
        .iter()
        .chain(["none"].iter())
        .zip(&margins)
    {
        table_row(&mut out, &[label], counts);
    }
    out.push('\n');

    // 4.6 Reasons of the Suspicious verdicts: lossless, lossy.
    let mut reasons = [[0u32; 2]; REASON_ROWS.len() + 1];
    for t in tracks
        .iter()
        .filter(|t| t.analysis.verdict == Verdict::Suspicious)
    {
        let column = match group_of(t.analysis.info.codec) {
            Some(Group::Lossless) => 0,
            Some(Group::Lossy) => 1,
            None => continue,
        };
        let mut any = false;
        for (row, reason) in REASON_ROWS.iter().enumerate() {
            if t.analysis.reasons.contains(reason) {
                reasons[row][column] += 1;
                any = true;
            }
        }
        if !any {
            reasons[REASON_ROWS.len()][column] += 1;
        }
    }
    table_header(&mut out, &["reason", "lossless", "lossy"]);
    for (row, counts) in reasons.iter().enumerate() {
        let label = REASON_ROWS.get(row).map_or("none", |r| reason_name(*r));
        table_row(&mut out, &[label], counts);
    }
    out.push('\n');

    // 4.7 Errors: unreadable, corruptStream, unsupportedFormat, tooShort.
    let mut errors: Vec<[u32; 4]> = vec![[0; 4]; CODEC_ROWS.len()];
    let mut any_error = false;
    for t in tracks
        .iter()
        .filter(|t| t.analysis.verdict == Verdict::Error)
    {
        any_error = true;
        let row = CODEC_ROWS
            .iter()
            .position(|c| *c == t.analysis.info.codec)
            .unwrap_or(CODEC_ROWS.len() - 1);
        let column = match &t.analysis.error {
            Some(TrackError::Unreadable(_)) => Some(0),
            Some(TrackError::CorruptStream(_)) => Some(1),
            Some(TrackError::UnsupportedFormat(_)) => Some(2),
            Some(TrackError::TooShort) => Some(3),
            None => None,
        };
        if let Some(c) = column {
            errors[row][c] += 1;
        }
    }
    if any_error {
        table_header(
            &mut out,
            &[
                "codec",
                "unreadable",
                "corruptStream",
                "unsupportedFormat",
                "tooShort",
            ],
        );
        // One row for each codec of 4.2 with at least one error track, in the order of 4.2.
        for ((codec, verdicts), kinds) in CODEC_ROWS.iter().zip(&by_codec).zip(&errors) {
            if verdicts[4] > 0 {
                table_row(&mut out, &[codec_label(*codec)], kinds);
            }
        }
    } else {
        out.push_str("errors: none\n");
    }
    // A closing blank line, so the `elapsed:` line that follows on stdout is not read as a table row.
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------------------------
// tracks.csv (section 5)
// ---------------------------------------------------------------------------------------------

/// RFC 4180 quoting: a field with a comma, a quote or a line break is quoted, a quote doubled.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The path of a track relative to `root`, with `/` between components (lossy for names that are
/// not valid Unicode). A path outside `root` (never the case in a run) keeps only its file name, so
/// no absolute path is ever written.
fn relative_path(path: &Path, root: &Path) -> String {
    let parts: Vec<String> = match path.strip_prefix(root) {
        Ok(rel) => rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(p) => Some(p.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect(),
        Err(_) => path
            .file_name()
            .map(|n| vec![n.to_string_lossy().into_owned()])
            .unwrap_or_default(),
    };
    parts.join("/")
}

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

/// The margin of the first evidence item of `kind`, in Rust's shortest `f32` form.
fn margin_of(analysis: &TrackAnalysis, kind: EvidenceKind) -> String {
    opt(analysis
        .evidence
        .iter()
        .find(|e| e.kind == kind)
        .map(|e| e.margin))
}

/// `tracks.csv` (section 5): one row per track sorted by the bytes of its `path`, paths relative to
/// `root`, UTF-8 without a byte-order mark, LF line ends.
pub fn tracks_csv(
    tracks: &[CensusTrack],
    albums: &BTreeMap<u32, AlbumSummary>,
    root: &Path,
) -> String {
    let strata_counts = strata(tracks, albums);
    let mut rows: Vec<Vec<String>> = tracks
        .iter()
        .map(|t| {
            let a = &t.analysis;
            let i = &a.info;
            let stratum = strata_counts
                .get(&t.album)
                .and_then(|(f, n)| stratum_of(*f, *n))
                .map_or("", Stratum::name);
            vec![
                relative_path(&i.path, root),
                t.album.to_string(),
                stratum.to_string(),
                i.codec.map_or("", codec_name).to_string(),
                i.sample_rate.to_string(),
                opt(i.bit_depth),
                i.channels.to_string(),
                opt(i.declared_bitrate_kbps),
                i.duration_ms.to_string(),
                i.size_bytes.to_string(),
                verdict_name(a.verdict).to_string(),
                a.reasons
                    .iter()
                    .map(|r| reason_name(*r))
                    .collect::<Vec<_>>()
                    .join("|"),
                a.evidence
                    .iter()
                    .map(|e| evidence_name(e.kind))
                    .collect::<Vec<_>>()
                    .join("|"),
                margin_of(a, EvidenceKind::GapStep),
                margin_of(a, EvidenceKind::CutoffStep),
                margin_of(a, EvidenceKind::Shortfall),
                opt(a.cutoff.as_ref().map(|c| c.frequency_hz)),
                a.error.as_ref().map_or("", error_code).to_string(),
            ]
        })
        .collect();
    rows.sort_by(|a, b| a[0].as_bytes().cmp(b[0].as_bytes()));
    let mut out = String::new();
    out.push_str(&CSV_HEADER.join(","));
    out.push('\n');
    for row in rows {
        let fields: Vec<String> = row.iter().map(|f| csv_field(f)).collect();
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The run (section 3)
// ---------------------------------------------------------------------------------------------

/// Parses `census FOLDER [--params FILE]` (the arguments after `census`). Every other command line
/// is the single usage line (exit 2).
pub fn parse(flags: &[String]) -> Result<CensusArgs, ToolError> {
    let usage = || ToolError::input(USAGE);
    let mut folder: Option<PathBuf> = None;
    let mut params: Option<PathBuf> = None;
    let mut i = 0;
    while i < flags.len() {
        let arg = &flags[i];
        if arg == "--params" {
            let value = flags.get(i + 1).ok_or_else(usage)?;
            if params.is_some() {
                return Err(usage());
            }
            params = Some(PathBuf::from(value));
            i += 2;
            continue;
        }
        if arg.starts_with('-') || folder.is_some() {
            return Err(usage());
        }
        folder = Some(PathBuf::from(arg));
        i += 1;
    }
    let folder = folder.ok_or_else(usage)?;
    Ok(CensusArgs { folder, params })
}

fn internal(detail: impl Into<String>) -> ToolError {
    ToolError {
        kind: crate::error::ExitKind::Internal,
        lines: vec![INTERNAL.to_string(), detail.into()],
    }
}

/// Creates `<parent>/<run>`, with `-2`, `-3`… appended while that folder exists.
fn create_run_dir(parent: &Path, run: &str) -> std::io::Result<(String, PathBuf)> {
    std::fs::create_dir_all(parent)?;
    let mut n = 1u32;
    loop {
        let name = if n == 1 {
            run.to_string()
        } else {
            format!("{run}-{n}")
        };
        let dir = parent.join(&name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok((name, dir)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e),
        }
    }
}

/// The census of section 3: checks (exit 2, nothing written), one run of the app's use case, then
/// `target/census/<run>/summary.md` and `tracks.csv`. Reads only `env.repo_root`.
pub fn census(args: &CensusArgs, env: &Env) -> Result<Outcome, ToolError> {
    let started = Instant::now();
    let (run_id, _) = run_stamp(now_secs());

    // 1. Checks, before anything is written.
    let folder = if args.folder.is_relative() {
        std::path::absolute(&args.folder).map_err(|_| ToolError::input(NOT_A_FOLDER))?
    } else {
        args.folder.clone()
    };
    if !folder.is_dir() {
        return Err(ToolError::input(NOT_A_FOLDER));
    }
    let (params, params_id): (DetectionParams, Option<String>) = match &args.params {
        None => (DetectionParams::default(), None),
        Some(path) => {
            let mut loaded = load_params(std::slice::from_ref(path))?;
            let l = loaded
                .pop()
                .ok_or_else(|| internal("the params file was not loaded"))?;
            (l.params, Some(l.hash))
        }
    };
    let Some(repo) = &env.repo_root else {
        return Err(ToolError::input(NO_REPOSITORY));
    };

    // 2. The run: one call of the app's use case.
    let tracks: Mutex<Vec<CensusTrack>> = Mutex::new(Vec::new());
    let albums: Mutex<BTreeMap<u32, AlbumSummary>> = Mutex::new(BTreeMap::new());
    let on_progress = |event: ProgressEvent| {
        if let ProgressEvent::TrackDone {
            analysis, album, ..
        } = event
        {
            // TrackDone is sent under the album lock, so the last one of an index carries the
            // album's final counts.
            albums
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(album.index, album.summary);
            tracks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(CensusTrack {
                    analysis,
                    album: album.index,
                });
        }
    };
    analyze_folder(
        &WalkDirScanner,
        &SymphoniaAnalyzer,
        &params,
        &folder,
        worker_threads(),
        &CancelToken::new(),
        &on_progress,
    )
    .map_err(|e| match e.code {
        AppErrorCode::NoAudioFound => ToolError::input(NO_AUDIO),
        AppErrorCode::NotAFolder | AppErrorCode::FolderUnreadable => ToolError::input(NOT_A_FOLDER),
        // The message of an AppError names the folder: only its code is printed.
        other => internal(format!("{other:?}")),
    })?;
    let mut tracks = tracks.into_inner().unwrap_or_else(|e| e.into_inner());
    let albums = albums.into_inner().unwrap_or_else(|e| e.into_inner());
    tracks.sort_by(|a, b| a.analysis.info.path.cmp(&b.analysis.info.path));

    // 3. The files, after the run.
    let csv = tracks_csv(&tracks, &albums, &folder);
    let (run, dir) = create_run_dir(&repo.join("target").join("census"), &run_id)
        .map_err(|e| internal(e.to_string()))?;
    let summary = summary_md(
        &CensusHeader {
            run,
            algorithm_version: params.algorithm_version,
            params_id,
        },
        &tracks,
        &albums,
    );
    let written = write_atomic(&dir.join("summary.md"), summary.as_bytes())
        .and_then(|()| write_atomic(&dir.join("tracks.csv"), csv.as_bytes()));
    if let Err(e) = written {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(internal(e.to_string()));
    }

    let mut stdout = summary;
    let _ = writeln!(stdout, "elapsed: {:.1} s", started.elapsed().as_secs_f64());
    Ok(Outcome {
        exit_code: 0,
        stdout,
        stderr: String::new(),
        report_dir: Some(dir),
        ..Outcome::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strata_compare_integers() {
        assert_eq!(stratum_of(0, 3), None);
        assert_eq!(stratum_of(3, 4), Some(Stratum::Whole));
        assert_eq!(stratum_of(1, 1), Some(Stratum::Whole));
        assert_eq!(stratum_of(2, 3), Some(Stratum::Mixed));
        assert_eq!(stratum_of(1, 4), Some(Stratum::Mixed));
        assert_eq!(stratum_of(1, 5), Some(Stratum::Isolated));
    }

    #[test]
    fn bands_include_their_lower_limit() {
        assert_eq!(margin_band(0.0), 0);
        assert_eq!(margin_band(f64::from(0.99_f32)), 0);
        assert_eq!(margin_band(1.0), 1);
        assert_eq!(margin_band(3.0), 2);
        assert_eq!(margin_band(f64::from(11.99_f32)), 3);
        assert_eq!(margin_band(12.0), 4);
    }

    #[test]
    fn command_line() {
        let a = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert!(parse(&a(&["x", "--params", "p"])).is_ok());
        assert!(parse(&a(&[])).is_err());
        assert!(parse(&a(&["--params", "p"])).is_err());
        assert!(parse(&a(&["x", "-v"])).is_err());
    }

    #[test]
    fn paths_are_relative_with_slashes() {
        let root = Path::new("r");
        assert_eq!(
            relative_path(&root.join("a").join("b.flac"), root),
            "a/b.flac"
        );
        assert_eq!(csv_field("a, b"), "\"a, b\"");
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
    }
}

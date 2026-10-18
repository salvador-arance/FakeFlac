//! Commands of the tool (calibration.md section 2) as a library entry point.

use crate::config::Config;
use crate::env::{Env, Roots};
use crate::error::ToolError;
use crate::evaluate::{class_totals, evaluate_row, evaluation, figure, ParamsRun, RowEval};
use crate::extract::{extract_group, worker_threads, Context, CorpusItem, GroupResult};
use crate::fsutil::write_atomic;
use crate::grid::{self, GridFile, RowKind, SearchRow};
use crate::hash::id16;
use crate::lists::{self, Lists};
use crate::model::{album_class_of, LabelRow, Truth};
use crate::render::{self, GridReport, ReportData};
use crate::report::{CacheStats, Evaluation, GridOutcome, PointScore};
use crate::split::{album_id, assign_class, Split};
use crate::transcode::Transcoder;
use fakeflac_core::{DetectionParams, Verdict};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DEFAULT_PARAMS: &str = "specs/detection/detection-params.v2.toml";
const DEFAULT_CONFIG: &str = "specs/calibration/calibration.v1.toml";
const DEFAULT_GRID: &str = "specs/calibration/calibrate-grid.v1.toml";

/// `accept [--params FILE]… [--config FILE] [--no-generate]`
#[derive(Debug, Clone)]
pub struct AcceptArgs {
    /// Empty means `specs/detection/detection-params.v2.toml`. The first file is the baseline.
    pub params: Vec<PathBuf>,
    pub config: PathBuf,
    pub no_generate: bool,
}

/// `calibrate [--grid FILE] [--config FILE]`
#[derive(Debug, Clone)]
pub struct CalibrateArgs {
    pub grid: PathBuf,
    pub config: PathBuf,
}

/// `privacy-check [--base REF]`
#[derive(Debug, Clone)]
pub struct PrivacyArgs {
    pub base: String,
}

#[derive(Debug, Clone)]
pub enum Command {
    Accept(AcceptArgs),
    Calibrate(CalibrateArgs),
    PrivacyCheck(PrivacyArgs),
}

/// Everything a run produces besides the files it writes.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub exit_code: i32,
    pub stdout: String,
    /// One error per line (`trusted.txt:6: …`, `error: FAKEFLAC_CORPUS is not set …`).
    pub stderr: String,
    /// `FAKEFLAC_CALIBRATION/reports/<run>/`, when a report was written.
    pub report_dir: Option<PathBuf>,
    /// One per extraction fingerprint used by the run.
    pub cache: Vec<CacheStats>,
    /// One per params file, in order (the baseline of `calibrate` first).
    pub evaluations: Vec<Evaluation>,
    /// `calibrate` only.
    pub grid: Option<GridOutcome>,
}

/// Runs one command. Never panics on bad input: errors become `exit_code` 1, 2 or 3 with their
/// message in `stderr`. The transcoder is only asked for its version or to transcode when a
/// transcode is needed.
pub fn run(command: &Command, env: &Env, transcoder: &dyn Transcoder) -> Outcome {
    let result = match command {
        Command::Accept(args) => pipeline(
            &Job {
                config: &args.config,
                params: &args.params,
                grid: None,
                no_generate: args.no_generate,
            },
            env,
            transcoder,
        ),
        Command::Calibrate(args) => pipeline(
            &Job {
                config: &args.config,
                params: &[],
                grid: Some(&args.grid),
                no_generate: false,
            },
            env,
            transcoder,
        ),
        Command::PrivacyCheck(args) => crate::privacy::check(args, env),
    };
    match result {
        Ok(outcome) => outcome,
        Err(e) => Outcome {
            exit_code: e.kind.code(),
            stderr: e.lines.iter().map(|l| format!("{l}\n")).collect(),
            ..Outcome::default()
        },
    }
}

const USAGE: &str = "usage:
  fakeflac-calibrate accept [--params FILE]... [--config FILE] [--no-generate]
  fakeflac-calibrate calibrate [--grid FILE] [--config FILE]
  fakeflac-calibrate privacy-check [--base REF]";

fn usage_error(message: String) -> ToolError {
    let mut lines = vec![format!("error: {message}")];
    lines.extend(USAGE.lines().map(str::to_string));
    ToolError::input_lines(lines)
}

/// Parses the command line (`accept`, `calibrate`, `privacy-check` and their flags). The program
/// name is not part of `args`.
pub fn parse_args(args: &[String]) -> Result<Command, ToolError> {
    let Some((command, flags)) = args.split_first() else {
        return Err(usage_error("missing command".to_string()));
    };
    // `--flag value` and `--flag=value`, in order.
    let mut parsed: Vec<(String, Option<String>)> = Vec::new();
    let mut i = 0;
    while i < flags.len() {
        let flag = &flags[i];
        if let Some((name, value)) = flag.split_once('=') {
            parsed.push((name.to_string(), Some(value.to_string())));
        } else if flag == "--no-generate" {
            parsed.push((flag.clone(), None));
        } else if flag.starts_with("--") {
            i += 1;
            let Some(value) = flags.get(i) else {
                return Err(usage_error(format!("{flag} needs a value")));
            };
            parsed.push((flag.clone(), Some(value.clone())));
        } else {
            return Err(usage_error(format!("unexpected argument '{flag}'")));
        }
        i += 1;
    }
    let value_of = |name: &str| {
        parsed
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| v.clone())
    };
    let allowed: &[&str] = match command.as_str() {
        "accept" => &["--params", "--config", "--no-generate"],
        "calibrate" => &["--grid", "--config"],
        "privacy-check" => &["--base"],
        other => return Err(usage_error(format!("unknown command '{other}'"))),
    };
    if let Some((bad, _)) = parsed.iter().find(|(n, _)| !allowed.contains(&n.as_str())) {
        return Err(usage_error(format!("unknown option '{bad}' for {command}")));
    }
    if parsed
        .iter()
        .any(|(n, v)| n != "--no-generate" && v.as_deref().is_none_or(str::is_empty))
    {
        return Err(usage_error("an option has no value".to_string()));
    }
    let config = PathBuf::from(value_of("--config").unwrap_or_else(|| DEFAULT_CONFIG.to_string()));
    Ok(match command.as_str() {
        "accept" => Command::Accept(AcceptArgs {
            params: parsed
                .iter()
                .filter(|(n, _)| n == "--params")
                .filter_map(|(_, v)| v.clone().map(PathBuf::from))
                .collect(),
            config,
            no_generate: parsed.iter().any(|(n, _)| n == "--no-generate"),
        }),
        "calibrate" => Command::Calibrate(CalibrateArgs {
            grid: PathBuf::from(value_of("--grid").unwrap_or_else(|| DEFAULT_GRID.to_string())),
            config,
        }),
        _ => Command::PrivacyCheck(PrivacyArgs {
            base: value_of("--base").unwrap_or_else(|| "main".to_string()),
        }),
    })
}

/// The binary: arguments from the process, `Env::from_process()`, `FfmpegTranscoder`, prints
/// `stdout`/`stderr` of the outcome, returns the exit code.
pub fn cli_main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return 0;
    }
    let command = match parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            for line in &e.lines {
                eprintln!("{line}");
            }
            return e.kind.code();
        }
    };
    let env = Env::from_process();
    let transcoder = crate::ffmpeg::FfmpegTranscoder::new(env.ffmpeg.clone());
    let outcome = run(&command, &env, &transcoder);
    print!("{}", outcome.stdout);
    eprint!("{}", outcome.stderr);
    outcome.exit_code
}

// ---------------------------------------------------------------------------------------------
// Pipeline (calibration.md section 4)
// ---------------------------------------------------------------------------------------------

struct Job<'a> {
    config: &'a Path,
    params: &'a [PathBuf],
    grid: Option<&'a Path>,
    no_generate: bool,
}

struct Loaded {
    name: String,
    text: String,
    params: DetectionParams,
    hash: String,
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn read_text(path: &Path) -> Result<String, ToolError> {
    let bytes = std::fs::read(path)
        .map_err(|e| ToolError::input(format!("error: cannot read {}: {e}", file_name(path))))?;
    String::from_utf8(bytes)
        .map_err(|_| ToolError::input(format!("error: {} is not valid UTF-8", file_name(path))))
}

fn load_config(path: &Path) -> Result<Config, ToolError> {
    let text = read_text(path)?;
    Config::parse(&text).map_err(|e| {
        let name = file_name(path);
        ToolError {
            kind: e.kind,
            lines: e.lines.iter().map(|l| format!("{name}: {l}")).collect(),
        }
    })
}

/// Loads one params file of the v2 rule family (evaluation.md section 9).
fn load_params_text(name: &str, text: &str) -> Result<DetectionParams, ToolError> {
    // Any file that declares an algorithm_version below 2 is another rule family.
    if let Ok(table) = toml::from_str::<toml::Table>(text) {
        if table
            .get("algorithm_version")
            .and_then(toml::Value::as_integer)
            .is_some_and(|v| v < 2)
        {
            return Err(ToolError::input(format!(
                "{name}: not a DetectionParams file of the v2 rule family (v1 is not supported, ADR-0036)"
            )));
        }
    }
    match toml::from_str::<DetectionParams>(text) {
        Ok(p) => {
            // Values that would make the extraction divide by zero or loop forever.
            if p.fft.band_hz <= 0.0 || p.fft.target_bin_hz <= 0.0 || p.gaps.band_hz <= 0.0 {
                return Err(ToolError::input(format!(
                    "{name}: fft.band_hz, fft.target_bin_hz and gaps.band_hz must be positive"
                )));
            }
            if !(0.0..1.0).contains(&p.fft.overlap) {
                return Err(ToolError::input(format!(
                    "{name}: fft.overlap must be in [0, 1)"
                )));
            }
            Ok(p)
        }
        Err(e) => Err(ToolError::input_lines(
            e.to_string()
                .lines()
                .map(|l| format!("{name}: {l}"))
                .collect(),
        )),
    }
}

fn load_params(paths: &[PathBuf]) -> Result<Vec<Loaded>, ToolError> {
    let mut loaded: Vec<Loaded> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for path in paths {
        let name = file_name(path);
        let text = match read_text(path) {
            Ok(t) => t,
            Err(e) => {
                errors.extend(e.lines);
                continue;
            }
        };
        match load_params_text(&name, &text) {
            Ok(params) => {
                let hash = id16(text.as_bytes());
                if let Some(other) = loaded.iter().find(|l| l.hash == hash) {
                    errors.push(format!("{name}: same content as {}", other.name));
                    continue;
                }
                loaded.push(Loaded {
                    name,
                    text,
                    params,
                    hash,
                });
            }
            Err(e) => errors.extend(e.lines),
        }
    }
    if errors.is_empty() {
        Ok(loaded)
    } else {
        Err(ToolError::input_lines(errors))
    }
}

/// The labelled corpus tracks with album and split (inputs.md section 4).
fn build_items(lists: &Lists, config: &Config) -> Vec<CorpusItem> {
    let prefixes = &config.split.disc_folder_prefixes;
    let mut classes: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for t in &lists.trusted {
        classes
            .entry(album_id(&t.track.rel, prefixes))
            .or_default()
            .push(t.class.as_str());
    }
    for f in &lists.fakes {
        classes
            .entry(album_id(&f.track.rel, prefixes))
            .or_default()
            .push("positive");
    }
    let album_class: BTreeMap<String, &'static str> = classes
        .into_iter()
        .map(|(album, c)| (album, album_class_of(c.into_iter())))
        .collect();
    let mut by_class: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (album, class) in &album_class {
        by_class.entry(class).or_default().push(album.clone());
    }
    let mut split: BTreeMap<String, Split> = BTreeMap::new();
    for albums in by_class.values() {
        split.extend(assign_class(albums, &config.split));
    }
    let split_of = |album: &str| split.get(album).copied().unwrap_or(Split::Tune);

    let mut items: Vec<CorpusItem> = Vec::new();
    for t in &lists.trusted {
        let album = album_id(&t.track.rel, prefixes);
        items.push(CorpusItem {
            track: t.track.clone(),
            truth: Truth::Real,
            origin: "trusted",
            trust_origin: t.origin.clone(),
            class: t.class.clone(),
            split: split_of(&album),
            album_id: album,
            watch_id: t.watch_id.clone(),
        });
    }
    for f in &lists.fakes {
        let album = album_id(&f.track.rel, prefixes);
        items.push(CorpusItem {
            track: f.track.clone(),
            truth: Truth::Fake,
            origin: "confirmed_fake",
            trust_origin: String::new(),
            class: "positive".to_string(),
            split: split_of(&album),
            album_id: album,
            watch_id: String::new(),
        });
    }
    items.sort_by(|a, b| a.track.rel.cmp(&b.track.rel));
    items
}

// ---------------------------------------------------------------------------------------------
// Time and run folders
// ---------------------------------------------------------------------------------------------

/// UTC (year, month, day, hour, minute, second) of a Unix time in seconds.
fn utc_parts(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        year,
        month,
        day,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
    )
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// `YYYYMMDDTHHMMSSZ` and `YYYY-MM-DD HH:MM:SS`.
fn run_stamp(secs: i64) -> (String, String) {
    let (y, mo, d, h, mi, s) = utc_parts(secs);
    (
        format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z"),
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}"),
    )
}

/// Creates `reports/<run>` atomically; on `AlreadyExists` the id gets `-2`, `-3`... appended.
fn create_report_dir(calibration: &Path, run_id: &str) -> std::io::Result<(String, PathBuf)> {
    let reports = calibration.join("reports");
    std::fs::create_dir_all(&reports)?;
    let mut n = 1;
    loop {
        let id = if n == 1 {
            run_id.to_string()
        } else {
            format!("{run_id}-{n}")
        };
        let dir = reports.join(&id);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok((id, dir)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e),
        }
    }
}

fn tool_commit(env: &Env) -> String {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["rev-parse", "HEAD"])
        .stdin(std::process::Stdio::null());
    if let Some(root) = &env.repo_root {
        cmd.current_dir(root);
    }
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

// ---------------------------------------------------------------------------------------------

fn pipeline(job: &Job<'_>, env: &Env, transcoder: &dyn Transcoder) -> Result<Outcome, ToolError> {
    // 1. Environment and inputs: every exit-2 check comes before any write.
    let roots: Roots = env.check()?;
    let config = load_config(job.config)?;
    for s in &config.generate.settings {
        if !transcoder.supports(&s.encoder) {
            return Err(ToolError::input(format!(
                "error: {}: setting '{}' uses the encoder '{}', which this binary does not support",
                file_name(job.config),
                s.id,
                s.encoder
            )));
        }
    }
    let setting_ids: Vec<String> = config
        .generate
        .settings
        .iter()
        .map(|s| s.id.clone())
        .collect();
    let grid_file: Option<GridFile> = match job.grid {
        Some(path) => {
            let text = read_text(path)?;
            let mut parsed = grid::parse_grid(&text, &setting_ids).map_err(|e| ToolError {
                kind: e.kind,
                lines: e
                    .lines
                    .iter()
                    .map(|l| format!("{}: {l}", file_name(path)))
                    .collect(),
            })?;
            parsed.baseline = parsed.baseline.trim().to_string();
            Some(parsed)
        }
        None => None,
    };
    let param_paths: Vec<PathBuf> = match &grid_file {
        Some(g) => vec![PathBuf::from(&g.baseline)],
        None if job.params.is_empty() => vec![PathBuf::from(DEFAULT_PARAMS)],
        None => job.params.to_vec(),
    };
    let loaded = load_params(&param_paths)?;
    let lists = lists::load(&roots.calibration, &roots.corpus)?;
    let items = build_items(&lists, &config);

    // 2. Extraction, one pass per extraction fingerprint.
    let mut fingerprints: Vec<(crate::cache::Fingerprint, usize)> = Vec::new();
    let mut group_of: Vec<usize> = Vec::new();
    for (i, l) in loaded.iter().enumerate() {
        let fp = crate::cache::fingerprint(&l.params);
        match fingerprints.iter().position(|(f, _)| f.id == fp.id) {
            Some(g) => group_of.push(g),
            None => {
                fingerprints.push((fp, i));
                group_of.push(fingerprints.len() - 1);
            }
        }
    }
    let ctx = Context {
        config: &config,
        roots: &roots,
        transcoder,
        no_generate: job.no_generate,
        threads: worker_threads(),
    };
    let mut groups: Vec<GroupResult> = Vec::new();
    for (fp, first) in &fingerprints {
        let mut group = extract_group(&ctx, &items, &loaded[*first].params, fp)?;
        group.rows.sort_by(|a, b| {
            (&a.album_id, a.root, &a.path, &a.setting).cmp(&(
                &b.album_id,
                b.root,
                &b.path,
                &b.setting,
            ))
        });
        groups.push(group);
    }

    // 3. Verdicts of every params file.
    let make_run = |l: &Loaded, group: usize, groups: &[GroupResult]| ParamsRun {
        name: l.name.clone(),
        params: l.params.clone(),
        hash: l.hash.clone(),
        fingerprint_id: groups[group].stats.fingerprint_id.clone(),
        group,
        evals: groups[group]
            .rows
            .iter()
            .map(|r| evaluate_row(&r.extracted, &l.params))
            .collect(),
    };
    let mut runs: Vec<ParamsRun> = loaded
        .iter()
        .zip(group_of.iter())
        .map(|(l, g)| make_run(l, *g, &groups))
        .collect();

    // 4. Grid search (calibrate). The proposal is built and checked before anything is written
    // (exit 1 leaves nothing new but cache lines).
    let (run_stamp_id, date) = run_stamp(now_secs());
    // The folder is reserved atomically here (retry on AlreadyExists) and removed again if the run
    // ends in an error, so an exit 1 leaves nothing visible but cache lines.
    let (run_id, report_dir) = create_report_dir(&roots.calibration, &run_stamp_id)?;
    let outcome = (|| -> Result<Outcome, ToolError> {
        let mut stdout = String::new();
        let mut grid_outcome: Option<GridOutcome> = None;
        let mut search_result: Option<grid::SearchResult> = None;
        let mut proposal: Option<grid::Proposal> = None;
        if let Some(file) = &grid_file {
            let base = &runs[0];
            let search_rows = tune_search_rows(&groups[base.group].rows, &base.evals);
            let result = grid::search(&base.params, &search_rows, file);
            if let Some(i) = result.proposal {
                let values = &result.points[i].values;
                proposal = Some(grid::build_proposal(
                    &loaded[0].text,
                    &base.params,
                    file,
                    values,
                    &run_id,
                )?);
            }
            search_result = Some(result);
        }
        if let Some(p) = &proposal {
            let hash = id16(p.text.as_bytes());
            let group = runs[0].group;
            let l = Loaded {
                name: p.file_name.clone(),
                text: p.text.clone(),
                params: p.params.clone(),
                hash,
            };
            let run = make_run(&l, group, &groups);
            runs.push(run);
        }

        // 5. Tables.
        let mut evaluations: Vec<Evaluation> = Vec::new();
        for (i, run) in runs.iter().enumerate() {
            let baseline = (i > 0).then(|| (&runs[0], groups[runs[0].group].rows.as_slice()));
            evaluations.push(evaluation(run, &groups[run.group].rows, &config, baseline));
        }

        // 6. Files: the report folder, then labels.csv last, so that an exit 1 leaves only cache lines.
        cleanup_generated(&config, &groups, &runs);

        let proposal_path = proposal.as_ref().map(|p| report_dir.join(&p.file_name));
        if let (Some(p), Some(path)) = (&proposal, &proposal_path) {
            write_atomic(path, p.text.as_bytes())?;
        }
        let mode = if grid_file.is_some() {
            "calibrate"
        } else {
            "accept"
        };
        let commit = tool_commit(env);
        // Confirmed fakes checked by constraint (2): all of them, tune and holdout.
        let confirmed = {
            let base = &runs[0];
            let checked = groups[base.group]
                .rows
                .iter()
                .zip(base.evals.iter())
                .filter(|(r, e)| r.origin == "confirmed_fake" && e.verdict != Verdict::Error);
            let (mut all, mut holdout) = (0_u32, 0_u32);
            for (r, _) in checked {
                all += 1;
                holdout += u32::from(r.split == Split::Holdout);
            }
            (all, holdout)
        };
        let data = ReportData {
            run_id: &run_id,
            date: &date,
            commit: &commit,
            mode,
            config: &config,
            roots: &roots,
            groups: &groups,
            runs: &runs,
            evaluations: &evaluations,
            grid: match (&grid_file, &search_result) {
                (Some(file), Some(result)) => Some(GridReport {
                    file,
                    result,
                    proposal: proposal.as_ref(),
                    confirmed_checked: confirmed.0,
                    confirmed_holdout: confirmed.1,
                }),
                _ => None,
            },
        };
        write_atomic(
            &report_dir.join("tracks.csv"),
            render::tracks_csv(&runs, &groups).as_bytes(),
        )?;
        write_atomic(
            &report_dir.join("summary.md"),
            render::markdown(&data, false).as_bytes(),
        )?;
        write_atomic(
            &report_dir.join("report.md"),
            render::markdown(&data, true).as_bytes(),
        )?;
        // labels.csv is rewritten by every run. Under --no-generate the rows are incomplete
        // (uncached tracks and entries are skipped), so it is left as the last full run wrote it.
        if !job.no_generate {
            write_atomic(
                &roots.calibration.join("labels.csv"),
                render::labels_csv(&groups[group_of[0]].rows).as_bytes(),
            )?;
        }

        if let Some(result) = &search_result {
            let to_score = |s: &grid::Scored| PointScore {
                values: grid_file
                    .iter()
                    .flat_map(|f| f.keys.iter())
                    .zip(s.values.iter())
                    .map(|(k, v)| (k.key.clone(), *v))
                    .collect(),
                cost: s.cost,
                feasible: s.feasible,
            };
            grid_outcome = Some(GridOutcome {
                baseline: to_score(&result.baseline),
                points: result.points.iter().map(to_score).collect(),
                skipped: result.skipped,
                proposal: proposal_path.clone(),
            });
            match &proposal {
            Some(p) => stdout.push_str(&format!(
                "proposal written: {}\n",
                proposal_path.as_ref().map_or(p.file_name.clone(), |x| {
                    crate::fsutil::plain_path(x).display().to_string()
                })
            )),
            None if result.baseline.feasible => {
                stdout.push_str("no change proposed (the baseline is at least as good)\n");
            }
            None => stdout.push_str(
                "no change proposed (no feasible point; see the best infeasible points in the report)\n",
            ),
        }
        }
        for (e, run) in evaluations.iter().zip(runs.iter()) {
            stdout.push_str(&format!(
                "{}: primary figure {}\n",
                e.params_name, e.primary_figure
            ));
            if mode == "calibrate" {
                let hold = class_totals(
                    &groups[run.group].rows,
                    &run.evals,
                    "primary",
                    crate::report::SplitSel::Holdout,
                );
                stdout.push_str(&format!(
                    "  holdout primary figure (headline): {}\n",
                    figure(&hold, config.report.z)
                ));
            }
            for t in e
                .targets
                .iter()
                .filter(|t| t.split == crate::report::SplitSel::All)
            {
                stdout.push_str(&format!(
                    "  {} {} ({})\n",
                    t.id,
                    if t.pass { "PASS" } else { "FAIL" },
                    t.count
                ));
            }
        }
        if grid_file.is_some() {
            stdout.push_str(&format!(
                "confirmed fakes checked by the feasibility constraint: {} ({} in holdout)\n",
                confirmed.0, confirmed.1
            ));
        }
        stdout.push_str(&format!(
            "report: {}\n",
            crate::fsutil::plain_path(&report_dir).display()
        ));

        Ok(Outcome {
            exit_code: 0,
            stdout,
            stderr: String::new(),
            report_dir: Some(report_dir.clone()),
            cache: groups.iter().map(|g| g.stats.clone()).collect(),
            evaluations,
            grid: grid_outcome,
        })
    })();
    if outcome.is_err() {
        let _ = std::fs::remove_dir_all(&report_dir);
    }
    outcome
}

/// The rows the grid search reads: tune negatives of the classes primary, reviewed and clean,
/// generated positives and confirmed fakes, without `Error` rows, plus the holdout confirmed fakes as
/// constraint-only rows (evaluation.md section 7).
fn tune_search_rows<'a>(rows: &'a [LabelRow], evals: &[RowEval]) -> Vec<SearchRow<'a>> {
    let mut out = Vec::new();
    for (row, eval) in rows.iter().zip(evals.iter()) {
        if eval.verdict == Verdict::Error {
            continue;
        }
        // Holdout rows are never read, except confirmed fakes: they constrain every point.
        let constraint_only = row.split != Split::Tune;
        if constraint_only && row.origin != "confirmed_fake" {
            continue;
        }
        let kind = if row.origin == "confirmed_fake" {
            RowKind::ConfirmedFake
        } else if row.origin == "generated" {
            RowKind::GeneratedPositive
        } else if row.truth == Truth::Real
            && matches!(row.class.as_str(), "primary" | "reviewed" | "clean")
        {
            RowKind::Negative
        } else {
            continue;
        };
        let spectra = match row.extracted.as_ref() {
            crate::cache::Extracted::Probe(p) => p.spectrum.as_ref(),
            crate::cache::Extracted::Error(_) => None,
        };
        out.push(SearchRow {
            kind,
            setting: if kind == RowKind::GeneratedPositive {
                row.setting.as_str()
            } else {
                ""
            },
            spectra,
            silent_verdict: eval.verdict,
            constraint_only,
        });
    }
    out
}

/// Generated audio is kept with `keep_audio`, or only for misclassified tracks with
/// `keep_misclassified` (OQ-9); everything else the run left on disk is deleted.
fn cleanup_generated(config: &Config, groups: &[GroupResult], runs: &[ParamsRun]) {
    if config.generate.keep_audio {
        return;
    }
    let mut retained: std::collections::BTreeSet<&Path> = std::collections::BTreeSet::new();
    if config.generate.keep_misclassified {
        for (g, group) in groups.iter().enumerate() {
            for (identity, path) in &group.kept {
                let Some(ri) = group.rows.iter().position(|r| &r.identity() == identity) else {
                    continue;
                };
                let truth = group.rows[ri].truth;
                let bad = runs.iter().filter(|r| r.group == g).any(|r| {
                    matches!(
                        (truth, r.evals[ri].verdict),
                        (Truth::Fake, Verdict::Real) | (Truth::Real, Verdict::Fake)
                    )
                });
                if bad {
                    retained.insert(path.as_path());
                }
            }
        }
    }
    for group in groups {
        for (_, path) in &group.kept {
            if !retained.contains(path.as_path()) {
                let _ = std::fs::remove_file(path);
                let _ = std::fs::remove_file(path.with_extension("json"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_civil_utc() {
        assert_eq!(utc_parts(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(utc_parts(1_791_990_000), (2026, 10, 14, 15, 0, 0));
        assert_eq!(run_stamp(86_400 * 365).0, "19710101T000000Z");
    }

    #[test]
    fn command_lines() {
        let args = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        let Ok(Command::Accept(a)) = parse_args(&args(&[
            "accept",
            "--params",
            "a.toml",
            "--params=b.toml",
            "--no-generate",
        ])) else {
            panic!("accept");
        };
        assert_eq!(a.params.len(), 2);
        assert!(a.no_generate);
        assert_eq!(a.config, PathBuf::from(DEFAULT_CONFIG));
        assert!(parse_args(&args(&["accept", "--nope"])).is_err());
        assert!(parse_args(&args(&[])).is_err());
        let Ok(Command::PrivacyCheck(p)) = parse_args(&args(&["privacy-check"])) else {
            panic!("privacy");
        };
        assert_eq!(p.base, "main");
    }

    #[test]
    fn confirmed_fakes_reach_the_grid_search() {
        use crate::cache::Extracted;
        use crate::model::{RowRoot, Truth};
        use fakeflac_core::TrackError;
        use std::sync::Arc;

        let row = |origin: &'static str, truth: Truth, class: &str, split: Split| LabelRow {
            root: RowRoot::Corpus,
            path: format!("{origin}-{class}.flac"),
            truth,
            origin,
            trust_origin: String::new(),
            class: class.to_string(),
            encoder: "none".to_string(),
            setting: if origin == "generated" {
                "fx128"
            } else {
                "none"
            }
            .to_string(),
            bitrate_kbps: None,
            encoder_version: String::new(),
            fragment: None,
            source_path: String::new(),
            album_id: "a".to_string(),
            split,
            watch_id: String::new(),
            extracted: Arc::new(Extracted::Error(TrackError::TooShort)),
        };
        let eval = |verdict: Verdict| RowEval {
            verdict,
            reasons: Vec::new(),
            error: None,
            m: None,
            margins: None,
        };
        let rows = vec![
            row("confirmed_fake", Truth::Fake, "positive", Split::Tune),
            row("trusted", Truth::Real, "primary", Split::Tune),
            row("generated", Truth::Fake, "positive", Split::Tune),
            row("trusted", Truth::Real, "reference", Split::Tune),
            row("confirmed_fake", Truth::Fake, "positive", Split::Holdout),
            row("trusted", Truth::Real, "clean", Split::Tune),
        ];
        let mut evals: Vec<RowEval> = vec![
            eval(Verdict::Real),
            eval(Verdict::Real),
            eval(Verdict::Fake),
            eval(Verdict::Real),
            eval(Verdict::Real),
            eval(Verdict::Error),
        ];
        let search = tune_search_rows(&rows, &evals);
        let kinds: Vec<RowKind> = search.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            vec![
                RowKind::ConfirmedFake,
                RowKind::Negative,
                RowKind::GeneratedPositive,
                RowKind::ConfirmedFake
            ]
        );
        let only: Vec<bool> = search.iter().map(|r| r.constraint_only).collect();
        assert_eq!(only, vec![false, false, false, true]);
        assert_eq!(search[2].setting, "fx128");
        // A confirmed fake classified Real makes the point infeasible.
        let t = grid::tally(
            search.iter().map(|r| (r.kind, r.setting, evals[0].verdict)),
            &[],
        );
        assert!(!t.feasible());
        evals[0] = eval(Verdict::Fake);
        let t = grid::tally(
            search.iter().map(|r| (r.kind, r.setting, evals[0].verdict)),
            &[],
        );
        assert!(t.feasible());
    }
}

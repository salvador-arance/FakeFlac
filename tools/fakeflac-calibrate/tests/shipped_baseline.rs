//! Tests derived from `specs/features/calibration.feature` (@RP-11): calibrating against the shipped
//! detection (`specs/calibration/calibration.md` OQ-18 to OQ-24, `specs/calibration/evaluation.md`
//! section 7, `specs/calibration/calibrate-grid.v3.toml`): `must_catch` as non-regression against the
//! baseline, confirmed fakes that lose `Fake`, `cost_exclude`, the default params file and grid, and
//! the supported grid versions. Protected after the `test(RP-11)` commit.
//!
//! The `must_catch` and confirmed-fake scenarios use a small variant of the sample corpus with ONE
//! transcode source (track 1, `full_band`), so that every setting has exactly one generated positive.
//! The fixture transcoder ignores the source audio, so all transcodes of one setting are identical
//! and a count of 1 needs a single source. Measured with v2 on these fixtures (`tracks.csv`):
//! `lp16k_dither` G 49.3 dB, D 49.1 dB at 16050 Hz (Fake by gap and step); `sfb21_gaps` G 19.0 dB,
//! D 6.15 dB (Fake by gap only); `sfb21_gaps_weak` G 5.0 dB, D 3.9 dB (Suspicious, gap);
//! `full_band` G 0.1 dB, D 0.2 dB (Real).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::grid::get_value;
use fakeflac_calibrate::report::GridOutcome;
use fakeflac_calibrate::run::{parse_args, DEFAULT_GRID, DEFAULT_PARAMS};
use fakeflac_calibrate::Outcome;
use fakeflac_core::DetectionParams;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

/// The default weights of the sample grid (evaluation.md section 7).
const DEFAULT_WEIGHTS: [f64; 4] = [10.0, 1.0, 0.5, 0.1];
/// `fn = 0`: a point that turns positives `Real` costs nothing for it, so it can cost less than the
/// baseline on the tune rows ("on the tune rows that candidate costs less than the baseline").
const FREE_FN_WEIGHTS: [f64; 4] = [10.0, 0.0, 0.5, 0.1];

/// A grid file. `baseline: None` leaves the key out; `cost_exclude: None` leaves the key out.
fn grid_text(
    version: u32,
    baseline: Option<&Path>,
    must_catch: &[&str],
    cost_exclude: Option<&[&str]>,
    weights: [f64; 4],
    values: &str,
) -> String {
    let list = |ids: &[&str]| {
        ids.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut text = format!("grid_version = {version}\n");
    if let Some(b) = baseline {
        text.push_str(&format!("baseline = '{}'\n", b.display()));
    }
    text.push_str(&format!("must_catch = [{}]\n", list(must_catch)));
    if let Some(ids) = cost_exclude {
        text.push_str(&format!("cost_exclude = [{}]\n", list(ids)));
    }
    text.push_str(&format!(
        "max_points = 1000\n\n[weights]\nfp = {:?}\nfn = {:?}\nsuspicious_negative = {:?}\nsuspicious_positive = {:?}\n\n[values]\n{values}\n",
        weights[0], weights[1], weights[2], weights[3]
    ));
    text
}

/// The `[values]` of the grid of sample-corpus.md section 5.
const GAP_GRID: &str = "\"verdict.gap_fake_db\" = [4.0, 6.0]";

/// The grid of sample-corpus.md section 5 (baseline v2, `must_catch = ["fx128"]`, default weights),
/// with the given version and optional `cost_exclude`.
fn sample_grid(sample: &Sample, version: u32, cost_exclude: Option<&[&str]>) -> PathBuf {
    sample.write_scratch(
        "grid.toml",
        &grid_text(
            version,
            Some(&v2_params()),
            &["fx128"],
            cost_exclude,
            DEFAULT_WEIGHTS,
            GAP_GRID,
        ),
    )
}

const WEAK_NEGATIVE: &str = "Extras/Débil/01 – Huecos.flac";

/// The one-source variant of the sample corpus: `trusted.txt` lists only track 1 (`full_band`,
/// `tienda`, class primary), `confirmed.txt` only the confirmed fake (track 11, `lp16k_dither`), and
/// the config has the settings `fx128` (`lp16k_dither`), `fxv0` (`sfb21_gaps`), `fxweak`
/// (`sfb21_gaps_weak`) and `fxreal` (`full_band`, so its transcode is `Real` under v2), with
/// `holdout_fraction = 0.0` (everything `tune`). With `weak_negative`, a second trusted track with
/// the fixture `sfb21_gaps_weak` (class clean, `Suspicious` under v2) is added; it is a transcode
/// source too.
fn one_source_sample(name: &str, weak_negative: bool) -> Sample {
    let sample = Sample::new(name);
    let mut trusted = vec![
        "# One-source variant of the sample trusted list.".to_string(),
        format!("library_root = {}", sample.corpus.display()),
        String::new(),
        "Compras\\Ñandú – Ácido, Vol. 1 (2024)\\01 – Intro ’A’.flac | tienda".to_string(),
    ];
    if weak_negative {
        place(
            "sfb21_gaps_weak",
            Container::Flac,
            &sample.corpus.join(WEAK_NEGATIVE),
        );
        trusted.push("Extras\\Débil | analisis-limpio".to_string());
    }
    sample.write_trusted(&trusted);
    sample.write_confirmed(&[
        format!("library_root = {}", sample.corpus.display()),
        "Sueltas\\Mixto\\06 – Falso.flac | fake | espectrograma".to_string(),
    ]);

    let spec = read(&spec_config());
    let head = spec[..spec.find("[[generate.settings]]").unwrap()]
        .replace("holdout_fraction = 0.30", "holdout_fraction = 0.0");
    let report = &spec[spec.find("[report]").unwrap()..spec.find("[[targets]]").unwrap()];
    let mut text = head;
    for (id, fixture) in [
        ("fx128", "lp16k_dither"),
        ("fxv0", "sfb21_gaps"),
        ("fxweak", "sfb21_gaps_weak"),
        ("fxreal", "full_band"),
    ] {
        text.push_str(&format!(
            "[[generate.settings]]\nid = \"{id}\"\nencoder = \"fixture\"\next = \"mp3\"\nargs = [\"{fixture}\"]\n\n"
        ));
    }
    text.push_str(report);
    text.push_str(
        "[[targets]]\nid = \"T1\"\ndescription = \"No primary track is Fake\"\nclasses = [\"primary\"]\nverdict = \"fake\"\nmax_count = 0\n\n\
         [[targets]]\nid = \"T2\"\ndescription = \"No fx128 transcode is Real\"\nsettings = [\"fx128\"]\nverdict = \"real\"\nmax_count = 0\n",
    );
    std::fs::write(sample.config_path(), text).unwrap();
    sample
}

/// The lines of the `## Grid search` section of a report file.
fn grid_section(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("## ") {
            inside = line == "## Grid search";
            continue;
        }
        if inside {
            out.push(line.to_string());
        }
    }
    assert!(!out.is_empty(), "no \"## Grid search\" section:\n{text}");
    out
}

/// The line that follows the `- baseline: cost …` line of the "Grid search" section.
fn line_after_baseline(text: &str) -> String {
    let lines = grid_section(text);
    let at = lines
        .iter()
        .position(|l| l.starts_with("- baseline: cost"))
        .unwrap_or_else(|| panic!("no \"- baseline: cost\" line:\n{}", lines.join("\n")));
    lines.get(at + 1).cloned().unwrap_or_default()
}

/// Cells of a Markdown table row (`| a | b |`), trimmed, with `\|` unescaped.
fn md_cells(line: &str) -> Vec<String> {
    let line = line.trim().replace("\\|", "\u{1}");
    line.trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|c| c.trim().replace('\u{1}', "|"))
        .collect()
}

/// The Markdown table that follows the line `heading`: header and rows.
fn md_table(text: &str, heading: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut lines = text
        .lines()
        .skip_while(|l| l.trim() != heading)
        .skip(1)
        .skip_while(|l| l.trim().is_empty() || !l.trim_start().starts_with('|'));
    let header = md_cells(
        lines
            .next()
            .unwrap_or_else(|| panic!("no table after {heading:?}")),
    );
    let _separator = lines.next();
    let rows = lines
        .take_while(|l| l.trim_start().starts_with('|'))
        .map(md_cells)
        .collect();
    (header, rows)
}

/// The best-points table of the "Grid search" section of `summary.md`.
struct BestPoints {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl BestPoints {
    fn of(outcome: &Outcome) -> BestPoints {
        let summary = report_text(outcome, "summary.md");
        let (header, rows) = md_table(&grid_section(&summary).join("\n"), "### Best points");
        BestPoints { header, rows }
    }

    fn cell<'a>(&self, row: &'a [String], column: &str) -> &'a str {
        let at = self
            .header
            .iter()
            .position(|h| h == column)
            .unwrap_or_else(|| {
                panic!(
                    "the best-points table has no column {column:?}: {:?}",
                    self.header
                )
            });
        &row[at]
    }

    /// The row whose rank is `rank` (`baseline`, `1`, `2`…).
    fn rank(&self, rank: &str) -> &[String] {
        self.rows
            .iter()
            .find(|r| self.cell(r, "rank") == rank)
            .unwrap_or_else(|| panic!("no row of rank {rank:?} in {:?}", self.rows))
    }

    /// The row of a point whose changed values contain `change` (`verdict.gap_fake_db 6.0 -> 4.0`).
    fn changed(&self, change: &str) -> &[String] {
        self.rows
            .iter()
            .find(|r| {
                self.cell(r, "rank") != "baseline"
                    && self.cell(r, "changed values").contains(change)
            })
            .unwrap_or_else(|| panic!("no point row with {change:?} in {:?}", self.rows))
    }
}

/// The point of the grid outcome whose `key` has `value`.
fn point<'a>(
    grid: &'a GridOutcome,
    key: &str,
    value: f64,
) -> &'a fakeflac_calibrate::report::PointScore {
    grid.points
        .iter()
        .find(|p| p.values.iter().any(|(k, v)| k == key && *v == value))
        .unwrap_or_else(|| panic!("no point with {key} = {value}"))
}

/// The `tracks.csv` row of `path` under the v2 baseline.
fn baseline_row<'a>(t: &'a Table, path: &str) -> &'a [String] {
    t.rows
        .iter()
        .find(|r| t.get(r, "params_file") == "detection-params.v2.toml" && t.get(r, "path") == path)
        .unwrap_or_else(|| panic!("no baseline row for {path}"))
}

fn no_proposal_file(outcome: &Outcome) {
    let report_dir = outcome.report_dir.as_ref().expect("report folder");
    let proposals: Vec<_> = files_under(report_dir)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("detection-params"))
        })
        .collect();
    assert!(
        proposals.is_empty(),
        "no params file is written: {proposals:?}"
    );
    assert!(outcome.grid.as_ref().unwrap().proposal.is_none());
}

/// The file name of the default params file (the constant of the tool, never named here).
fn default_params_name() -> String {
    Path::new(DEFAULT_PARAMS)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// The `## Parameter files` table of `summary.md`: (file, algorithm_version) per row.
fn parameter_files(outcome: &Outcome) -> Vec<(String, String)> {
    let summary = report_text(outcome, "summary.md");
    let (header, rows) = md_table(&summary, "## Parameter files");
    let file = header.iter().position(|h| h == "file").unwrap();
    let version = header
        .iter()
        .position(|h| h == "algorithm_version")
        .unwrap();
    rows.iter()
        .map(|r| (r[file].clone(), r[version].clone()))
        .collect()
}

/// "Given the current folder is not the repository root": `cargo test` runs from the package folder
/// `tools/fakeflac-calibrate`, so defaults resolved against the current folder instead of the
/// repository root (calibration.md section 2) are not found.
fn assert_not_in_repo_root() {
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let root = repo_root().canonicalize().unwrap();
    assert_ne!(
        cwd, root,
        "precondition: the current folder is not the repository root"
    );
}

// ---------------------------------------------------------------------------------------------
// must_catch as non-regression (calibration.md OQ-19) and confirmed fakes (OQ-20)
// ---------------------------------------------------------------------------------------------

// Scenario: A must_catch positive that the baseline already misses does not block the search (@RP-11)
//
// One source: `fxreal` (`full_band`) has one detectable tune positive, `Real` under v2 and under both
// points of the sample grid (`G` 0.1 dB); `fx128` (the other must_catch setting) is `Fake` under
// both, and so is the confirmed fake.
#[test]
fn rp11_a_must_catch_positive_the_baseline_already_misses_does_not_block_the_search() {
    let sample = one_source_sample("mc-baseline-miss", false);
    let grid = sample.write_scratch(
        "grid.toml",
        &grid_text(
            1,
            Some(&v2_params()),
            &["fx128", "fxreal"],
            None,
            DEFAULT_WEIGHTS,
            GAP_GRID,
        ),
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    // Preconditions (Given): the fxreal positive is Real under the baseline, the confirmed fake Fake.
    let t = tracks(&o);
    let base_rows: Vec<&Vec<String>> = t
        .rows
        .iter()
        .filter(|r| t.get(r, "params_file") == "detection-params.v2.toml")
        .collect();
    let fxreal: Vec<&&Vec<String>> = base_rows
        .iter()
        .filter(|r| t.get(r, "setting") == "fxreal")
        .collect();
    assert_eq!(fxreal.len(), 1, "one fxreal positive");
    assert_eq!(t.get(fxreal[0], "verdict"), "real");
    assert_eq!(t.get(fxreal[0], "split"), "tune");
    let fxreal_path = t.get(fxreal[0], "path").to_string();
    for r in base_rows.iter().filter(|r| t.get(r, "setting") == "fx128") {
        assert_eq!(
            t.get(r, "verdict"),
            "fake",
            "fx128 is Fake under the baseline"
        );
    }
    let confirmed = baseline_row(&t, TRACK_11);
    assert_eq!(t.get(confirmed, "verdict"), "fake");
    let result = o.grid.as_ref().expect("grid outcome");
    assert!(result.points.len() <= 10);

    // Then the baseline and the candidate point are feasible.
    let candidate = point(result, "verdict.gap_fake_db", 4.0);
    assert!(
        result.baseline.feasible,
        "a must_catch positive that the baseline already misses does not make the baseline infeasible"
    );
    assert!(
        candidate.feasible,
        "nor a point under which it is still Real (and no other must_catch positive is Real)"
    );

    // And the line "- must_catch Real at baseline: 1" right after the "- baseline: cost" line.
    let summary = report_text(&o, "summary.md");
    assert_eq!(
        line_after_baseline(&summary),
        "- must_catch Real at baseline: 1",
        "summary.md"
    );

    // And report.md lists that positive under "### must_catch Real at baseline".
    let report = report_text(&o, "report.md");
    let (header, rows) = md_table(&report, "### must_catch Real at baseline");
    for column in ["path", "setting", "split", "G", "D", "fs", "fe"] {
        assert!(
            header.iter().any(|h| h == column),
            "column {column} in {header:?}"
        );
    }
    let col = |name: &str| header.iter().position(|h| h == name).unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0][col("path")], fxreal_path);
    assert_eq!(rows[0][col("setting")], "fxreal");
    assert_eq!(rows[0][col("split")], "tune");

    // And in the best-points table the baseline row and the candidate row both have
    // "must_catch Real" 1 and "must_catch lost" 0.
    let table = BestPoints::of(&o);
    let base_row = table.rank("baseline");
    let cand_row = table.changed("verdict.gap_fake_db 6.0 -> 4.0");
    for (name, row) in [("baseline", base_row), ("candidate", cand_row)] {
        assert_eq!(table.cell(row, "must_catch Real"), "1", "{name} row");
        assert_eq!(table.cell(row, "must_catch lost"), "0", "{name} row");
    }

    // And that positive still counts as a false negative in the cost of both: baseline
    // 1.0 (fxreal Real) + 0.1 (fxweak Suspicious) = 1.1; gap_fake_db 4.0 makes fxweak Fake: 1.0.
    assert!(
        (result.baseline.cost - 1.1).abs() < 1e-9,
        "baseline cost {}",
        result.baseline.cost
    );
    assert!(
        (candidate.cost - 1.0).abs() < 1e-9,
        "candidate cost {}",
        candidate.cost
    );
}

/// Scenario Outline: A point that loses a must_catch detection of the baseline is infeasible (@RP-11).
///
/// One source; the only point raises `gap_fake_db` and `gap_suspicious_db` to 30 dB, so `fxv0`
/// (`G` 19 dB, `D` 6.15 dB: Fake under v2) and `fxweak` (`G` 5 dB: Suspicious under v2) become `Real`,
/// while `fx128` and the confirmed fake (`G` and `D` 49 dB) stay `Fake`. With `fn = 0` the point costs
/// 0 and the baseline 0.1 (fxweak Suspicious).
fn must_catch_lost(test: &str, must_catch: &str, baseline_verdict: &str) {
    let sample = one_source_sample(test, false);
    let grid = sample.write_scratch(
        "grid.toml",
        &grid_text(
            1,
            Some(&v2_params()),
            &[must_catch],
            None,
            FREE_FN_WEIGHTS,
            "\"verdict.gap_fake_db\" = [30.0]\n\"verdict.gap_suspicious_db\" = [30.0]",
        ),
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    // Preconditions (Given).
    let t = tracks(&o);
    let positive = t
        .rows
        .iter()
        .find(|r| {
            t.get(r, "params_file") == "detection-params.v2.toml"
                && t.get(r, "setting") == must_catch
        })
        .expect("the must_catch positive");
    assert_eq!(t.get(positive, "verdict"), baseline_verdict);
    assert_eq!(t.get(positive, "split"), "tune");
    let result = o.grid.as_ref().expect("grid outcome");
    assert_eq!(result.points.len(), 1);
    let candidate = &result.points[0];
    assert!(
        candidate.cost < result.baseline.cost,
        "the candidate ({}) costs less than the baseline ({}) on the tune rows",
        candidate.cost,
        result.baseline.cost
    );

    // Then the candidate point is marked infeasible.
    assert!(
        !candidate.feasible,
        "the point loses a must_catch detection"
    );
    assert!(result.baseline.feasible, "the baseline is feasible on (1)");

    // And its row in the best-points table has "must_catch lost" 1.
    let table = BestPoints::of(&o);
    assert_eq!(table.cell(table.rank("1"), "must_catch lost"), "1");
    assert_eq!(table.cell(table.rank("baseline"), "must_catch lost"), "0");

    // And the line "- must_catch Real at baseline: 0".
    let summary = report_text(&o, "summary.md");
    assert!(
        grid_section(&summary)
            .iter()
            .any(|l| l == "- must_catch Real at baseline: 0"),
        "summary.md:\n{}",
        grid_section(&summary).join("\n")
    );

    // And it is not proposed.
    no_proposal_file(&o);
}

// Scenario Outline: A point that loses a must_catch detection of the baseline is infeasible (@RP-11), row "fake"
#[test]
fn rp11_a_point_that_loses_a_must_catch_detection_is_infeasible_fake() {
    must_catch_lost("mc-lost-fake", "fxv0", "fake");
}

// Scenario Outline: A point that loses a must_catch detection of the baseline is infeasible (@RP-11), row "suspicious"
#[test]
fn rp11_a_point_that_loses_a_must_catch_detection_is_infeasible_suspicious() {
    must_catch_lost("mc-lost-suspicious", "fxweak", "suspicious");
}

// Scenario: A point under which a confirmed fake loses Fake is infeasible (@RP-11)
//
// One source plus a clean negative with the fixture `sfb21_gaps_weak` (`Suspicious` under v2). The
// only point raises `gap_fake_db`, `gap_suspicious_db` and `step_fake_db` to 60 dB: the confirmed fake
// (`G` and `D` 49 dB, `fs` 16050 Hz) loses V3 and V4 and is `Suspicious` by its step (V6,
// `step_suspicious_db` 7 dB), never `Real`; the weak negative becomes `Real`. With `fn = 0` the point
// costs 0.3 and the baseline 0.7.
#[test]
fn rp11_a_point_under_which_a_confirmed_fake_loses_fake_is_infeasible() {
    let sample = one_source_sample("confirmed-lost", true);
    let grid = sample.write_scratch(
        "grid.toml",
        &grid_text(
            1,
            Some(&v2_params()),
            &["fx128"],
            None,
            FREE_FN_WEIGHTS,
            "\"verdict.gap_fake_db\" = [60.0]\n\"verdict.gap_suspicious_db\" = [60.0]\n\
             \"verdict.step_fake_db\" = [60.0]",
        ),
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    // Preconditions (Given): the confirmed fake is Fake under the baseline and Suspicious under the
    // point (the point is checked through an accept run of the same parameters).
    let t = tracks(&o);
    let confirmed = baseline_row(&t, TRACK_11);
    assert_eq!(t.get(confirmed, "verdict"), "fake");
    let point_params = sample.write_scratch(
        "point.toml",
        &read(&v2_params())
            .replace("algorithm_version = 2", "algorithm_version = 90")
            .replace("gap_fake_db = 6.0", "gap_fake_db = 60.0")
            .replace("gap_suspicious_db = 4.0", "gap_suspicious_db = 60.0")
            .replace("step_fake_db = 10.0", "step_fake_db = 60.0"),
    );
    let check = sample.accept(&[point_params]);
    assert_ok(&check);
    let ct = tracks(&check);
    let confirmed_at_point = ct.find("path", TRACK_11).expect("confirmed fake row");
    assert_eq!(
        ct.get(confirmed_at_point, "verdict"),
        "suspicious",
        "precondition: the confirmed fake is Suspicious under the point"
    );
    let result = o.grid.as_ref().expect("grid outcome");
    assert_eq!(result.points.len(), 1);
    let candidate = &result.points[0];
    assert!(
        candidate.cost < result.baseline.cost,
        "the candidate ({}) costs less than the baseline ({}) on the tune rows",
        candidate.cost,
        result.baseline.cost
    );

    // Then the candidate point is marked infeasible.
    assert!(
        !candidate.feasible,
        "a confirmed fake that is Fake at the baseline and not Fake under the point makes it infeasible"
    );
    assert!(result.baseline.feasible);

    // And its row has "confirmed lost" 1, and the baseline row has "confirmed lost" 0.
    let table = BestPoints::of(&o);
    assert_eq!(table.cell(table.rank("1"), "confirmed lost"), "1");
    assert_eq!(table.cell(table.rank("baseline"), "confirmed lost"), "0");
    assert_eq!(table.cell(table.rank("1"), "confirmed fakes Real"), "0");

    // And it is not proposed.
    no_proposal_file(&o);
}

// ---------------------------------------------------------------------------------------------
// cost_exclude (calibration.md OQ-24)
// ---------------------------------------------------------------------------------------------

// Scenario: Positives of cost_exclude settings leave the cost but stay in the tables (@RP-11)
#[test]
fn rp11_positives_of_cost_exclude_settings_leave_the_cost_but_stay_in_the_tables() {
    let sample = Sample::new("cost-exclude");
    sample.write_config(Some(0.0));
    let grid = sample_grid(&sample, 1, Some(&["fxweak"]));
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    let summary = report_text(&o, "summary.md");
    assert!(
        grid_section(&summary)
            .iter()
            .any(|l| l == "- cost excludes the positives of the settings [fxweak]"),
        "summary.md:\n{}",
        grid_section(&summary).join("\n")
    );

    // Baseline: 0.5 (the low_hf negative is Suspicious); the 6 fxweak Suspicious positives are out
    // of the cost. gap_fake_db = 4.0 only turns them Fake: 0.5 too.
    let result = o.grid.as_ref().expect("grid outcome");
    assert!(
        (result.baseline.cost - 0.5).abs() < 1e-9,
        "baseline cost {}",
        result.baseline.cost
    );
    let p = point(result, "verdict.gap_fake_db", 4.0);
    assert!((p.cost - 0.5).abs() < 1e-9, "cost {}", p.cost);

    assert!(
        o.stdout.contains("no change proposed"),
        "stdout: {}",
        o.stdout
    );
    no_proposal_file(&o);

    // The per-setting table of the baseline still shows fxweak with 6 Suspicious.
    let fxweak = o.evaluations[0]
        .by_setting
        .iter()
        .find(|r| r.setting == "fxweak")
        .expect("fxweak row");
    assert_eq!(fxweak.counts.suspicious, 6);
    let (header, rows) = md_table(&summary, "### Per encoder setting");
    let col = |name: &str| header.iter().position(|h| h == name).unwrap();
    let row = rows
        .iter()
        .find(|r| r[col("setting")] == "fxweak")
        .expect("fxweak in the per-setting table of summary.md");
    assert_eq!(row[col("Suspicious")], "6");
}

/// Scenario Outline: Invalid cost_exclude settings (@RP-11).
fn invalid_cost_exclude(test: &str, setting: &str, message: &str) {
    let sample = Sample::new(test);
    let grid = sample_grid(&sample, 1, Some(&[setting]));
    let corpus_before = snapshot(&sample.corpus);
    let calibration_before = names(&sample.calibration);
    let status_before = git_status();
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(o.stderr.contains(message), "stderr: {}", o.stderr);
    assert_eq!(
        names(&sample.calibration),
        calibration_before,
        "nothing is written"
    );
    assert_eq!(snapshot(&sample.corpus), corpus_before, "corpus untouched");
    assert_eq!(git_status(), status_before, "repository untouched");
}

// Scenario Outline: Invalid cost_exclude settings (@RP-11), row "fx999"
#[test]
fn rp11_invalid_cost_exclude_unknown_setting() {
    invalid_cost_exclude(
        "cost-exclude-unknown",
        "fx999",
        "cost_exclude: unknown setting 'fx999'",
    );
}

// Scenario Outline: Invalid cost_exclude settings (@RP-11), row "fx128"
#[test]
fn rp11_invalid_cost_exclude_also_in_must_catch() {
    invalid_cost_exclude(
        "cost-exclude-must-catch",
        "fx128",
        "cost_exclude: 'fx128' is also in must_catch",
    );
}

// ---------------------------------------------------------------------------------------------
// Defaults (calibration.md OQ-18)
// ---------------------------------------------------------------------------------------------

/// Flattens a TOML table to dotted keys (quoted keys like `"verdict.gap_fake_db"` stay as they are).
fn flat_values(prefix: &str, table: &toml::Table, out: &mut Vec<(String, toml::Value)>) {
    for (k, v) in table {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::Table(t) => flat_values(&key, t, out),
            other => out.push((key, other.clone())),
        }
    }
}

// Scenario: The default params are the app's (@RP-11 @RF-04)
#[test]
fn rp11_rf04_the_default_params_are_the_apps() {
    // The default params file deserialises to DetectionParams::default().
    let params_path = repo_root().join(DEFAULT_PARAMS);
    let params: DetectionParams = toml::from_str(&read(&params_path)).unwrap();
    assert_eq!(
        params,
        DetectionParams::default(),
        "the default params file of the tool ({}) is the one the app ships",
        default_params_name()
    );

    // The default grid has no baseline key.
    let grid_path = repo_root().join(DEFAULT_GRID);
    let grid: toml::Table = toml::from_str(&read(&grid_path)).unwrap();
    assert!(
        !grid.contains_key("baseline"),
        "the default grid has no baseline key"
    );

    // Every list in the default grid's [values] contains the default params file's value of its key.
    let values = grid["values"].as_table().expect("[values]");
    let mut flat = Vec::new();
    flat_values("", values, &mut flat);
    assert!(!flat.is_empty());
    for (key, list) in flat {
        let value = get_value(&params, &key)
            .unwrap_or_else(|| panic!("{key} is a rule parameter of the default params"));
        let list: Vec<f64> = list
            .as_array()
            .unwrap_or_else(|| panic!("{key}: a list"))
            .iter()
            .map(|v| {
                v.as_float()
                    .or_else(|| v.as_integer().map(|i| i as f64))
                    .unwrap()
            })
            .collect();
        assert!(
            list.contains(&value),
            "{key}: {list:?} contains the default value {value}"
        );
    }
}

// Scenario: Without --params the baseline is the params file the app ships (@RP-11)
#[test]
fn rp11_without_params_the_baseline_is_the_params_file_the_app_ships() {
    assert_not_in_repo_root();
    let sample = Sample::new("default-params");
    let o = sample.accept(&[]);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    // The report header lists exactly one params file, the default params file, with the
    // algorithm_version of DetectionParams::default().
    let expected_version = DetectionParams::default().algorithm_version.to_string();
    assert_eq!(
        parameter_files(&o),
        vec![(default_params_name(), expected_version)]
    );
    assert_eq!(o.evaluations.len(), 1);
    assert_eq!(o.evaluations[0].params_name, default_params_name());
    assert_eq!(
        o.evaluations[0].algorithm_version,
        DetectionParams::default().algorithm_version
    );
}

// Scenario: A grid without baseline searches from the default params file (@RP-11)
#[test]
fn rp11_a_grid_without_baseline_searches_from_the_default_params_file() {
    assert_not_in_repo_root();
    let sample = Sample::new("grid-no-baseline");
    sample.write_config(Some(0.0));
    let grid = sample.write_scratch(
        "grid.toml",
        &grid_text(1, None, &["fx128"], None, DEFAULT_WEIGHTS, GAP_GRID),
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    // The report header names the default params file as the baseline (its first params file).
    let files = parameter_files(&o);
    assert_eq!(
        files.first().map(|(f, _)| f.as_str()),
        Some(default_params_name().as_str()),
        "{files:?}"
    );
    assert_eq!(o.evaluations[0].params_name, default_params_name());
    assert_eq!(
        o.evaluations[0].algorithm_version,
        DetectionParams::default().algorithm_version
    );
}

// Scenario: Without --grid the search reads calibrate-grid.v3.toml (@RP-11)
#[test]
fn rp11_without_grid_the_search_reads_calibrate_grid_v3() {
    assert_not_in_repo_root();
    let sample = Sample::new("default-grid");
    let config = sample.config_path().to_string_lossy().into_owned();
    let command = parse_args(&["calibrate".to_string(), "--config".to_string(), config])
        .expect("calibrate with no --grid");
    let corpus_before = snapshot(&sample.corpus);
    let calibration_before = names(&sample.calibration);
    let status_before = git_status();

    let o = fakeflac_calibrate::run(&command, &sample.env(), &FixtureTranscoder);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr.contains("calibrate-grid.v3.toml"),
        "stderr: {}",
        o.stderr
    );
    assert!(
        o.stderr.contains("must_catch: unknown setting 'cbr128'"),
        "stderr: {}",
        o.stderr
    );
    assert_eq!(
        names(&sample.calibration),
        calibration_before,
        "nothing is written"
    );
    assert_eq!(snapshot(&sample.corpus), corpus_before, "corpus untouched");
    assert_eq!(git_status(), status_before, "repository untouched");
}

// ---------------------------------------------------------------------------------------------
// Grid versions (evaluation.md section 7)
// ---------------------------------------------------------------------------------------------

fn grid_version_exit(test: &str, version: u32) -> Outcome {
    let sample = Sample::new(test);
    let grid = sample_grid(&sample, version, None);
    sample.calibrate(&grid)
}

// Scenario Outline: Supported grid versions (@RP-11), row "1"
#[test]
fn rp11_supported_grid_version_1() {
    let o = grid_version_exit("grid-version-1", 1);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
}

// Scenario Outline: Supported grid versions (@RP-11), row "2"
#[test]
fn rp11_supported_grid_version_2() {
    let o = grid_version_exit("grid-version-2", 2);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
}

// Scenario Outline: Supported grid versions (@RP-11), row "3"
#[test]
fn rp11_supported_grid_version_3() {
    let o = grid_version_exit("grid-version-3", 3);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
}

// Scenario: An unknown grid version is an error (@RP-11)
#[test]
fn rp11_an_unknown_grid_version_is_an_error() {
    let o = grid_version_exit("grid-version-4", 4);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr
            .contains("grid_version 4 is not supported (expected 1, 2 or 3)"),
        "stderr: {}",
        o.stderr
    );
}

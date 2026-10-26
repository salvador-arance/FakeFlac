//! Calibration report additions of RF-16 (`specs/calibration/lossy.md` sections 6.3 and 6.8, C13;
//! acceptance K of `specs/detection/lossy-source.md` section 11.7), derived from
//! `specs/features/detection-streaming-source.feature` (@RF-16 @RP-11): the lossy verdict-change
//! table between params files and its "of which `streamingSampleRate`" count, the `streaming` and
//! `streaming rate` columns, the per-sample-rate sub-rows `<group> @ <bucket>`, the strong-labels-only
//! rows of confirmed lossy fakes, K5 read from the 48 kHz sub-rows, and the lossy sample corpus with
//! v6 and v7. Hand-built lossy rows with given verdicts (the evaluation step, no audio), and the
//! fixture transcoder for the sample corpus. No ffmpeg, no real audio.
//!
//! Names chosen by the test author (lossy.md section 6.8 leaves them open):
//!   `report::LossyGroupRow` gains `streaming: u32` (rows whose reasons include
//!   `streamingSampleRate`) and `fc_median_hz`, `fc_p5_hz`, `fc_p95_hz: Option<f64>`;
//!   `report::LossyEvaluation` gains `changes: Option<LossyVerdictChanges>` (`None` for the lossy
//!   baseline, the first `--params` file with `[lossy]`);
//!   `report::LossyVerdictChanges { against: String, counts: BTreeMap<(String, String), u32>,
//!   streaming: u32 }` (keys as in `VerdictChanges`: lower-case verdict names, all 16 pairs);
//!   `lossy::lossy_verdict_changes(against, group, baseline_evals, evals) -> LossyVerdictChanges`;
//!   the sub-rows are `LossyGroupRow`s named `<group> @ <bucket>`; the rendered per-group table has
//!   the columns `streaming` and `streaming rate`.
//!   `SuspicionReason::StreamingSampleRate` (the core's new reason).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::cache::{Extracted, ProbeData, StreamData};
use fakeflac_calibrate::env::Roots;
use fakeflac_calibrate::evaluate::{evaluation, ParamsRun};
use fakeflac_calibrate::extract::GroupResult;
use fakeflac_calibrate::lossy::{
    chains, lossy_tables, lossy_verdict_changes, lossy_watch, not_evaluated_count, LossyRowEval,
};
use fakeflac_calibrate::model::{LabelRow, RowMeta, RowRoot, Truth};
use fakeflac_calibrate::render::{markdown, ReportData};
use fakeflac_calibrate::report::{
    CacheStats, Evaluation, LossyCounts, LossyEvaluation, LossyGroupRow, SplitSel,
};
use fakeflac_calibrate::split::Split;
use fakeflac_calibrate::Config;
use fakeflac_core::{AudioCodec, BitrateMode, DetectionParams, SuspicionReason, Verdict};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

const V6: &str = "detection-params.v6.toml";
const V7: &str = "detection-params.v7.toml";

fn params_path(name: &str) -> PathBuf {
    repo_root().join("specs/detection").join(name)
}

fn params(name: &str) -> DetectionParams {
    toml::from_str(&read(&params_path(name))).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The acceptance config of lossy-source.md section 11.7 (its `[[generate.lossy]]` entries include
/// `cbr256`, `cbr320` and `opus128_cbr320`).
fn config() -> Config {
    Config::parse(&read(
        &repo_root().join("specs/calibration/calibration.v3.toml"),
    ))
    .expect("calibration.v3.toml loads")
}

// ---- Hand-built lossy rows ----------------------------------------------------------------------

/// The group column of the scenario tables: a `[[generate.lossy]]` id, or `confirmed_fake opus`.
#[derive(Clone, Copy)]
enum Group {
    Generated(&'static str),
    ConfirmedFakeOpus,
}

#[derive(Clone, Copy, PartialEq)]
enum Label {
    Strong,
    Weak,
}

/// One row of the scenario tables: its truth, sample rate and reported cutoff, and the verdict and
/// reasons of each params file.
struct Given {
    group: Group,
    truth: Truth,
    label: Label,
    rate: u32,
    fc: f64,
    /// (verdict, reasons) with v6 and with v7.
    v6: (Verdict, Vec<SuspicionReason>),
    v7: (Verdict, Vec<SuspicionReason>),
    source_limited: bool,
}

use SuspicionReason::{CutoffBelowDeclared, StreamingSampleRate};
use Verdict::{Fake, Real, Suspicious};

fn given(
    group: Group,
    truth: Truth,
    rate: u32,
    fc: f64,
    v6: (Verdict, Vec<SuspicionReason>),
    v7: (Verdict, Vec<SuspicionReason>),
) -> Given {
    Given {
        group,
        truth,
        label: Label::Strong,
        rate,
        fc,
        v6,
        v7,
        source_limited: false,
    }
}

/// Rows, their lossy columns and the evaluation of each params file, from the scenario table. Paths
/// hold names that `summary.md` must never show (lossy.md section 6.6).
struct Built {
    /// One extraction group: the rows of both params files.
    groups: Vec<GroupResult>,
    v6: Vec<LossyRowEval>,
    v7: Vec<LossyRowEval>,
}

const SECRET_ARTIST: &str = "Artista Secretísima";
const SECRET_ALBUM: &str = "Disco Ocultísimo";
const SECRET_TRACK: &str = "Canción Privadísima";

fn build(rows: &[Given]) -> Built {
    let mut lossy_rows = Vec::new();
    let mut meta = BTreeMap::new();
    let (mut v6, mut v7) = (Vec::new(), Vec::new());
    for (i, g) in rows.iter().enumerate() {
        let (origin, setting, root, class): (&'static str, String, RowRoot, &str) = match g.group {
            Group::Generated(id) => ("generated", id.to_string(), RowRoot::Generated, "generated"),
            Group::ConfirmedFakeOpus => {
                ("confirmed_fake", String::new(), RowRoot::Corpus, "positive")
            }
        };
        let path = format!(
            "{SECRET_ARTIST}/{SECRET_ALBUM}/{:02} – {SECRET_TRACK} {i}.mp3",
            i + 1
        );
        let row = LabelRow {
            root,
            path,
            truth: g.truth,
            origin,
            trust_origin: String::new(),
            class: class.to_string(),
            encoder: String::new(),
            setting,
            bitrate_kbps: Some(320),
            encoder_version: String::new(),
            fragment: None,
            source_path: String::new(),
            album_id: format!("album{i}"),
            split: Split::Tune,
            watch_id: String::new(),
            extracted: Arc::new(Extracted::Probe(ProbeData {
                sample_rate: g.rate,
                bit_depth: None,
                channels: 2,
                duration_ms: 240_000,
                spectrum: None,
                stream: StreamData {
                    codec: Some(AudioCodec::Mp3),
                    declared_bitrate_kbps: Some(320),
                    bitrate_mode: Some(BitrateMode::Cbr),
                    encoder: None,
                    encoder_lowpass_hz: None,
                },
            })),
            undetectable: false,
        };
        let m = RowMeta {
            upstream: match g.group {
                Group::ConfirmedFakeOpus => "opus".to_string(),
                Group::Generated(id) if id.starts_with("opus") => "opus".to_string(),
                Group::Generated(_) => String::new(),
            },
            weak: g.label == Label::Weak,
            honest_generated: g.truth == Truth::Real && matches!(g.group, Group::Generated(_)),
            split_unit: format!("album{i}"),
            ..RowMeta::default()
        };
        meta.insert(row.identity(), m);
        let eval = |(verdict, reasons): &(Verdict, Vec<SuspicionReason>)| LossyRowEval {
            evaluated: true,
            verdict: *verdict,
            reasons: reasons.clone(),
            error: None,
            estimate_kbps: if reasons.contains(&StreamingSampleRate) {
                None
            } else {
                Some(320)
            },
            m: None,
            fc_hz: Some(g.fc),
            c_exp_hz: Some(20_500.0),
            shortfall_hz: Some(20_500.0 - g.fc),
            source_limited: g.source_limited,
        };
        v6.push(eval(&g.v6));
        v7.push(eval(&g.v7));
        lossy_rows.push(row);
    }
    Built {
        groups: vec![GroupResult {
            rows: Vec::new(),
            lossy_rows,
            meta,
            real_excluded: Vec::new(),
            stats: CacheStats {
                fingerprint_id: "f".to_string(),
                hits: 0,
                decoded: 0,
                transcoded: 0,
                generation_failures: 0,
                skipped_lines: 0,
            },
            skipped_sources: Vec::new(),
            failures: Vec::new(),
            not_generated: 0,
            ffmpeg_version: None,
            ffmpeg_missing: false,
            kept: Vec::new(),
        }],
        v6,
        v7,
    }
}

/// "When the lossy tables are written": the evaluations of the params files in order (the first is
/// the lossy baseline) as the run assembles them, and `summary.md`.
fn write_tables(built: &Built, files: &[(&str, &[LossyRowEval])]) -> (Vec<Evaluation>, String) {
    let config = config();
    let groups = &built.groups;
    let runs: Vec<ParamsRun> = files
        .iter()
        .map(|(name, evals)| ParamsRun {
            name: name.to_string(),
            params: params(name),
            hash: "0".repeat(16),
            fingerprint_id: "f".to_string(),
            group: 0,
            evals: Vec::new(),
            lossy_evals: evals.to_vec(),
        })
        .collect();
    let group = &groups[0];
    let mut evaluations = Vec::new();
    for (i, run) in runs.iter().enumerate() {
        let baseline = (i > 0).then(|| (&runs[0], group.rows.as_slice()));
        let mut eval = evaluation(run, &group.rows, &config, baseline);
        eval.lossy_not_evaluated = not_evaluated_count(&run.lossy_evals);
        eval.lossy_watch = lossy_watch(group, &run.lossy_evals);
        if run.params.lossy.is_some() {
            let mut tables = lossy_tables(group, &run.lossy_evals, &config);
            if i > 0 {
                tables.changes = Some(lossy_verdict_changes(
                    &runs[0].name,
                    group,
                    &runs[0].lossy_evals,
                    &run.lossy_evals,
                ));
            }
            eval.lossy = Some(tables);
            eval.chains = chains(group, run, &run.lossy_evals);
        }
        evaluations.push(eval);
    }
    let roots = Roots {
        corpus: PathBuf::from("corpus"),
        calibration: PathBuf::from("calibration"),
    };
    let unit_strata = BTreeMap::new();
    let data = ReportData {
        run_id: "rf16",
        date: "2026-10-26",
        commit: "0000000",
        mode: "accept",
        config: &config,
        roots: &roots,
        groups,
        runs: &runs,
        evaluations: &evaluations,
        unit_strata: &unit_strata,
        grid: None,
    };
    let summary = markdown(&data, false);
    (evaluations, summary)
}

fn lossy_of<'a>(evaluations: &'a [Evaluation], name: &str) -> &'a LossyEvaluation {
    evaluations
        .iter()
        .find(|e| e.params_name == name)
        .unwrap_or_else(|| panic!("no evaluation of {name}"))
        .lossy
        .as_ref()
        .unwrap_or_else(|| panic!("{name}: lossy tables"))
}

fn row<'a>(lossy: &'a LossyEvaluation, group: &str) -> &'a LossyGroupRow {
    lossy
        .groups
        .iter()
        .find(|g| g.group == group)
        .unwrap_or_else(|| {
            panic!(
                "no row {group} in {:?}",
                lossy.groups.iter().map(|g| &g.group).collect::<Vec<_>>()
            )
        })
}

/// The denominator of the rates of lossy.md 6.2 and of the `streaming rate`: the row's rows minus
/// `Undetectable`, `Source-limited` and `Error`.
fn denominator(c: &LossyCounts) -> u32 {
    c.total() - c.undetectable - c.source_limited - c.error
}

/// The `streaming rate` as the report shows it: one decimal and ` %`, empty without a denominator.
fn streaming_rate(g: &LossyGroupRow) -> String {
    let n = denominator(&g.counts);
    if n == 0 {
        String::new()
    } else {
        format!(
            "{} %",
            fakeflac_calibrate::fmt::pct_value(f64::from(g.streaming) / f64::from(n))
        )
    }
}

/// The cell of `column` in the row `group` of the per-group table of the section of `params` in a
/// rendered report.
fn rendered_cell(summary: &str, params: &str, group: &str, column: &str) -> String {
    let section = summary
        .split(&format!("## {params}"))
        .nth(1)
        .unwrap_or_else(|| panic!("no section {params}"));
    let table = section
        .split("### Lossy per group")
        .nth(1)
        .unwrap_or_else(|| panic!("{params}: no per-group table"));
    let cells = |line: &str| -> Vec<String> {
        // No cell of these tables holds an escaped `|`.
        let line = line.trim();
        let inner = line.strip_prefix('|').unwrap_or(line);
        let inner = inner.strip_suffix('|').unwrap_or(inner);
        inner.split('|').map(|c| c.trim().to_string()).collect()
    };
    let mut lines = table.lines().filter(|l| l.trim_start().starts_with('|'));
    let header = cells(lines.next().expect("header"));
    let at = header
        .iter()
        .position(|h| h == column)
        .unwrap_or_else(|| panic!("no column {column} in {header:?}"));
    let found = lines
        .map(cells)
        .find(|r| r.first().map(String::as_str) == Some(group))
        .unwrap_or_else(|| panic!("{params}: no rendered row {group}"));
    found[at].clone()
}

fn changes_count(lossy: &LossyEvaluation, from: &str, to: &str) -> u32 {
    let c = lossy
        .changes
        .as_ref()
        .expect("a lossy verdict-change table");
    *c.counts
        .get(&(from.to_string(), to.to_string()))
        .unwrap_or_else(|| panic!("no pair {from} -> {to}"))
}

const VERDICTS: [&str; 4] = ["real", "suspicious", "fake", "error"];

// ---------------------------------------------------------------------------------------------
// Scenario: Lossy verdict changes, streaming counts and sample-rate rows (@RF-16 @RP-11)
// ---------------------------------------------------------------------------------------------

fn scenario_rows() -> Vec<Given> {
    use Group::{ConfirmedFakeOpus as Opus, Generated as Gen};
    let s = || (Suspicious, vec![StreamingSampleRate]);
    let r = || (Real, vec![]);
    vec![
        given(
            Gen("opus128_cbr320"),
            Truth::Fake,
            48_000,
            20_050.0,
            r(),
            s(),
        ),
        given(
            Gen("opus128_cbr320"),
            Truth::Fake,
            48_000,
            20_150.0,
            r(),
            s(),
        ),
        given(
            Gen("opus128_cbr320"),
            Truth::Fake,
            48_000,
            17_050.0,
            (Fake, vec![]),
            (Fake, vec![]),
        ),
        given(Gen("cbr320"), Truth::Real, 48_000, 20_450.0, r(), r()),
        given(Gen("cbr320"), Truth::Real, 44_100, 20_450.0, r(), r()),
        given(
            Gen("cbr320"),
            Truth::Real,
            44_100,
            18_550.0,
            (Suspicious, vec![CutoffBelowDeclared]),
            (Suspicious, vec![CutoffBelowDeclared]),
        ),
        given(Opus, Truth::Fake, 48_000, 20_050.0, r(), s()),
    ]
}

#[test]
fn rf16_the_lossy_verdict_change_table_counts_real_to_suspicious_by_streaming_sample_rate() {
    let built = build(&scenario_rows());
    let (evaluations, _) = write_tables(&built, &[(V6, &built.v6), (V7, &built.v7)]);
    assert!(
        lossy_of(&evaluations, V6).changes.is_none(),
        "the lossy baseline has no change table"
    );
    let v7 = lossy_of(&evaluations, V7);
    let changes = v7.changes.as_ref().expect("v6 x v7");
    assert_eq!(changes.against, V6);
    assert_eq!(changes.counts.len(), 16, "all 16 pairs");
    for from in VERDICTS {
        for to in VERDICTS {
            if from == to {
                continue;
            }
            let want = u32::from(from == "real" && to == "suspicious") * 3;
            assert_eq!(changes_count(v7, from, to), want, "{from} -> {to}");
        }
    }
    assert_eq!(changes.streaming, 3, "of which streamingSampleRate");
    // The diagonal holds the unchanged rows: 7 rows in all.
    let total: u32 = changes.counts.values().sum();
    assert_eq!(total, 7);
}

#[test]
fn rf16_the_per_group_table_counts_streaming_rows_and_has_sample_rate_sub_rows() {
    let built = build(&scenario_rows());
    let (evaluations, _) = write_tables(&built, &[(V6, &built.v6), (V7, &built.v7)]);
    let v7 = lossy_of(&evaluations, V7);

    // "the group row opus128_cbr320 has streaming 2, caught 3 of 3 and fc median 20050 Hz"
    let opus = row(v7, "opus128_cbr320");
    assert_eq!(opus.streaming, 2);
    assert!(opus.positive);
    let n = denominator(&opus.counts);
    assert_eq!(
        (opus.counts.fake + opus.counts.suspicious, n),
        (3, 3),
        "caught 3 of 3"
    );
    assert_eq!(opus.fc_median_hz, Some(20_050.0));
    assert_eq!(opus.fc_p5_hz, Some(17_050.0));
    assert_eq!(opus.fc_p95_hz, Some(20_150.0));

    // "the row cbr320 @ 48000 has 1 row, streaming 0 and Real 1"
    let at48 = row(v7, "cbr320 @ 48000");
    assert_eq!(at48.counts.total(), 1);
    assert_eq!(at48.streaming, 0);
    assert_eq!(at48.counts.real, 1);
    assert!(!at48.positive, "a sub-row is negative as its group");

    // "the row cbr320 @ 44100 has 2 rows, streaming 0, Real 1 and Suspicious 1"
    let at44 = row(v7, "cbr320 @ 44100");
    assert_eq!(at44.counts.total(), 2);
    assert_eq!(at44.streaming, 0);
    assert_eq!((at44.counts.real, at44.counts.suspicious), (1, 1));
    // The group row holds both buckets; no bucket without rows.
    assert_eq!(row(v7, "cbr320").counts.total(), 3);
    assert!(!v7.groups.iter().any(|g| g.group == "cbr320 @ other"));
    // Sub-rows follow their group.
    let order: Vec<&str> = v7.groups.iter().map(|g| g.group.as_str()).collect();
    let at = |g: &str| order.iter().position(|x| *x == g).unwrap();
    assert!(at("cbr320") < at("cbr320 @ 44100") && at("cbr320 @ 44100") < at("cbr320 @ 48000"));

    // "the rows confirmed_fake opus and confirmed_fake 48000 each have 1 row, streaming 1 and
    // Suspicious 1"
    for name in ["confirmed_fake opus", "confirmed_fake 48000"] {
        let g = row(v7, name);
        assert_eq!(g.counts.total(), 1, "{name}");
        assert_eq!(g.streaming, 1, "{name}");
        assert_eq!(g.counts.suspicious, 1, "{name}");
    }

    // "with v6 every streaming count is 0"
    let v6 = lossy_of(&evaluations, V6);
    for g in &v6.groups {
        assert_eq!(g.streaming, 0, "v6 {}", g.group);
    }
}

#[test]
fn rf16_summary_names_no_file_folder_or_artist() {
    let built = build(&scenario_rows());
    let (_, summary) = write_tables(&built, &[(V6, &built.v6), (V7, &built.v7)]);
    for name in [SECRET_ARTIST, SECRET_ALBUM, SECRET_TRACK] {
        assert!(!summary.contains(name), "summary.md names {name}");
    }
    // Not vacuous: the v7 per-group table with its new columns is there.
    assert_eq!(
        rendered_cell(&summary, V7, "opus128_cbr320", "streaming"),
        "2"
    );
}

// ---------------------------------------------------------------------------------------------
// Scenario: Weak labels are counted only in the weak row (@RF-16 @RP-11)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_weak_labels_are_counted_only_in_the_weak_row() {
    let s = || (Suspicious, vec![StreamingSampleRate]);
    let strong = given(
        Group::ConfirmedFakeOpus,
        Truth::Fake,
        48_000,
        20_050.0,
        (Real, vec![]),
        s(),
    );
    let weak = Given {
        label: Label::Weak,
        ..given(
            Group::ConfirmedFakeOpus,
            Truth::Fake,
            48_000,
            20_150.0,
            (Real, vec![]),
            s(),
        )
    };
    let built = build(&[strong, weak]);
    let (evaluations, _) = write_tables(&built, &[(V6, &built.v6), (V7, &built.v7)]);
    let v7 = lossy_of(&evaluations, V7);
    for name in ["confirmed_fake opus", "confirmed_fake 48000", "weak"] {
        let g = row(v7, name);
        assert_eq!(g.counts.total(), 1, "{name}");
        assert_eq!(g.streaming, 1, "{name}");
        assert_eq!(g.counts.suspicious, 1, "{name}");
    }
    assert_eq!(changes_count(v7, "real", "suspicious"), 2);
    assert_eq!(v7.changes.as_ref().unwrap().streaming, 2);
    // The confusion matrix is unchanged by the strong-labels-only rows (lossy.md 6.8).
    let m = v7.matrix[&SplitSel::All];
    assert_eq!(m.fake_row.total(), 1);
    assert_eq!(m.weak_row.total(), 1);
}

// ---------------------------------------------------------------------------------------------
// K5 (lossy-source.md section 11.7, Q22), read from the 48 kHz sub-rows
// ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Bound {
    Holds,
    Fails,
    NotApplicable,
}

/// K5's bound on a sub-row: `streaming <= max_percent % x denominator`, checked on the counts; n/a
/// when the row is absent or its denominator is 0.
fn k5(lossy: &LossyEvaluation, group: &str, max_percent: u32) -> Bound {
    match lossy.groups.iter().find(|g| g.group == group) {
        None => Bound::NotApplicable,
        Some(g) => {
            let n = denominator(&g.counts);
            if n == 0 {
                Bound::NotApplicable
            } else if g.streaming * 100 <= max_percent * n {
                Bound::Holds
            } else {
                Bound::Fails
            }
        }
    }
}

fn k5_320(lossy: &LossyEvaluation) -> Bound {
    k5(lossy, "cbr320 @ 48000", 10)
}

fn k5_256(lossy: &LossyEvaluation) -> Bound {
    k5(lossy, "cbr256 @ 48000", 2)
}

/// `count` honest rows of `entry` at `rate` that v7 makes suspicious with streamingSampleRate
/// (`flagged`) or real.
fn honest(entry: &'static str, rate: u32, count: u32, flagged: bool) -> Vec<Given> {
    (0..count)
        .map(|_| {
            let v7 = if flagged {
                (Suspicious, vec![StreamingSampleRate])
            } else {
                (Real, vec![])
            };
            given(
                Group::Generated(entry),
                Truth::Real,
                rate,
                if flagged { 20_050.0 } else { 20_350.0 },
                (Real, vec![]),
                v7,
            )
        })
        .collect()
}

/// The K5 rows of the outlines: `a` flagged and `b` real rows of `cbr320` at 48000 (plus one
/// source-limited real row when `source_limited`), `c` flagged and `d` real rows of `cbr256`.
fn k5_rows(a: u32, b: u32, c: u32, d: u32, source_limited: bool) -> Vec<Given> {
    let mut rows = honest("cbr320", 48_000, a, true);
    rows.extend(honest("cbr320", 48_000, b, false));
    if source_limited {
        let mut sl = honest("cbr320", 48_000, 1, false).remove(0);
        sl.source_limited = true;
        sl.fc = 17_050.0;
        rows.push(sl);
    }
    rows.extend(honest("cbr256", 48_000, c, true));
    rows.extend(honest("cbr256", 48_000, d, false));
    rows
}

fn v7_tables(rows: &[Given]) -> (LossyEvaluation, String) {
    let built = build(rows);
    let (evaluations, summary) = write_tables(&built, &[(V7, &built.v7)]);
    (lossy_of(&evaluations, V7).clone(), summary)
}

// Scenario Outline: K5 passes when the honest 48 kHz MP3 320 and 256 rows stay within their bounds
#[test]
fn rf16_k5_passes_with_both_rows_at_their_bounds_and_with_nothing_flagged() {
    // | case | a | b | c | d | rows320 | rate320 | rows256 | rate256 |
    for (case, a, b, c, d, rows320, rate320, rows256, rate256) in [
        (
            "both at their bounds",
            1,
            9,
            1,
            49,
            11,
            "10.0 %",
            50,
            "2.0 %",
        ),
        ("nothing flagged", 0, 10, 0, 50, 11, "0.0 %", 50, "0.0 %"),
    ] {
        let (lossy, summary) = v7_tables(&k5_rows(a, b, c, d, true));
        let r320 = row(&lossy, "cbr320 @ 48000");
        assert_eq!(r320.counts.total(), rows320, "{case}");
        assert_eq!(r320.counts.source_limited, 1, "{case}");
        assert_eq!(r320.streaming, a, "{case}");
        assert_eq!(streaming_rate(r320), rate320, "{case}");
        assert_eq!(
            rendered_cell(&summary, V7, "cbr320 @ 48000", "streaming rate"),
            rate320,
            "{case}: the rendered column"
        );
        let r256 = row(&lossy, "cbr256 @ 48000");
        assert_eq!(r256.counts.total(), rows256, "{case}");
        assert_eq!(r256.streaming, c, "{case}");
        assert_eq!(streaming_rate(r256), rate256, "{case}");
        assert_eq!(
            rendered_cell(&summary, V7, "cbr256 @ 48000", "streaming rate"),
            rate256,
            "{case}: the rendered column"
        );
        // The source-limited row is left out of the denominator (1 of 10, not 1 of 11).
        assert_eq!(denominator(&r320.counts), 10, "{case}");
        assert_eq!(k5_320(&lossy), Bound::Holds, "{case}");
        assert_eq!(k5_256(&lossy), Bound::Holds, "{case}");
    }
}

// Scenario Outline: K5 fails when an honest 48 kHz MP3 320 or 256 row exceeds its bound
#[test]
fn rf16_k5_fails_when_a_row_exceeds_its_bound() {
    // | case | a | b | c | d | rate320 | rate256 | row | other |
    for (case, a, b, c, d, rate320, rate256, failing) in [
        (
            "cbr320 above 10 %",
            2,
            17,
            0,
            50,
            "10.5 %",
            "0.0 %",
            "cbr320 @ 48000",
        ),
        (
            "cbr256 above 2 %",
            1,
            9,
            2,
            48,
            "10.0 %",
            "4.0 %",
            "cbr256 @ 48000",
        ),
    ] {
        let (lossy, summary) = v7_tables(&k5_rows(a, b, c, d, false));
        let r320 = row(&lossy, "cbr320 @ 48000");
        assert_eq!(r320.streaming, a, "{case}");
        assert_eq!(streaming_rate(r320), rate320, "{case}");
        assert_eq!(
            rendered_cell(&summary, V7, "cbr320 @ 48000", "streaming rate"),
            rate320,
            "{case}"
        );
        let r256 = row(&lossy, "cbr256 @ 48000");
        assert_eq!(r256.streaming, c, "{case}");
        assert_eq!(streaming_rate(r256), rate256, "{case}");
        assert_eq!(
            rendered_cell(&summary, V7, "cbr256 @ 48000", "streaming rate"),
            rate256,
            "{case}"
        );
        let (on_320, on_256) = (k5_320(&lossy), k5_256(&lossy));
        if failing == "cbr320 @ 48000" {
            assert_eq!((on_320, on_256), (Bound::Fails, Bound::Holds), "{case}");
        } else {
            assert_eq!((on_320, on_256), (Bound::Holds, Bound::Fails), "{case}");
        }
    }
}

// Scenario: A K5 bound with no 48 kHz rows is n/a and does not block
#[test]
fn rf16_a_k5_bound_with_no_48k_rows_is_not_applicable() {
    let mut rows = honest("cbr320", 48_000, 10, false);
    rows.extend(honest("cbr256", 44_100, 50, false));
    let (lossy, summary) = v7_tables(&rows);
    assert!(
        !lossy.groups.iter().any(|g| g.group == "cbr256 @ 48000"),
        "no sub-row without rows"
    );
    assert_eq!(row(&lossy, "cbr256 @ 44100").counts.total(), 50);
    assert_eq!(k5_256(&lossy), Bound::NotApplicable);
    assert_eq!(k5_320(&lossy), Bound::Holds);
    assert_eq!(
        rendered_cell(&summary, V7, "cbr320 @ 48000", "streaming rate"),
        "0.0 %"
    );
}

// ---------------------------------------------------------------------------------------------
// Scenario: The lossy sample corpus with v6 and v7 (@RF-16 @RP-11)
// ---------------------------------------------------------------------------------------------

#[test]
fn rf16_the_lossy_sample_corpus_with_v6_and_v7() {
    let sample = Sample::new_lossy("lossy-v6-v7");
    let o = sample.accept(&[params_path(V6), params_path(V7)]);
    assert_ok(&o);
    assert_eq!(o.evaluations.len(), 2);
    assert_eq!(o.evaluations[0].params_name, V6);
    assert_eq!(o.evaluations[1].params_name, V7);
    assert_eq!(o.evaluations[1].algorithm_version, 7);
    let (v6, v7) = (
        o.evaluations[0].lossy.as_ref().expect("v6 lossy tables"),
        o.evaluations[1].lossy.as_ref().expect("v7 lossy tables"),
    );

    // The lossy verdict-change table v6 x v7 is diagonal, "of which streamingSampleRate" 0.
    assert!(v6.changes.is_none(), "v6 is the lossy baseline");
    let changes = v7.changes.as_ref().expect("v6 x v7");
    assert_eq!(changes.against, V6);
    for ((from, to), n) in &changes.counts {
        if from != to {
            assert_eq!(*n, 0, "{from} -> {to}");
        }
    }
    assert!(changes.counts.values().sum::<u32>() > 0, "not vacuous");
    assert_eq!(changes.streaming, 0);

    // The lossy matrix on split `all` equals the v5 matrix of lossy.md section 9, for both files.
    let counts = |real, fake, undetectable, source_limited| LossyCounts {
        real,
        suspicious: 0,
        fake,
        undetectable,
        source_limited,
        error: 0,
    };
    for (name, lossy) in [(V6, v6), (V7, v7)] {
        let m = lossy.matrix[&SplitSel::All];
        assert_eq!(m.real_row, counts(11, 0, 0, 2), "{name}: real row");
        assert_eq!(m.fake_row, counts(0, 7, 1, 0), "{name}: fake row");
        assert_eq!(m.weak_row, LossyCounts::default(), "{name}: weak row");
        // Every streaming count is 0.
        for g in &lossy.groups {
            assert_eq!(g.streaming, 0, "{name}: {}", g.group);
        }
        // Every lossy fixture of the variant is 44.1 kHz: only `@ 44100` sub-rows.
        assert!(
            !lossy.groups.iter().any(|g| g.group.ends_with("@ 48000")),
            "{name}"
        );
        assert!(
            lossy.groups.iter().any(|g| g.group == "l320 @ 44100"),
            "{name}: the sub-rows are there"
        );
    }
}

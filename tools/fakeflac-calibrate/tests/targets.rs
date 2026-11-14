//! Acceptance-target scenarios of `specs/features/calibration.feature` (@RP-11), approved in
//! calibration.md OQ-25 to OQ-31. Synthetic sample corpus only; no ffmpeg or personal audio.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::cache::Extracted;
use fakeflac_calibrate::env::Roots;
use fakeflac_calibrate::evaluate::{evaluation, ParamsRun, RowEval};
use fakeflac_calibrate::extract::GroupResult;
use fakeflac_calibrate::model::{LabelRow, RowRoot, Truth};
use fakeflac_calibrate::render::{markdown, ReportData};
use fakeflac_calibrate::report::SplitSel;
use fakeflac_calibrate::split::Split;
use fakeflac_calibrate::{Config, Outcome};
use fakeflac_core::{DetectionParams, TrackError, Verdict};
use std::collections::BTreeMap;
use std::sync::Arc;

fn add_target(sample: &Sample, selection: &str, verdict: &str, bound: &str) {
    let text = read(&sample.config_path());
    std::fs::write(
        sample.config_path(),
        format!("{text}\n[[targets]]\nid = 'T9'\n{selection}\nverdict = '{verdict}'\n{bound}\n"),
    )
    .unwrap();
}

fn assert_target(o: &Outcome, split: SplitSel, pass: bool, count: u32) {
    assert_ok(o);
    let t = o.evaluations[0]
        .targets
        .iter()
        .find(|t| t.id == "T9" && t.split == split)
        .expect("T9 result");
    assert_eq!((t.pass, t.count), (pass, count), "{t:?}");
}

fn assert_report_line(o: &Outcome, expected: &str) {
    for file in ["summary.md", "report.md"] {
        let text = report_text(o, file);
        assert!(
            text.lines().any(|l| l == expected),
            "{file} lacks {expected:?}"
        );
    }
}

fn assert_side_cell(o: &Outcome, label: &str, expected: &str) {
    for file in ["summary.md", "report.md"] {
        let text = report_text(o, file);
        let row = text
            .lines()
            .find(|l| l.starts_with(&format!("| {label} |")))
            .unwrap_or_else(|| panic!("{file} lacks side-by-side row {label}"));
        let cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
        assert_eq!(cells[1], expected, "{file}: {row}");
    }
}

// Scenario Outline: A target with settings leaves out undetectable transcodes (@RP-11).
#[test]
fn rp11_count_targets_exclude_undetectable_transcodes() {
    for (lowpass, fake, undetectable, status) in [(false, 6, 0, "FAIL"), (true, 5, 1, "PASS")] {
        let sample = Sample::new(&format!("target-count-{lowpass}"));
        sample.write_config_opts(Some(0.0), lowpass);
        add_target(&sample, "settings = ['fx128']", "fake", "max_count = 5");
        let o = sample.accept_v2();
        assert_target(&o, SplitSel::All, status == "PASS", fake);
        let matrix = o.evaluations[0]
            .by_setting
            .iter()
            .find(|s| s.setting == "fx128")
            .unwrap();
        assert_eq!(
            (matrix.counts.fake, matrix.counts.undetectable),
            (fake, undetectable)
        );
        assert_report_line(&o, &format!("- T9: all {status} ({fake} of max 5; {undetectable} undetectable excluded), holdout PASS (0 of max 5; 0 undetectable excluded)"));
        assert_side_cell(&o, "T9 (all)", &format!("{status} ({fake})"));
    }
}

// Scenario: The undetectable line and list name the criterion (@RP-11 @RNF-09).
#[test]
fn rp11_undetectable_reports_name_extent_and_include_zero_counts() {
    for lowpass in [false, true] {
        let sample = Sample::new(&format!("target-criterion-{lowpass}"));
        sample.write_config_opts(Some(0.0), lowpass);
        let o = sample.accept_v2();
        assert_ok(&o);
        let count = u32::from(lowpass);
        assert_report_line(
            &o,
            &format!("- undetectable transcodes by criterion: extent {count}, hf drop 0"),
        );
        for file in ["summary.md", "report.md"] {
            let text = report_text(&o, file);
            let at = text
                .find("- undetectable transcodes by criterion:")
                .unwrap();
            let before = &text[..at];
            assert!(
                before.contains("| Undetectable |")
                    || before
                        .lines()
                        .rev()
                        .take(5)
                        .any(|l| l.contains("Undetectable")),
                "criterion counts follow the global all matrix: {before}"
            );
        }
        if lowpass {
            let text = report_text(&o, "report.md");
            let section = text
                .split("### Undetectable transcodes\n")
                .nth(1)
                .expect("undetectable list");
            let lines: Vec<_> = section
                .lines()
                .skip_while(|l| !l.starts_with('|'))
                .take_while(|l| l.starts_with('|'))
                .collect();
            assert_eq!(
                lines.len(),
                3,
                "header, separator, one transcode: {lines:?}"
            );
            assert_eq!(
                lines[0],
                "| path | setting | source_path | verdict | criterion |"
            );
            let cells: Vec<_> = lines[2]
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            assert_eq!(
                (&cells[1], &cells[2], &cells[3], &cells[4]),
                (&"fx128", &TRACK_5, &"fake", &"extent")
            );
        }
    }
}

// Scenario: The undetectable line and list name the criterion (@RP-11 @RNF-09),
// combined with "The undetectable rule as the tool applies it": equal band-limited
// source/transcode spectra above the lowpass match "hf drop", not "extent".
#[test]
fn rp11_undetectable_report_identifies_the_hf_drop_criterion() {
    let sample = Sample::new("target-criterion-hf-drop");
    place("low_hf", Container::Flac, &sample.corpus.join(TRACK_1));
    sample.write_trusted(&[
        format!("library_root = {}", sample.corpus.display()),
        format!("{TRACK_1} | tienda"),
    ]);
    sample.write_confirmed(&[format!("library_root = {}", sample.corpus.display())]);
    sample.write_config_with_lowpass(Some(0.0));
    let text = read(&sample.config_path())
        .replace("lowpass_hz = 16000", "lowpass_hz = 1000")
        .replace("args = [\"lp16k_dither\"]", "args = [\"low_hf\"]");
    std::fs::write(sample.config_path(), text).unwrap();
    let o = sample.accept_v2();
    assert_ok(&o);
    let row = o.evaluations[0]
        .by_setting
        .iter()
        .find(|r| r.setting == "fx128")
        .unwrap();
    assert_eq!(
        row.counts.undetectable, 1,
        "the source extends above 1 kHz but its transcode removes no HF content"
    );
    assert_report_line(
        &o,
        "- undetectable transcodes by criterion: extent 0, hf drop 1",
    );
    let report = report_text(&o, "report.md");
    let section = report
        .split("### Undetectable transcodes\n")
        .nth(1)
        .unwrap();
    let rows: Vec<_> = section
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(
        rows[2].contains(&format!("| fx128 | {TRACK_1} |")),
        "{}",
        rows[2]
    );
    assert!(rows[2].ends_with("| hf drop |"), "{}", rows[2]);
}

// Scenario Outline: A rate target (@RP-11).
#[test]
fn rp11_rate_targets_report_pass_fail_without_gating_the_run() {
    let sample = Sample::new("target-rate");
    for (bound, status) in [
        ("50.0", "PASS"),
        ("49.9", "FAIL"),
        ("100.0", "PASS"),
        ("0.0", "FAIL"),
    ] {
        sample.write_config(Some(0.0));
        add_target(
            &sample,
            "settings = ['fx128', 'fxweak']",
            "suspicious",
            &format!("max_rate_pct = {bound}"),
        );
        let o = sample.accept_v2();
        assert_target(&o, SplitSel::All, status == "PASS", 6);
        assert_report_line(&o, &format!("- T9: all {status} (6 of 12 = 50.0 %, max {bound} %; 0 undetectable excluded), holdout PASS (0 of 0 = n/a, max {bound} %; 0 undetectable excluded)"));
        assert_side_cell(&o, "T9 (all)", &format!("{status} (50.0 %)"));
        assert_side_cell(&o, "T9 (holdout)", "PASS (n/a)");
    }
}

// Scenario Outline: A rate leaves out undetectable transcodes and is compared exactly (@RP-11).
#[test]
fn rp11_rate_denominator_excludes_undetectable_and_is_not_rounded_for_comparison() {
    let sample = Sample::new("target-rate-exclude");
    for (bound, status) in [("54.5", "FAIL"), ("54.6", "PASS")] {
        sample.write_config_with_lowpass(Some(0.0));
        add_target(
            &sample,
            "settings = ['fx128', 'fxweak']",
            "suspicious",
            &format!("max_rate_pct = {bound}"),
        );
        let o = sample.accept_v2();
        assert_target(&o, SplitSel::All, status == "PASS", 6);
        assert_report_line(&o, &format!("- T9: all {status} (6 of 11 = 54.5 %, max {bound} %; 1 undetectable excluded), holdout PASS (0 of 0 = n/a, max {bound} %; 0 undetectable excluded)"));
        assert_side_cell(&o, "T9 (all)", &format!("{status} (54.5 %)"));
    }
}

// Scenario: A rate target with no row in a split passes without a rate (@RP-11).
#[test]
fn rp11_rate_target_empty_split_passes_as_not_applicable() {
    let sample = Sample::new("target-rate-empty");
    sample.write_config(Some(0.0));
    add_target(
        &sample,
        "settings = ['fx128']",
        "real",
        "max_rate_pct = 0.0",
    );
    let o = sample.accept_v2();
    assert_target(&o, SplitSel::All, true, 0);
    assert_target(&o, SplitSel::Holdout, true, 0);
    assert_report_line(&o, "- T9: all PASS (0 of 6 = 0.0 %, max 0.0 %; 0 undetectable excluded), holdout PASS (0 of 0 = n/a, max 0.0 %; 0 undetectable excluded)");
    assert_side_cell(&o, "T9 (all)", "PASS (0.0 %)");
    assert_side_cell(&o, "T9 (holdout)", "PASS (n/a)");
}

// Scenario: Error tracks are left out of a target's rate (@RP-11 @RNF-04).
#[test]
fn rp11_class_rate_target_excludes_error_rows() {
    let sample = Sample::new("target-rate-errors");
    sample.write_config(Some(0.0));
    add_target(
        &sample,
        "classes = ['reviewed']",
        "real",
        "max_rate_pct = 100.0",
    );
    let o = sample.accept_v2();
    assert_target(&o, SplitSel::All, true, 1);
    assert_eq!(o.evaluations[0].matrix[&SplitSel::All].real_row.error, 2);
    assert_report_line(
        &o,
        "- T9: all PASS (1 of 1 = 100.0 %, max 100.0 %), holdout PASS (0 of 0 = n/a, max 100.0 %)",
    );
    assert_side_cell(&o, "T9 (all)", "PASS (100.0 %)");
}

// Scenario Outline: Invalid target bounds and selections (@RP-11).
#[test]
fn rp11_invalid_target_bounds_and_selections_write_nothing() {
    let sample = Sample::new("target-invalid");
    let variants = [
        (
            "max_count = 0\nmax_rate_pct = 1.0",
            None,
            "max_count and max_rate_pct cannot both be set",
        ),
        ("", None, "needs max_count or max_rate_pct"),
        (
            "max_rate_pct = 100.5",
            None,
            "max_rate_pct must be between 0 and 100",
        ),
        (
            "max_rate_pct = -0.5",
            None,
            "max_rate_pct must be between 0 and 100",
        ),
        (
            "max_rate_pct = nan",
            None,
            "max_rate_pct must be between 0 and 100",
        ),
        (
            "max_rate_pct = inf",
            None,
            "max_rate_pct must be between 0 and 100",
        ),
        (
            "max_rate_pct = -inf",
            None,
            "max_rate_pct must be between 0 and 100",
        ),
        (
            "max_rate_pct = 0.0005",
            None,
            "max_rate_pct must have at most 3 decimals",
        ),
        (
            "max_rate_pct = 1.0001",
            None,
            "max_rate_pct must have at most 3 decimals",
        ),
        (
            "max_rate_pct = 1.000000002",
            None,
            "max_rate_pct must have at most 3 decimals",
        ),
        (
            "max_count = 0",
            Some("classes = ['primary']\nsettings = ['fx128']"),
            "classes and settings cannot both be set",
        ),
        ("max_count = 0", Some(""), "needs classes or settings"),
        (
            "max_count = 0",
            Some("settings = ['missing']"),
            "unknown setting 'missing'",
        ),
        (
            "max_rate_pct = 1.0",
            Some("classes = ['missing']"),
            "unknown class 'missing'",
        ),
    ];
    for (bound, selection, message) in variants {
        sample.write_config(Some(0.0));
        let original = read(&sample.config_path());
        let start = original.find("id = \"T2\"").unwrap();
        let mut text = original[..start].to_string();
        text.push_str(&format!(
            "id = 'T2'\n{}\nverdict = 'real'\n{bound}\n",
            selection.unwrap_or("settings = ['fx128']")
        ));
        std::fs::write(sample.config_path(), text).unwrap();
        let corpus_before = snapshot(&sample.corpus);
        let calibration_before = snapshot(&sample.calibration);
        let scratch_before = snapshot(&sample.scratch);
        let o = sample.accept_v2();
        assert_eq!(o.exit_code, 2, "{bound}: {}", o.stderr);
        assert!(
            o.stderr.contains(&format!("targets 'T2': {message}")),
            "{bound}: {}",
            o.stderr
        );
        assert_eq!(
            snapshot(&sample.calibration),
            calibration_before,
            "nothing written for {bound}"
        );
        assert_eq!(
            snapshot(&sample.corpus),
            corpus_before,
            "corpus unchanged for {bound}"
        );
        assert_eq!(
            snapshot(&sample.scratch),
            scratch_before,
            "inputs unchanged for {bound}"
        );
        assert!(o.report_dir.is_none());
    }
}

// Scenario: Configs v1 to v4 keep their targets (@RP-11).
#[test]
fn rp11_old_config_versions_keep_their_count_targets() {
    for version in 1..=4 {
        let text =
            read(&repo_root().join(format!("specs/calibration/calibration.v{version}.toml")));
        let config = Config::parse(&text).unwrap();
        assert_eq!(config.config_version, version);
        let value: toml::Table = toml::from_str(&text).unwrap();
        let targets = value["targets"].as_array().unwrap();
        assert_eq!(targets.len(), 3);
        for (index, target) in targets.iter().enumerate() {
            let t = target.as_table().unwrap();
            assert_eq!(t["max_count"].as_integer(), Some(0));
            assert!(!t.contains_key("max_rate_pct"));
            assert_eq!(t.contains_key("classes"), index == 0);
            assert_eq!(t.contains_key("settings"), index != 0);
        }
    }
}

// Scenario: calibration.v5.toml is calibration.v2.toml with targets version 2 (@RP-11).
#[test]
fn rp11_v5_preserves_v2_except_for_the_approved_targets() {
    let text = read(&repo_root().join("specs/calibration/calibration.v5.toml"));
    let config = Config::parse(&text).expect("v5 loads");
    assert_eq!(config.config_version, 5);
    let mut v5: toml::Table = toml::from_str(&text).unwrap();
    let mut v2: toml::Table = toml::from_str(&read(&spec_config_v2())).unwrap();
    let new_targets = v5.remove("targets").unwrap();
    let old_targets = v2.remove("targets").unwrap();
    assert_eq!(new_targets[0], old_targets[0], "T1 unchanged");
    let mut expected_t2 = old_targets[1].as_table().unwrap().clone();
    expected_t2.insert(
        "description".into(),
        "No detectable transcode from 128 or 192 kbps is Real (H1 delivery criterion)".into(),
    );
    assert_eq!(new_targets[1].as_table().unwrap(), &expected_t2);
    let mut expected_t3 = old_targets[2].as_table().unwrap().clone();
    expected_t3.remove("max_count");
    expected_t3.insert("max_rate_pct".into(), toml::Value::Float(1.0));
    expected_t3.insert(
        "description".into(),
        "At most 1 % of the detectable transcodes from 320 kbps are Real (planning 2.7)".into(),
    );
    assert_eq!(new_targets[2].as_table().unwrap(), &expected_t3);
    v5.remove("config_version");
    v2.remove("config_version");
    assert_eq!(
        v5, v2,
        "every other section, setting and lossy entry is unchanged"
    );
}

/// Judging large exact-rate examples needs only labelled rows and verdicts, not audio. All rows
/// have one shared extraction error placeholder; the supplied RowEval is the input under test.
fn exact_rate_report(bound: &str, k: usize, n: usize) -> (bool, String) {
    let text = read(&spec_config());
    let text = format!("{}[[targets]]\nid = 'T9'\nsettings = ['cbr128']\nverdict = 'fake'\nmax_rate_pct = {bound}\n", &text[..text.find("[[targets]]").unwrap()]);
    let config = Config::parse(&text).expect("a rate target loads");
    let row = LabelRow {
        root: RowRoot::Generated,
        path: "synthetic.flac".into(),
        truth: Truth::Fake,
        origin: "generated",
        trust_origin: String::new(),
        class: "positive".into(),
        encoder: "fixture".into(),
        setting: "cbr128".into(),
        bitrate_kbps: Some(128),
        encoder_version: "fixture".into(),
        fragment: None,
        source_path: "source.flac".into(),
        album_id: "album".into(),
        split: Split::Tune,
        watch_id: String::new(),
        extracted: Arc::new(Extracted::Error(TrackError::TooShort)),
        undetectable: false,
    };
    let run = ParamsRun {
        name: "synthetic.toml".into(),
        params: DetectionParams::default(),
        hash: "0000000000000000".into(),
        fingerprint_id: "synthetic".into(),
        group: 0,
        evals: (0..n)
            .map(|i| RowEval {
                verdict: if i < k { Verdict::Fake } else { Verdict::Real },
                reasons: Vec::new(),
                error: None,
                m: None,
                margins: None,
            })
            .collect(),
        lossy_evals: Vec::new(),
    };
    let group = GroupResult {
        rows: vec![row; n],
        lossy_rows: Vec::new(),
        meta: BTreeMap::new(),
        real_excluded: Vec::new(),
        stats: fakeflac_calibrate::report::CacheStats {
            fingerprint_id: "synthetic".into(),
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
    };
    let eval = evaluation(&run, &group.rows, &config, None);
    let pass = eval
        .targets
        .iter()
        .find(|t| t.id == "T9" && t.split == SplitSel::All)
        .unwrap()
        .pass;
    let roots = Roots {
        corpus: "synthetic-corpus".into(),
        calibration: "synthetic-calibration".into(),
    };
    let text = markdown(
        &ReportData {
            run_id: "synthetic",
            date: "2026-11-14",
            commit: "synthetic",
            mode: "accept",
            config: &config,
            roots: &roots,
            groups: &[group],
            runs: &[run],
            evaluations: &[eval],
            unit_strata: &BTreeMap::new(),
            grid: None,
        },
        false,
    );
    (pass, text)
}

// Scenario Outline: A rate bound is compared in integer thousandths of a percent (@RP-11).
#[test]
fn rp11_rate_bounds_use_exact_integer_thousandths_and_preserve_display_precision() {
    for (bound, k, n, pass, shown) in [
        ("16.15", 323, 2000, true, "16.15"),
        ("16.15", 324, 2000, false, "16.15"),
        ("8.075", 323, 4000, true, "8.075"),
        ("1.0", 20, 2000, true, "1.0"),
        ("1.0", 21, 2000, false, "1.0"),
        ("0.001", 1, 100000, true, "0.001"),
        ("0.001", 1, 99999, false, "0.001"),
        ("100.0", 7, 7, true, "100.0"),
        ("100.0", 100000, 100000, true, "100.0"),
        ("16.150", 323, 2000, true, "16.15"),
        ("1.0000000005", 20, 2000, true, "1.0"),
    ] {
        let (actual, text) = exact_rate_report(bound, k, n);
        assert_eq!(actual, pass, "max {bound}, {k} of {n}");
        let line = text.lines().find(|l| l.starts_with("- T9: all")).unwrap();
        assert!(line.contains(&format!("max {shown} %;")), "{line}");
    }
}

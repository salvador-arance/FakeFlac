//! Tests derived from `specs/features/calibration-lossy.feature` (@RP-11 @RF-15 @RNF-05): acceptance on
//! the lossy variant of the sample corpus (`specs/calibration/lossy.md` section 9): lossless tables,
//! labels, decodes, lossy matrix and groups, `tracks.csv` columns, chains and privacy. Synthetic
//! lossy fixtures, fixture transcoder, no ffmpeg, no real audio. Protected after the `test(RF-15)`
//! commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::report::{Evaluation, LossyCounts, SplitSel};
use fakeflac_core::{DetectionParams, Verdict};

const V2: &str = "detection-params.v2.toml";
const V5: &str = "detection-params.v5.toml";

fn corpus_row<'a>(labels: &'a Table, path: &str) -> Option<&'a Vec<String>> {
    let (p, r) = (labels.col("path"), labels.col("root"));
    labels
        .rows
        .iter()
        .find(|row| row[p] == path && row[r] == "corpus")
}

fn lossy_counts(real: u32, fake: u32, undetectable: u32, source_limited: u32) -> LossyCounts {
    LossyCounts {
        real,
        suspicious: 0,
        fake,
        undetectable,
        source_limited,
        error: 0,
    }
}

fn v5_eval(o: &fakeflac_calibrate::Outcome) -> &Evaluation {
    let e = &o.evaluations[1];
    assert_eq!(e.params_name, V5);
    e
}

// Scenario: The lossless tables equal those of the base sample corpus (@RP-11 @RF-15)
#[test]
fn rf15_the_lossless_tables_equal_those_of_the_base_sample_corpus() {
    let lossy = shared_lossy();
    let base = shared_base_v2_v5();
    assert_ok(&lossy.outcome);
    assert_ok(&base.outcome);
    assert_eq!(lossy.outcome.evaluations.len(), 2);
    for (got, want) in lossy
        .outcome
        .evaluations
        .iter()
        .zip(&base.outcome.evaluations)
    {
        let name = &got.params_name;
        assert_eq!(
            got.matrix[&SplitSel::All],
            want.matrix[&SplitSel::All],
            "{name}"
        );
        assert_eq!(got.by_setting, want.by_setting, "{name}: per setting");
        assert_eq!(
            got.by_setting_split[&SplitSel::All],
            want.by_setting_split[&SplitSel::All],
            "{name}: per setting, split all"
        );
        assert_eq!(
            got.fp_rows[&SplitSel::All],
            want.fp_rows[&SplitSel::All],
            "{name}: false positives"
        );
        assert_eq!(got.primary_figure, want.primary_figure, "{name}");
        assert_eq!(got.watch, want.watch, "{name}: watch list");
        assert_eq!(
            got.full_band_by_setting, want.full_band_by_setting,
            "{name}"
        );
        assert_eq!(got.changes, want.changes, "{name}: verdict changes");
        assert_eq!(got.setting_changes, want.setting_changes, "{name}");
        let all = |e: &Evaluation| {
            e.targets
                .iter()
                .filter(|t| t.split == SplitSel::All)
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(all(got), all(want), "{name}: targets");
    }
    // No lossy row in the lossless global confusion matrix: the base counts of the v2 corpus.
    let matrix = lossy.outcome.evaluations[0].matrix[&SplitSel::All];
    assert_eq!(matrix.real_row.total(), 7);
    assert_eq!(matrix.fake_row.total(), 25);
}

// Scenario: Lossy rows are labelled (@RP-11 @RF-15)
#[test]
fn rf15_lossy_rows_are_labelled() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    let labels = labels(&shared.outcome, &shared.sample);
    assert_eq!(labels.rows.len(), 55, "labels.csv rows");

    let generated: Vec<_> = labels
        .rows
        .iter()
        .filter(|r| labels.get(r, "domain") == "lossy" && labels.get(r, "root") == "generated")
        .collect();
    assert_eq!(generated.len(), 18, "6 sources x 3 lossy entries");
    for setting in ["l128", "l320", "l128_320"] {
        let rows: Vec<_> = generated
            .iter()
            .filter(|r| labels.get(r, "setting") == setting)
            .collect();
        assert_eq!(rows.len(), 6, "{setting}");
        for r in rows {
            if setting == "l128_320" {
                assert_eq!(labels.get(r, "truth"), "fake");
                assert_eq!(labels.get(r, "class"), "positive");
                assert_eq!(labels.get(r, "stages"), "fxmp3_128>fxmp3_320_src128");
                assert_eq!(labels.get(r, "source_bitrate_kbps"), "128");
            } else {
                assert_eq!(labels.get(r, "truth"), "real");
                assert_eq!(labels.get(r, "class"), "generated");
            }
        }
    }

    let aac = corpus_row(&labels, TRACK_13).expect("row of the m4a");
    assert_eq!(labels.get(aac, "truth"), "real");
    assert_eq!(labels.get(aac, "origin"), "confirmed_real");
    assert_eq!(labels.get(aac, "class"), "reviewed");
    assert_eq!(labels.get(aac, "codec"), "aac");
    assert_eq!(labels.get(aac, "declared_bitrate_kbps"), "128");

    let fake = corpus_row(&labels, TRACK_12).expect("row of the mp3");
    assert_eq!(labels.get(fake, "truth"), "fake");
    assert_eq!(labels.get(fake, "origin"), "confirmed_fake");
    assert_eq!(labels.get(fake, "upstream"), "aac");
    assert_eq!(labels.get(fake, "source_bitrate_kbps"), "128");

    let opus = corpus_row(&labels, TRACK_14).expect("row of the Opus file");
    assert_eq!(labels.get(opus, "not_evaluated"), "opus decoder pending");
}

// Scenario: A cold run decodes every lossy file once, except Opus (@RP-11 @RF-15)
#[test]
fn rf15_a_cold_run_decodes_every_lossy_file_once_except_opus() {
    let sample = Sample::new_lossy("lossy-cold-run");
    assert!(
        files_under(&sample.features_dir()).is_empty(),
        "an empty features folder"
    );
    let first = sample.accept_lossy();
    assert_ok(&first);
    assert_eq!(decoded(&first), 54);
    let second = sample.accept_lossy();
    assert_ok(&second);
    assert_eq!(decoded(&second), 0);
    assert_eq!(transcoded(&second), 0);
}

// Scenario: Params without a [lossy] section do not evaluate lossy rows (@RP-11 @RF-15)
#[test]
fn rf15_params_without_lossy_section_do_not_evaluate_lossy_rows() {
    let shared = shared_lossy();
    let o = &shared.outcome;
    assert_ok(o);
    let (v2, v5) = (&o.evaluations[0], v5_eval(o));
    assert_eq!(v2.params_name, V2);
    assert_eq!(v2.lossy_not_evaluated, 22);
    assert!(v2.lossy.is_none(), "no lossy tables for v2");
    assert_eq!(v5.lossy_not_evaluated, 1, "the Opus file");
    assert!(v5.lossy.is_some());

    let header = format!(
        "{}\n{}",
        report_text(o, "summary.md"),
        report_text(o, "report.md")
    );
    let line_with = |needle: &str| header.lines().any(|l| l.contains(needle));
    assert!(line_with(&format!(
        "{V2} evaluates no lossy row (22 not evaluated)"
    )));
    assert!(line_with(&format!("{V5} leaves 1 lossy row not evaluated")));

    // No lossy row is counted in any Error column: the lossless v2 error count is the base's 2 and 0.
    let matrix = v2.matrix[&SplitSel::All];
    assert_eq!(matrix.real_row.error, 2);
    assert_eq!(matrix.fake_row.error, 0);
    let lossy = v5.lossy.as_ref().unwrap();
    for split in [SplitSel::All, SplitSel::Tune, SplitSel::Holdout] {
        let m = lossy.matrix[&split];
        assert_eq!(m.real_row.error + m.fake_row.error + m.weak_row.error, 0);
    }
}

// Scenario: Lossy confusion matrix (@RP-11 @RF-15)
#[test]
fn rf15_lossy_confusion_matrix() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    let lossy = v5_eval(&shared.outcome)
        .lossy
        .as_ref()
        .expect("lossy tables");
    let m = lossy.matrix[&SplitSel::All];
    assert_eq!(m.real_row, lossy_counts(11, 0, 0, 2));
    assert_eq!(m.real_row.total(), 13);
    assert_eq!(m.fake_row, lossy_counts(0, 7, 1, 0));
    assert_eq!(m.fake_row.total(), 8);
    assert_eq!(m.weak_row, LossyCounts::default());
    // The splits partition the rows.
    let (t, h) = (
        lossy.matrix[&SplitSel::Tune],
        lossy.matrix[&SplitSel::Holdout],
    );
    assert_eq!(t.real_row.total() + h.real_row.total(), 13);
    assert_eq!(t.fake_row.total() + h.fake_row.total(), 8);
}

// Scenario: Lossy table per group (@RP-11 @RF-15)
#[test]
fn rf15_lossy_table_per_group() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    let lossy = v5_eval(&shared.outcome)
        .lossy
        .as_ref()
        .expect("lossy tables");
    let row = |group: &str| {
        lossy
            .groups
            .iter()
            .find(|g| g.group == group)
            .unwrap_or_else(|| panic!("no group {group} in {:?}", lossy.groups))
    };
    assert_eq!(row("l128").counts, lossy_counts(5, 0, 0, 1));
    assert_eq!(row("l320").counts, lossy_counts(5, 0, 0, 1));
    assert_eq!(row("l128_320").counts, lossy_counts(0, 5, 1, 0));
    assert_eq!(row("confirmed_real").counts, lossy_counts(1, 0, 0, 0));
    assert_eq!(row("confirmed_fake aac").counts, lossy_counts(0, 2, 0, 0));
    assert_eq!(row("l128_320").median_estimate_kbps, Some(128.0));
    assert_eq!(
        row("confirmed_fake aac").within_source,
        Some((1, 1)),
        "1 of 1 estimates is at most its bound"
    );
    // Config order first: the three lossy entries of the sample config come in this order.
    let order: Vec<_> = lossy.groups.iter().map(|g| g.group.as_str()).collect();
    let at = |g: &str| order.iter().position(|x| *x == g).unwrap();
    assert!(at("l128") < at("l320") && at("l320") < at("l128_320"));
    assert!(at("l128_320") < at("confirmed_real"));
}

// Scenario: Lossy columns in tracks.csv (@RP-11 @RF-15)
#[test]
fn rf15_lossy_columns_in_tracks_csv() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    let params: DetectionParams = toml::from_str(&read(&v5_params())).unwrap();
    let fake_shortfall = params.lossy.as_ref().unwrap().fake_shortfall_hz;
    let labels = labels(&shared.outcome, &shared.sample);
    let lossy_keys: std::collections::HashSet<(String, String)> = labels
        .rows
        .iter()
        .filter(|r| labels.get(r, "domain") == "lossy")
        .map(|r| {
            (
                labels.get(r, "root").to_string(),
                labels.get(r, "path").to_string(),
            )
        })
        .collect();
    let t = tracks(&shared.outcome);
    let lossy_columns = [
        "codec",
        "declared_bitrate_kbps",
        "bitrate_mode",
        "encoder_lowpass_hz",
        "fc_hz",
        "c_exp_hz",
        "shortfall_hz",
        "m_lossy_suspicious_hz",
        "m_lossy_fake_hz",
        "m_lossy_step_db",
        "estimate_kbps",
        "source_limited",
    ];
    let lossless_margins = [
        "m_low_hf_db",
        "m_gap_fake_db",
        "m_gap_suspicious_db",
        "m_step_fake_db",
        "m_step_suspicious_db",
        "m_step_low_db",
        "m_step_high_db",
        "m_gradual_hz",
        "fs_fake_range_hz",
        "nearest_fake_margin_db",
        "m_full_band_level_db",
        "m_full_band_burst_db",
        "m_full_band_step_db",
    ];
    for c in lossy_columns {
        assert!(t.has_column(c), "tracks.csv lacks {c}");
    }
    let mut checked = 0;
    for row in &t.rows {
        let key = (
            t.get(row, "root").to_string(),
            t.get(row, "path").to_string(),
        );
        let lossy_row = lossy_keys.contains(&key);
        let v5 = t.get(row, "params_file") == V5;
        if !lossy_row {
            for c in lossy_columns {
                assert_eq!(t.get(row, c), "", "lossless row {key:?}, column {c}");
            }
            continue;
        }
        if !v5 {
            assert_eq!(t.get(row, "fc_hz"), "", "v2 does not evaluate lossy rows");
            continue;
        }
        if t.get(row, "H").is_empty() {
            continue; // no spectra
        }
        checked += 1;
        for c in [
            "codec",
            "declared_bitrate_kbps",
            "fc_hz",
            "c_exp_hz",
            "shortfall_hz",
            "m_lossy_suspicious_hz",
            "m_lossy_fake_hz",
            "m_lossy_step_db",
            "estimate_kbps",
        ] {
            assert_ne!(t.get(row, c), "", "lossy row {key:?}, column {c}");
        }
        let num = |c: &str| t.get(row, c).parse::<f64>().unwrap();
        // Written values may be rounded to 2 decimals.
        assert!(
            (num("m_lossy_fake_hz") - (fake_shortfall - num("shortfall_hz"))).abs() <= 0.011,
            "m_lossy_fake_hz of {key:?}"
        );
        for c in lossless_margins {
            if t.has_column(c) {
                assert_eq!(t.get(row, c), "", "lossy row {key:?}, lossless margin {c}");
            }
        }
    }
    assert_eq!(
        checked, 21,
        "18 generated rows and 3 decodable corpus files"
    );
}

// Scenario: Tracks of one master are paired across versions (@RP-11 @RF-15)
#[test]
fn rf15_tracks_of_one_master_are_paired_across_versions() {
    let shared = shared_lossy();
    let o = &shared.outcome;
    assert_ok(o);
    let chain = v5_eval(o)
        .chains
        .iter()
        .find(|c| c.id == "C1")
        .expect("chain C1");
    assert_eq!(chain.keys, vec!["1", "2", "3"]);
    assert_eq!(chain.pair_groups, vec!["1"], "1 pair group");
    let entry = |key: &str, version: &str| {
        chain
            .entries
            .iter()
            .find(|e| e.key == key && e.version == version)
    };
    let pcm = entry("1", "pcm").expect("the WAV version of key 1");
    assert_eq!((pcm.path.as_str(), pcm.verdict), (TRACK_7, Verdict::Real));
    let mp3 = entry("1", "mp3").expect("the MP3 version of key 1");
    assert_eq!((mp3.path.as_str(), mp3.verdict), (TRACK_15, Verdict::Fake));
    assert_eq!(mp3.estimate_kbps, Some(128));
    for key in ["2", "3"] {
        let versions: Vec<_> = chain.entries.iter().filter(|e| e.key == key).collect();
        assert_eq!(versions.len(), 1, "key {key} has one version only");
        assert_eq!(versions[0].verdict, Verdict::Error, "key {key}");
    }

    let report = report_text(o, "report.md");
    assert!(report.contains(TRACK_7) && report.contains(TRACK_15));
    let summary = report_text(o, "summary.md");
    assert!(summary.contains("C1"), "summary.md shows the chain id");
    for name in PRIVATE_NAMES.iter().chain(LOSSY_PRIVATE_NAMES.iter()) {
        assert!(!summary.contains(name), "summary.md contains '{name}'");
    }
}

// Scenario: The corpus is not modified by lossy evaluation (@RP-11 @RF-15 @RNF-05)
#[test]
fn rf15_the_corpus_is_not_modified_by_lossy_evaluation() {
    let sample = Sample::new_lossy("lossy-corpus-untouched");
    let before = snapshot(&sample.corpus);
    assert!(before.contains_key(TRACK_12) && before.contains_key(TRACK_14));
    let o = sample.accept_lossy();
    assert_ok(&o);
    assert_eq!(snapshot(&sample.corpus), before);
}

// Scenario: summary.md holds no lossy corpus names (@RP-11 @RF-15)
#[test]
fn rf15_summary_holds_no_lossy_corpus_names() {
    let shared = shared_lossy();
    let o = &shared.outcome;
    assert_ok(o);
    let summary = report_text(o, "summary.md");
    for name in LOSSY_PRIVATE_NAMES.iter().chain(PRIVATE_NAMES.iter()) {
        assert!(!summary.contains(name), "summary.md contains '{name}'");
    }
    for root in [&shared.sample.corpus, &shared.sample.calibration] {
        let text = root.to_string_lossy().into_owned();
        for variant in [text.replace('\\', "/"), text.replace('/', "\\")] {
            assert!(!summary.contains(&variant), "summary.md contains {variant}");
        }
    }
}

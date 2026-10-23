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

// ---- Round 2: follow-ups of the spec PR #57 and surviving mutants -------------------------------------

fn group_row<'a>(
    lossy: &'a fakeflac_calibrate::report::LossyEvaluation,
    group: &str,
) -> &'a fakeflac_calibrate::report::LossyGroupRow {
    lossy
        .groups
        .iter()
        .find(|g| g.group == group)
        .unwrap_or_else(|| panic!("no group {group} in {:?}", lossy.groups))
}

// Scenario: Lossy table per group (@RP-11 @RF-15): the median estimate of the `Fake` rows (lossy.md
// section 6.3), the figure that B8 gates. The all-rows median is 192 here, the Fake rows' one is 128.
// Also Scenario: Lossy confusion matrix: a Suspicious lossy row is not a Fake one.
#[test]
fn rf15_suspicious_lossy_rows_are_their_own_column_and_the_fake_median_ignores_them() {
    let sample = Sample::new_lossy("lossy-suspicious-rows");
    // mp3_320_lp18k5 is Suspicious (cutoffBelowDeclared) with an estimate of 192 under detection v5.
    for name in ["11 – Sospecha A.mp3", "12 – Sospecha B.mp3"] {
        place_lossy(
            "mp3_320_lp18k5",
            &sample.corpus.join("Pérdidas/Vídeo").join(name),
        );
    }
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.push("Pérdidas\\Vídeo\\11 – Sospecha A.mp3 | fake upstream=aac".to_string());
    confirmed.push("Pérdidas\\Vídeo\\12 – Sospecha B.mp3 | fake upstream=aac".to_string());
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let lossy = v5_eval(&o).lossy.as_ref().expect("lossy tables");

    let m = lossy.matrix[&SplitSel::All];
    let expected = LossyCounts {
        suspicious: 2,
        ..lossy_counts(0, 7, 1, 0)
    };
    assert_eq!(m.fake_row, expected, "Suspicious is not Fake");
    assert_eq!(m.real_row, lossy_counts(11, 0, 0, 2), "real rows unchanged");

    let aac = group_row(lossy, "confirmed_fake aac");
    assert_eq!(
        aac.counts,
        LossyCounts {
            suspicious: 2,
            ..lossy_counts(0, 2, 0, 0)
        }
    );
    assert_eq!(
        aac.median_estimate_kbps,
        Some(192.0),
        "all rows with an estimate"
    );
    assert_eq!(
        aac.fake_median_estimate_kbps,
        Some(128.0),
        "the Fake rows only"
    );
}

// Scenario: Lossy table per group (@RP-11 @RF-15): the median estimate of the Fake rows of l128_320 is 128,
// and only positive groups carry it.
#[test]
fn rf15_the_fake_row_median_estimate_is_for_positive_groups_only() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    let lossy = v5_eval(&shared.outcome)
        .lossy
        .as_ref()
        .expect("lossy tables");
    for g in &lossy.groups {
        if g.positive {
            continue;
        }
        assert_eq!(
            g.fake_median_estimate_kbps, None,
            "negative group {}",
            g.group
        );
        assert_eq!(g.within_source, None, "negative group {}", g.group);
    }
    assert_eq!(
        group_row(lossy, "l128_320").fake_median_estimate_kbps,
        Some(128.0)
    );
    assert_eq!(
        group_row(lossy, "confirmed_fake aac").fake_median_estimate_kbps,
        Some(128.0)
    );
    assert_eq!(group_row(lossy, "l128").fake_median_estimate_kbps, None);
    assert_eq!(
        group_row(lossy, "confirmed_real").fake_median_estimate_kbps,
        None
    );
}

// Scenario: Lossy confusion matrix (@RP-11 @RF-15): weak labels have their own row and stay out of the
// strong `fake` row (lossy.md section 6.2)
#[test]
fn rf15_a_weak_label_is_counted_in_the_weak_row_only() {
    let sample = Sample::new_lossy("lossy-weak-row");
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed[3] =
        "Pérdidas\\Vídeo\\07 – Falso 320.mp3 | fake upstream=aac source_max_kbps=128 weak"
            .to_string();
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let lossy = v5_eval(&o).lossy.as_ref().expect("lossy tables");
    let m = lossy.matrix[&SplitSel::All];
    assert_eq!(m.fake_row, lossy_counts(0, 6, 1, 0), "the strong fake row");
    assert_eq!(m.weak_row, lossy_counts(0, 1, 0, 0), "the weak row");
    let weak = group_row(lossy, "weak");
    assert_eq!(weak.counts, lossy_counts(0, 1, 0, 0));
    assert!(weak.positive);
    assert_eq!(weak.within_source, Some((1, 1)));
    assert_eq!(weak.fake_median_estimate_kbps, Some(128.0));
}

// Scenario: Lossy table per group (@RP-11 @RF-15), lossy.md section 4.2: a cascade is a fake only when its
// lowest earlier bitrate is LOWER than the last stage's; at equal bitrates it is an honest (negative) row, and a
// negative cascade has a source bitrate but neither a share of estimates within it nor a Fake median.
#[test]
fn rf15_cascades_at_equal_or_higher_bitrates_are_negatives_without_a_share() {
    let sample = Sample::new_lossy("lossy-cascade-truth");
    let path = sample.config_path();
    let mut text = read(&path);
    text.push_str(
        "\n[[generate.lossy]]\nid = \"c128_128\"\nstages = [\"fxmp3_128\", \"fxmp3_128\"]\n\n\
         [[generate.lossy]]\nid = \"c320_128\"\nstages = [\"fxmp3_320\", \"fxmp3_128\"]\n",
    );
    std::fs::write(&path, text).unwrap();
    let o = sample.accept_lossy();
    assert_ok(&o);

    let labels = labels(&o, &sample);
    for (setting, stages, source) in [
        ("c128_128", "fxmp3_128>fxmp3_128", "128"),
        ("c320_128", "fxmp3_320>fxmp3_128", "320"),
    ] {
        let rows: Vec<_> = labels
            .rows
            .iter()
            .filter(|r| labels.get(r, "domain") == "lossy" && labels.get(r, "setting") == setting)
            .collect();
        assert_eq!(rows.len(), 6, "{setting}");
        for r in rows {
            assert_eq!(labels.get(r, "truth"), "real", "{setting}");
            assert_eq!(labels.get(r, "class"), "generated", "{setting}");
            assert_eq!(labels.get(r, "stages"), stages, "{setting}");
            assert_eq!(labels.get(r, "source_bitrate_kbps"), source, "{setting}");
        }
    }

    let lossy = v5_eval(&o).lossy.as_ref().expect("lossy tables");
    for id in ["c128_128", "c320_128"] {
        let g = group_row(lossy, id);
        assert!(!g.positive, "{id} is a negative group");
        assert_eq!(
            g.within_source, None,
            "{id}: no share for a negative cascade"
        );
        assert_eq!(g.fake_median_estimate_kbps, None, "{id}");
    }
    let positive = group_row(lossy, "l128_320");
    assert!(positive.positive);
    let (within, of) = positive
        .within_source
        .expect("a share for a positive cascade");
    assert!(of > 0 && within == of, "{within} of {of}");
    // The real rows of the lossy matrix gain the 12 honest cascade encodes (all of them detected or
    // source-limited, none a Fake row).
    let m = lossy.matrix[&SplitSel::All];
    assert_eq!(m.real_row.total(), 13 + 12);
    assert_eq!(m.fake_row.total(), 8, "the strong fake row is unchanged");
}

// Scenario: Lossy table per group (@RP-11 @RF-15): a lossless source whose own cutoff ends before the
// final file's `c_exp - suspicious_shortfall_hz` makes an honest encode source-limited, per row, and the
// test applies to honest generated rows only (lossy.md section 4.5). Unit test of `evaluate_lossy_row`.
#[test]
fn rf15_the_tool_marks_source_limited_encodes_like_the_spec() {
    use fakeflac_audio::SymphoniaAnalyzer;
    use fakeflac_calibrate::cache::Extracted;
    use fakeflac_calibrate::lossy::evaluate_lossy_row;
    use fakeflac_calibrate::model::{LabelRow, RowMeta, RowRoot};
    use fakeflac_calibrate::split::Split;
    use fakeflac_core::ports::SpectrumAnalyzer;
    use std::sync::Arc;

    let params: DetectionParams = toml::from_str(&read(&v5_params())).unwrap();
    let lossy = params.lossy.as_ref().unwrap();
    let extract = |name: &str| {
        Arc::new(Extracted::from_analysis(
            SymphoniaAnalyzer.analyze(&lossy_fixture(name), &params),
        ))
    };
    // The final file: MP3 320 with a 20.5 kHz cutoff, c_exp = 20500.
    let final_file = extract("mp3_320_lp20k5");
    let row = |truth| LabelRow {
        root: RowRoot::Generated,
        path: "0123456789abcdef.mp3".to_string(),
        truth,
        origin: "generated",
        trust_origin: String::new(),
        class: "generated".to_string(),
        encoder: "fixture".to_string(),
        setting: "x".to_string(),
        bitrate_kbps: Some(320),
        encoder_version: String::new(),
        fragment: None,
        source_path: String::new(),
        album_id: String::new(),
        split: Split::Tune,
        watch_id: String::new(),
        extracted: final_file.clone(),
        undetectable: false,
    };
    let meta = |source: &str, honest: bool| RowMeta {
        source: Some(extract(source)),
        honest_generated: honest,
        ..RowMeta::default()
    };
    let limited = |source: &str, honest: bool| {
        let e = evaluate_lossy_row(
            &row(fakeflac_calibrate::model::Truth::Real),
            &meta(source, honest),
            &params,
        );
        assert!(e.evaluated, "{source}");
        e.source_limited
    };
    let c_exp = 20_500.0;
    // A source cutoff of 18528 Hz lies between c - fake_shortfall (18000) and c - suspicious_shortfall (19500).
    assert!(
        c_exp - lossy.fake_shortfall_hz < 18_528.0
            && 18_528.0 < c_exp - lossy.suspicious_shortfall_hz
    );
    assert!(
        limited("mp3_320_lp18k5", true),
        "between the two thresholds: the limit is c - suspicious_shortfall"
    );
    assert!(limited("mp3_320_lp17k", true), "below both thresholds");
    assert!(
        !limited("mp3_320_lp20k5", true),
        "a source that reaches c_exp is not limiting"
    );
    // Only honest generated rows (not confirmed or trusted rows, not fake cascades).
    assert!(!limited("mp3_320_lp18k5", false));
    assert!(!limited("mp3_320_lp17k", false));
    // A row without a source is never source-limited.
    let none = evaluate_lossy_row(
        &row(fakeflac_calibrate::model::Truth::Real),
        &RowMeta {
            honest_generated: true,
            ..RowMeta::default()
        },
        &params,
    );
    assert!(!none.source_limited);
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

// ---- Round 3: surviving mutants ---------------------------------------------------------------------

// Scenario: Lossy table per group (@RP-11 @RF-15), lossy.md section 6.3 (mutant M5a): a negative group
// has no Fake median even when one of its rows is judged Fake. A trusted `tienda` MP3 with the 17 kHz
// low-pass of a 320 kbps file is a Fake-bucket row of the group `trusted primary`.
#[test]
fn rf15_a_negative_group_with_a_fake_row_has_no_fake_median() {
    let sample = Sample::new_lossy("lossy-negative-fake-row");
    let mut trusted = sample.lossy_trusted_lines();
    trusted.push("Pérdidas\\Vídeo\\07 – Falso 320.mp3 | tienda".to_string());
    sample.write_trusted(&trusted);
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.remove(3); // the fake label of the same file
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let lossy = v5_eval(&o).lossy.as_ref().expect("lossy tables");
    let g = group_row(lossy, "trusted primary");
    assert!(!g.positive);
    assert_eq!(g.counts, lossy_counts(0, 1, 0, 0), "judged Fake: {g:?}");
    assert!(
        g.median_estimate_kbps.is_some(),
        "it has an estimate: {g:?}"
    );
    assert_eq!(g.fake_median_estimate_kbps, None);
    assert_eq!(g.within_source, None);
}

/// The sum of the "N entries not generated" warnings of `summary.md`.
fn not_generated(o: &fakeflac_calibrate::Outcome) -> u32 {
    report_text(o, "summary.md")
        .lines()
        .filter_map(|l| l.strip_suffix(" entries not generated"))
        .filter_map(|l| l.rsplit(' ').next()?.parse::<u32>().ok())
        .sum()
}

// Scenario: --no-generate leaves labels.csv untouched (@RP-11 @RF-15), calibration.md section 2 and
// lossy.md section 4.1 (the still-open bug of `extract.rs`): with a cold cache nothing is cached, and the
// entries not generated are those of the trusted LOSSLESS sources: 8 trusted lossless tracks of the sample,
// each with 4 lossless settings and 3 lossy entries. A trusted `.mp3` or `.m4a` is never a source (lossy
// files are never sources), so it adds 0 whatever its codec.
#[test]
fn rf15_uncached_trusted_lossy_files_add_no_entries_not_generated() {
    let plain = Sample::new_lossy("lossy-nogen-cold-plain");
    let before = plain.accept_opts(&[v2_params()], true);
    assert_ok(&before);
    assert_eq!(not_generated(&before), 8 * (4 + 3), "trusted lossless only");

    let sample = Sample::new_lossy("lossy-nogen-cold-lossy");
    let mut trusted = sample.lossy_trusted_lines();
    trusted.push("Pérdidas\\Vídeo\\07 – Falso 320.mp3 | tienda".to_string());
    trusted.push("Pérdidas\\Vídeo\\08 – Señal ’AAC’.m4a | tienda".to_string());
    sample.write_trusted(&trusted);
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.drain(3..5); // the fake and real labels of those two files
    sample.write_confirmed(&confirmed);
    let o = sample.accept_opts(&[v2_params()], true);
    assert_ok(&o);
    assert_eq!(decoded(&o), 0, "no decoding under --no-generate");
    assert_eq!(not_generated(&o), 8 * (4 + 3), "lossy tracks add 0");
}

// Scenario: --no-generate leaves labels.csv untouched (@RP-11 @RF-15), owner decision (a), mutant M6: a
// labelled track without cached spectra is not decoded under --no-generate and is not counted as an
// "entry not generated" (it is a labelled track, not a generated entry).
#[test]
fn rf15_an_uncached_labelled_track_is_not_an_entry_not_generated() {
    let sample = Sample::new_lossy("lossy-nogen-warm");
    assert_ok(&sample.accept_lossy());
    place_lossy(
        "mp3_320_lp17k",
        &sample.corpus.join("Pérdidas/Vídeo/11 – Nuevo.mp3"),
    );
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.push("Pérdidas\\Vídeo\\11 – Nuevo.mp3 | fake upstream=aac".to_string());
    sample.write_confirmed(&confirmed);
    let o = sample.accept_opts(&[v2_params(), v5_params()], true);
    assert_ok(&o);
    assert_eq!(decoded(&o), 0, "no decoding under --no-generate");
    assert_eq!(not_generated(&o), 0);
    assert!(
        !report_text(&o, "summary.md").contains("entries not generated"),
        "no such warning"
    );
}

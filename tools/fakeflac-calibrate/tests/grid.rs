//! Tests derived from `specs/features/calibration.feature` (@RP-11): `/calibrate` grid search
//! (`specs/calibration/evaluation.md` section 7, grid of `sample-corpus.md` section 5). Protected
//! after the `test(RP-11)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::grid::{cost, is_feasible, PointRow, RowKind, Weights};
use fakeflac_core::{DetectionParams, Verdict};

/// The grid of sample-corpus.md section 5; the baseline is given as an absolute path.
fn sample_grid(sample: &Sample, values: &str) -> std::path::PathBuf {
    let text = format!(
        "grid_version = 1\nbaseline = '{}'\nmust_catch = [\"fx128\"]\nmax_points = 1000\n\n\
         [weights]\nfp = 10.0\nfn = 1.0\nsuspicious_negative = 0.5\nsuspicious_positive = 0.1\n\n\
         [values]\n{values}\n",
        v2_params().display()
    );
    sample.write_scratch("grid.toml", &text)
}

const GAP_GRID: &str = "\"verdict.gap_fake_db\" = [4.0, 6.0]";

// Scenario: The grid search proposes new parameters from tune data (@RP-11)
#[test]
fn rp11_grid_search_proposes_new_parameters_from_tune_data() {
    let sample = Sample::new("grid-proposes");
    sample.write_config(Some(0.0));
    let grid = sample_grid(&sample, GAP_GRID);
    let status_before = git_status();

    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
    let result = o.grid.as_ref().expect("grid outcome");
    assert!(
        (result.baseline.cost - 1.1).abs() < 1e-9,
        "baseline cost {}",
        result.baseline.cost
    );
    let point = result
        .points
        .iter()
        .find(|p| {
            p.values
                .iter()
                .any(|(k, v)| k == "verdict.gap_fake_db" && *v == 4.0)
        })
        .expect("point gap_fake_db = 4.0");
    assert!((point.cost - 0.5).abs() < 1e-9, "cost {}", point.cost);
    assert!(point.feasible && result.baseline.feasible);

    let report_dir = o.report_dir.as_ref().expect("report folder");
    let proposal = report_dir.join("detection-params.v4.toml");
    assert_eq!(result.proposal.as_deref(), Some(proposal.as_path()));
    let text = read(&proposal);

    // It parses to v2 with algorithm_version 4 (the next free version, evaluation.md section 7)
    // and gap_fake_db 4.0.
    let mut expected: DetectionParams = toml::from_str(&read(&v2_params())).unwrap();
    expected.algorithm_version = 4;
    expected.verdict.gap_fake_db = 4.0;
    let parsed: DetectionParams = toml::from_str(&text).unwrap();
    assert_eq!(parsed, expected);

    // Two comment lines first, then the baseline text with the two values replaced on their lines.
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[0].starts_with("# PROPOSAL from /calibrate run ")
            && lines[0].ends_with("Not approved: it needs a spec PR (G1)."),
        "{}",
        lines[0]
    );
    assert_eq!(lines[1], "# Changed: verdict.gap_fake_db 6.0 -> 4.0");
    let baseline = read(&v2_params())
        .replace("algorithm_version = 2", "algorithm_version = 4")
        .replace("gap_fake_db = 6.0", "gap_fake_db = 4.0");
    assert_eq!(lines[2..], baseline.lines().collect::<Vec<_>>()[..]);

    assert_eq!(
        git_status(),
        status_before,
        "no file in the repository changed"
    );
}

// Scenario: Holdout data never chooses the parameters (@RP-11)
#[test]
fn rp11_holdout_data_never_chooses_the_parameters() {
    let sample = Sample::new("grid-holdout");
    sample.write_config(Some(1.0));
    let grid = sample_grid(&sample, GAP_GRID);
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
    assert!(
        o.stdout.contains("no change proposed"),
        "stdout: {}",
        o.stdout
    );
    let report_dir = o.report_dir.as_ref().expect("report folder");
    let proposals: Vec<_> = files_under(report_dir)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("detection-params"))
        })
        .collect();
    assert!(proposals.is_empty(), "{proposals:?}");
    assert!(o.grid.as_ref().unwrap().proposal.is_none());
}

// Scenario: Extraction parameters cannot be searched (@RP-11)
#[test]
fn rp11_extraction_parameters_cannot_be_searched() {
    let sample = Sample::new("grid-extraction");
    let grid = sample_grid(&sample, "\"fft.window\" = [\"hann\", \"nuttall_4\"]");
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr
            .contains("extraction parameter; compare params files with accept instead"),
        "{}",
        o.stderr
    );
}

fn row(kind: RowKind, setting: &str, verdict: Verdict) -> PointRow {
    PointRow {
        kind,
        setting: setting.to_string(),
        verdict,
        undetectable: false,
    }
}

// Scenario: Undetectable transcodes are excluded from must_catch feasibility and cost (@RP-11)
#[test]
fn rp11_undetectable_transcodes_are_excluded_from_must_catch_feasibility_and_cost() {
    let weights = Weights {
        fp: 10.0,
        fn_: 1.0,
        suspicious_negative: 0.5,
        suspicious_positive: 0.1,
    };
    let must_catch = vec!["fx128".to_string()];

    // Given a must_catch setting with an undetectable transcode that receives verdict "real"
    let rows = vec![
        row(RowKind::Negative, "", Verdict::Real),
        PointRow {
            kind: RowKind::GeneratedPositive,
            setting: "fx128".to_string(),
            verdict: Verdict::Real,
            undetectable: true,
        },
    ];

    // Then the undetectable transcode does not cause must_catch feasibility to fail
    assert!(
        is_feasible(&rows, &must_catch),
        "undetectable transcode with verdict Real does not cause must_catch feasibility to fail"
    );

    // And it does not contribute to the grid cost
    assert_eq!(
        cost(&rows, &weights),
        0.0,
        "undetectable transcode does not contribute to grid cost"
    );

    // Control: a detectable transcode with verdict Real DOES cause must_catch to fail and costs 1.0
    let detectable = vec![
        row(RowKind::Negative, "", Verdict::Real),
        row(RowKind::GeneratedPositive, "fx128", Verdict::Real),
    ];
    assert!(!is_feasible(&detectable, &must_catch));
    assert_eq!(cost(&detectable, &weights), 1.0);
}

// Scenario: Feasibility requires that no confirmed fake is Real (@RP-11)
#[test]
fn rp11_feasibility_requires_that_no_confirmed_fake_is_real() {
    let must_catch = vec!["fx128".to_string()];
    let ok = vec![
        row(RowKind::Negative, "", Verdict::Real),
        row(RowKind::GeneratedPositive, "fx128", Verdict::Fake),
        row(RowKind::ConfirmedFake, "", Verdict::Fake),
    ];
    assert!(is_feasible(&ok, &must_catch), "control: feasible");

    let mut confirmed_real = ok.clone();
    confirmed_real[2] = row(RowKind::ConfirmedFake, "", Verdict::Real);
    assert!(
        !is_feasible(&confirmed_real, &must_catch),
        "a confirmed fake classified Real makes the point infeasible"
    );

    // Hard constraint (1): a must_catch positive that is Real.
    let mut must_catch_real = ok.clone();
    must_catch_real[1] = row(RowKind::GeneratedPositive, "fx128", Verdict::Real);
    assert!(!is_feasible(&must_catch_real, &must_catch));

    // A Real among the positives of another setting does not matter for feasibility.
    let mut other = ok;
    other.push(row(RowKind::GeneratedPositive, "fxweak", Verdict::Real));
    assert!(is_feasible(&other, &must_catch));
}

// Scenario: The grid search proposes new parameters from tune data (@RP-11), the cost of
// sample-corpus.md section 5: 0.5 * 1 Suspicious negative + 0.1 * 6 Suspicious positives = 1.1.
#[test]
fn rp11_cost_is_the_weighted_sum() {
    let weights = Weights {
        fp: 10.0,
        fn_: 1.0,
        suspicious_negative: 0.5,
        suspicious_positive: 0.1,
    };
    let mut rows = vec![
        row(RowKind::Negative, "", Verdict::Real),
        row(RowKind::Negative, "", Verdict::Suspicious),
    ];
    rows.extend((0..6).map(|_| row(RowKind::GeneratedPositive, "fxweak", Verdict::Suspicious)));
    rows.extend((0..19).map(|_| row(RowKind::GeneratedPositive, "fx128", Verdict::Fake)));
    assert!((cost(&rows, &weights) - 1.1).abs() < 1e-9);

    let worse = vec![
        row(RowKind::Negative, "", Verdict::Fake),
        row(RowKind::GeneratedPositive, "fx128", Verdict::Real),
        row(RowKind::ConfirmedFake, "", Verdict::Real),
    ];
    assert!((cost(&worse, &weights) - 12.0).abs() < 1e-9);
}

// Scenario: A confirmed fake in holdout still constrains the search (@RP-11)
//
// End to end through `calibrate`: the known-truth fake (track 11, `Sueltas/Mixto`, class reference) is
// put in holdout by giving the class two more albums, while the other classes stay in tune. The grid
// has one point that turns everything but the quiet track into Real: with `fn = 0` it costs less than
// the baseline on the tune rows, but it makes the holdout confirmed fake Real.
#[test]
fn rp11_a_confirmed_fake_in_holdout_still_constrains_the_search() {
    use fakeflac_calibrate::split::{assign_class, Split};
    let sample = Sample::new("grid-holdout-constraint");
    let config = fakeflac_calibrate::Config::parse(&read(&sample.write_config(None))).unwrap();

    // Two extra reference albums such that "Sueltas/Mixto" is among the 2 holdout albums of 3.
    let extra = (0..200)
        .map(|i| (format!("Extras/A{i}"), format!("Extras/B{i}")))
        .find(|(a, b)| {
            let albums = ["Sueltas/Mixto".to_string(), a.clone(), b.clone()];
            assign_class(&albums, &config.split)["Sueltas/Mixto"] == Split::Holdout
        })
        .expect("a pair of album names that puts Sueltas/Mixto in holdout");
    let mut lines = sample.trusted_lines();
    for album in [&extra.0, &extra.1] {
        let rel = format!("{album}/01 – Extra.flac");
        place("full_band", Container::Flac, &sample.corpus.join(&rel));
        lines.push(format!("{} | analisis", rel.replace('/', "\\")));
    }
    sample.write_trusted(&lines);

    let grid = sample.write_scratch(
        "grid.toml",
        &format!(
            "grid_version = 1\nbaseline = '{}'\nmust_catch = []\nmax_points = 100\n\n\
             [weights]\nfp = 10.0\nfn = 0.0\nsuspicious_negative = 0.5\nsuspicious_positive = 0.1\n\n\
             [values]\n\"verdict.gap_fake_db\" = [100.0]\n\"verdict.gap_suspicious_db\" = [100.0]\n\
             \"verdict.step_fake_db\" = [100.0]\n\"verdict.step_suspicious_db\" = [100.0]\n\
             \"verdict.gradual_min_cutoff_hz\" = [1000.0]\n",
            v2_params().display()
        ),
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);

    // Preconditions: the confirmed fake is in holdout, and the point is cheaper on the tune rows.
    let labels = Table::read(&sample.calibration.join("labels.csv"));
    let fake = labels.find("path", TRACK_11).expect("confirmed fake row");
    assert_eq!(labels.get(fake, "split"), "holdout");
    let result = o.grid.as_ref().expect("grid outcome");
    assert_eq!(result.points.len(), 1);
    let point = &result.points[0];
    assert!(
        point.cost < result.baseline.cost,
        "candidate {} must cost less than the baseline {}",
        point.cost,
        result.baseline.cost
    );

    // The scenario: infeasible, not proposed.
    assert!(result.baseline.feasible);
    assert!(
        !point.feasible,
        "a holdout confirmed fake that is Real makes it infeasible"
    );
    assert!(result.proposal.is_none());
    let report_dir = o.report_dir.as_ref().expect("report folder");
    assert!(
        files_under(report_dir).iter().all(|p| !p
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("detection-params"))),
        "no params file is written"
    );
}

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
    let proposal = report_dir.join("detection-params.v3.toml");
    assert_eq!(result.proposal.as_deref(), Some(proposal.as_path()));
    let text = read(&proposal);

    // It parses to v2 with algorithm_version 3 and gap_fake_db 4.0.
    let mut expected: DetectionParams = toml::from_str(&read(&v2_params())).unwrap();
    expected.algorithm_version = 3;
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
        .replace("algorithm_version = 2", "algorithm_version = 3")
        .replace("gap_fake_db = 6.0", "gap_fake_db = 4.0");
    assert_eq!(lines[2..], baseline.lines().collect::<Vec<_>>()[..]);

    assert_eq!(
        git_status(),
        status_before,
        "no file in the repository changed"
    );
}

// Scenario: Holdout data never chooses the parameters (@RP-11)
//
// BLOCKED: this test cannot pass as the spec is written. With `holdout_fraction = 1.0` every class of
// the sample corpus has a single album, and inputs.md section 4 says that a class with fewer than 2
// albums has k = 0 (all `tune`). So the sample corpus has tune data and the grid proposes a change.
// Needs a G1 decision (see the test report); un-ignore it when the spec is fixed.
#[test]
#[ignore = "BLOCKED: spec contradiction, holdout_fraction 1.0 cannot make a one-album class holdout (inputs.md section 4)"]
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
    }
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

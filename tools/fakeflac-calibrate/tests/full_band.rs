//! Tests derived from the @RF-04 scenarios of `specs/features/calibration.feature` (detection v3
//! in the calibration tool): v2 and v3 side by side, the full-band columns of `tracks.csv`,
//! params files that fail `DetectionParams::validate`, and the search of `[full_band]` keys.
//! Spec: `specs/calibration/{calibration,evaluation,sample-corpus}.md`, ADR-0038. Synthetic
//! fixtures and the fixture transcoder only (no ffmpeg, no real audio).
//!
//! Figures that the tool reports only as Markdown (the per-setting change table and the
//! per-setting `full_band` / `B` columns) are checked through `tracks.csv`, whose columns the spec
//! names; the Markdown tables are checked for their presence only, because their layout is not pinned.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::report::{Counts, SplitSel};
use fakeflac_core::DetectionParams;
use std::collections::BTreeMap;

fn counts(real: u32, suspicious: u32, fake: u32, error: u32) -> Counts {
    Counts {
        real,
        suspicious,
        fake,
        undetectable: 0,
        error,
    }
}

const V2: &str = "detection-params.v2.toml";
const V3: &str = "detection-params.v3.toml";

/// The rows of `tracks.csv` of one params file.
fn rows_of<'a>(t: &'a Table, params_file: &str) -> Vec<&'a Vec<String>> {
    t.rows
        .iter()
        .filter(|r| t.get(r, "params_file") == params_file)
        .collect()
}

fn num(text: &str) -> f64 {
    text.parse()
        .unwrap_or_else(|_| panic!("not a number: '{text}'"))
}

// Scenario: v2 and v3 side by side in one run (@RP-11 @RF-04)
#[test]
fn rf04_v2_and_v3_side_by_side() {
    let shared = shared_v2_and_v3();
    let o = &shared.outcome;
    assert_ok(o);
    assert_eq!(decoded(o), 33);
    assert_eq!(transcoded(o), 24);
    assert_eq!(o.evaluations.len(), 2);
    assert_eq!(o.evaluations[0].params_name, V2);
    assert_eq!(o.evaluations[1].params_name, V3);
    assert_eq!(o.evaluations[1].algorithm_version, 3);
    assert_eq!(
        o.evaluations[0].fingerprint_id, o.evaluations[1].fingerprint_id,
        "[full_band] is a rule parameter: no extra decode"
    );

    let (v2, v3) = (&o.evaluations[0], &o.evaluations[1]);
    let all = SplitSel::All;
    assert_eq!(v3.matrix[&all].fake_row, counts(0, 12, 13, 0));
    assert_eq!(v3.matrix[&all].real_row, v2.matrix[&all].real_row);

    // Verdict-change table against v2: exactly 6 changes, all "fake" to "suspicious".
    let changes = v3.changes.as_ref().expect("verdict changes");
    assert_eq!(changes.against, V2);
    let moved: Vec<_> = changes
        .counts
        .iter()
        .filter(|((from, to), n)| from != to && **n > 0)
        .collect();
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(
        changes
            .counts
            .get(&("fake".to_string(), "suspicious".to_string())),
        Some(&6)
    );

    // Per-setting change table: the 6 changes are on fxv0 and on no other setting (from tracks.csv).
    let t = tracks(o);
    let mut changed: BTreeMap<String, u32> = BTreeMap::new();
    for r2 in rows_of(&t, V2) {
        let r3 = rows_of(&t, V3)
            .into_iter()
            .find(|r| {
                t.get(r, "path") == t.get(r2, "path") && t.get(r, "setting") == t.get(r2, "setting")
            })
            .expect("every row is evaluated with both files");
        if t.get(r2, "verdict") != t.get(r3, "verdict") {
            assert_eq!(t.get(r2, "truth"), "fake");
            assert_eq!(t.get(r2, "verdict").to_lowercase(), "fake");
            assert_eq!(t.get(r3, "verdict").to_lowercase(), "suspicious");
            *changed.entry(t.get(r2, "setting").to_string()).or_default() += 1;
        }
    }
    assert_eq!(changed, BTreeMap::from([("fxv0".to_string(), 6)]));

    // The caught rate (Fake + Suspicious) of every setting is the same for both files.
    assert_eq!(v2.by_setting.len(), v3.by_setting.len());
    for (a, b) in v2.by_setting.iter().zip(&v3.by_setting) {
        assert_eq!(a.setting, b.setting);
        assert_eq!(
            a.counts.fake + a.counts.suspicious,
            b.counts.fake + b.counts.suspicious,
            "caught rate of {}",
            a.setting
        );
    }

    // The per-setting table of v3 has the full-band columns (the layout is not pinned: presence
    // only), and the figures behind them are those of the sample corpus: fxv0 is full band, the
    // transcodes with a stationary floor and fxweak (B about 2.4 dB) are not.
    let summary = report_text(o, "summary.md");
    assert!(
        summary
            .lines()
            .any(|l| l.trim_start().starts_with('|') && l.contains("full_band")),
        "summary.md has no per-setting table with a full_band column"
    );
    let mut full_band_by_setting: BTreeMap<String, u32> = BTreeMap::new();
    for r in rows_of(&t, V3) {
        let setting = t.get(r, "setting");
        if t.get(r, "truth") == "fake" && !setting.is_empty() && t.get(r, "full_band") == "true" {
            *full_band_by_setting.entry(setting.to_string()).or_default() += 1;
        }
    }
    assert_eq!(
        full_band_by_setting,
        BTreeMap::from([("fxv0".to_string(), 6)]),
        "FullBand count of the detectable positives per setting"
    );

    // Watch list: W1 (hf_bursts) shows U, B and full_band "true" with v3, empty with v2.
    let w = |file: &str| {
        let r = rows_of(&t, file)
            .into_iter()
            .find(|r| t.get(r, "watch_id") == "W1")
            .expect("W1 row");
        (
            t.get(r, "U").to_string(),
            t.get(r, "B").to_string(),
            t.get(r, "full_band").to_string(),
        )
    };
    let (u, b, fb) = w(V3);
    assert!(!u.is_empty() && !b.is_empty(), "U and B of W1 with v3");
    assert_eq!(fb, "true");
    assert_eq!(w(V2), (String::new(), String::new(), String::new()));
    assert!(report_text(o, "summary.md").contains("W1"));
}

// Scenario: Full-band columns in tracks.csv (@RP-11 @RF-04)
#[test]
fn rf04_full_band_columns_in_tracks_csv() {
    let o = &shared_v2_and_v3().outcome;
    assert_ok(o);
    let t = tracks(o);
    for column in [
        "U",
        "B",
        "full_band",
        "m_full_band_level_db",
        "m_full_band_burst_db",
        "full_band_margin_db",
    ] {
        assert!(t.has_column(column), "tracks.csv lacks {column}");
        for r in rows_of(&t, V2) {
            assert_eq!(t.get(r, column), "", "{column} is empty on v2 rows");
        }
    }

    let mut with_spectra = 0;
    for r in rows_of(&t, V3) {
        if t.get(r, "G").is_empty() {
            continue; // Error row: no spectra
        }
        with_spectra += 1;
        let margin = t.get(r, "full_band_margin_db");
        assert!(
            !margin.is_empty(),
            "v3 row with spectra has full_band_margin_db"
        );
        let margin = num(margin);
        assert_eq!(
            t.get(r, "full_band"),
            if margin >= 0.0 { "true" } else { "false" },
            "full_band is true exactly when full_band_margin_db >= 0 (margin {margin})"
        );
        // full_band_margin_db = min(m_full_band_level_db, m_full_band_burst_db)
        let level = num(t.get(r, "m_full_band_level_db"));
        let burst = num(t.get(r, "m_full_band_burst_db"));
        assert!((margin - level.min(burst)).abs() <= 0.011);
        // nearest_fake_margin_db = max(a, margin) when full band, else a
        let terms: Vec<f64> = ["m_gap_fake_db", "m_step_fake_db"]
            .iter()
            .map(|c| t.get(r, c))
            .filter(|v| !v.is_empty())
            .map(num)
            .collect();
        let a = terms.into_iter().fold(f64::INFINITY, f64::min);
        let expected = if margin >= 0.0 { a.max(margin) } else { a };
        let got = num(t.get(r, "nearest_fake_margin_db"));
        assert!(
            (got - expected).abs() <= 0.02,
            "nearest_fake_margin_db {got}, expected {expected}"
        );
    }
    assert_eq!(with_spectra, 31, "rows with spectra");
}

// Scenario Outline: A params file that fails DetectionParams::validate is rejected (@RP-11 @RF-04)
fn assert_rejected(name: &str, from: &str, to: &str, inequality: &str) {
    let sample = Sample::new(&format!("validate-{name}"));
    let text = read(&v3_params()).replace("\r\n", "\n");
    assert!(text.contains(from), "{from} not found in v3");
    let params = sample.write_scratch("invalid-v3.toml", &text.replacen(from, to, 1));
    let o = sample.accept(&[params]);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr.contains("invalid-v3.toml"),
        "names the file: {}",
        o.stderr
    );
    assert!(
        o.stderr.contains(inequality),
        "names '{inequality}': {}",
        o.stderr
    );
}

#[test]
fn rf04_full_band_low_hz_below_fake_max_cutoff_is_rejected() {
    assert_rejected(
        "low",
        "\nlow_hz = 20700.0",
        "\nlow_hz = 20000.0",
        "fake_max_cutoff_hz < full_band.low_hz",
    );
}

#[test]
fn rf04_full_band_high_hz_equal_to_low_hz_is_rejected() {
    assert_rejected(
        "high",
        "\nhigh_hz = 21000.0",
        "\nhigh_hz = 20700.0",
        "full_band.low_hz < full_band.high_hz",
    );
}

#[test]
fn rf04_gap_suspicious_above_gap_fake_is_rejected() {
    assert_rejected(
        "gap",
        "\ngap_suspicious_db = 4.0",
        "\ngap_suspicious_db = 6.5",
        "gap_suspicious_db ≤ gap_fake_db",
    );
}

#[test]
fn rf04_step_suspicious_above_step_fake_is_rejected() {
    assert_rejected(
        "step",
        "\nstep_suspicious_db = 7.0",
        "\nstep_suspicious_db = 10.5",
        "step_suspicious_db ≤ step_fake_db",
    );
}

// Scenario: A [full_band] section with a missing field is a TOML error (@RP-11 @RF-04)
#[test]
fn rf04_full_band_section_with_a_missing_field_is_a_toml_error() {
    let sample = Sample::new("validate-missing-field");
    let text = read(&v3_params()).replace("\r\n", "\n");
    let without: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("min_burst_db"))
        .collect();
    assert_eq!(without.len() + 1, text.lines().count());
    let params = sample.write_scratch("missing-field-v3.toml", &without.join("\n"));
    let o = sample.accept(&[params]);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr.contains("min_burst_db"),
        "parser message: {}",
        o.stderr
    );
}

// ---- Grid search ------------------------------------------------------------------------------

fn grid_with_baseline(
    version: u32,
    sample: &Sample,
    baseline: &std::path::Path,
    values: &str,
) -> std::path::PathBuf {
    let text = format!(
        "grid_version = {version}\nbaseline = '{}'\nmust_catch = [\"fx128\"]\nmax_points = 1000\n\n\
         [weights]\nfp = 10.0\nfn = 1.0\nsuspicious_negative = 0.5\nsuspicious_positive = 0.1\n\n\
         [values]\n{values}\n",
        baseline.display(),
        version = version
    );
    sample.write_scratch("grid.toml", &text)
}

// Scenario: A full_band key needs a baseline with a [full_band] section (@RP-11 @RF-04)
#[test]
fn rf04_full_band_key_needs_a_baseline_with_a_full_band_section() {
    let sample = Sample::new("grid-full-band-v2-baseline");
    let grid = grid_with_baseline(
        1,
        &sample,
        &v2_params(),
        "\"full_band.min_burst_db\" = [3.0, 6.0]",
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr
            .contains("full_band.min_burst_db: the baseline has no [full_band] section"),
        "{}",
        o.stderr
    );
}

// Scenario: The full-band guard can be searched from a v3 baseline (@RP-11 @RF-04)
// With everything in tune, the v3 baseline costs 0.5 (1 Suspicious negative) + 0.1 * 12 (6 fxv0 and
// 6 fxweak Suspicious positives) = 1.7. With min_burst_db = 100 the guard never fires: the 6 fxv0
// entries are Fake instead of Suspicious (0.6 less), so the point costs 1.1.
#[test]
fn rf04_the_full_band_guard_can_be_searched_from_a_v3_baseline() {
    let sample = Sample::new("grid-full-band-v3-baseline");
    sample.write_config(Some(0.0));
    let grid = grid_with_baseline(
        2,
        &sample,
        &v3_params(),
        "\"full_band.min_burst_db\" = [3.0, 100.0]",
    );
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 0, "stderr: {}", o.stderr);
    let result = o.grid.as_ref().expect("grid outcome");
    assert!(
        (result.baseline.cost - 1.7).abs() < 1e-9,
        "baseline cost {}",
        result.baseline.cost
    );
    let point = result
        .points
        .iter()
        .find(|p| {
            p.values
                .iter()
                .any(|(k, v)| k == "full_band.min_burst_db" && *v == 100.0)
        })
        .expect("point full_band.min_burst_db = 100.0");
    assert!((point.cost - 1.1).abs() < 1e-9, "cost {}", point.cost);
    assert!(point.cost < result.baseline.cost);
    assert!(point.feasible && result.baseline.feasible);

    let report_dir = o.report_dir.as_ref().expect("report folder");
    let proposal = report_dir.join("detection-params.v4.toml");
    assert_eq!(result.proposal.as_deref(), Some(proposal.as_path()));
    let text = read(&proposal);
    let parsed: DetectionParams = toml::from_str(&text).unwrap();
    assert_eq!(parsed.algorithm_version, 4);
    let value: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(
        value["full_band"]["min_burst_db"].as_float(),
        Some(100.0),
        "{text}"
    );
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1], "# Changed: full_band.min_burst_db 3.0 -> 100.0");
}

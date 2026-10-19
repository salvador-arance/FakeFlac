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
use fakeflac_calibrate::cache::Extracted;
use fakeflac_calibrate::evaluate::{is_near, Margins, RowEval};
use fakeflac_calibrate::model::{LabelRow, RowRoot, Truth};
use fakeflac_calibrate::report::{Counts, SplitSel};
use fakeflac_calibrate::split::Split;
use fakeflac_calibrate::{run, CalibrateArgs, Command, Config, Env};
use fakeflac_core::{DetectionParams, TrackError, Verdict};
use std::collections::BTreeMap;
use std::sync::Arc;

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

    // The per-setting full-band table of v3 (evaluation.md section 4): one row per setting, the
    // FullBand count of the detectable positives and the median, p95 and maximum of B. The figures
    // are those of the sample corpus: fxv0 is full band, the transcodes with a stationary floor and
    // fxweak (B about 2.4 dB) are not. A params file without [full_band] has no such rows.
    assert!(v2.full_band_by_setting.is_empty());
    assert_eq!(
        v3.full_band_by_setting
            .iter()
            .map(|r| r.setting.as_str())
            .collect::<Vec<_>>(),
        v3.by_setting
            .iter()
            .map(|r| r.setting.as_str())
            .collect::<Vec<_>>(),
        "a row per setting, in the order of the per-setting table"
    );
    for row in &v3.full_band_by_setting {
        let expected = if row.setting == "fxv0" { 6 } else { 0 };
        assert_eq!(
            row.full_band, expected,
            "full_band count of {}",
            row.setting
        );
        let burst: Vec<f64> = rows_of(&t, V3)
            .into_iter()
            .filter(|r| t.get(r, "truth") == "fake" && t.get(r, "setting") == row.setting)
            .map(|r| t.get(r, "B"))
            .filter(|b| !b.is_empty())
            .map(num)
            .collect();
        if burst.is_empty() {
            continue;
        }
        let (median, p95, max) = (
            row.burst_median
                .unwrap_or_else(|| panic!("median of B for {}", row.setting)),
            row.burst_p95
                .unwrap_or_else(|| panic!("p95 of B for {}", row.setting)),
            row.burst_max
                .unwrap_or_else(|| panic!("max of B for {}", row.setting)),
        );
        assert!(
            median <= p95 + 1e-9 && p95 <= max + 1e-9,
            "{} {median} {p95} {max}",
            row.setting
        );
        let csv_max = burst.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (max - csv_max).abs() <= 0.011,
            "max of B of {}: {max} against tracks.csv {csv_max}",
            row.setting
        );
    }
    let fxv0 = v3
        .full_band_by_setting
        .iter()
        .find(|r| r.setting == "fxv0")
        .expect("fxv0 row");
    assert!(
        fxv0.burst_median.is_some() && fxv0.burst_p95.is_some() && fxv0.burst_max.is_some(),
        "{fxv0:?}"
    );

    // Per-setting change table, directly: 6 fake -> suspicious on fxv0 and nothing elsewhere; the
    // baseline has none.
    assert!(v2.setting_changes.is_empty());
    assert!(!v3.setting_changes.is_empty());
    for row in &v3.setting_changes {
        if row.setting == "fxv0" {
            assert_eq!(
                row.counts,
                BTreeMap::from([(("fake".to_string(), "suspicious".to_string()), 6)])
            );
        } else {
            assert!(row.counts.is_empty(), "{}: {:?}", row.setting, row.counts);
        }
    }
    assert!(v3.setting_changes.iter().any(|r| r.setting == "fxv0"));

    // Watch list, directly: W1 has U, B and full_band true with v3, none with v2.
    let w1 = |e: &fakeflac_calibrate::report::Evaluation| {
        e.watch
            .iter()
            .find(|w| w.watch_id == "W1")
            .expect("W1 entry")
            .clone()
    };
    let (w3, w2) = (w1(v3), w1(v2));
    assert!(w3.upper_level_db.is_some() && w3.upper_burst_db.is_some());
    assert_eq!(w3.full_band, Some(true));
    assert_eq!(
        (w2.upper_level_db, w2.upper_burst_db, w2.full_band),
        (None, None, None)
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
// With everything in tune (sample-corpus.md sections 4 and 5), the v3 baseline costs 0.5 (1
// Suspicious negative) + 0.1 * 12 (6 fxv0 and 6 fxweak Suspicious positives) = 1.7. With
// min_burst_db = 100 the guard never fires: the 6 fxv0 entries are Fake instead of Suspicious
// (0.6 less), so the point costs 1.1. The proposal is that point, and calibrate evaluates it as
// the last params file of the run, so its verdicts are checked on that evaluation.
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
    let point = result
        .points
        .iter()
        .find(|p| {
            p.values
                .iter()
                .any(|(k, v)| k == "full_band.min_burst_db" && *v == 100.0)
        })
        .expect("point full_band.min_burst_db = 100.0");
    assert!(
        (result.baseline.cost - 1.7).abs() < 1e-9,
        "baseline cost {}",
        result.baseline.cost
    );
    assert!((point.cost - 1.1).abs() < 1e-9, "cost {}", point.cost);
    assert!(
        point.cost < result.baseline.cost,
        "point cost {} baseline cost {}",
        point.cost,
        result.baseline.cost
    );
    assert!(point.feasible && result.baseline.feasible);

    // "with the point min_burst_db = 100.0 the fxv0 entries are fake instead of suspicious"
    assert_eq!(o.evaluations.len(), 2, "baseline and proposal");
    let fxv0 = |i: usize| {
        o.evaluations[i]
            .by_setting
            .iter()
            .find(|r| r.setting == "fxv0")
            .expect("fxv0 row")
            .counts
    };
    assert_eq!(
        (fxv0(0).fake, fxv0(0).suspicious),
        (0, 6),
        "baseline: fxv0 suspicious"
    );
    assert_eq!(
        (fxv0(1).fake, fxv0(1).suspicious),
        (6, 0),
        "point: fxv0 fake"
    );

    let report_dir = o.report_dir.as_ref().expect("report folder");
    let proposal = report_dir.join(format!("detection-params.v{}.toml", next_free_version()));
    assert_eq!(result.proposal.as_deref(), Some(proposal.as_path()));
    let text = read(&proposal);
    let parsed: DetectionParams = toml::from_str(&text).unwrap();
    assert_eq!(parsed.algorithm_version, next_free_version());
    let value: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(
        value["full_band"]["min_burst_db"].as_float(),
        Some(100.0),
        "{text}"
    );
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1], "# Changed: full_band.min_burst_db 3.0 -> 100.0");
}

// Scenario: The full-band guard can be searched from a v3 baseline (@RP-11 @RF-04), exit-code
// table of calibration.md section 2 and evaluation.md section 7: when the repository is not found
// the proposal's version cannot be allocated safely, so the tool exits 2 and, as every exit 2,
// writes nothing: no cache lines, no generated fragments, no report folder.
#[test]
fn rf04_calibrate_without_a_repository_root_exits_2_and_writes_nothing() {
    let sample = Sample::new("grid-no-repo-root");
    sample.write_config(Some(0.0));
    let grid = grid_with_baseline(
        2,
        &sample,
        &v3_params(),
        "\"full_band.min_burst_db\" = [3.0, 100.0]",
    );
    let before = snapshot(&sample.calibration);
    let env = Env {
        repo_root: None,
        ..sample.env()
    };
    let o = run(
        &Command::Calibrate(CalibrateArgs {
            grid,
            config: sample.config_path(),
        }),
        &env,
        &FixtureTranscoder,
    );
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(o.report_dir.is_none());
    assert_eq!(
        snapshot(&sample.calibration),
        before,
        "exit 2 writes nothing in FAKEFLAC_CALIBRATION"
    );
}

// ---- Near-threshold list (evaluation.md section 8) --------------------------------------------

fn near_row(truth: Truth) -> LabelRow {
    LabelRow {
        root: RowRoot::Corpus,
        path: "a/b.flac".to_string(),
        truth,
        origin: "trusted",
        trust_origin: String::new(),
        class: "primary".to_string(),
        encoder: String::new(),
        setting: String::new(),
        bitrate_kbps: None,
        encoder_version: String::new(),
        fragment: None,
        source_path: String::new(),
        album_id: "a".to_string(),
        split: Split::Tune,
        watch_id: String::new(),
        extracted: Arc::new(Extracted::Error(TrackError::TooShort)),
        undetectable: false,
    }
}

/// Measurements of a track with `a = m_gap_fake_db` (no step) and the given full-band margin.
fn near_eval(a: f64, full_band_margin_db: Option<f64>) -> RowEval {
    let nearest = match full_band_margin_db {
        Some(f) if f >= 0.0 => a.max(f),
        _ => a,
    };
    RowEval {
        verdict: Verdict::Suspicious,
        reasons: Vec::new(),
        error: None,
        m: None,
        margins: Some(Margins {
            m_low_hf_db: 50.0,
            m_gap_fake_db: a,
            m_gap_suspicious_db: a + 2.0,
            m_step_fake_db: None,
            m_step_suspicious_db: None,
            m_step_low_db: None,
            m_step_high_db: None,
            m_gradual_hz: None,
            fs_fake_range_hz: None,
            m_full_band_level_db: full_band_margin_db,
            m_full_band_burst_db: full_band_margin_db,
            full_band_margin_db,
            nearest_fake_margin_db: nearest,
        }),
    }
}

// Spec: "For a params file with a [full_band] section, also a negative kept from Fake only by the
// guard at its threshold: a <= 0 and full_band_margin_db = 0 (its nearest_fake_margin_db is 0).
// For a params file without the section the list is exactly v2's."
#[test]
fn rf04_a_negative_kept_from_fake_only_by_the_guard_at_its_threshold_is_near() {
    let config = Config::parse(&read(&spec_config())).unwrap();
    let negative = near_row(Truth::Real);

    // With [full_band]: a = -2 and full_band_margin_db = 0 -> nearest_fake_margin_db = 0.
    let guarded = near_eval(-2.0, Some(0.0));
    assert_eq!(
        guarded.margins.as_ref().unwrap().nearest_fake_margin_db,
        0.0
    );
    assert!(
        is_near(&negative, &guarded, &config),
        "at the guard's threshold"
    );

    // The same a without the section (v2's list): nearest = a = -2, not near for a negative.
    assert!(!is_near(&negative, &near_eval(-2.0, None), &config));

    // With [full_band] but the guard well inside (margin 5 dB) or failing (< 0): not near.
    assert!(!is_near(&negative, &near_eval(-2.0, Some(5.0)), &config));
    assert!(!is_near(&negative, &near_eval(-2.0, Some(-0.5)), &config));
    // a > 0 (not Fake by itself) at margin 0 is not the guard case: nearest = a = 3 dB, not near.
    assert!(!is_near(&negative, &near_eval(3.0, Some(0.0)), &config));
}

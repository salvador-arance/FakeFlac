//! Calibration tool, detection v6 (ADR-0042, @RF-04 / @RP-11): `calibration.v3.toml` and the
//! per-setting `decode_args` (generation.md section 2), the edge margins and the `edge_step` column
//! (evaluation.md section 8), the `D_Q` / `F_top` / `R` / `edge_step` columns of `tracks.csv`, the
//! edge report (evaluation.md section 4) and the refusal to search `edge.*` keys (section 7).
//! The spec has no Gherkin scenario for these: the tests follow the explicit contracts of those
//! sections. Synthetic fixtures and the fixture transcoder only (no ffmpeg, no real audio).
//!
//! The layout of the Markdown report is not pinned by the spec, so the edge report is checked for
//! its presence and its column names only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::evaluate::{edge_margins, margins, nearest_margin};
use fakeflac_calibrate::Config;
use fakeflac_core::detect::Measurements;
use fakeflac_core::DetectionParams;
use std::path::PathBuf;
use std::sync::OnceLock;

fn v6_params() -> PathBuf {
    repo_root().join("specs/detection/detection-params.v6.toml")
}

fn config_v3() -> PathBuf {
    repo_root().join("specs/calibration/calibration.v3.toml")
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

// ---- calibration.v3.toml and the per-setting decode_args -------------------------------------------

const S24: [&str; 4] = ["cbr128_s24", "cbr192_s24", "cbr256_s24", "cbr320_s24"];

// generation.md section 2, "Per-setting decode": calibration.v3.toml loads, v2 and v1 still load.
#[test]
fn rf04_calibration_v3_loads_with_four_24_bit_settings() {
    let v3 = Config::parse(&read(&config_v3())).expect("calibration.v3.toml loads");
    let v2 = Config::parse(&read(&spec_config_v2())).expect("calibration.v2.toml loads");
    assert_eq!(v3.config_version, 3);
    assert_eq!(v3.generate.settings.len(), v2.generate.settings.len() + 4);
    // Every v2 setting is unchanged, in order, with no decode_args of its own.
    for (a, b) in v2.generate.settings.iter().zip(&v3.generate.settings) {
        assert_eq!(a, b);
        assert_eq!(b.decode_args, None, "{}", b.id);
    }
    assert_eq!(
        v3.generate.lossy, v2.generate.lossy,
        "lossy entries unchanged"
    );
    assert_eq!(v3.generate.decode_args, v2.generate.decode_args);
    assert!(v3.generate.settings.iter().all(|s| s.lossless_transcode));

    let wanted = strings(&[
        "-c:a",
        "flac",
        "-sample_fmt",
        "s32",
        "-bits_per_raw_sample",
        "24",
    ]);
    for id in S24 {
        let s24 = v3
            .generate
            .settings
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("{id} missing"));
        assert_eq!(s24.decode_args.as_deref(), Some(wanted.as_slice()), "{id}");
        // Same encoder arguments, ext, bitrate and low-pass as the setting it is named after.
        let base_id = id.trim_end_matches("_s24");
        let base = v3
            .generate
            .settings
            .iter()
            .find(|s| s.id == base_id)
            .unwrap();
        assert_eq!(
            (
                &s24.encoder,
                &s24.ext,
                s24.bitrate_kbps,
                s24.lowpass_hz,
                &s24.args
            ),
            (
                &base.encoder,
                &base.ext,
                base.bitrate_kbps,
                base.lowpass_hz,
                &base.args
            ),
            "{id} against {base_id}"
        );
    }
    // The 24-bit settings are not used by any lossy entry as a stage.
    for e in &v3.generate.lossy {
        assert!(
            e.stages.iter().all(|s| !S24.contains(&s.as_str())),
            "{}",
            e.id
        );
    }

    Config::parse(&read(&spec_config())).expect("calibration.v1.toml still loads");
}

// generation.md section 2: a setting without its own decode_args uses generate.decode_args.
#[test]
fn rf04_decode_args_per_setting() {
    let v3 = Config::parse(&read(&config_v3())).unwrap();
    let default = strings(&["-c:a", "flac", "-sample_fmt", "s16"]);
    let s24 = strings(&[
        "-c:a",
        "flac",
        "-sample_fmt",
        "s32",
        "-bits_per_raw_sample",
        "24",
    ]);
    for setting in &v3.generate.settings {
        let got = v3.generate.decode_args_for(setting);
        if S24.contains(&setting.id.as_str()) {
            assert_eq!(got, s24.as_slice(), "{}", setting.id);
        } else {
            assert_eq!(got, default.as_slice(), "{}", setting.id);
        }
    }
}

// ---- Margins and edge_step (evaluation.md section 8) -----------------------------------------------

fn measured(g: f64, d: f64, fs: f64, d_q: Option<f64>, r: Option<f64>) -> Measurements {
    Measurements {
        noise_floor_db: -80.0,
        high_frequency_db: -20.0,
        extent_hz: 22_050.0,
        step_db: d,
        step_hz: Some(fs),
        gap_db: g,
        upper_level_db: Some(-40.0),
        upper_burst_db: Some(8.0),
        median_step_db: d_q,
        top_floor_db: r.map(|_| -60.0),
        residual_db: r,
    }
}

fn params_file(path: &std::path::Path) -> DetectionParams {
    toml::from_str(&read(path)).unwrap()
}

#[test]
fn rf04_v6_edge_margins_and_edge_step() {
    let v6 = params_file(&v6_params());
    assert_eq!(v6.algorithm_version, 6);
    // W4-like: D_Q 0.32 (margin 3 - 0.32), R 17.48 (margin 2.48), D 14.22 (margin 16 - 14.22).
    let m = measured(-0.25, 14.22, 19_450.0, Some(0.32), Some(17.48));
    let e = edge_margins(&m, &v6).expect("the params have [edge]");
    let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
    assert!(close(e.m_edge_median_db, 2.68), "{e:?}");
    assert!(close(e.m_edge_residual_db, 2.48), "{e:?}");
    assert!(close(e.m_edge_step_db, 1.78), "{e:?}");
    assert!(e.edge_step);

    // Boundaries: median margin 0 is not an edge step (needs > 0); residual margin 0 is (needs >= 0);
    // step margin 0 is not (needs > 0).
    let at = |d_q: f64, r: f64, d: f64| {
        edge_margins(&measured(-0.25, d, 19_450.0, Some(d_q), Some(r)), &v6)
            .unwrap()
            .edge_step
    };
    assert!(!at(3.0, 17.48, 14.22));
    assert!(at(2.99, 17.48, 14.22));
    assert!(at(0.32, 15.0, 14.22));
    assert!(!at(0.32, 14.99, 14.22));
    assert!(!at(0.32, 17.48, 16.0));
    assert!(at(0.32, 17.48, 15.99));

    // Undefined D_Q or R: the margin is undefined and the step is not an edge step (an empty cell,
    // not `false`, is only for params without [edge]).
    let e = edge_margins(&measured(-0.25, 14.22, 19_450.0, None, None), &v6).unwrap();
    assert_eq!((e.m_edge_median_db, e.m_edge_residual_db), (None, None));
    assert!(!e.edge_step);

    // m_edge_step_db needs InRange(fs).
    let out = edge_margins(
        &measured(-0.25, 14.22, 20_300.0, Some(0.32), Some(17.48)),
        &v6,
    )
    .unwrap();
    assert_eq!(out.m_edge_step_db, None);
    assert!(!out.edge_step);

    // No [edge] section (v5 file): no edge margins, edge_step empty.
    assert_eq!(edge_margins(&m, &params_file(&v5_params())), None);
}

// "When edge_step is true, m_step_fake_db is left out of a (V4 cannot fire)".
#[test]
fn rf04_v6_nearest_fake_margin_leaves_out_the_step_margin_of_an_edge_step() {
    let v6 = params_file(&v6_params());
    let v5 = params_file(&v5_params());
    let edge = measured(-0.25, 14.22, 19_450.0, Some(0.32), Some(17.48));
    // a = min(m_gap_fake_db, m_step_fake_db) = min(6.25, -4.22) with v5; with v6 only the gap: 6.25.
    assert!((margins(&edge, &v5).nearest_fake_margin_db - (-4.22)).abs() < 1e-9);
    assert!((margins(&edge, &v6).nearest_fake_margin_db - 6.25).abs() < 1e-9);
    assert!((nearest_margin(&edge, &v6) - 6.25).abs() < 1e-9);
    // Not an edge step (D_Q at its limit): the step margin counts, as in v5.
    let not_edge = measured(-0.25, 14.22, 19_450.0, Some(3.0), Some(17.48));
    assert!((margins(&not_edge, &v6).nearest_fake_margin_db - (-4.22)).abs() < 1e-9);
}

// ---- tracks.csv and the report with v5 and v6 side by side -----------------------------------------

fn shared_v5_and_v6() -> &'static SharedRun {
    static CELL: OnceLock<SharedRun> = OnceLock::new();
    CELL.get_or_init(|| {
        let sample = Sample::new_shared("shared-v5-v6");
        let outcome = sample.accept(&[v5_params(), v6_params()]);
        SharedRun { sample, outcome }
    })
}

// evaluation.md section 8: D_Q, F_top, R and edge_step after full_band and before the margins; the
// margins m_edge_*; empty on the rows of a params file without [edge].
#[test]
fn rf04_v6_columns_in_tracks_csv() {
    let o = &shared_v5_and_v6().outcome;
    assert_ok(o);
    let t = tracks(o);
    let pos = |name: &str| {
        t.header
            .iter()
            .position(|h| h == name)
            .unwrap_or_else(|| panic!("tracks.csv lacks {name}"))
    };
    let full_band = pos("full_band");
    let first_margin = pos("m_low_hf_db");
    for (i, c) in ["D_Q", "F_top", "R", "edge_step"].iter().enumerate() {
        let at = pos(c);
        assert!(
            at > full_band && at < first_margin,
            "{c} between full_band and the margins"
        );
        assert_eq!(at, full_band + 1 + i, "{c} in order");
    }
    for c in ["m_edge_median_db", "m_edge_residual_db", "m_edge_step_db"] {
        pos(c);
    }
    let mut v6_rows = 0;
    for r in &t.rows {
        let file = t.get(r, "params_file");
        if file == "detection-params.v5.toml" {
            for c in [
                "D_Q",
                "F_top",
                "R",
                "edge_step",
                "m_edge_median_db",
                "m_edge_residual_db",
                "m_edge_step_db",
            ] {
                assert_eq!(t.get(r, c), "", "{c} is empty on a v5 row");
            }
        } else if file == "detection-params.v6.toml" && !t.get(r, "G").is_empty() {
            v6_rows += 1;
            // With spectra: edge_step is true or false, never empty.
            let edge = t.get(r, "edge_step");
            assert!(edge == "true" || edge == "false", "edge_step '{edge}'");
            if edge == "true" {
                // The conjunction of section 8: margins and fs in range.
                assert!(!t.get(r, "m_edge_median_db").is_empty());
                assert!(t.get(r, "m_edge_median_db").parse::<f64>().unwrap() > 0.0);
                assert!(t.get(r, "m_edge_residual_db").parse::<f64>().unwrap() >= 0.0);
                assert!(t.get(r, "m_edge_step_db").parse::<f64>().unwrap() > 0.0);
            }
        }
    }
    assert!(v6_rows > 0, "v6 rows with spectra");
}

// evaluation.md section 4: the edge report exists for a params file with [edge].
#[test]
fn rf04_v6_report_has_an_edge_report() {
    let o = &shared_v5_and_v6().outcome;
    assert_ok(o);
    let report = report_text(o, "report.md");
    let lower = report.to_lowercase();
    assert!(
        lower.contains("edge report"),
        "report.md has no edge report"
    );
    for column in ["V4-only", "EdgeStep"] {
        assert!(
            report.contains(column),
            "edge report lacks the column '{column}'"
        );
    }
}

// ---- Grid search (evaluation.md section 7) ---------------------------------------------------------

#[test]
fn rf04_an_edge_key_cannot_be_searched() {
    let sample = Sample::new("grid-edge-key");
    let text = format!(
        "grid_version = 2\nbaseline = '{}'\nmust_catch = [\"fx128\"]\nmax_points = 1000\n\n\
         [weights]\nfp = 10.0\nfn = 1.0\nsuspicious_negative = 0.5\nsuspicious_positive = 0.1\n\n\
         [values]\n\"edge.max_step_db\" = [14.0, 16.0]\n",
        v6_params().display()
    );
    let grid = sample.write_scratch("grid.toml", &text);
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr
            .contains("edge.max_step_db: edge parameters cannot be searched yet"),
        "{}",
        o.stderr
    );
}

// evaluation.md section 7 / rules.md section 4: an [edge] section that breaks a check is exit 2.
#[test]
fn rf04_an_invalid_edge_section_in_a_params_file_exits_2() {
    let sample = Sample::new("accept-invalid-edge");
    let text = read(&v6_params()).replace("max_step_db = 16.0", "max_step_db = 10.0");
    let bad = sample.write_scratch("detection-params.v6-bad.toml", &text);
    let o = sample.accept(&[v5_params(), bad]);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr.contains("edge.max_step_db > step_fake_db"),
        "{}",
        o.stderr
    );
}

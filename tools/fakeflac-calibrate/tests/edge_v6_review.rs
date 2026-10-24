//! Detection v6 (ADR-0042, @RF-04 / @RP-11), second test round after the code review:
//! the v6 near-threshold rule (evaluation.md section 8), the behaviour of the edge report
//! (section 4) and the per-setting `decode_args` in the cache recipe and in the decode call
//! (generation.md section 2). Hand-built measurements and the fixture transcoder: no ffmpeg, no
//! real audio.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::cache::Extracted;
use fakeflac_calibrate::evaluate::{
    edge_report, margin_summary, margins, EdgeReportRow, ParamsRun, RowEval,
};
use fakeflac_calibrate::model::{LabelRow, RowRoot, Truth};
use fakeflac_calibrate::report::SplitSel;
use fakeflac_calibrate::split::Split;
use fakeflac_calibrate::transcode::{
    LossyOutput, LossyRequest, TranscodeError, TranscodeOutput, TranscodeRequest, Transcoder,
};
use fakeflac_calibrate::{run, Command, Config};
use fakeflac_core::detect::Measurements;
use fakeflac_core::{DetectionParams, TrackError, Verdict};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn v6_params() -> PathBuf {
    repo_root().join("specs/detection/detection-params.v6.toml")
}

fn params_file(path: &std::path::Path) -> DetectionParams {
    toml::from_str(&read(path)).unwrap()
}

/// A lossless track with H, U and B as in the edge tests (not full band once `D >= 3`).
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

fn label_row(truth: Truth, setting: &str, origin: &'static str, split: Split) -> LabelRow {
    LabelRow {
        root: RowRoot::Corpus,
        path: "a/b.flac".to_string(),
        truth,
        origin,
        trust_origin: String::new(),
        class: if truth == Truth::Real {
            "primary"
        } else {
            "positive"
        }
        .to_string(),
        encoder: String::new(),
        setting: setting.to_string(),
        bitrate_kbps: None,
        encoder_version: String::new(),
        fragment: None,
        source_path: String::new(),
        album_id: "a".to_string(),
        split,
        watch_id: String::new(),
        extracted: Arc::new(Extracted::Error(TrackError::TooShort)),
        undetectable: false,
    }
}

fn eval_of(m: Measurements, params: &DetectionParams) -> RowEval {
    RowEval {
        verdict: Verdict::Suspicious,
        reasons: Vec::new(),
        error: None,
        margins: Some(margins(&m, params)),
        m: Some(m),
    }
}

fn run_of(params: DetectionParams, evals: Vec<RowEval>) -> ParamsRun {
    ParamsRun {
        name: "p.toml".to_string(),
        params,
        hash: "0".repeat(16),
        fingerprint_id: "f".to_string(),
        group: 0,
        evals,
        lossy_evals: Vec::new(),
    }
}

// ---- B1: the v6 near-threshold rule (evaluation.md section 8) ---------------------------------

/// Near negatives of the class `primary` for one negative with these measurements.
fn near_negatives(params: &DetectionParams, m: Measurements) -> u32 {
    let config = Config::parse(&read(&spec_config())).unwrap();
    let rows = vec![label_row(Truth::Real, "", "trusted", Split::Tune)];
    let run = run_of(params.clone(), vec![eval_of(m, params)]);
    let summary = margin_summary(&run, &rows, &config);
    summary
        .iter()
        .find(|s| s.class == "primary")
        .expect("primary class")
        .near_negatives
}

// Spec: "(v6) For a params file with an [edge] section, also a negative kept from Fake only by
// EdgeStep near one of its thresholds: m_step_fake_db <= 0, edge_step is true and the smallest of
// m_edge_median_db, m_edge_residual_db and m_edge_step_db is at most report.near_db."
#[test]
fn rf04_v6_a_negative_kept_from_fake_only_by_the_edge_step_near_a_threshold_is_near() {
    let v6 = params_file(&v6_params());
    // D 12, fs 19450, G 0, D_Q 2.5 (margin 0.5), R 17 (margin 2): EdgeStep holds, nearest_fake_margin
    // is the gap's 6, the step is 850 Hz from the range edge: only the new rule can list it.
    let example = measured(0.0, 12.0, 19_450.0, Some(2.5), Some(17.0));
    let g = margins(&example, &v6);
    assert_eq!(g.nearest_fake_margin_db, 6.0);
    assert!(g.m_step_fake_db.is_some_and(|s| s <= 0.0));
    assert!(g.fs_fake_range_hz.is_some_and(|r| r.abs() > 300.0));
    assert_eq!(near_negatives(&v6, example), 1, "median margin 0.5 dB");

    // Each of the three margins can be the near one; the bound is inclusive (1.0 dB).
    let by_residual = measured(0.0, 12.0, 19_450.0, Some(0.5), Some(15.5));
    assert_eq!(
        near_negatives(&v6, by_residual),
        1,
        "residual margin 0.5 dB"
    );
    let by_step = measured(0.0, 15.5, 19_450.0, Some(0.5), Some(19.0));
    assert_eq!(near_negatives(&v6, by_step), 1, "step margin 0.5 dB");
    let at_bound = measured(0.0, 12.0, 19_450.0, Some(2.0), Some(19.0));
    assert_eq!(
        near_negatives(&v6, at_bound),
        1,
        "median margin exactly near_db"
    );
}

// Negative controls of the same rule.
#[test]
fn rf04_v6_an_edge_step_far_from_its_thresholds_is_not_near() {
    let v6 = params_file(&v6_params());
    // Same track, every edge margin above near_db (D_Q 0.5 -> 2.5, R 19 -> 4, D 12 -> 4).
    let far = measured(0.0, 12.0, 19_450.0, Some(0.5), Some(19.0));
    assert_eq!(near_negatives(&v6, far), 0);
    // Just above the bound.
    let above = measured(0.0, 12.0, 19_450.0, Some(1.9), Some(19.0));
    assert_eq!(near_negatives(&v6, above), 0, "median margin 1.1 dB");
    // Not an edge step (D_Q at its limit): the track is not kept by EdgeStep.
    let not_edge = measured(0.0, 12.0, 19_450.0, Some(3.0), Some(17.0));
    assert_eq!(near_negatives(&v6, not_edge), 0);
    // The same measurements under a params file without [edge] (v5): no new rule.
    let v5 = params_file(&v5_params());
    let example = measured(0.0, 12.0, 19_450.0, Some(2.5), Some(17.0));
    assert_eq!(near_negatives(&v5, example), 0);
}

// Spec: when edge_step is true, m_step_fake_db is left out of `a`; so the guard case
// ("a <= 0, full_band true, g = 0") must not read it either. A params file with [full_band] and
// [edge] but without max_lossy_step_db keeps the guard alive for an in-range step.
#[test]
fn rf04_v6_the_guard_test_leaves_out_the_step_margin_of_an_edge_step() {
    let text = read(&v6_params())
        .lines()
        .filter(|l| !l.trim_start().starts_with("max_lossy_step_db"))
        .collect::<Vec<_>>()
        .join("\n");
    let params: DetectionParams = toml::from_str(&text).unwrap();
    assert!(params.full_band.is_some() && params.edge.is_some());
    // U at the level threshold (guard margin exactly 0), edge margins all above near_db, G margin 3.
    let mut m = measured(3.0, 12.0, 19_450.0, Some(0.5), Some(19.0));
    m.upper_level_db = Some(-70.0);
    m.upper_burst_db = Some(3.5);
    let g = margins(&m, &params);
    assert_eq!(g.full_band_margin_db, Some(0.0));
    assert!(g.m_step_fake_db.is_some_and(|s| s <= 0.0));
    // a = m_gap_fake_db = 3 > 0 (the step margin is left out): the guard is not what keeps it from Fake.
    assert_eq!(g.nearest_fake_margin_db, 3.0);
    assert_eq!(near_negatives(&params, m), 0);
}

// ---- I2: the edge report (evaluation.md section 4) --------------------------------------------

fn find<'a>(report: &'a [EdgeReportRow], split: SplitSel, setting: &str) -> &'a EdgeReportRow {
    report
        .iter()
        .find(|r| r.split == split && r.setting == setting)
        .unwrap_or_else(|| panic!("no edge report row for {setting}"))
}

#[test]
fn rf04_v6_edge_report_counts_the_v4_only_positives_and_the_near_conjunction_groups() {
    let v6 = params_file(&v6_params());
    let config = Config::parse(&read(&spec_config())).unwrap();
    let ids: Vec<String> = config.lossless_settings().map(|s| s.id.clone()).collect();
    let (s0, s1) = (ids[0].as_str(), ids[1].as_str());
    let m = measured;

    let mut items: Vec<(LabelRow, Measurements)> = Vec::new();
    let tune = |setting: &str| label_row(Truth::Fake, setting, "generated", Split::Tune);
    // V4-only and EdgeStep: D_Q 0.5, R 17, D 12.
    items.push((tune(s0), m(0.0, 12.0, 19_450.0, Some(0.5), Some(17.0))));
    // V4-only, R 13.5 (below min_residual_db - near_db = 14).
    items.push((tune(s0), m(0.0, 12.0, 19_450.0, Some(0.5), Some(13.5))));
    // V4-only, R 14.5 (close: >= 14, not an EdgeStep).
    items.push((tune(s0), m(0.0, 12.0, 19_450.0, Some(0.5), Some(14.5))));
    // V4-only, D_Q 5, R 20: only the D_Q group (R >= 15, D < 16).
    items.push((tune(s0), m(0.0, 12.0, 19_450.0, Some(5.0), Some(20.0))));
    // V4-only, D_Q and R undefined: in neither group.
    items.push((tune(s0), m(0.0, 12.0, 19_450.0, None, None)));
    // V4-only, D 17 (>= max_step_db): in neither group.
    items.push((tune(s0), m(0.0, 17.0, 19_450.0, Some(0.5), Some(20.0))));
    // Not V4-only: G >= gap_fake_db (V3 decides); D < step_fake_db; H below the threshold; fs out of range.
    items.push((tune(s0), m(7.0, 12.0, 19_450.0, Some(0.5), Some(17.0))));
    items.push((tune(s0), m(0.0, 9.0, 19_450.0, Some(0.5), Some(17.0))));
    let mut low_hf = m(0.0, 12.0, 19_450.0, Some(0.5), Some(17.0));
    low_hf.high_frequency_db = -70.0;
    items.push((tune(s0), low_hf));
    items.push((tune(s0), m(0.0, 12.0, 20_300.0, Some(0.5), Some(17.0))));
    // Undetectable positive and a negative: not in any count.
    let mut undetectable = tune(s0);
    undetectable.undetectable = true;
    items.push((undetectable, m(0.0, 12.0, 19_450.0, Some(0.5), Some(17.0))));
    items.push((
        label_row(Truth::Real, s0, "trusted", Split::Tune),
        m(0.0, 12.0, 19_450.0, Some(0.5), Some(17.0)),
    ));
    // Holdout, EdgeStep, R 16: alone in its group (p95 = max = 16).
    items.push((
        label_row(Truth::Fake, s0, "generated", Split::Holdout),
        m(0.0, 12.0, 19_450.0, Some(0.5), Some(16.0)),
    ));
    // The other setting: one V4-only row with undefined D_Q and R.
    items.push((tune(s1), m(0.0, 12.0, 19_450.0, None, None)));
    // A confirmed fake (reported under confirmed_fake, whatever its setting field).
    items.push((
        label_row(Truth::Fake, "", "confirmed_fake", Split::Tune),
        m(0.0, 12.0, 19_450.0, Some(1.5), Some(30.0)),
    ));

    let rows: Vec<LabelRow> = items.iter().map(|(r, _)| r.clone()).collect();
    let evals: Vec<RowEval> = items.into_iter().map(|(_, m)| eval_of(m, &v6)).collect();
    let run = run_of(v6, evals);
    let report = edge_report(&run, &rows, &config);

    // One row per split and setting, then confirmed_fake.
    assert_eq!(report.len(), 3 * (ids.len() + 1));
    for split in [SplitSel::All, SplitSel::Tune, SplitSel::Holdout] {
        let of_split: Vec<&str> = report
            .iter()
            .filter(|r| r.split == split)
            .map(|r| r.setting.as_str())
            .collect();
        let mut expected: Vec<&str> = ids.iter().map(String::as_str).collect();
        expected.push("confirmed_fake");
        assert_eq!(of_split, expected, "order of the rows");
    }
    let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);

    // Tune, first setting: V4-only rows 1-6; EdgeStep: row 1.
    let t = find(&report, SplitSel::Tune, s0);
    assert_eq!((t.v4_only, t.edge_step), (6, 1), "{t:?}");
    // D_Q < 3 and D < 16 with R defined: rows 1, 2, 3 (R 17, 13.5, 14.5); close (R >= 14): rows 1, 3.
    assert_eq!((t.near_r_count, t.near_r_close), (3, 2), "{t:?}");
    assert!(near(t.near_r_max, 17.0), "{t:?}");
    let p95 = t.near_r_p95.expect("a p95");
    assert!((14.5..=17.0).contains(&p95), "{t:?}");
    // R >= 15 and D < 16: rows 1 (D_Q 0.5) and 4 (D_Q 5).
    assert_eq!(t.near_dq_count, 2, "{t:?}");
    assert!(near(t.near_dq_min, 0.5), "{t:?}");

    // All: adds the holdout row.
    let a = find(&report, SplitSel::All, s0);
    assert_eq!((a.v4_only, a.edge_step), (7, 2), "{a:?}");
    assert_eq!((a.near_r_count, a.near_r_close), (4, 3), "{a:?}");
    assert!(near(a.near_r_max, 17.0), "{a:?}");
    assert_eq!(a.near_dq_count, 3, "{a:?}");
    assert!(near(a.near_dq_min, 0.5), "{a:?}");

    // Holdout: the single row.
    let h = find(&report, SplitSel::Holdout, s0);
    assert_eq!((h.v4_only, h.edge_step), (1, 1), "{h:?}");
    assert_eq!((h.near_r_count, h.near_r_close), (1, 1), "{h:?}");
    assert!(
        near(h.near_r_p95, 16.0) && near(h.near_r_max, 16.0),
        "{h:?}"
    );
    assert_eq!(h.near_dq_count, 1, "{h:?}");
    assert!(near(h.near_dq_min, 0.5), "{h:?}");

    // A setting whose only V4-only row has undefined D_Q and R: counted, in no group, empty cells.
    let u = find(&report, SplitSel::Tune, s1);
    assert_eq!((u.v4_only, u.edge_step), (1, 0), "{u:?}");
    assert_eq!((u.near_r_count, u.near_r_close, u.near_dq_count), (0, 0, 0));
    assert_eq!(
        (u.near_r_p95, u.near_r_max, u.near_dq_min),
        (None, None, None)
    );

    // A setting with no rows at all: zeros and empty cells.
    let empty = find(&report, SplitSel::All, ids[2].as_str());
    assert_eq!(
        (empty.v4_only, empty.edge_step, empty.near_r_count),
        (0, 0, 0)
    );
    assert_eq!(
        (empty.near_r_p95, empty.near_r_max, empty.near_dq_min),
        (None, None, None)
    );

    // The confirmed_fake row: D_Q 1.5, R 30, D 12 -> EdgeStep, in both groups; absent from the others.
    let c = find(&report, SplitSel::Tune, "confirmed_fake");
    assert_eq!((c.v4_only, c.edge_step), (1, 1), "{c:?}");
    assert_eq!(
        (c.near_r_count, c.near_r_close, c.near_dq_count),
        (1, 1, 1),
        "{c:?}"
    );
    assert!(
        near(c.near_r_max, 30.0) && near(c.near_dq_min, 1.5),
        "{c:?}"
    );
    assert_eq!(
        find(&report, SplitSel::Holdout, "confirmed_fake").v4_only,
        0
    );
}

// Without an [edge] section the report is empty.
#[test]
fn rf04_v6_no_edge_report_without_an_edge_section() {
    let v5 = params_file(&v5_params());
    let config = Config::parse(&read(&spec_config())).unwrap();
    let run = run_of(v5, Vec::new());
    assert!(edge_report(&run, &[], &config).is_empty());
}

// ---- I3: per-setting decode_args reach the cache recipe and the decode call -------------------

/// The fixture transcoder that records the decode arguments of every call.
struct Recording {
    calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl Transcoder for Recording {
    fn supports(&self, encoder: &str) -> bool {
        FixtureTranscoder.supports(encoder)
    }
    fn version(&self) -> Result<String, fakeflac_calibrate::error::ToolError> {
        FixtureTranscoder.version()
    }
    fn transcode(&self, request: &TranscodeRequest<'_>) -> Result<TranscodeOutput, TranscodeError> {
        self.calls
            .lock()
            .unwrap()
            .push((request.setting.id.clone(), request.decode_args.to_vec()));
        FixtureTranscoder.transcode(request)
    }
    fn transcode_lossy(&self, request: &LossyRequest<'_>) -> Result<LossyOutput, TranscodeError> {
        FixtureTranscoder.transcode_lossy(request)
    }
}

type Calls = Vec<(String, Vec<String>)>;

fn accept_recording(sample: &Sample) -> (fakeflac_calibrate::run::Outcome, Calls) {
    let t = Recording {
        calls: Mutex::new(Vec::new()),
    };
    let outcome = run(
        &Command::Accept(sample.accept_args(&[v2_params()])),
        &sample.env(),
        &t,
    );
    let calls = t.calls.into_inner().unwrap();
    (outcome, calls)
}

// generation.md section 2: a setting with its own decode_args is decoded with them and its cache
// entries are keyed by them; the other settings keep generate.decode_args and their cache.
#[test]
fn rf04_a_setting_with_its_own_decode_args_gets_its_own_recipe_and_decode_call() {
    let sample = Sample::new("decode-args-recipe");
    let global = vec!["-c:a", "flac", "-sample_fmt", "s16"];
    let own = vec![
        "-c:a",
        "flac",
        "-sample_fmt",
        "s32",
        "-bits_per_raw_sample",
        "24",
    ];

    // Cold run: every setting uses the global decode arguments.
    let (o, first) = accept_recording(&sample);
    assert_ok(&o);
    assert!(!first.is_empty());
    for (_, args) in &first {
        assert_eq!(args, &global);
    }
    let first_fx128 = first.iter().filter(|(id, _)| id == "fx128").count();
    assert!(first_fx128 > 0);

    // fx128 now has its own decode_args: its recipes change (cache miss, transcoded again with the
    // new arguments); the other settings are served from the cache.
    let quoted: Vec<String> = own.iter().map(|a| format!("\"{a}\"")).collect();
    let before = "args = [\"lp16k_dither\"]";
    let text = read(&sample.config_path());
    assert!(text.contains(before));
    let text = text.replace(
        before,
        &format!("{before}\ndecode_args = [{}]", quoted.join(", ")),
    );
    std::fs::write(sample.config_path(), text).unwrap();
    let (o, second) = accept_recording(&sample);
    assert_ok(&o);
    assert_eq!(
        second.len(),
        first_fx128,
        "only fx128 is transcoded again: {second:?}"
    );
    for (id, args) in &second {
        assert_eq!(id, "fx128");
        assert_eq!(
            args, &own,
            "the decode call carries the setting's own arguments"
        );
    }

    // Third run, same config: fully cached (the new recipe was stored).
    let (o, third) = accept_recording(&sample);
    assert_ok(&o);
    assert!(third.is_empty(), "{third:?}");
}

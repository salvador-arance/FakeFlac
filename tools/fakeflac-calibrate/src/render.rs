//! Rendering of the outputs: `labels.csv`, `tracks.csv`, `summary.md` and `report.md`
//! (evaluation.md section 1, calibration.md section 6).
//!
//! `summary.md` holds aggregate figures only: no album id, folder or file name, note or path.
//! `report.md` adds the per-track lists, with paths relative to the roots, never absolute.

use crate::cache::{bitrate_mode_name, codec_name, error_code, Extracted};
use crate::config::Config;
use crate::env::Roots;
use crate::evaluate::{
    changed_rows, class_totals, clean_fakes, figure, is_near, margin_summary, reason_name,
    small_sample_warning, verdict_name, ParamsRun, RowEval, Totals, SPLITS,
};
use crate::extract::GroupResult;
use crate::fmt;
use crate::grid::{GridFile, Proposal, SearchResult};
use crate::lossy::LossyRowEval;
use crate::model::RowMeta;
use crate::model::{album_class_of, LabelRow, Truth, CLASSES};
use crate::report::{ConfusionMatrix, Evaluation, SettingRow, SplitSel};
use fakeflac_core::detect::{is_full_band, Measurements};
use fakeflac_core::Verdict;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------------------------
// CSV (RFC 4180: CRLF, fields quoted when they hold `,`, `"` or a line break)
// ---------------------------------------------------------------------------------------------

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn csv_row(fields: &[String]) -> String {
    let mut line = fields
        .iter()
        .map(|f| csv_field(f))
        .collect::<Vec<_>>()
        .join(",");
    line.push_str("\r\n");
    line
}

pub const LABELS_HEADER: [&str; 29] = [
    "path",
    "root",
    "truth",
    "origin",
    "trust_origin",
    "class",
    "encoder",
    "setting",
    "bitrate_kbps",
    "encoder_version",
    "sample_rate_hz",
    "fragment_start_s",
    "fragment_s",
    "source_path",
    "album_id",
    "split",
    "watch_id",
    "domain",
    "codec",
    "declared_bitrate_kbps",
    "bitrate_mode",
    "stages",
    "source_bitrate_kbps",
    "upstream",
    "label_strength",
    "chain_id",
    "chain_key",
    "split_unit",
    "not_evaluated",
];

/// `labels.csv` (inputs.md section 3, lossy.md section 2): the rows of both domains, in the sort
/// order of the rows (`album_id`, root, path, setting).
pub fn labels_csv(group: &GroupResult) -> String {
    let mut out = csv_row(&LABELS_HEADER.map(str::to_string));
    let mut rows: Vec<(&LabelRow, bool)> = group
        .rows
        .iter()
        .map(|r| (r, false))
        .chain(group.lossy_rows.iter().map(|r| (r, true)))
        .collect();
    rows.sort_by(|(a, _), (b, _)| {
        (&a.album_id, a.root, &a.path, &a.setting).cmp(&(&b.album_id, b.root, &b.path, &b.setting))
    });
    let default_meta = RowMeta::default();
    for (r, lossy) in rows {
        let meta = group.meta.get(&r.identity()).unwrap_or(&default_meta);
        let stream = match r.extracted.as_ref() {
            Extracted::Probe(p) => Some(&p.stream),
            Extracted::Error(_) => None,
        };
        out.push_str(&csv_row(&[
            r.path.clone(),
            r.root.as_str().to_string(),
            r.truth.as_str().to_string(),
            r.origin.to_string(),
            r.trust_origin.clone(),
            r.class.clone(),
            r.encoder.clone(),
            r.setting.clone(),
            r.bitrate_kbps.map(|b| b.to_string()).unwrap_or_default(),
            r.encoder_version.clone(),
            r.sample_rate().map(|s| s.to_string()).unwrap_or_default(),
            r.fragment
                .map(|f| fmt::secs3(f.start_s))
                .unwrap_or_default(),
            r.fragment
                .map(|f| fmt::secs3(f.duration_s))
                .unwrap_or_default(),
            r.source_path.clone(),
            r.album_id.clone(),
            r.split.as_str().to_string(),
            r.watch_id.clone(),
            if lossy { "lossy" } else { "lossless" }.to_string(),
            stream
                .and_then(|s| s.codec)
                .map(|c| codec_name(c).to_string())
                .unwrap_or_default(),
            stream
                .filter(|_| lossy)
                .and_then(|s| s.declared_bitrate_kbps)
                .map(|b| b.to_string())
                .unwrap_or_default(),
            stream
                .filter(|_| lossy)
                .and_then(|s| s.bitrate_mode)
                .map(|m| bitrate_mode_name(m).to_string())
                .unwrap_or_default(),
            meta.stages.clone(),
            meta.source_bitrate_kbps
                .map(|b| b.to_string())
                .unwrap_or_default(),
            meta.upstream.clone(),
            if meta.weak { "weak" } else { "" }.to_string(),
            meta.chain_id.clone(),
            meta.chain_key.clone(),
            meta.split_unit.clone(),
            meta.not_evaluated.clone(),
        ]));
    }
    out
}

pub const TRACKS_HEADER: [&str; 49] = [
    "params_file",
    "algorithm_version",
    "path",
    "root",
    "truth",
    "class",
    "trust_origin",
    "setting",
    "album_id",
    "split",
    "watch_id",
    "verdict",
    "reasons",
    "error",
    "H",
    "F",
    "fe",
    "D",
    "fs",
    "G",
    "U",
    "B",
    "full_band",
    "m_low_hf_db",
    "m_gap_fake_db",
    "m_gap_suspicious_db",
    "m_step_fake_db",
    "m_step_suspicious_db",
    "m_step_low_db",
    "m_step_high_db",
    "m_gradual_hz",
    "fs_fake_range_hz",
    "m_full_band_level_db",
    "m_full_band_burst_db",
    "full_band_margin_db",
    "m_full_band_step_db",
    "nearest_fake_margin_db",
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

/// `true`, `false`, or empty when the file has no `[full_band]` section or `U` or `B` is undefined.
fn full_band_cell(run: &ParamsRun, m: Option<&Measurements>) -> String {
    match m {
        Some(m) if run.params.full_band.is_some() && m.upper_level_db.is_some() => {
            is_full_band(m, &run.params).to_string()
        }
        _ => String::new(),
    }
}

/// `tracks.csv`: one row per (params file, labelled track).
pub fn tracks_csv(runs: &[ParamsRun], groups: &[GroupResult]) -> String {
    let mut out = csv_row(&TRACKS_HEADER.map(str::to_string));
    for run in runs {
        for (r, e) in groups[run.group].rows.iter().zip(run.evals.iter()) {
            let m = e.m.as_ref();
            let g = e.margins.as_ref();
            out.push_str(&csv_row(&[
                run.name.clone(),
                run.params.algorithm_version.to_string(),
                r.path.clone(),
                r.root.as_str().to_string(),
                r.truth.as_str().to_string(),
                r.class.clone(),
                r.trust_origin.clone(),
                r.setting.clone(),
                r.album_id.clone(),
                r.split.as_str().to_string(),
                r.watch_id.clone(),
                verdict_name(e.verdict).to_string(),
                e.reasons
                    .iter()
                    .map(|x| reason_name(*x))
                    .collect::<Vec<_>>()
                    .join(";"),
                e.error
                    .as_ref()
                    .map(|x| error_code(x).to_string())
                    .unwrap_or_default(),
                fmt::opt_db(m.map(|m| m.high_frequency_db)),
                fmt::opt_db(m.map(|m| m.noise_floor_db)),
                fmt::opt_hz(m.map(|m| m.extent_hz)),
                fmt::opt_db(m.map(|m| m.step_db)),
                fmt::opt_hz(m.and_then(|m| m.step_hz)),
                fmt::opt_db(m.map(|m| m.gap_db)),
                fmt::opt_db(m.and_then(|m| m.upper_level_db)),
                fmt::opt_db(m.and_then(|m| m.upper_burst_db)),
                full_band_cell(run, m),
                fmt::opt_db(g.map(|g| g.m_low_hf_db)),
                fmt::opt_db(g.map(|g| g.m_gap_fake_db)),
                fmt::opt_db(g.map(|g| g.m_gap_suspicious_db)),
                fmt::opt_db(g.and_then(|g| g.m_step_fake_db)),
                fmt::opt_db(g.and_then(|g| g.m_step_suspicious_db)),
                fmt::opt_db(g.and_then(|g| g.m_step_low_db)),
                fmt::opt_db(g.and_then(|g| g.m_step_high_db)),
                fmt::opt_hz(g.and_then(|g| g.m_gradual_hz)),
                fmt::opt_hz(g.and_then(|g| g.fs_fake_range_hz)),
                fmt::opt_db(g.and_then(|g| g.m_full_band_level_db)),
                fmt::opt_db(g.and_then(|g| g.m_full_band_burst_db)),
                fmt::opt_db(g.and_then(|g| g.full_band_margin_db)),
                fmt::opt_db(g.and_then(|g| g.m_full_band_step_db)),
                fmt::opt_db(g.map(|g| g.nearest_fake_margin_db)),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]));
        }
        let group = &groups[run.group];
        for (r, e) in group.lossy_rows.iter().zip(run.lossy_evals.iter()) {
            let honest = group
                .meta
                .get(&r.identity())
                .is_some_and(|m| m.honest_generated);
            out.push_str(&csv_row(&lossy_track_cells(run, r, e, honest)));
        }
    }
    out
}

/// One row of `tracks.csv` for a lossy track (lossy.md section 6.4): the measurements fill the
/// columns of evaluation.md section 8 and its margins stay empty.
fn lossy_track_cells(run: &ParamsRun, r: &LabelRow, e: &LossyRowEval, honest: bool) -> Vec<String> {
    let m = e.m.as_ref();
    let lossy = run.params.lossy.as_ref().filter(|_| e.evaluated);
    let stream = match r.extracted.as_ref() {
        Extracted::Probe(p) => Some(&p.stream),
        Extracted::Error(_) => None,
    };
    let mut cells = vec![
        run.name.clone(),
        run.params.algorithm_version.to_string(),
        r.path.clone(),
        r.root.as_str().to_string(),
        r.truth.as_str().to_string(),
        r.class.clone(),
        r.trust_origin.clone(),
        r.setting.clone(),
        r.album_id.clone(),
        r.split.as_str().to_string(),
        r.watch_id.clone(),
        if e.evaluated {
            verdict_name(e.verdict).to_string()
        } else {
            String::new()
        },
        e.reasons
            .iter()
            .map(|x| reason_name(*x))
            .collect::<Vec<_>>()
            .join(";"),
        e.error
            .as_ref()
            .map(|x| error_code(x).to_string())
            .unwrap_or_default(),
        fmt::opt_db(m.map(|m| m.high_frequency_db)),
        fmt::opt_db(m.map(|m| m.noise_floor_db)),
        fmt::opt_hz(m.map(|m| m.extent_hz)),
        fmt::opt_db(m.map(|m| m.step_db)),
        fmt::opt_hz(m.and_then(|m| m.step_hz)),
        fmt::opt_db(m.map(|m| m.gap_db)),
        fmt::opt_db(m.and_then(|m| m.upper_level_db)),
        fmt::opt_db(m.and_then(|m| m.upper_burst_db)),
    ];
    // `full_band` and the 14 lossless margin columns.
    cells.extend(std::iter::repeat_n(String::new(), 15));
    cells.extend([
        stream
            .and_then(|s| s.codec)
            .map(|c| codec_name(c).to_string())
            .unwrap_or_default(),
        stream
            .and_then(|s| s.declared_bitrate_kbps)
            .map(|b| b.to_string())
            .unwrap_or_default(),
        stream
            .and_then(|s| s.bitrate_mode)
            .map(|b| bitrate_mode_name(b).to_string())
            .unwrap_or_default(),
        stream
            .and_then(|s| s.encoder_lowpass_hz)
            .map(|b| b.to_string())
            .unwrap_or_default(),
        fmt::opt_hz2(e.fc_hz),
        fmt::opt_hz2(e.c_exp_hz),
        fmt::opt_hz2(e.shortfall_hz),
        fmt::opt_hz2(
            lossy
                .zip(e.shortfall_hz)
                .map(|(l, s)| l.suspicious_shortfall_hz - s),
        ),
        fmt::opt_hz2(
            lossy
                .zip(e.shortfall_hz)
                .map(|(l, s)| l.fake_shortfall_hz - s),
        ),
        fmt::opt_db(lossy.zip(m).map(|(l, m)| m.step_db - l.fake_min_step_db)),
        e.estimate_kbps.map(|k| k.to_string()).unwrap_or_default(),
        if honest && e.evaluated {
            e.source_limited.to_string()
        } else {
            String::new()
        },
    ]);
    cells
}

// ---------------------------------------------------------------------------------------------
// Markdown
// ---------------------------------------------------------------------------------------------

/// The grid search part of a `calibrate` report.
pub struct GridReport<'a> {
    pub file: &'a GridFile,
    pub result: &'a SearchResult,
    pub proposal: Option<&'a Proposal>,
    /// Confirmed fakes checked by feasibility constraint (2), and how many of them are in holdout.
    pub confirmed_checked: u32,
    pub confirmed_holdout: u32,
}

/// Everything the Markdown reports are made of.
pub struct ReportData<'a> {
    pub run_id: &'a str,
    pub date: &'a str,
    pub commit: &'a str,
    pub mode: &'a str,
    pub config: &'a Config,
    pub roots: &'a Roots,
    pub groups: &'a [GroupResult],
    pub runs: &'a [ParamsRun],
    pub evaluations: &'a [Evaluation],
    /// Split units per stratum with their (tune, holdout) counts (lossy.md section 3).
    pub unit_strata: &'a BTreeMap<String, (u32, u32)>,
    pub grid: Option<GridReport<'a>>,
}

#[derive(Default)]
pub(crate) struct Md(pub(crate) String);

impl Md {
    pub(crate) fn line(&mut self, s: impl AsRef<str>) {
        self.0.push_str(s.as_ref());
        self.0.push('\n');
    }

    pub(crate) fn blank(&mut self) {
        self.0.push('\n');
    }

    pub(crate) fn table(&mut self, header: &[&str], rows: &[Vec<String>]) {
        let cell = |s: &str| s.replace('|', "\\|");
        self.line(format!(
            "| {} |",
            header
                .iter()
                .map(|h| cell(h))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        self.line(format!("|{}|", vec!["---"; header.len()].join("|")));
        for r in rows {
            self.line(format!(
                "| {} |",
                r.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | ")
            ));
        }
        self.blank();
    }
}

/// Replaces the absolute paths of both roots in a message coming from an external tool.
fn scrub(text: &str, roots: &Roots) -> String {
    let mut out = text.to_string();
    for (root, label) in [
        (&roots.corpus, "<corpus>"),
        (&roots.calibration, "<calibration>"),
    ] {
        let plain = crate::fsutil::plain_path(root)
            .to_string_lossy()
            .into_owned();
        let verbatim = root.to_string_lossy().into_owned();
        for variant in [verbatim.clone(), plain.clone()] {
            for v in [
                variant.clone(),
                variant.replace('\\', "/"),
                variant.replace('/', "\\"),
            ] {
                if !v.is_empty() {
                    out = out.replace(&v, label);
                }
            }
        }
    }
    out
}

fn setting_rows_table(md: &mut Md, rows: &[SettingRow]) {
    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            let c = &r.counts;
            let n = c.total() - c.error - c.undetectable;
            vec![
                r.setting.clone(),
                c.real.to_string(),
                c.suspicious.to_string(),
                c.fake.to_string(),
                c.undetectable.to_string(),
                c.error.to_string(),
                c.total().to_string(),
                fmt::rate(c.fake, n),
                fmt::rate(c.real, n),
            ]
        })
        .collect();
    md.table(
        &[
            "setting",
            "Real",
            "Suspicious",
            "Fake",
            "Undetectable",
            "Error",
            "total",
            "detection rate",
            "FN rate",
        ],
        &table,
    );
}

/// Per setting: detectable positives that are `FullBand`, and the distribution of `B`
/// (evaluation.md section 4). Only for a params file with a `[full_band]` section.
fn full_band_table(md: &mut Md, eval: &Evaluation) {
    if eval.full_band_by_setting.is_empty() {
        return;
    }
    md.line("Full-band guard per setting (detectable positives; `B` over those with `B` defined):");
    md.blank();
    let table: Vec<Vec<String>> = eval
        .full_band_by_setting
        .iter()
        .map(|r| {
            vec![
                r.setting.clone(),
                r.full_band.to_string(),
                fmt::opt_db(r.burst_median),
                fmt::opt_db(r.burst_p95),
                fmt::opt_db(r.burst_max),
            ]
        })
        .collect();
    md.table(
        &["setting", "full_band", "B median", "B p95", "B max"],
        &table,
    );
}

/// Verdict changes of the positives against the baseline, one row per setting (section 9).
fn setting_changes_table(md: &mut Md, eval: &Evaluation) {
    if eval.setting_changes.is_empty() {
        return;
    }
    md.line("Changes on positives per setting:");
    md.blank();
    let table: Vec<Vec<String>> = eval
        .setting_changes
        .iter()
        .map(|c| {
            let changes = if c.counts.is_empty() {
                "none".to_string()
            } else {
                c.counts
                    .iter()
                    .map(|((from, to), n)| format!("{from} -> {to}: {n}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            vec![
                c.setting.clone(),
                changes,
                c.detection_rate_change_pp
                    .map(|x| format!("{} pp", fmt::pct_value(x / 100.0)))
                    .unwrap_or_else(|| "n/a".to_string()),
            ]
        })
        .collect();
    md.table(&["setting", "changes", "detection rate change"], &table);
}

fn matrix_table(md: &mut Md, m: &ConfusionMatrix) {
    let r = &m.real_row;
    let real = vec![
        "real".to_string(),
        r.real.to_string(),
        r.suspicious.to_string(),
        r.fake.to_string(),
        "—".to_string(),
        r.error.to_string(),
        r.total().to_string(),
    ];
    let f = &m.fake_row;
    let fake = vec![
        "fake".to_string(),
        f.real.to_string(),
        f.suspicious.to_string(),
        f.fake.to_string(),
        f.undetectable.to_string(),
        f.error.to_string(),
        f.total().to_string(),
    ];
    md.table(
        &[
            "truth \\ verdict",
            "Real",
            "Suspicious",
            "Fake",
            "Undetectable",
            "Error",
            "total",
        ],
        &[real, fake],
    );
    let rn = r.total() - r.error;
    let fnn = f.total() - f.error - f.undetectable;
    md.line(format!(
        "FP rate {} · soft-FP rate {} · FN rate {} · detection rate {} · caught rate {}",
        fmt::rate(r.fake, rn),
        fmt::rate(r.suspicious, rn),
        fmt::rate(f.real, fnn),
        fmt::rate(f.fake, fnn),
        fmt::rate(f.fake + f.suspicious, fnn),
    ));
    md.blank();
}

fn fp_cells(label: &str, class: &str, t: &Totals, z: f64) -> Vec<String> {
    let ci = match fmt::wilson(t.fake, t.tracks, z) {
        Some((lo, hi)) => format!("{}–{} %", fmt::pct_value(lo), fmt::pct_value(hi)),
        None => "n/a".to_string(),
    };
    vec![
        label.to_string(),
        class.to_string(),
        t.tracks.to_string(),
        t.albums.to_string(),
        t.fake.to_string(),
        fmt::rate(t.fake, t.tracks),
        ci,
        t.albums_fake.to_string(),
        t.suspicious.to_string(),
        fmt::rate(t.suspicious, t.tracks),
        t.error.to_string(),
    ]
}

fn fp_table(md: &mut Md, data: &ReportData<'_>, idx: usize, sel: SplitSel) {
    let run = &data.runs[idx];
    let rows = &data.groups[run.group].rows;
    let eval = &data.evaluations[idx];
    let z = data.config.report.z;
    let mut table: Vec<Vec<String>> = Vec::new();
    for class in ["primary", "reviewed", "clean", "reference"] {
        let in_class: Vec<_> = eval.fp_rows[&sel]
            .iter()
            .filter(|r| r.class == class)
            .collect();
        if in_class.is_empty() {
            continue;
        }
        for r in in_class {
            let t = Totals {
                tracks: r.tracks,
                albums: r.albums,
                real: r.tracks - r.fake - r.suspicious,
                suspicious: r.suspicious,
                fake: r.fake,
                error: r.error,
                albums_fake: r.albums_fake,
            };
            table.push(fp_cells(&r.origin, class, &t, z));
        }
        let t = class_totals(rows, &run.evals, class, sel);
        table.push(fp_cells(&format!("**{class} total**"), class, &t, z));
    }
    md.table(
        &[
            "origin",
            "class",
            "tracks",
            "albums",
            "Fake",
            "Fake rate",
            "95 % CI",
            "albums with ≥ 1 Fake",
            "Suspicious",
            "Suspicious rate",
            "Error",
        ],
        &table,
    );
}

fn side_by_side(md: &mut Md, data: &ReportData<'_>) {
    let mut header: Vec<&str> = vec![""];
    header.extend(data.runs.iter().map(|r| r.name.as_str()));
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut push = |label: String, f: &dyn Fn(usize) -> String| {
        let mut cells = vec![label];
        cells.extend((0..data.runs.len()).map(f));
        rows.push(cells);
    };
    push("algorithm_version".to_string(), &|i| {
        data.runs[i].params.algorithm_version.to_string()
    });
    push("content hash".to_string(), &|i| data.runs[i].hash.clone());
    push("fingerprint".to_string(), &|i| {
        data.runs[i].fingerprint_id.clone()
    });
    push("primary figure".to_string(), &|i| {
        data.evaluations[i].primary_figure.clone()
    });
    for class in ["primary", "reviewed", "clean"] {
        let totals = |i: usize| {
            let run = &data.runs[i];
            class_totals(
                &data.groups[run.group].rows,
                &run.evals,
                class,
                SplitSel::All,
            )
        };
        push(format!("FP rate, {class}"), &|i| {
            let t = totals(i);
            fmt::rate(t.fake, t.tracks)
        });
        push(format!("soft-FP rate, {class}"), &|i| {
            let t = totals(i);
            fmt::rate(t.suspicious, t.tracks)
        });
    }
    push("FN rate".to_string(), &|i| {
        let f = data.evaluations[i].matrix[&SplitSel::All].fake_row;
        fmt::rate(f.real, f.total() - f.error - f.undetectable)
    });
    let settings: Vec<String> = data
        .config
        .lossless_settings()
        .map(|s| s.id.clone())
        .chain(std::iter::once("confirmed_fake".to_string()))
        .collect();
    for (n, setting) in settings.iter().enumerate() {
        push(format!("detection rate, {setting}"), &|i| {
            data.evaluations[i]
                .by_setting
                .get(n)
                .map(|r| {
                    fmt::rate(
                        r.counts.fake,
                        r.counts.total() - r.counts.error - r.counts.undetectable,
                    )
                })
                .unwrap_or_default()
        });
    }
    push("smallest negative margin (dB)".to_string(), &|i| {
        let run = &data.runs[i];
        let summary = margin_summary(run, &data.groups[run.group].rows, data.config);
        let smallest = summary
            .iter()
            .filter_map(|s| s.smallest_negative_margin)
            .fold(None, |a: Option<f64>, x| Some(a.map_or(x, |a| a.min(x))));
        smallest.map(fmt::db).unwrap_or_else(|| "n/a".to_string())
    });
    for target in &data.config.targets {
        for sel in [SplitSel::All, SplitSel::Holdout] {
            push(format!("{} ({})", target.id, sel.as_str()), &|i| {
                data.evaluations[i]
                    .targets
                    .iter()
                    .find(|t| t.id == target.id && t.split == sel)
                    .map(|t| format!("{} ({})", if t.pass { "PASS" } else { "FAIL" }, t.count))
                    .unwrap_or_default()
            });
        }
    }
    md.table(&header, &rows);
}

/// Per album class (the class used for the split): tracks, albums in tune, albums in holdout.
fn albums_by_class(
    rows: &[LabelRow],
) -> BTreeMap<&'static str, (u32, BTreeSet<&str>, BTreeSet<&str>)> {
    let mut album_classes: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.origin != "generated") {
        album_classes
            .entry(r.album_id.as_str())
            .or_default()
            .push(r.class.as_str());
    }
    let class_of: BTreeMap<&str, &'static str> = album_classes
        .into_iter()
        .map(|(album, classes)| (album, album_class_of(classes.into_iter())))
        .collect();
    let mut map: BTreeMap<&'static str, (u32, BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.origin != "generated") {
        let Some(class) = class_of.get(r.album_id.as_str()) else {
            continue;
        };
        let entry = map.entry(class).or_default();
        entry.0 += 1;
        match r.split {
            crate::split::Split::Tune => entry.1.insert(r.album_id.as_str()),
            crate::split::Split::Holdout => entry.2.insert(r.album_id.as_str()),
        };
    }
    map
}

/// Warnings of the header (evaluation.md section 1).
fn warnings(data: &ReportData<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let base = &data.groups[data.runs[0].group];
    for class in CLASSES {
        if let Some((_, tune, holdout)) = albums_by_class(&base.rows).get(class) {
            if (holdout.len() as u32) < data.config.split.min_holdout_albums {
                out.push(format!(
                    "class {class}: {} albums in holdout (fewer than {}), {} in tune",
                    holdout.len(),
                    data.config.split.min_holdout_albums,
                    tune.len()
                ));
            }
        }
    }
    // Units are the split unit (lossy.md section 3), so a stratum can be short of holdout even when
    // its albums are not.
    for (stratum, (tune, holdout)) in data.unit_strata {
        if *holdout < data.config.split.min_holdout_albums {
            out.push(format!(
                "stratum {stratum}: {holdout} units in holdout (fewer than {}), {tune} in tune",
                data.config.split.min_holdout_albums
            ));
        }
    }
    for (run, eval) in data.runs.iter().zip(data.evaluations.iter()) {
        let rows = &data.groups[run.group].rows;
        let t = class_totals(rows, &run.evals, "primary", SplitSel::All);
        if let Some(w) = small_sample_warning(&t, data.config) {
            out.push(format!("{}: primary figure — {w}", eval.params_name));
        }
    }
    for g in data.groups {
        if g.stats.generation_failures > 0 {
            out.push(format!(
                "fingerprint {}: {} generation failures (listed in report.md)",
                g.stats.fingerprint_id, g.stats.generation_failures
            ));
        }
        if g.not_generated > 0 {
            out.push(format!(
                "fingerprint {}: {} entries not generated",
                g.stats.fingerprint_id, g.not_generated
            ));
        }
        if !g.skipped_sources.is_empty() {
            let mut reasons: BTreeMap<&str, u32> = BTreeMap::new();
            for (_, reason) in &g.skipped_sources {
                *reasons.entry(reason.as_str()).or_default() += 1;
            }
            let list: Vec<String> = reasons.iter().map(|(r, n)| format!("{r} {n}")).collect();
            out.push(format!(
                "fingerprint {}: {} sources skipped ({})",
                g.stats.fingerprint_id,
                g.skipped_sources.len(),
                list.join(", ")
            ));
        }
        if g.ffmpeg_missing {
            out.push(
                "ffmpeg was not found; the cached generated entries were used as they are"
                    .to_string(),
            );
        }
    }
    out
}

fn header(md: &mut Md, data: &ReportData<'_>, full: bool) {
    md.line(if full {
        "# Calibration report"
    } else {
        "# Calibration summary"
    });
    md.blank();
    // The primary figure comes first (evaluation.md section 3).
    for (run, eval) in data.runs.iter().zip(data.evaluations.iter()) {
        let rows = &data.groups[run.group].rows;
        let all = class_totals(rows, &run.evals, "primary", SplitSel::All);
        md.line(format!(
            "**Primary figure, {} (all): {}** · Suspicious: {}",
            run.name,
            eval.primary_figure,
            fmt::rate(all.suspicious, all.tracks)
        ));
        md.blank();
    }
    md.line(format!("- run id: {}", data.run_id));
    md.line(format!("- date (UTC): {}", data.date));
    md.line(format!("- tool commit: {}", data.commit));
    md.line(format!("- mode: {}", data.mode));
    let ffmpeg = data
        .groups
        .iter()
        .find_map(|g| g.ffmpeg_version.clone())
        .unwrap_or_else(|| "not found or not needed".to_string());
    md.line(format!("- ffmpeg_version: {ffmpeg}"));
    md.blank();

    md.line("## Parameter files");
    md.blank();
    let rows: Vec<Vec<String>> = data
        .runs
        .iter()
        .map(|r| {
            vec![
                r.name.clone(),
                r.params.algorithm_version.to_string(),
                r.hash.clone(),
                r.fingerprint_id.clone(),
            ]
        })
        .collect();
    md.table(
        &["file", "algorithm_version", "content hash", "fingerprint"],
        &rows,
    );

    md.line("## Encoder versions");
    md.blank();
    let base = &data.groups[data.runs[0].group];
    let mut rows = Vec::new();
    for s in data.config.lossless_settings() {
        let versions: BTreeSet<&str> = base
            .rows
            .iter()
            .filter(|r| r.setting == s.id)
            .map(|r| r.encoder_version.as_str())
            .collect();
        let text = if versions.is_empty() {
            "(not generated)".to_string()
        } else {
            versions.into_iter().collect::<Vec<_>>().join(" / ")
        };
        rows.push(vec![s.id.clone(), s.encoder.clone(), text]);
    }
    for entry in &data.config.generate.lossy {
        let versions: BTreeSet<&str> = base
            .lossy_rows
            .iter()
            .filter(|r| r.origin == "generated" && r.setting == entry.id)
            .map(|r| r.encoder_version.as_str())
            .collect();
        let text = if versions.is_empty() {
            "(not generated)".to_string()
        } else {
            versions.into_iter().collect::<Vec<_>>().join(" / ")
        };
        rows.push(vec![
            format!("lossy {}", entry.id),
            data.config
                .stages_of(entry)
                .last()
                .map(|s| s.encoder.clone())
                .unwrap_or_default(),
            text,
        ]);
    }
    md.table(&["setting", "encoder", "encoder_version"], &rows);

    md.line("## Corpus");
    md.blank();
    let by_class = albums_by_class(&base.rows);
    let rows: Vec<Vec<String>> = CLASSES
        .iter()
        .filter_map(|c| {
            by_class.get(c).map(|(n, tune, holdout)| {
                let all: BTreeSet<&&str> = tune.iter().chain(holdout.iter()).collect();
                vec![
                    (*c).to_string(),
                    n.to_string(),
                    all.len().to_string(),
                    tune.len().to_string(),
                    holdout.len().to_string(),
                ]
            })
        })
        .collect();
    md.table(
        &[
            "class",
            "tracks",
            "albums",
            "albums in tune",
            "albums in holdout",
        ],
        &rows,
    );
    let mut per_setting = Vec::new();
    for s in data.config.lossless_settings() {
        let n = base.rows.iter().filter(|r| r.setting == s.id).count();
        per_setting.push(format!("{} {n}", s.id));
    }
    md.line(format!("Generated entries: {}", per_setting.join(", ")));
    md.blank();
    crate::render_lossy::header_lossy(md, data);

    md.line("## Cache statistics");
    md.blank();
    for g in data.groups {
        let s = &g.stats;
        md.line(format!(
            "- cache {}: hits {}, decoded {}, transcoded {}, generation failures {}, skipped lines {}",
            s.fingerprint_id, s.hits, s.decoded, s.transcoded, s.generation_failures, s.skipped_lines
        ));
    }
    md.blank();

    let w = warnings(data);
    md.line("## Warnings");
    md.blank();
    if w.is_empty() {
        md.line("- none");
    }
    for line in w {
        md.line(format!("- {line}"));
    }
    md.blank();
}

fn watch_section(md: &mut Md, eval: &Evaluation, full: bool) {
    md.line("### Watch list");
    md.blank();
    if eval.watch.is_empty() {
        md.line("No `false-positive` tracks.");
        md.blank();
        return;
    }
    let mut head = vec!["id"];
    if full {
        head.push("path");
    }
    head.extend([
        "verdict",
        "reasons",
        "G",
        "D",
        "fs",
        "fe",
        "H",
        "U",
        "B",
        "full_band",
        "nearest_fake_margin_db",
    ]);
    let rows: Vec<Vec<String>> = eval
        .watch
        .iter()
        .map(|w| {
            let mut cells = vec![w.watch_id.clone()];
            if full {
                cells.push(w.path.clone());
            }
            cells.extend([
                verdict_name(w.verdict).to_string(),
                w.reasons
                    .iter()
                    .map(|x| reason_name(*x))
                    .collect::<Vec<_>>()
                    .join(";"),
                fmt::opt_db(w.gap_db),
                fmt::opt_db(w.step_db),
                fmt::opt_hz(w.step_hz),
                fmt::opt_hz(w.extent_hz),
                fmt::opt_db(w.high_frequency_db),
                fmt::opt_db(w.upper_level_db),
                fmt::opt_db(w.upper_burst_db),
                w.full_band.map(|b| b.to_string()).unwrap_or_default(),
                fmt::opt_db(w.nearest_fake_margin_db),
            ]);
            cells
        })
        .collect();
    md.table(&head, &rows);
}

fn params_section(md: &mut Md, data: &ReportData<'_>, idx: usize, full: bool) {
    let run = &data.runs[idx];
    let eval = &data.evaluations[idx];
    let rows = &data.groups[run.group].rows;
    let z = data.config.report.z;
    md.line(format!("## {}", run.name));
    md.blank();
    let all = class_totals(rows, &run.evals, "primary", SplitSel::All);
    md.line(format!(
        "**Primary figure (all): {}** · Suspicious: {}",
        eval.primary_figure,
        fmt::rate(all.suspicious, all.tracks)
    ));
    md.blank();
    if data.mode == "calibrate" {
        let hold = class_totals(rows, &run.evals, "primary", SplitSel::Holdout);
        md.line(format!(
            "**Primary figure (holdout, headline): {}** · Suspicious: {}",
            figure(&hold, z),
            fmt::rate(hold.suspicious, hold.tracks)
        ));
        md.blank();
    }
    if let Some(w) = small_sample_warning(&all, data.config) {
        md.line(format!("Warning: {w}"));
        md.blank();
    }

    md.line("### False positives by origin");
    md.blank();
    for sel in SPLITS {
        md.line(format!("Split `{}`:", sel.as_str()));
        md.blank();
        fp_table(md, data, idx, sel);
    }

    md.line("### Confusion matrix");
    md.blank();
    for sel in SPLITS {
        md.line(format!("Split `{}`:", sel.as_str()));
        md.blank();
        matrix_table(md, &eval.matrix[&sel]);
    }

    md.line("### Per encoder setting");
    md.blank();
    let sels: &[SplitSel] = if data.mode == "calibrate" {
        &SPLITS
    } else {
        &[SplitSel::All]
    };
    for sel in sels {
        md.line(format!("Split `{}`:", sel.as_str()));
        md.blank();
        setting_rows_table(md, &eval.by_setting_split[sel]);
    }
    full_band_table(md, eval);

    watch_section(md, eval, full);
    crate::render_lossy::lossy_section(md, data, idx, full);

    md.line("### Clean albums that turn Fake");
    md.blank();
    let baseline = (idx > 0).then(|| {
        (
            &data.runs[0],
            data.groups[data.runs[0].group].rows.as_slice(),
        )
    });
    let cleans = clean_fakes(run, rows, baseline);
    let tracks: usize = cleans.iter().map(|c| c.tracks.len()).sum();
    let new = cleans.iter().filter(|c| c.new).count();
    md.line(format!(
        "{} albums, {tracks} tracks, {new} new against the baseline",
        cleans.len()
    ));
    md.blank();
    if full {
        for c in &cleans {
            md.line(format!(
                "- {}{}",
                c.album_id,
                if c.new { " (new)" } else { "" }
            ));
            for t in &c.tracks {
                md.line(format!("  - {t}"));
            }
        }
        if !cleans.is_empty() {
            md.blank();
        }
    }

    md.line("### Margin summary");
    md.blank();
    let table: Vec<Vec<String>> = margin_summary(run, rows, data.config)
        .into_iter()
        .map(|s| {
            vec![
                s.class,
                s.smallest_negative_margin
                    .map(fmt::db)
                    .unwrap_or_else(|| "n/a".to_string()),
                s.near_negatives.to_string(),
                s.near_positives.to_string(),
            ]
        })
        .collect();
    md.table(
        &[
            "class",
            "smallest nearest_fake_margin_db among negatives that are not Fake",
            "near negatives",
            "near positives",
        ],
        &table,
    );

    md.line("### Acceptance targets");
    md.blank();
    for target in &data.config.targets {
        let cell = |sel: SplitSel| {
            eval.targets
                .iter()
                .find(|t| t.id == target.id && t.split == sel)
                .map(|t| {
                    format!(
                        "{} ({} of max {})",
                        if t.pass { "PASS" } else { "FAIL" },
                        t.count,
                        target.max_count
                    )
                })
                .unwrap_or_default()
        };
        md.line(format!(
            "- {}: all {}, holdout {}",
            target.id,
            cell(SplitSel::All),
            cell(SplitSel::Holdout)
        ));
    }
    md.blank();

    if let Some(changes) = &eval.changes {
        md.line(format!("### Verdict changes against {}", changes.against));
        md.blank();
        let names = ["real", "suspicious", "fake", "error"];
        let mut head = vec!["baseline \\ this file"];
        head.extend(names);
        let table: Vec<Vec<String>> = names
            .iter()
            .map(|from| {
                let mut cells = vec![(*from).to_string()];
                for to in names {
                    let n = changes
                        .counts
                        .get(&((*from).to_string(), to.to_string()))
                        .copied()
                        .unwrap_or(0);
                    cells.push(n.to_string());
                }
                cells
            })
            .collect();
        md.table(&head, &table);
        setting_changes_table(md, eval);
    }

    if full {
        full_lists(md, data, idx);
    }
}

fn row_label(r: &LabelRow) -> String {
    if r.truth == Truth::Fake {
        if r.origin == "confirmed_fake" {
            "confirmed_fake".to_string()
        } else {
            r.setting.clone()
        }
    } else {
        r.class.clone()
    }
}

fn measure_cells(e: &RowEval) -> Vec<String> {
    let m = e.m.as_ref();
    vec![
        fmt::opt_db(m.map(|m| m.high_frequency_db)),
        fmt::opt_db(m.map(|m| m.noise_floor_db)),
        fmt::opt_hz(m.map(|m| m.extent_hz)),
        fmt::opt_db(m.map(|m| m.step_db)),
        fmt::opt_hz(m.and_then(|m| m.step_hz)),
        fmt::opt_db(m.map(|m| m.gap_db)),
        fmt::opt_db(m.and_then(|m| m.upper_level_db)),
        fmt::opt_db(m.and_then(|m| m.upper_burst_db)),
    ]
}

const MEASURE_HEAD: [&str; 8] = ["H", "F", "fe", "D", "fs", "G", "U", "B"];

/// The per-track lists of `report.md` (evaluation.md sections 5, 8 and 9).
fn full_lists(md: &mut Md, data: &ReportData<'_>, idx: usize) {
    let run = &data.runs[idx];
    let rows = &data.groups[run.group].rows;

    md.line("### Near a threshold");
    md.blank();
    let mut table = Vec::new();
    for (r, e) in rows.iter().zip(run.evals.iter()) {
        if is_near(r, e, data.config) {
            let mut cells = vec![
                r.path.clone(),
                r.truth.as_str().to_string(),
                row_label(r),
                verdict_name(e.verdict).to_string(),
            ];
            cells.extend(measure_cells(e));
            cells.push(fmt::opt_db(
                e.margins.as_ref().map(|m| m.nearest_fake_margin_db),
            ));
            table.push(cells);
        }
    }
    let mut head = vec!["path", "truth", "class or setting", "verdict"];
    head.extend(MEASURE_HEAD);
    head.push("nearest_fake_margin_db");
    md.table(&head, &table);

    md.line("### Errors and misclassifications");
    md.blank();
    let mut list: Vec<(u8, String, String, Vec<String>)> = Vec::new();
    for (r, e) in rows.iter().zip(run.evals.iter()) {
        let kind: Option<(u8, &str)> = match (r.truth, e.verdict) {
            (_, Verdict::Error) => Some((4, "error")),
            (Truth::Real, Verdict::Fake) => Some((0, "FP")),
            (Truth::Real, Verdict::Suspicious) => Some((1, "soft FP")),
            (Truth::Fake, Verdict::Real) => Some((2, "FN")),
            (Truth::Fake, Verdict::Suspicious) => Some((3, "caught, not detected")),
            _ => None,
        };
        let Some((order, kind)) = kind else {
            continue;
        };
        let mut cells = vec![
            kind.to_string(),
            r.path.clone(),
            r.truth.as_str().to_string(),
            row_label(r),
            verdict_name(e.verdict).to_string(),
            e.reasons
                .iter()
                .map(|x| reason_name(*x))
                .collect::<Vec<_>>()
                .join(";"),
        ];
        cells.extend(measure_cells(e));
        cells.push(
            e.error
                .as_ref()
                .map(|x| error_code(x).to_string())
                .unwrap_or_default(),
        );
        list.push((order, r.class.clone(), r.path.clone(), cells));
    }
    list.sort_by(|a, b| (a.0, &a.1, &a.2).cmp(&(b.0, &b.1, &b.2)));
    let mut head = vec![
        "kind",
        "path",
        "truth",
        "class or setting",
        "verdict",
        "reasons",
    ];
    head.extend(MEASURE_HEAD);
    head.push("error");
    let table: Vec<Vec<String>> = list.into_iter().map(|x| x.3).collect();
    md.table(&head, &table);

    let undetectable_tracks: Vec<(&LabelRow, &RowEval)> = rows
        .iter()
        .zip(run.evals.iter())
        .filter(|(r, _)| r.undetectable)
        .collect();
    if !undetectable_tracks.is_empty() {
        md.line("### Undetectable transcodes");
        md.blank();
        let table: Vec<Vec<String>> = undetectable_tracks
            .into_iter()
            .map(|(r, e)| {
                vec![
                    r.path.clone(),
                    r.setting.clone(),
                    r.source_path.clone(),
                    verdict_name(e.verdict).to_string(),
                ]
            })
            .collect();
        md.table(&["path", "setting", "source_path", "verdict"], &table);
    }

    if idx > 0 {
        let base = &data.runs[0];
        let base_rows = &data.groups[base.group].rows;
        md.line(format!(
            "### Tracks that changed verdict against {}",
            base.name
        ));
        md.blank();
        let table: Vec<Vec<String>> = changed_rows(run, rows, base, base_rows)
            .into_iter()
            .map(|c| {
                let r = &rows[c.index];
                let e = &run.evals[c.index];
                vec![
                    r.path.clone(),
                    r.truth.as_str().to_string(),
                    row_label(r),
                    verdict_name(c.from).to_string(),
                    verdict_name(c.to).to_string(),
                    fmt::opt_db(e.m.as_ref().map(|m| m.gap_db)),
                    fmt::opt_db(e.m.as_ref().map(|m| m.step_db)),
                    fmt::opt_hz(e.m.as_ref().and_then(|m| m.step_hz)),
                    fmt::opt_db(e.m.as_ref().and_then(|m| m.upper_level_db)),
                    fmt::opt_db(e.m.as_ref().and_then(|m| m.upper_burst_db)),
                ]
            })
            .collect();
        md.table(
            &[
                "path",
                "truth",
                "class or setting",
                "from",
                "to",
                "G",
                "D",
                "fs",
                "U",
                "B",
            ],
            &table,
        );
    }
}

/// Skipped sources and generation failures (once per fingerprint).
fn generation_lists(md: &mut Md, data: &ReportData<'_>) {
    md.line("## Sources skipped and generation failures");
    md.blank();
    for g in data.groups {
        md.line(format!("Fingerprint {}:", g.stats.fingerprint_id));
        md.blank();
        let skipped: Vec<Vec<String>> = g
            .skipped_sources
            .iter()
            .map(|(p, r)| vec![p.clone(), r.clone()])
            .collect();
        md.table(&["skipped source", "reason"], &skipped);
        let failed: Vec<Vec<String>> = g
            .failures
            .iter()
            .map(|f| {
                vec![
                    f.source.clone(),
                    f.setting.clone(),
                    scrub(&f.message, data.roots),
                ]
            })
            .collect();
        md.table(&["source", "setting", "ffmpeg message"], &failed);
    }
}

fn grid_section(md: &mut Md, data: &ReportData<'_>, grid: &GridReport<'_>) {
    let w = &grid.file.weights;
    md.line("## Grid search");
    md.blank();
    md.line(format!(
        "- objective: cost = {} · #Fake(neg) + {} · #Real(pos) + {} · #Suspicious(neg) + {} · #Suspicious(pos), on `tune` rows only",
        w.fp, w.fn_, w.suspicious_negative, w.suspicious_positive
    ));
    md.line(format!(
        "- feasible: no Real among the tune positives of the settings [{}], and no confirmed fake classified Real",
        grid.file.must_catch.join(", ")
    ));
    md.line(format!(
        "- confirmed fakes checked by that constraint: {} ({} in holdout; they never enter the cost or the ranking)",
        grid.confirmed_checked, grid.confirmed_holdout
    ));
    md.line(format!(
        "- points evaluated: {}, skipped (broken invariants): {}",
        grid.result.points.len(),
        grid.result.skipped
    ));
    let b = &grid.result.baseline;
    md.line(format!(
        "- baseline: cost {:.3}, {}",
        b.cost,
        if b.feasible { "feasible" } else { "infeasible" }
    ));
    match (grid.proposal, grid.result.proposal) {
        (Some(p), _) => md.line(format!(
            "- result: proposal {} (changed: {})",
            p.file_name,
            p.changed
                .iter()
                .map(|(k, o, n)| format!("{k} {o} -> {n}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
        (None, _) if !grid.result.baseline.feasible => md.line(
            "- result: no feasible point; nothing is proposed (the best infeasible points are listed below)",
        ),
        (None, _) => md.line("- result: no change proposed (the baseline is at least as good)"),
    }
    md.blank();
    md.line("### Best points");
    md.blank();
    let keys: Vec<&str> = grid.file.keys.iter().map(|k| k.key.as_str()).collect();
    let table: Vec<Vec<String>> = grid
        .result
        .top
        .iter()
        .enumerate()
        .map(|(rank, i)| {
            let p = &grid.result.points[*i];
            let changed: Vec<String> = keys
                .iter()
                .zip(p.values.iter().zip(b.values.iter()))
                .filter(|(_, (n, o))| n != o)
                .map(|(k, (n, o))| {
                    format!(
                        "{k} {} -> {}",
                        crate::grid::float_token(*o),
                        crate::grid::float_token(*n)
                    )
                })
                .collect();
            vec![
                (rank + 1).to_string(),
                if changed.is_empty() {
                    "(baseline values)".to_string()
                } else {
                    changed.join(", ")
                },
                format!("{:.3}", p.cost),
                if p.feasible { "yes" } else { "no" }.to_string(),
                p.tally.must_catch_real.to_string(),
                p.tally.confirmed_real.to_string(),
                format!(
                    "{} / {} / {} / {}",
                    p.tally.fake_negatives,
                    p.tally.suspicious_negatives,
                    p.tally.real_positives,
                    p.tally.suspicious_positives
                ),
                if p.min_margin.is_finite() {
                    fmt::db(p.min_margin)
                } else {
                    "n/a".to_string()
                },
            ]
        })
        .collect();
    md.table(
        &[
            "rank",
            "changed values",
            "cost",
            "feasible",
            "must_catch Real",
            "confirmed fakes Real",
            "tune: Fake neg / Suspicious neg / Real pos / Suspicious pos",
            "smallest margin (dB)",
        ],
        &table,
    );
    let _ = data;
}

/// `summary.md` (`full = false`) or `report.md` (`full = true`).
pub fn markdown(data: &ReportData<'_>, full: bool) -> String {
    let mut md = Md::default();
    header(&mut md, data, full);
    md.line("## Side by side");
    md.blank();
    side_by_side(&mut md, data);
    if let Some(grid) = &data.grid {
        grid_section(&mut md, data, grid);
    }
    for idx in 0..data.runs.len() {
        params_section(&mut md, data, idx, full);
    }
    if full {
        generation_lists(&mut md, data);
    }
    md.line("## Reference");
    md.blank();
    md.line("v1 (ADR-0035, Rust v1 code, known-truth album): 0 of 7 transcodes found, all 16 tracks `Real`.");
    md.0
}

//! Verdicts, margins and the tables of evaluation.md sections 3 to 9, computed from cached spectra.
//! Measurements and verdicts come from `fakeflac_core::detect` (ADR-0036): nothing is reimplemented.

use crate::cache::{error_code, Extracted};
use crate::config::Config;
use crate::fmt;
use crate::lists::ORIGINS;
use crate::model::{LabelRow, Truth, CLASSES};
use crate::report::{
    ConfusionMatrix, Counts, Evaluation, FpRow, FullBandRow, SettingChange, SettingRow, SplitSel,
    TargetResult, VerdictChanges, WatchEntry,
};
use crate::split::Split;
use fakeflac_core::detect::{decide, detect, is_full_band, measure, Measurements};
use fakeflac_core::{DetectionParams, SuspicionReason, TrackError, Verdict};
use std::collections::{BTreeMap, BTreeSet};

pub const SPLITS: [SplitSel; 3] = [SplitSel::All, SplitSel::Tune, SplitSel::Holdout];

impl SplitSel {
    pub fn matches(self, split: Split) -> bool {
        match self {
            SplitSel::All => true,
            SplitSel::Tune => split == Split::Tune,
            SplitSel::Holdout => split == Split::Holdout,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SplitSel::All => "all",
            SplitSel::Tune => "tune",
            SplitSel::Holdout => "holdout",
        }
    }
}

pub fn verdict_name(v: Verdict) -> &'static str {
    match v {
        Verdict::Real => "real",
        Verdict::Suspicious => "suspicious",
        Verdict::Fake => "fake",
        Verdict::Error => "error",
    }
}

pub fn reason_name(r: SuspicionReason) -> &'static str {
    match r {
        SuspicionReason::LowHighFrequencyContent => "lowHighFrequencyContent",
        SuspicionReason::GradualRolloff => "gradualRolloff",
        SuspicionReason::AmbiguousCutoff => "ambiguousCutoff",
        SuspicionReason::HighBandGaps => "highBandGaps",
        SuspicionReason::CutoffBelowDeclared => "cutoffBelowDeclared",
    }
}

/// Signed margins against every threshold of rules.md section 3 (evaluation.md section 8).
#[derive(Debug, Clone, PartialEq)]
pub struct Margins {
    pub m_low_hf_db: f64,
    pub m_gap_fake_db: f64,
    pub m_gap_suspicious_db: f64,
    pub m_step_fake_db: Option<f64>,
    pub m_step_suspicious_db: Option<f64>,
    pub m_step_low_db: Option<f64>,
    pub m_step_high_db: Option<f64>,
    pub m_gradual_hz: Option<f64>,
    pub fs_fake_range_hz: Option<f64>,
    /// `U - full_band.min_level_db` (v3).
    pub m_full_band_level_db: Option<f64>,
    /// `B - full_band.min_burst_db` (v3).
    pub m_full_band_burst_db: Option<f64>,
    /// `min` of the two above; `FullBand` when `>= 0`.
    pub full_band_margin_db: Option<f64>,
    /// `full_band.max_lossy_step_db - D` when the params have it and `InRange(fs)` (v4); `LossyStep` when `<= 0`.
    pub m_full_band_step_db: Option<f64>,
    pub nearest_fake_margin_db: f64,
}

pub fn margins(m: &Measurements, params: &DetectionParams) -> Margins {
    let v = &params.verdict;
    let fs = m.step_hz;
    let d = m.step_db;
    let in_range = fs.filter(|f| *f >= v.lossy_min_cutoff_hz && *f < v.fake_max_cutoff_hz);
    let m_gap_fake_db = v.gap_fake_db - m.gap_db;
    let m_step_fake_db = in_range.map(|_| v.step_fake_db - d);
    let fs_fake_range_hz = fs.filter(|_| d > 0.0).map(|f| {
        if f < v.lossy_min_cutoff_hz {
            v.lossy_min_cutoff_hz - f
        } else if f >= v.fake_max_cutoff_hz {
            f - v.fake_max_cutoff_hz
        } else {
            -(f - v.lossy_min_cutoff_hz).min(v.fake_max_cutoff_hz - f)
        }
    });
    let fb = params.full_band.as_ref();
    let m_full_band_level_db = fb.zip(m.upper_level_db).map(|(f, u)| u - f.min_level_db);
    let m_full_band_burst_db = fb.zip(m.upper_burst_db).map(|(f, b)| b - f.min_burst_db);
    let full_band_margin_db = m_full_band_level_db
        .zip(m_full_band_burst_db)
        .map(|(l, b)| l.min(b));
    let m_full_band_step_db = lossy_step_margin(m, params);
    let a = m_step_fake_db.map_or(m_gap_fake_db, |s| s.min(m_gap_fake_db));
    Margins {
        m_low_hf_db: m.high_frequency_db - v.low_hf_threshold_db,
        m_gap_fake_db,
        m_gap_suspicious_db: v.gap_suspicious_db - m.gap_db,
        m_step_fake_db,
        m_step_suspicious_db: in_range.map(|_| v.step_suspicious_db - d),
        m_step_low_db: fs
            .filter(|f| *f < v.lossy_min_cutoff_hz)
            .map(|_| v.step_low_suspicious_db - d),
        m_step_high_db: fs
            .filter(|f| *f >= v.fake_max_cutoff_hz && *f < v.ambiguous_max_cutoff_hz)
            .map(|_| v.step_high_suspicious_db - d),
        m_gradual_hz: (d < v.step_suspicious_db).then_some(m.extent_hz - v.gradual_min_cutoff_hz),
        fs_fake_range_hz,
        m_full_band_level_db,
        m_full_band_burst_db,
        full_band_margin_db,
        m_full_band_step_db,
        nearest_fake_margin_db: guarded(a, full_band_margin_db, m_full_band_step_db),
    }
}

/// `full_band.max_lossy_step_db - D` when the params have the field and `InRange(fs)`.
fn lossy_step_margin(m: &Measurements, params: &DetectionParams) -> Option<f64> {
    let v = &params.verdict;
    let max = params.full_band.as_ref()?.max_lossy_step_db?;
    m.step_hz
        .filter(|f| *f >= v.lossy_min_cutoff_hz && *f < v.fake_max_cutoff_hz)
        .map(|_| max - m.step_db)
}

/// Guard margin `g = min(full_band_margin_db, m_full_band_step_db)` (an undefined term is
/// ignored); `max(a, g)` when `FullBand` holds (`full_band_margin_db >= 0` and the step margin
/// undefined or `> 0`), otherwise `a` (evaluation.md section 8).
fn guarded(a: f64, full_band_margin_db: Option<f64>, step_margin_db: Option<f64>) -> f64 {
    match full_band_margin_db {
        Some(f) if f >= 0.0 && step_margin_db.is_none_or(|s| s > 0.0) => {
            a.max(step_margin_db.map_or(f, |s| s.min(f)))
        }
        _ => a,
    }
}

/// `nearest_fake_margin_db` alone (the grid search ranks by it).
pub fn nearest_margin(m: &Measurements, params: &DetectionParams) -> f64 {
    let v = &params.verdict;
    let gap = v.gap_fake_db - m.gap_db;
    let a = match m.step_hz {
        Some(f) if f >= v.lossy_min_cutoff_hz && f < v.fake_max_cutoff_hz => {
            gap.min(v.step_fake_db - m.step_db)
        }
        _ => gap,
    };
    let fb = params.full_band.as_ref();
    let margin = fb
        .zip(m.upper_level_db)
        .zip(m.upper_burst_db)
        .map(|((f, u), b)| (u - f.min_level_db).min(b - f.min_burst_db));
    guarded(a, margin, lossy_step_margin(m, params))
}

/// The verdict of one labelled row under one params file.
#[derive(Debug, Clone, PartialEq)]
pub struct RowEval {
    pub verdict: Verdict,
    pub reasons: Vec<SuspicionReason>,
    pub error: Option<TrackError>,
    /// `None` for `Error` rows and for silent tracks.
    pub m: Option<Measurements>,
    pub margins: Option<Margins>,
}

/// As `analyze_track` of the core: too short is an error, then `detect` (rules.md section 1.6 for silent tracks).
pub fn evaluate_row(extracted: &Extracted, params: &DetectionParams) -> RowEval {
    let error = |e: TrackError| RowEval {
        verdict: Verdict::Error,
        reasons: Vec::new(),
        error: Some(e),
        m: None,
        margins: None,
    };
    match extracted {
        Extracted::Error(e) => error(e.clone()),
        Extracted::Probe(p) => {
            if (p.duration_ms as f64) < params.input.min_duration_s * 1000.0 {
                return error(TrackError::TooShort);
            }
            let detection = detect(p.spectrum.as_ref(), params);
            match &p.spectrum {
                None => RowEval {
                    verdict: detection.verdict,
                    reasons: detection.reasons,
                    error: None,
                    m: None,
                    margins: None,
                },
                Some(s) => {
                    let m = measure(s, params);
                    let (verdict, reasons) = decide(&m, params);
                    let margins = margins(&m, params);
                    RowEval {
                        verdict,
                        reasons,
                        error: None,
                        m: Some(m),
                        margins: Some(margins),
                    }
                }
            }
        }
    }
}

/// A params file of the run with the verdict of every row of its group.
#[derive(Debug, Clone)]
pub struct ParamsRun {
    pub name: String,
    pub params: DetectionParams,
    /// 16 hex digits of the content hash.
    pub hash: String,
    pub fingerprint_id: String,
    /// Index of the extraction group (the rows) this file is evaluated on.
    pub group: usize,
    pub evals: Vec<RowEval>,
}

impl ParamsRun {
    pub fn code(&self, i: usize) -> Option<&'static str> {
        self.evals[i].error.as_ref().map(error_code)
    }
}

pub fn bump(c: &mut Counts, v: Verdict) {
    match v {
        Verdict::Real => c.real += 1,
        Verdict::Suspicious => c.suspicious += 1,
        Verdict::Fake => c.fake += 1,
        Verdict::Error => c.error += 1,
    }
}

/// Counts of a set of rows that are not `Error`, with the albums they come from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals {
    pub tracks: u32,
    pub albums: u32,
    pub real: u32,
    pub suspicious: u32,
    pub fake: u32,
    pub error: u32,
    pub albums_fake: u32,
}

pub fn totals<'a>(rows: impl Iterator<Item = (&'a LabelRow, &'a RowEval)>) -> Totals {
    let mut t = Totals::default();
    let mut albums: BTreeSet<&str> = BTreeSet::new();
    let mut albums_fake: BTreeSet<&str> = BTreeSet::new();
    for (row, eval) in rows {
        match eval.verdict {
            Verdict::Error => t.error += 1,
            v => {
                t.tracks += 1;
                albums.insert(row.album_id.as_str());
                match v {
                    Verdict::Real => t.real += 1,
                    Verdict::Suspicious => t.suspicious += 1,
                    _ => {
                        t.fake += 1;
                        albums_fake.insert(row.album_id.as_str());
                    }
                }
            }
        }
    }
    t.albums = albums.len() as u32;
    t.albums_fake = albums_fake.len() as u32;
    t
}

fn selected<'a>(
    rows: &'a [LabelRow],
    evals: &'a [RowEval],
    sel: SplitSel,
) -> impl Iterator<Item = (&'a LabelRow, &'a RowEval)> {
    rows.iter()
        .zip(evals.iter())
        .filter(move |(r, _)| sel.matches(r.split))
}

/// Totals of the negatives of one class.
pub fn class_totals(rows: &[LabelRow], evals: &[RowEval], class: &str, sel: SplitSel) -> Totals {
    totals(selected(rows, evals, sel).filter(|(r, _)| r.truth == Truth::Real && r.class == class))
}

/// `k/n tracks (p %, 95 % CI a–b %), j/m albums`.
pub fn figure(t: &Totals, z: f64) -> String {
    format!(
        "{}/{} tracks ({}), {}/{} albums",
        t.fake,
        t.tracks,
        fmt::rate_with_interval(t.fake, t.tracks, z),
        t.albums_fake,
        t.albums
    )
}

/// Small-sample warning of a row, if any (evaluation.md section 3).
pub fn small_sample_warning(t: &Totals, config: &Config) -> Option<String> {
    (t.tracks < config.report.small_sample_tracks || t.albums < config.report.small_sample_albums)
        .then(|| {
            format!(
                "small sample: {} tracks from {} albums; tracks of one album are correlated, so the interval is optimistic",
                t.tracks, t.albums
            )
        })
}

/// Everything of evaluation.md sections 3 to 6 and 9 for one params file.
pub fn evaluation(
    run: &ParamsRun,
    rows: &[LabelRow],
    config: &Config,
    baseline: Option<(&ParamsRun, &[LabelRow])>,
) -> Evaluation {
    let evals = &run.evals;

    let mut matrix: BTreeMap<SplitSel, ConfusionMatrix> = BTreeMap::new();
    let mut by_setting_split: BTreeMap<SplitSel, Vec<SettingRow>> = BTreeMap::new();
    let mut fp_rows: BTreeMap<SplitSel, Vec<FpRow>> = BTreeMap::new();
    for sel in SPLITS {
        let mut m = ConfusionMatrix::default();
        for (row, eval) in selected(rows, evals, sel).filter(|(r, _)| r.in_evaluation_set()) {
            match row.truth {
                Truth::Real => bump(&mut m.real_row, eval.verdict),
                Truth::Fake if row.undetectable => m.fake_row.undetectable += 1,
                Truth::Fake => bump(&mut m.fake_row, eval.verdict),
            }
        }
        matrix.insert(sel, m);

        let mut settings: Vec<SettingRow> = config
            .generate
            .settings
            .iter()
            .map(|s| SettingRow {
                setting: s.id.clone(),
                counts: Counts::default(),
            })
            .collect();
        settings.push(SettingRow {
            setting: "confirmed_fake".to_string(),
            counts: Counts::default(),
        });
        for (row, eval) in selected(rows, evals, sel).filter(|(r, _)| r.truth == Truth::Fake) {
            let id = if row.origin == "confirmed_fake" {
                "confirmed_fake"
            } else {
                row.setting.as_str()
            };
            if let Some(s) = settings.iter_mut().find(|s| s.setting == id) {
                if row.undetectable {
                    s.counts.undetectable += 1;
                } else {
                    bump(&mut s.counts, eval.verdict);
                }
            }
        }
        by_setting_split.insert(sel, settings);

        let mut table = Vec::new();
        for (origin, class) in ORIGINS {
            let group: Vec<(&LabelRow, &RowEval)> = selected(rows, evals, sel)
                .filter(|(r, _)| r.truth == Truth::Real && r.trust_origin == origin)
                .collect();
            if group.is_empty() {
                continue;
            }
            let t = totals(group.into_iter());
            table.push(FpRow {
                origin: origin.to_string(),
                class: class.to_string(),
                tracks: t.tracks,
                albums: t.albums,
                fake: t.fake,
                suspicious: t.suspicious,
                error: t.error,
                albums_fake: t.albums_fake,
            });
        }
        fp_rows.insert(sel, table);
    }

    let primary = class_totals(rows, evals, "primary", SplitSel::All);
    let primary_figure = figure(&primary, config.report.z);

    let mut watch: Vec<WatchEntry> = rows
        .iter()
        .zip(evals.iter())
        .filter(|(r, _)| !r.watch_id.is_empty())
        .map(|(r, e)| WatchEntry {
            watch_id: r.watch_id.clone(),
            path: r.path.clone(),
            verdict: e.verdict,
            reasons: e.reasons.clone(),
            gap_db: e.m.as_ref().map(|m| m.gap_db),
            step_db: e.m.as_ref().map(|m| m.step_db),
            step_hz: e.m.as_ref().and_then(|m| m.step_hz),
            extent_hz: e.m.as_ref().map(|m| m.extent_hz),
            high_frequency_db: e.m.as_ref().map(|m| m.high_frequency_db),
            nearest_fake_margin_db: e.margins.as_ref().map(|m| m.nearest_fake_margin_db),
            upper_level_db: e.m.as_ref().and_then(|m| m.upper_level_db),
            upper_burst_db: e.m.as_ref().and_then(|m| m.upper_burst_db),
            full_band: e.m.as_ref().and_then(|m| {
                run.params
                    .full_band
                    .as_ref()
                    .and(m.upper_level_db.zip(m.upper_burst_db))
                    .map(|_| is_full_band(m, &run.params))
            }),
        })
        .collect();
    watch.sort_by_key(|w| {
        w.watch_id
            .trim_start_matches('W')
            .parse::<u32>()
            .unwrap_or(0)
    });

    let mut targets = Vec::new();
    for target in &config.targets {
        for sel in [SplitSel::All, SplitSel::Holdout] {
            let count = selected(rows, evals, sel)
                .filter(|(r, e)| {
                    let by_class = target
                        .classes
                        .as_ref()
                        .is_some_and(|c| r.truth == Truth::Real && c.contains(&r.class));
                    let by_setting = target
                        .settings
                        .as_ref()
                        .is_some_and(|s| r.origin == "generated" && s.contains(&r.setting));
                    (by_class || by_setting) && verdict_name(e.verdict) == target.verdict
                })
                .count() as u32;
            targets.push(TargetResult {
                id: target.id.clone(),
                split: sel,
                pass: count <= target.max_count,
                count,
            });
        }
    }

    let changes = baseline.map(|(base, base_rows)| verdict_changes(run, rows, base, base_rows));

    Evaluation {
        params_name: run.name.clone(),
        algorithm_version: run.params.algorithm_version,
        fingerprint_id: run.fingerprint_id.clone(),
        content_hash: run.hash.clone(),
        by_setting: by_setting_split
            .get(&SplitSel::All)
            .cloned()
            .unwrap_or_default(),
        matrix,
        by_setting_split,
        fp_rows,
        primary_figure,
        watch,
        targets,
        full_band_by_setting: full_band_rows(run, rows, config),
        setting_changes: baseline
            .map(|(base, base_rows)| setting_changes(run, rows, base, base_rows, config))
            .unwrap_or_default(),
        changes,
    }
}

/// The setting a positive row is reported under (`confirmed_fake` for the confirmed fakes).
fn setting_id(row: &LabelRow) -> &str {
    if row.origin == "confirmed_fake" {
        "confirmed_fake"
    } else {
        row.setting.as_str()
    }
}

fn setting_ids(config: &Config) -> Vec<String> {
    config
        .generate
        .settings
        .iter()
        .map(|s| s.id.clone())
        .chain(std::iter::once("confirmed_fake".to_string()))
        .collect()
}

/// Nearest-index percentile of a non-empty, sorted slice.
fn percentile_sorted(sorted: &[f64], pct: f64) -> f64 {
    let rank = ((pct / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

/// Evaluation.md section 4: `FullBand` count and `B` distribution of the detectable positives of
/// each setting. Empty for a params file without a `[full_band]` section.
fn full_band_rows(run: &ParamsRun, rows: &[LabelRow], config: &Config) -> Vec<FullBandRow> {
    if run.params.full_band.is_none() {
        return Vec::new();
    }
    setting_ids(config)
        .into_iter()
        .map(|setting| {
            let mut count = 0_u32;
            let mut bursts: Vec<f64> = Vec::new();
            for (row, eval) in rows.iter().zip(run.evals.iter()) {
                if row.truth != Truth::Fake || row.undetectable || setting_id(row) != setting {
                    continue;
                }
                let Some(m) = &eval.m else { continue };
                count += u32::from(is_full_band(m, &run.params));
                if let Some(b) = m.upper_burst_db {
                    bursts.push(b);
                }
            }
            bursts.sort_by(f64::total_cmp);
            let has = !bursts.is_empty();
            FullBandRow {
                setting,
                full_band: count,
                burst_median: has.then(|| percentile_sorted(&bursts, 50.0)),
                burst_p95: has.then(|| percentile_sorted(&bursts, 95.0)),
                burst_max: bursts.last().copied(),
            }
        })
        .collect()
}

/// Detection rate (`Fake` over detectable, non-error positives) of one setting, in percent.
fn detection_rate(rows: &[LabelRow], evals: &[RowEval], setting: &str) -> Option<f64> {
    let mut c = Counts::default();
    for (row, eval) in rows.iter().zip(evals.iter()) {
        if row.truth == Truth::Fake && !row.undetectable && setting_id(row) == setting {
            bump(&mut c, eval.verdict);
        }
    }
    let n = c.real + c.suspicious + c.fake;
    (n > 0).then(|| f64::from(c.fake) / f64::from(n) * 100.0)
}

/// Evaluation.md section 9: the verdict changes of the positives, per setting.
fn setting_changes(
    run: &ParamsRun,
    rows: &[LabelRow],
    base: &ParamsRun,
    base_rows: &[LabelRow],
    config: &Config,
) -> Vec<SettingChange> {
    let paired = paired(rows, base, base_rows);
    setting_ids(config)
        .into_iter()
        .map(|setting| {
            let mut counts: BTreeMap<(String, String), u32> = BTreeMap::new();
            for (i, from) in &paired {
                let row = &rows[*i];
                if row.truth != Truth::Fake || setting_id(row) != setting {
                    continue;
                }
                let to = run.evals[*i].verdict;
                if *from != to {
                    let key = (
                        verdict_name(*from).to_string(),
                        verdict_name(to).to_string(),
                    );
                    *counts.entry(key).or_default() += 1;
                }
            }
            let detection_rate_change_pp = detection_rate(rows, &run.evals, &setting)
                .zip(detection_rate(base_rows, &base.evals, &setting))
                .map(|(a, b)| a - b);
            SettingChange {
                setting,
                counts,
                detection_rate_change_pp,
            }
        })
        .collect()
}

const VERDICT_NAMES: [&str; 4] = ["real", "suspicious", "fake", "error"];

/// A row that changed verdict against the baseline.
#[derive(Debug, Clone)]
pub struct ChangedRow {
    pub index: usize,
    pub from: Verdict,
    pub to: Verdict,
}

/// Rows of the evaluation set present in both runs, as (index in `rows`, baseline verdict).
fn paired<'a>(
    rows: &'a [LabelRow],
    base: &'a ParamsRun,
    base_rows: &'a [LabelRow],
) -> Vec<(usize, Verdict)> {
    let index: BTreeMap<String, usize> = base_rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.identity(), i))
        .collect();
    rows.iter()
        .enumerate()
        .filter(|(_, r)| r.in_evaluation_set())
        .filter_map(|(i, r)| {
            index
                .get(&r.identity())
                .map(|b| (i, base.evals[*b].verdict))
        })
        .collect()
}

pub fn verdict_changes(
    run: &ParamsRun,
    rows: &[LabelRow],
    base: &ParamsRun,
    base_rows: &[LabelRow],
) -> VerdictChanges {
    let mut counts: BTreeMap<(String, String), u32> = BTreeMap::new();
    for a in VERDICT_NAMES {
        for b in VERDICT_NAMES {
            counts.insert((a.to_string(), b.to_string()), 0);
        }
    }
    for (i, from) in paired(rows, base, base_rows) {
        let key = (
            verdict_name(from).to_string(),
            verdict_name(run.evals[i].verdict).to_string(),
        );
        *counts.entry(key).or_default() += 1;
    }
    VerdictChanges {
        against: base.name.clone(),
        counts,
    }
}

/// The rows whose verdict differs from the baseline's (for `report.md`).
pub fn changed_rows(
    run: &ParamsRun,
    rows: &[LabelRow],
    base: &ParamsRun,
    base_rows: &[LabelRow],
) -> Vec<ChangedRow> {
    paired(rows, base, base_rows)
        .into_iter()
        .filter(|(i, from)| *from != run.evals[*i].verdict)
        .map(|(i, from)| ChangedRow {
            index: i,
            from,
            to: run.evals[i].verdict,
        })
        .collect()
}

/// Margin summary of a class (evaluation.md section 8).
#[derive(Debug, Clone, PartialEq)]
pub struct MarginSummary {
    pub class: String,
    pub smallest_negative_margin: Option<f64>,
    pub near_negatives: u32,
    pub near_positives: u32,
}

/// Whether a row is near a threshold (evaluation.md section 8).
pub fn is_near(row: &LabelRow, eval: &RowEval, config: &Config) -> bool {
    let Some(m) = &eval.margins else {
        return false;
    };
    let near_db = config.report.near_db;
    let nearest = m.nearest_fake_margin_db;
    let by_margin = match row.truth {
        Truth::Real => nearest > 0.0 && nearest <= near_db,
        Truth::Fake => nearest > -near_db && nearest <= 0.0,
    };
    // A negative kept from `Fake` only by the guard, at its threshold (v4: and no lossy step).
    let by_guard = row.truth == Truth::Real
        && m.full_band_margin_db == Some(0.0)
        && m.m_full_band_step_db.is_none_or(|s| s > 0.0)
        && m.m_step_fake_db
            .map_or(m.m_gap_fake_db, |s| s.min(m.m_gap_fake_db))
            <= 0.0;
    let by_step = m
        .fs_fake_range_hz
        .is_some_and(|r| r.abs() <= config.report.near_hz);
    by_margin || by_step || by_guard
}

pub fn margin_summary(run: &ParamsRun, rows: &[LabelRow], config: &Config) -> Vec<MarginSummary> {
    CLASSES
        .iter()
        .filter_map(|class| {
            let group: Vec<(&LabelRow, &RowEval)> = rows
                .iter()
                .zip(run.evals.iter())
                .filter(|(r, _)| r.class == *class)
                .collect();
            if group.is_empty() {
                return None;
            }
            let smallest = group
                .iter()
                .filter(|(r, e)| r.truth == Truth::Real && e.verdict != Verdict::Fake)
                .filter_map(|(_, e)| e.margins.as_ref().map(|m| m.nearest_fake_margin_db))
                .fold(None, |acc: Option<f64>, x| {
                    Some(acc.map_or(x, |a| a.min(x)))
                });
            let near = |truth: Truth| {
                group
                    .iter()
                    .filter(|(r, e)| r.truth == truth && is_near(r, e, config))
                    .count() as u32
            };
            Some(MarginSummary {
                class: (*class).to_string(),
                smallest_negative_margin: smallest,
                near_negatives: near(Truth::Real),
                near_positives: near(Truth::Fake),
            })
        })
        .collect()
}

/// An album of class `clean` with at least one `Fake` track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanFake {
    pub album_id: String,
    pub tracks: Vec<String>,
    /// The baseline has no `Fake` track in that album.
    pub new: bool,
}

pub fn clean_fakes(
    run: &ParamsRun,
    rows: &[LabelRow],
    baseline: Option<(&ParamsRun, &[LabelRow])>,
) -> Vec<CleanFake> {
    let mut by_album: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (row, eval) in rows.iter().zip(run.evals.iter()) {
        if row.truth == Truth::Real && row.class == "clean" && eval.verdict == Verdict::Fake {
            by_album
                .entry(row.album_id.clone())
                .or_default()
                .push(row.path.clone());
        }
    }
    let baseline_fake_albums: BTreeSet<&str> = baseline
        .map(|(base, base_rows)| {
            base_rows
                .iter()
                .zip(base.evals.iter())
                .filter(|(r, e)| r.truth == Truth::Real && e.verdict == Verdict::Fake)
                .map(|(r, _)| r.album_id.as_str())
                .collect()
        })
        .unwrap_or_default();
    by_album
        .into_iter()
        .map(|(album_id, tracks)| CleanFake {
            new: baseline.is_some() && !baseline_fake_albums.contains(album_id.as_str()),
            album_id,
            tracks,
        })
        .collect()
}

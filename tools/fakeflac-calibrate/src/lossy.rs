//! Lossy rows (lossy.md section 6): verdicts from the pure functions of `fakeflac_core::detect`, the
//! lossy confusion matrix, the per-group table with its sample-rate sub-rows, the lossy verdict
//! changes, the watch list and the chains.
//!
//! Lossy rows live apart from the lossless ones (`GroupResult::lossy_rows`), so every table of
//! evaluation.md is computed on lossless-domain rows exactly as before (lossy.md section 6.1).

use crate::cache::{codec_name, Extracted};
use crate::config::Config;
use crate::evaluate::{verdict_name, ParamsRun, RowEval};
use crate::extract::GroupResult;
use crate::model::{LabelRow, RowMeta, Truth, CLASSES};
use crate::report::{
    ChainEntry, ChainReport, LossyChangedRow, LossyCounts, LossyEvaluation, LossyGroupRow,
    LossyMatrix, LossyVerdictChanges, LossyWatchEntry, SplitSel,
};
use crate::split::Split;
use fakeflac_core::detect::{
    cutoff_estimate, decide_track, detect_track, expected_cutoff_hz, measure, track_guard,
    Measurements,
};
use fakeflac_core::{DetectionParams, SuspicionReason, TrackError, Verdict};
use std::collections::{BTreeMap, BTreeSet};

/// The three splits of every matrix.
const SPLITS: [SplitSel; 3] = [SplitSel::All, SplitSel::Tune, SplitSel::Holdout];

/// The verdict of one lossy row under one params file (lossy.md sections 4.5 and 6.4).
#[derive(Debug, Clone, PartialEq)]
pub struct LossyRowEval {
    /// `false` for rows whose codec has no table in these parameters.
    pub evaluated: bool,
    pub verdict: Verdict,
    pub reasons: Vec<SuspicionReason>,
    pub error: Option<TrackError>,
    pub estimate_kbps: Option<u32>,
    pub m: Option<Measurements>,
    /// `fc`: the reported cutoff.
    pub fc_hz: Option<f64>,
    /// `c_exp`.
    pub c_exp_hz: Option<f64>,
    /// `s = c_exp - fc`.
    pub shortfall_hz: Option<f64>,
    /// An honest generated encode whose source already ends early (lossy.md section 4.5, C9).
    pub source_limited: bool,
}

impl LossyRowEval {
    fn not_evaluated() -> LossyRowEval {
        LossyRowEval {
            evaluated: false,
            verdict: Verdict::Error,
            reasons: Vec::new(),
            error: None,
            estimate_kbps: None,
            m: None,
            fc_hz: None,
            c_exp_hz: None,
            shortfall_hz: None,
            source_limited: false,
        }
    }

    fn error(e: TrackError) -> LossyRowEval {
        LossyRowEval {
            evaluated: true,
            error: Some(e),
            ..LossyRowEval::not_evaluated()
        }
    }
}

/// The verdict, measurements and expected cutoff of a lossy row. Everything comes from
/// `fakeflac_core::detect` (ADR-0036); only the source-limited test compares two of its results.
pub fn evaluate_lossy_row(
    row: &LabelRow,
    meta: &RowMeta,
    params: &DetectionParams,
) -> LossyRowEval {
    let Some(lossy) = params.lossy.as_ref() else {
        return LossyRowEval::not_evaluated();
    };
    if !meta.not_evaluated.is_empty() {
        return LossyRowEval::not_evaluated();
    }
    // C14: excluding an Opus row for a params file without Opus rows precedes
    // extraction errors. Prefer the decoded codec; use the extension only when
    // extraction could not identify one.
    let codec = match row.extracted.as_ref() {
        Extracted::Probe(p) => p.stream.codec,
        Extracted::Error(_) => None,
    };
    let is_opus = codec.map_or_else(
        || crate::extract::is_opus(&row.path),
        |codec| codec == fakeflac_core::AudioCodec::Opus,
    );
    if is_opus && lossy.cutoffs.opus.is_none() {
        return LossyRowEval::not_evaluated();
    }
    let probe = match row.extracted.as_ref() {
        Extracted::Error(e) => return LossyRowEval::error(e.clone()),
        Extracted::Probe(p) => p,
    };
    let Some(stream) = probe.stream_info() else {
        return LossyRowEval::error(TrackError::UnsupportedFormat(
            "the stream has no codec".to_string(),
        ));
    };
    // L0, then L1b: the guards of `analyze_track`, from the same function of the core.
    if let Some(e) = track_guard(
        Some(stream.codec),
        probe.duration_ms,
        stream.declared_bitrate_kbps,
        params,
    ) {
        return LossyRowEval::error(e);
    }
    let Some(spectrum) = probe.spectrum.as_ref() else {
        // Every frame silent: the core's verdict for silence.
        let d = detect_track(None, &stream, params);
        return LossyRowEval {
            evaluated: true,
            verdict: d.verdict,
            reasons: d.reasons,
            ..LossyRowEval::not_evaluated()
        };
    };
    let m = measure(spectrum, params);
    let decision = decide_track(&stream, &m, params);
    let fc = f64::from(cutoff_estimate(&m, params).frequency_hz);
    let c_exp = expected_cutoff_hz(&stream, params);
    let shortfall = c_exp.map(|c| c - fc);
    let source_limited = meta.honest_generated
        && c_exp.is_some_and(|c| {
            source_cutoff(meta, params).is_some_and(|s| s < c - lossy.suspicious_shortfall_hz)
        });
    LossyRowEval {
        evaluated: true,
        verdict: decision.verdict,
        reasons: decision.reasons,
        error: decision.error,
        estimate_kbps: decision.estimated_source_bitrate_kbps,
        m: Some(m),
        fc_hz: Some(fc),
        c_exp_hz: c_exp,
        shortfall_hz: shortfall,
        source_limited,
    }
}

/// The cutoff the lossless rules of `params` report for the source of a generated row.
fn source_cutoff(meta: &RowMeta, params: &DetectionParams) -> Option<f64> {
    let Extracted::Probe(p) = meta.source.as_ref()?.as_ref() else {
        return None;
    };
    let spectrum = p.spectrum.as_ref()?;
    let m = measure(spectrum, params);
    Some(f64::from(cutoff_estimate(&m, params).frequency_hz))
}

/// Verdict evaluation of every lossy row of a group.
pub fn evaluate_group(group: &GroupResult, params: &DetectionParams) -> Vec<LossyRowEval> {
    group
        .lossy_rows
        .iter()
        .map(|row| {
            let meta = group.meta.get(&row.identity()).cloned().unwrap_or_default();
            evaluate_lossy_row(row, &meta, params)
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Real,
    Suspicious,
    Fake,
    Undetectable,
    SourceLimited,
    Error,
}

fn bucket(row: &LabelRow, e: &LossyRowEval) -> Bucket {
    if e.verdict == Verdict::Error {
        Bucket::Error
    } else if row.truth == Truth::Fake && row.undetectable {
        Bucket::Undetectable
    } else if e.source_limited {
        Bucket::SourceLimited
    } else {
        match e.verdict {
            Verdict::Real => Bucket::Real,
            Verdict::Suspicious => Bucket::Suspicious,
            _ => Bucket::Fake,
        }
    }
}

fn add(counts: &mut LossyCounts, b: Bucket) {
    match b {
        Bucket::Real => counts.real += 1,
        Bucket::Suspicious => counts.suspicious += 1,
        Bucket::Fake => counts.fake += 1,
        Bucket::Undetectable => counts.undetectable += 1,
        Bucket::SourceLimited => counts.source_limited += 1,
        Bucket::Error => counts.error += 1,
    }
}

fn selected(sel: SplitSel, split: Split) -> bool {
    match sel {
        SplitSel::All => true,
        SplitSel::Tune => split == Split::Tune,
        SplitSel::Holdout => split == Split::Holdout,
    }
}

/// Nearest-index percentile of values (the one of evaluation.md): `round(p * (n - 1))`.
fn percentile(values: &mut [f64], pct: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let rank = ((pct / 100.0) * (values.len() - 1) as f64).round() as usize;
    Some(values[rank.min(values.len() - 1)])
}

/// The sample-rate buckets of lossy.md sections 6.3 and 6.8, in table order.
const RATE_BUCKETS: [&str; 3] = ["44100", "48000", "other"];

/// The verdict names of the change tables, in table order.
const VERDICT_NAMES: [&str; 4] = ["real", "suspicious", "fake", "error"];

/// The sample-rate bucket of a lossy row (lossy.md sections 6.3 and 6.8); `None` when it was not
/// decoded.
fn rate_group(row: &LabelRow) -> Option<&'static str> {
    match row.sample_rate()? {
        44_100 => Some("44100"),
        48_000 => Some("48000"),
        _ => Some("other"),
    }
}

/// Everything of lossy.md section 6 for one params file with a `[lossy]` section.
pub fn lossy_tables(
    group: &GroupResult,
    evals: &[LossyRowEval],
    config: &Config,
) -> LossyEvaluation {
    let rows: Vec<(&LabelRow, &RowMeta, &LossyRowEval)> = group
        .lossy_rows
        .iter()
        .zip(evals.iter())
        .filter(|(_, e)| e.evaluated)
        .filter_map(|(r, e)| group.meta.get(&r.identity()).map(|m| (r, m, e)))
        .collect();

    let mut matrix: BTreeMap<SplitSel, LossyMatrix> = BTreeMap::new();
    for sel in SPLITS {
        let mut m = LossyMatrix::default();
        for (row, meta, e) in rows.iter().filter(|(r, _, _)| selected(sel, r.split)) {
            let b = bucket(row, e);
            match (row.truth, meta.weak) {
                (Truth::Real, _) => add(&mut m.real_row, b),
                (Truth::Fake, false) => add(&mut m.fake_row, b),
                (Truth::Fake, true) => add(&mut m.weak_row, b),
            }
        }
        matrix.insert(sel, m);
    }

    // Groups, in the order of lossy.md section 6.3. The flags: shown even without rows; followed by
    // per-sample-rate sub-rows (6.8).
    type Pick = Box<dyn Fn(&LabelRow, &RowMeta) -> bool>;
    let mut specs: Vec<(String, Pick, bool, bool)> = Vec::new();
    for entry in &config.generate.lossy {
        let id = entry.id.clone();
        specs.push((
            id.clone(),
            Box::new(move |r, _| r.origin == "generated" && r.setting == id),
            true,
            true,
        ));
    }
    for class in CLASSES {
        specs.push((
            format!("trusted {class}"),
            Box::new(move |r, _| r.origin == "trusted" && r.class == class),
            false,
            true,
        ));
    }
    specs.push((
        "confirmed_real".to_string(),
        Box::new(|r, _| r.origin == "confirmed_real"),
        false,
        true,
    ));
    // The per-upstream and per-sample-rate rows of confirmed lossy fakes hold strong labels only;
    // a weak label is counted in the `weak` row alone (6.3, 6.8).
    for up in ["aac", "opus", "mp3", "unknown"] {
        specs.push((
            format!("confirmed_fake {up}"),
            Box::new(move |r, m| r.origin == "confirmed_fake" && !m.weak && m.upstream == up),
            false,
            false,
        ));
    }
    for rate in RATE_BUCKETS {
        specs.push((
            format!("confirmed_fake {rate}"),
            Box::new(move |r, m| {
                r.origin == "confirmed_fake" && !m.weak && rate_group(r) == Some(rate)
            }),
            false,
            false,
        ));
    }
    specs.push(("weak".to_string(), Box::new(|_, m| m.weak), false, false));

    // The labelled rows a pick selects, evaluated or not.
    let labelled = |pick: &dyn Fn(&LabelRow, &RowMeta) -> bool| -> Vec<&LabelRow> {
        group
            .lossy_rows
            .iter()
            .filter(|r| group.meta.get(&r.identity()).is_some_and(|m| pick(r, m)))
            .collect()
    };
    let mut groups: Vec<LossyGroupRow> = Vec::new();
    for (name, pick, always, sub_rows) in &specs {
        let members = labelled(pick.as_ref());
        if members.is_empty() && !always {
            continue;
        }
        let positive = members.iter().any(|r| r.truth == Truth::Fake);
        groups.push(group_row(name.clone(), &rows, pick.as_ref(), positive));
        if !*sub_rows {
            continue;
        }
        for bucket in RATE_BUCKETS {
            let in_bucket = |r: &LabelRow, m: &RowMeta| pick(r, m) && rate_group(r) == Some(bucket);
            if labelled(&in_bucket).is_empty() {
                continue;
            }
            // A sub-row is positive or negative as its group (6.8).
            groups.push(group_row(
                format!("{name} @ {bucket}"),
                &rows,
                &in_bucket,
                positive,
            ));
        }
    }

    LossyEvaluation {
        matrix,
        groups,
        changes: None,
    }
}

/// One row of the per-group table (lossy.md sections 6.3 and 6.8) over the evaluated rows `pick`
/// selects.
fn group_row(
    name: String,
    rows: &[(&LabelRow, &RowMeta, &LossyRowEval)],
    pick: &dyn Fn(&LabelRow, &RowMeta) -> bool,
    positive: bool,
) -> LossyGroupRow {
    let mut counts = LossyCounts::default();
    let mut estimates: Vec<f64> = Vec::new();
    let mut fake_estimates: Vec<f64> = Vec::new();
    let mut shortfalls: Vec<f64> = Vec::new();
    let mut cutoffs: Vec<f64> = Vec::new();
    let (mut within, mut with_bound) = (0_u32, 0_u32);
    let (mut streaming, mut streaming_in_denominator) = (0_u32, 0_u32);
    for (row, meta, e) in rows.iter().filter(|(r, m, _)| pick(r, m)) {
        let b = bucket(row, e);
        add(&mut counts, b);
        if e.reasons.contains(&SuspicionReason::StreamingSampleRate) {
            streaming += 1;
            if matches!(b, Bucket::Real | Bucket::Suspicious | Bucket::Fake) {
                streaming_in_denominator += 1;
            }
        }
        if let Some(k) = e.estimate_kbps {
            estimates.push(f64::from(k));
            if positive && b == Bucket::Fake {
                fake_estimates.push(f64::from(k));
            }
            if let (true, Some(bound)) = (positive, meta.source_bitrate_kbps) {
                with_bound += 1;
                within += u32::from(k <= bound);
            }
        }
        if let Some(s) = e.shortfall_hz {
            shortfalls.push(s);
        }
        if let Some(fc) = e.fc_hz {
            cutoffs.push(fc);
        }
    }
    LossyGroupRow {
        group: name,
        counts,
        median_estimate_kbps: percentile(&mut estimates, 50.0),
        fake_median_estimate_kbps: percentile(&mut fake_estimates, 50.0),
        within_source: (with_bound > 0).then_some((within, with_bound)),
        s_median_hz: percentile(&mut shortfalls.clone(), 50.0),
        s_p5_hz: percentile(&mut shortfalls, 5.0),
        positive,
        streaming,
        streaming_in_denominator,
        fc_median_hz: percentile(&mut cutoffs.clone(), 50.0),
        fc_p5_hz: percentile(&mut cutoffs.clone(), 5.0),
        fc_p95_hz: percentile(&mut cutoffs, 95.0),
    }
}

/// The per-group row a lossy track belongs to, for the list of changed rows (lossy.md section 6.8).
fn group_label(row: &LabelRow, meta: &RowMeta) -> String {
    if meta.weak {
        return "weak".to_string();
    }
    match row.origin {
        "generated" => row.setting.clone(),
        "trusted" => format!("trusted {}", row.class),
        "confirmed_fake" => format!("confirmed_fake {}", meta.upstream),
        other => other.to_string(),
    }
}

/// The lossy verdict-change table (lossy.md section 6.8) of `evals` against `baseline_evals`, both
/// evaluations of the lossy rows of `group`.
pub fn lossy_verdict_changes(
    against: &str,
    group: &GroupResult,
    baseline_evals: &[LossyRowEval],
    evals: &[LossyRowEval],
) -> LossyVerdictChanges {
    lossy_verdict_changes_between(against, group, baseline_evals, group, evals)
}

/// [`lossy_verdict_changes`] when the baseline was evaluated on another extraction group: rows are
/// paired by identity. Only rows that both files evaluate count (strong and weak labels).
pub fn lossy_verdict_changes_between(
    against: &str,
    baseline_group: &GroupResult,
    baseline_evals: &[LossyRowEval],
    group: &GroupResult,
    evals: &[LossyRowEval],
) -> LossyVerdictChanges {
    let base: BTreeMap<String, &LossyRowEval> = baseline_group
        .lossy_rows
        .iter()
        .zip(baseline_evals.iter())
        .map(|(r, e)| (r.identity(), e))
        .collect();
    let mut counts: BTreeMap<(String, String), u32> = BTreeMap::new();
    for a in VERDICT_NAMES {
        for b in VERDICT_NAMES {
            counts.insert((a.to_string(), b.to_string()), 0);
        }
    }
    let flagged = |e: &LossyRowEval| e.reasons.contains(&SuspicionReason::StreamingSampleRate);
    let mut streaming = 0_u32;
    let mut changed = Vec::new();
    for (row, e) in group.lossy_rows.iter().zip(evals.iter()) {
        let Some(old) = base.get(&row.identity()) else {
            continue;
        };
        if !e.evaluated || !old.evaluated {
            continue;
        }
        let key = (
            verdict_name(old.verdict).to_string(),
            verdict_name(e.verdict).to_string(),
        );
        *counts.entry(key).or_default() += 1;
        if old.verdict == e.verdict {
            continue;
        }
        if flagged(e) && !flagged(old) {
            streaming += 1;
        }
        let meta = group.meta.get(&row.identity()).cloned().unwrap_or_default();
        let probe = match row.extracted.as_ref() {
            Extracted::Probe(p) => Some(p),
            Extracted::Error(_) => None,
        };
        changed.push(LossyChangedRow {
            path: row.path.clone(),
            truth: row.truth.as_str().to_string(),
            group: group_label(row, &meta),
            codec: probe.and_then(|p| p.stream.codec.map(|c| codec_name(c).to_string())),
            sample_rate: probe.map(|p| p.sample_rate),
            declared_bitrate_kbps: probe.and_then(|p| p.stream.declared_bitrate_kbps),
            fc_hz: e.fc_hz,
            shortfall_hz: e.shortfall_hz,
            step_db: e.m.as_ref().map(|m| m.step_db),
            from: old.verdict,
            to: e.verdict,
        });
    }
    LossyVerdictChanges {
        against: against.to_string(),
        counts,
        streaming,
        rows: changed,
    }
}

/// The lossy `false-positive` tracks with their measurements under one params file (lossy.md
/// section 6.7); "not evaluated" entries for params without `[lossy]`.
pub fn lossy_watch(group: &GroupResult, evals: &[LossyRowEval]) -> Vec<LossyWatchEntry> {
    let mut list: Vec<LossyWatchEntry> = group
        .lossy_rows
        .iter()
        .zip(evals.iter())
        .filter(|(r, _)| !r.watch_id.is_empty())
        .map(|(r, e)| LossyWatchEntry {
            watch_id: r.watch_id.clone(),
            path: r.path.clone(),
            evaluated: e.evaluated,
            verdict: e.verdict,
            reasons: e.reasons.clone(),
            codec: match r.extracted.as_ref() {
                Extracted::Probe(p) => p.stream.codec.map(|c| codec_name(c).to_string()),
                Extracted::Error(_) => None,
            },
            declared_bitrate_kbps: match r.extracted.as_ref() {
                Extracted::Probe(p) => p.stream.declared_bitrate_kbps,
                Extracted::Error(_) => None,
            },
            fc_hz: e.fc_hz,
            c_exp_hz: e.c_exp_hz,
            shortfall_hz: e.shortfall_hz,
            step_db: e.m.as_ref().map(|m| m.step_db),
            estimate_kbps: e.estimate_kbps,
        })
        .collect();
    list.sort_by_key(|w| {
        w.watch_id
            .trim_start_matches('W')
            .parse::<u32>()
            .unwrap_or(0)
    });
    list
}

/// Lossy rows this params file does not evaluate: all of them without `[lossy]`, the Opus tracks
/// with it (lossy.md section 6.1).
pub fn not_evaluated_count(evals: &[LossyRowEval]) -> u32 {
    evals.iter().filter(|e| !e.evaluated).count() as u32
}

/// The version of a chained track: its codec, or the one its extension implies when it was not read.
fn version_of(row: &LabelRow) -> String {
    if let Extracted::Probe(p) = row.extracted.as_ref() {
        if let Some(c) = p.stream.codec {
            return codec_name(c).to_string();
        }
    }
    let ext = row.path.rsplit_once('.').map_or("", |(_, e)| e);
    match ext.to_ascii_lowercase().as_str() {
        "wav" => "pcm",
        "flac" => "flac",
        "mp3" => "mp3",
        "m4a" => "aac",
        "opus" => "opus",
        _ => "unknown",
    }
    .to_string()
}

fn key_order(key: &str) -> (u8, u32, String) {
    match key.parse::<u32>() {
        Ok(n) => (0, n, String::new()),
        Err(_) => (1, 0, key.to_string()),
    }
}

/// Chains (lossy.md section 6.5): lossless versions are judged with the lossless rules of the same
/// params file, lossy ones with the lossy rules.
pub fn chains(
    group: &GroupResult,
    run: &ParamsRun,
    lossy_evals: &[LossyRowEval],
) -> Vec<ChainReport> {
    let mut by_chain: BTreeMap<String, Vec<ChainEntry>> = BTreeMap::new();
    let mut push = |row: &LabelRow, e: ChainEntry| {
        if let Some(meta) = group.meta.get(&row.identity()) {
            if !meta.chain_id.is_empty() {
                let mut e = e;
                e.key = meta.chain_key.clone();
                by_chain.entry(meta.chain_id.clone()).or_default().push(e);
            }
        }
    };
    let declared = |row: &LabelRow| match row.extracted.as_ref() {
        Extracted::Probe(p) => p.stream.declared_bitrate_kbps,
        Extracted::Error(_) => None,
    };
    for (row, e) in group.rows.iter().zip(run.evals.iter()) {
        push(row, lossless_entry(row, e, &run.params, declared(row)));
    }
    for (row, e) in group.lossy_rows.iter().zip(lossy_evals.iter()) {
        if !e.evaluated {
            continue;
        }
        push(
            row,
            ChainEntry {
                path: row.path.clone(),
                version: version_of(row),
                key: String::new(),
                verdict: e.verdict,
                reasons: e.reasons.clone(),
                estimate_kbps: e.estimate_kbps,
                fc_hz: e.fc_hz,
                declared_bitrate_kbps: declared(row),
                watch_id: row.watch_id.clone(),
            },
        );
    }
    by_chain
        .into_iter()
        .map(|(id, mut entries)| {
            entries.sort_by(|a, b| {
                (key_order(&a.key), &a.version, &a.path).cmp(&(
                    key_order(&b.key),
                    &b.version,
                    &b.path,
                ))
            });
            let key_list: Vec<String> = {
                let mut seen: Vec<String> = Vec::new();
                for e in &entries {
                    if !seen.contains(&e.key) {
                        seen.push(e.key.clone());
                    }
                }
                seen
            };
            let pair_groups: Vec<String> = key_list
                .iter()
                .filter(|k| {
                    let versions: Vec<&str> = entries
                        .iter()
                        .filter(|e| &e.key == *k)
                        .map(|e| e.version.as_str())
                        .collect();
                    let distinct: BTreeSet<&str> = versions.iter().copied().collect();
                    distinct.len() >= 2 && distinct.len() == versions.len()
                })
                .cloned()
                .collect();
            ChainReport {
                id,
                keys: key_list,
                pair_groups,
                entries,
            }
        })
        .collect()
}

fn lossless_entry(
    row: &LabelRow,
    e: &RowEval,
    params: &DetectionParams,
    declared: Option<u32>,
) -> ChainEntry {
    ChainEntry {
        path: row.path.clone(),
        version: version_of(row),
        key: String::new(),
        verdict: e.verdict,
        reasons: e.reasons.clone(),
        estimate_kbps: None,
        fc_hz: e
            .m
            .as_ref()
            .map(|m| f64::from(cutoff_estimate(m, params).frequency_hz)),
        declared_bitrate_kbps: declared,
        watch_id: row.watch_id.clone(),
    }
}

/// Whether a lossy row is near a threshold (lossy.md section 6.4).
pub fn is_near(e: &LossyRowEval, params: &DetectionParams, config: &Config) -> bool {
    let (Some(lossy), Some(s)) = (params.lossy.as_ref(), e.shortfall_hz) else {
        return false;
    };
    let near_hz = config.report.near_hz;
    let by_hz = (lossy.suspicious_shortfall_hz - s).abs() <= near_hz
        || (lossy.fake_shortfall_hz - s).abs() <= near_hz;
    let by_db =
        e.m.as_ref()
            .is_some_and(|m| (m.step_db - lossy.fake_min_step_db).abs() <= config.report.near_db);
    by_hz || by_db
}

//! Grid search over rule parameters (evaluation.md section 7).
//!
//! Only rule parameters (`[bands]`, `[extent]`, `[step]`, `[gaps]`, `[full_band]`, `[verdict]`) can be searched: they
//! are re-applied to cached spectra without decoding. Measurements are computed once per distinct
//! non-verdict parameter set. The search reads `tune` rows only (the caller selects them).

use crate::error::ToolError;
use crate::evaluate::nearest_margin;
use fakeflac_core::detect::{decide, measure, Measurements};
use fakeflac_core::{BandSpectra, DetectionParams, Verdict};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Tune negative (class primary, reviewed or clean).
    Negative,
    /// Generated transcode.
    GeneratedPositive,
    /// A track of `confirmed.txt` of kind `fake`.
    ConfirmedFake,
}

/// One tune row evaluated under a candidate parameter set. `Error` rows are left out by the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct PointRow {
    pub kind: RowKind,
    /// Setting id for generated positives, empty otherwise.
    pub setting: String,
    pub verdict: Verdict,
    pub undetectable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weights {
    pub fp: f64,
    #[serde(rename = "fn")]
    pub fn_: f64,
    pub suspicious_negative: f64,
    pub suspicious_positive: f64,
}

/// Verdict counts of the rows of one point, and the two hard constraints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub fake_negatives: u32,
    pub suspicious_negatives: u32,
    pub real_positives: u32,
    pub suspicious_positives: u32,
    /// `Real` among the generated positives of the `must_catch` settings.
    pub must_catch_real: u32,
    /// Confirmed fakes classified `Real`.
    pub confirmed_real: u32,
}

impl Tally {
    pub fn feasible(&self) -> bool {
        self.must_catch_real == 0 && self.confirmed_real == 0
    }

    pub fn cost(&self, w: &Weights) -> f64 {
        w.fp * f64::from(self.fake_negatives)
            + w.fn_ * f64::from(self.real_positives)
            + w.suspicious_negative * f64::from(self.suspicious_negatives)
            + w.suspicious_positive * f64::from(self.suspicious_positives)
    }
}

pub fn tally<'a>(
    rows: impl Iterator<Item = (RowKind, &'a str, Verdict)>,
    must_catch: &[String],
) -> Tally {
    let mut t = Tally::default();
    for (kind, setting, verdict) in rows {
        match (kind, verdict) {
            (RowKind::Negative, Verdict::Fake) => t.fake_negatives += 1,
            (RowKind::Negative, Verdict::Suspicious) => t.suspicious_negatives += 1,
            (RowKind::Negative, _) => {}
            (_, Verdict::Real) => {
                t.real_positives += 1;
                if kind == RowKind::ConfirmedFake {
                    t.confirmed_real += 1;
                } else if must_catch.iter().any(|s| s == setting) {
                    t.must_catch_real += 1;
                }
            }
            (_, Verdict::Suspicious) => t.suspicious_positives += 1,
            _ => {}
        }
    }
    t
}

fn tally_rows(rows: &[PointRow], must_catch: &[String]) -> Tally {
    tally(
        rows.iter()
            .filter(|r| !r.undetectable)
            .map(|r| (r.kind, r.setting.as_str(), r.verdict)),
        must_catch,
    )
}

/// Feasible: no `Real` among the generated positives of the `must_catch` settings, and no confirmed
/// fake classified `Real` (OQ-11).
pub fn is_feasible(rows: &[PointRow], must_catch: &[String]) -> bool {
    tally_rows(rows, must_catch).feasible()
}

/// `fp * #Fake(neg) + fn * #Real(pos) + suspicious_negative * #Suspicious(neg) + suspicious_positive * #Suspicious(pos)`.
/// Positives are generated positives and confirmed fakes.
pub fn cost(rows: &[PointRow], weights: &Weights) -> f64 {
    tally_rows(rows, &[]).cost(weights)
}

// ---------------------------------------------------------------------------------------------
// The grid file
// ---------------------------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGrid {
    grid_version: u32,
    baseline: String,
    must_catch: Vec<String>,
    max_points: u64,
    weights: Weights,
    values: toml::Table,
}

/// A searched parameter and its values.
#[derive(Debug, Clone, PartialEq)]
pub struct GridKey {
    pub key: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridFile {
    pub baseline: String,
    pub must_catch: Vec<String>,
    pub max_points: u64,
    pub weights: Weights,
    /// In file order: the first key varies slowest.
    pub keys: Vec<GridKey>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Float,
    Int,
}

/// Every searchable field (rule parameters) and its type.
const RULE_FIELDS: [(&str, Kind); 30] = [
    ("bands.ref_band_low_hz", Kind::Float),
    ("bands.ref_band_high_hz", Kind::Float),
    ("bands.hf_band_low_hz", Kind::Float),
    ("bands.hf_band_high_hz", Kind::Float),
    ("bands.nyquist_guard_hz", Kind::Float),
    ("bands.noise_floor_percentile", Kind::Float),
    ("extent.content_drop_db", Kind::Float),
    ("extent.content_min_width_hz", Kind::Float),
    ("step.search_low_hz", Kind::Float),
    ("step.search_top_margin_hz", Kind::Float),
    ("step.window_bands", Kind::Int),
    ("step.rebound_db", Kind::Float),
    ("gaps.edge_hz", Kind::Float),
    ("gaps.band_hz", Kind::Float),
    ("full_band.low_hz", Kind::Float),
    ("full_band.high_hz", Kind::Float),
    ("full_band.min_level_db", Kind::Float),
    ("full_band.min_burst_db", Kind::Float),
    ("full_band.max_lossy_step_db", Kind::Float),
    ("verdict.low_hf_threshold_db", Kind::Float),
    ("verdict.gap_fake_db", Kind::Float),
    ("verdict.gap_suspicious_db", Kind::Float),
    ("verdict.step_fake_db", Kind::Float),
    ("verdict.step_suspicious_db", Kind::Float),
    ("verdict.step_low_suspicious_db", Kind::Float),
    ("verdict.step_high_suspicious_db", Kind::Float),
    ("verdict.lossy_min_cutoff_hz", Kind::Float),
    ("verdict.fake_max_cutoff_hz", Kind::Float),
    ("verdict.ambiguous_max_cutoff_hz", Kind::Float),
    ("verdict.gradual_min_cutoff_hz", Kind::Float),
];

enum FieldRef<'a> {
    F(&'a mut f64),
    U(&'a mut u32),
}

fn field_ref<'a>(p: &'a mut DetectionParams, key: &str) -> Option<FieldRef<'a>> {
    use FieldRef::{F, U};
    Some(match key {
        "bands.ref_band_low_hz" => F(&mut p.bands.ref_band_low_hz),
        "bands.ref_band_high_hz" => F(&mut p.bands.ref_band_high_hz),
        "bands.hf_band_low_hz" => F(&mut p.bands.hf_band_low_hz),
        "bands.hf_band_high_hz" => F(&mut p.bands.hf_band_high_hz),
        "bands.nyquist_guard_hz" => F(&mut p.bands.nyquist_guard_hz),
        "bands.noise_floor_percentile" => F(&mut p.bands.noise_floor_percentile),
        "extent.content_drop_db" => F(&mut p.extent.content_drop_db),
        "extent.content_min_width_hz" => F(&mut p.extent.content_min_width_hz),
        "step.search_low_hz" => F(&mut p.step.search_low_hz),
        "step.search_top_margin_hz" => F(&mut p.step.search_top_margin_hz),
        "step.window_bands" => U(&mut p.step.window_bands),
        "step.rebound_db" => F(&mut p.step.rebound_db),
        "gaps.edge_hz" => F(&mut p.gaps.edge_hz),
        "gaps.band_hz" => F(&mut p.gaps.band_hz),
        "full_band.low_hz" => F(&mut p.full_band.as_mut()?.low_hz),
        "full_band.high_hz" => F(&mut p.full_band.as_mut()?.high_hz),
        "full_band.min_level_db" => F(&mut p.full_band.as_mut()?.min_level_db),
        "full_band.min_burst_db" => F(&mut p.full_band.as_mut()?.min_burst_db),
        "full_band.max_lossy_step_db" => F(p.full_band.as_mut()?.max_lossy_step_db.as_mut()?),
        "verdict.low_hf_threshold_db" => F(&mut p.verdict.low_hf_threshold_db),
        "verdict.gap_fake_db" => F(&mut p.verdict.gap_fake_db),
        "verdict.gap_suspicious_db" => F(&mut p.verdict.gap_suspicious_db),
        "verdict.step_fake_db" => F(&mut p.verdict.step_fake_db),
        "verdict.step_suspicious_db" => F(&mut p.verdict.step_suspicious_db),
        "verdict.step_low_suspicious_db" => F(&mut p.verdict.step_low_suspicious_db),
        "verdict.step_high_suspicious_db" => F(&mut p.verdict.step_high_suspicious_db),
        "verdict.lossy_min_cutoff_hz" => F(&mut p.verdict.lossy_min_cutoff_hz),
        "verdict.fake_max_cutoff_hz" => F(&mut p.verdict.fake_max_cutoff_hz),
        "verdict.ambiguous_max_cutoff_hz" => F(&mut p.verdict.ambiguous_max_cutoff_hz),
        "verdict.gradual_min_cutoff_hz" => F(&mut p.verdict.gradual_min_cutoff_hz),
        _ => return None,
    })
}

/// Sets a searched field. `false` if the key is not a rule parameter.
pub fn set_value(p: &mut DetectionParams, key: &str, value: f64) -> bool {
    match field_ref(p, key) {
        Some(FieldRef::F(r)) => {
            *r = value;
            true
        }
        Some(FieldRef::U(r)) => {
            *r = value as u32;
            true
        }
        None => false,
    }
}

pub fn get_value(p: &DetectionParams, key: &str) -> Option<f64> {
    let mut copy = p.clone();
    match field_ref(&mut copy, key)? {
        FieldRef::F(r) => Some(*r),
        FieldRef::U(r) => Some(f64::from(*r)),
    }
}

/// The invariants of evaluation.md section 7 (also checked for every params file).
pub fn invariant_violations(p: &DetectionParams) -> Vec<&'static str> {
    let v = &p.verdict;
    let mut out = Vec::new();
    if v.gap_suspicious_db > v.gap_fake_db {
        out.push("gap_suspicious_db must not exceed gap_fake_db");
    }
    if v.step_suspicious_db > v.step_fake_db {
        out.push("step_suspicious_db must not exceed step_fake_db");
    }
    if !(v.lossy_min_cutoff_hz < v.fake_max_cutoff_hz
        && v.fake_max_cutoff_hz <= v.ambiguous_max_cutoff_hz)
    {
        out.push("lossy_min_cutoff_hz < fake_max_cutoff_hz <= ambiguous_max_cutoff_hz must hold");
    }
    if p.bands.ref_band_low_hz >= p.bands.ref_band_high_hz {
        out.push("ref_band_low_hz must be below ref_band_high_hz");
    }
    if p.bands.hf_band_low_hz >= p.bands.hf_band_high_hz {
        out.push("hf_band_low_hz must be below hf_band_high_hz");
    }
    if let Some(fb) = &p.full_band {
        if !(v.fake_max_cutoff_hz < fb.low_hz && fb.low_hz < fb.high_hz) {
            out.push("fake_max_cutoff_hz < full_band.low_hz < full_band.high_hz must hold");
        }
        if let Some(max) = fb.max_lossy_step_db {
            if !(0.0 <= max && max <= v.step_fake_db) {
                out.push("0 <= full_band.max_lossy_step_db <= step_fake_db must hold");
            }
        }
    }
    out
}

/// Keys of the `[values]` table in file order (the `toml` table does not keep it).
fn values_key_order(text: &str) -> Vec<String> {
    let mut in_values = false;
    let mut prefix = String::new();
    let mut keys = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            let header = t
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            in_values = header == "values" || header.starts_with("values.");
            prefix = header
                .strip_prefix("values.")
                .map(|p| format!("{}.", p.replace('"', "")))
                .unwrap_or_default();
            continue;
        }
        if !in_values || t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = t.split_once('=') {
            let name = name.trim().trim_matches('"').trim_matches('\'').trim();
            let name = format!("{prefix}{name}");
            if name.len() > prefix.len() && !keys.contains(&name) {
                keys.push(name);
            }
        }
    }
    keys
}

fn flatten(prefix: &str, table: &toml::Table, out: &mut BTreeMap<String, toml::Value>) {
    for (k, v) in table {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::Table(t) => flatten(&key, t, out),
            other => {
                out.insert(key, other.clone());
            }
        }
    }
}

/// Parses and validates a grid file (exit 2 on any problem). `settings` are the setting ids of the config.
pub fn parse_grid(text: &str, settings: &[String]) -> Result<GridFile, ToolError> {
    let raw: RawGrid = toml::from_str(text)
        .map_err(|e| ToolError::input_lines(e.to_string().lines().map(str::to_string).collect()))?;
    let mut errors: Vec<String> = Vec::new();
    if !matches!(raw.grid_version, 1 | 2) {
        errors.push(format!(
            "grid_version {} is not supported (expected 1 or 2)",
            raw.grid_version
        ));
    }
    for s in &raw.must_catch {
        if !settings.contains(s) {
            errors.push(format!("must_catch: unknown setting '{s}'"));
        }
    }
    let order = values_key_order(text);
    let mut keys: Vec<GridKey> = Vec::new();
    // `fft.window = [...]` (unquoted) parses as a nested table: flatten to dotted keys.
    let mut flat: BTreeMap<String, toml::Value> = BTreeMap::new();
    flatten("", &raw.values, &mut flat);
    let mut names: Vec<&String> = order.iter().collect();
    for k in flat.keys() {
        if !names.contains(&k) {
            names.push(k);
        }
    }
    for name in names {
        let Some(value) = flat.get(name) else {
            continue;
        };
        let extraction = name == "algorithm_version"
            || name.starts_with("input.")
            || name.starts_with("fft.")
            || name.starts_with("spectrogram.");
        if extraction {
            errors.push(format!(
                "{name}: extraction parameter; compare params files with accept instead"
            ));
            continue;
        }
        let Some((_, kind)) = RULE_FIELDS.iter().find(|(k, _)| k == name) else {
            errors.push(format!("{name}: unknown grid key"));
            continue;
        };
        let Some(list) = value.as_array() else {
            errors.push(format!("{name}: expected a list of values"));
            continue;
        };
        if list.is_empty() {
            errors.push(format!("{name}: the list of values is empty"));
            continue;
        }
        let mut values = Vec::new();
        for item in list {
            let number = match (kind, item) {
                (Kind::Float, toml::Value::Float(f)) => Some(*f),
                (Kind::Float, toml::Value::Integer(i)) => Some(*i as f64),
                (Kind::Int, toml::Value::Integer(i)) if *i >= 0 && *i <= i64::from(u32::MAX) => {
                    Some(*i as f64)
                }
                _ => None,
            };
            match number {
                Some(n) => values.push(n),
                None => {
                    let expected = if *kind == Kind::Int {
                        "integer"
                    } else {
                        "number"
                    };
                    errors.push(format!("{name}: expected {expected} values"));
                    break;
                }
            }
        }
        keys.push(GridKey {
            key: name.clone(),
            values,
        });
    }
    if keys.is_empty() && errors.is_empty() {
        errors.push("[values] is empty".to_string());
    }
    if errors.is_empty() {
        let total = keys
            .iter()
            .fold(1_u128, |acc, k| acc.saturating_mul(k.values.len() as u128));
        if total > u128::from(raw.max_points) {
            errors.push(format!(
                "the grid has {total} points, more than max_points ({})",
                raw.max_points
            ));
        }
    }
    if !errors.is_empty() {
        return Err(ToolError::input_lines(errors));
    }
    Ok(GridFile {
        baseline: raw.baseline,
        must_catch: raw.must_catch,
        max_points: raw.max_points,
        weights: raw.weights,
        keys,
    })
}

// ---------------------------------------------------------------------------------------------
// The search
// ---------------------------------------------------------------------------------------------

/// A tune row prepared for the search.
#[derive(Debug, Clone)]
pub struct SearchRow<'a> {
    pub kind: RowKind,
    pub setting: &'a str,
    /// `None`: silent track (verdict `silent_verdict`, no measurements).
    pub spectra: Option<&'a BandSpectra>,
    pub silent_verdict: Verdict,
    /// Holdout confirmed fake: counts for feasibility constraint (2) only, never for the cost,
    /// the ranking, the margin tie-break or constraint (1).
    pub constraint_only: bool,
}

/// One scored point (or the baseline).
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// 0 for the baseline, then 1, 2… in grid order.
    pub index: usize,
    pub values: Vec<f64>,
    pub tally: Tally,
    pub cost: f64,
    pub feasible: bool,
    /// Smallest `nearest_fake_margin_db` among tune negatives that are not `Fake` (+inf if none).
    pub min_margin: f64,
    /// Values different from the baseline.
    pub changed: usize,
}

/// Rank order of evaluation.md section 7: feasible first, lower cost, larger smallest margin,
/// fewer changed values, earlier in grid order.
pub fn rank_cmp(a: &Scored, b: &Scored) -> Ordering {
    (!a.feasible)
        .cmp(&!b.feasible)
        .then_with(|| {
            if (a.cost - b.cost).abs() < 1e-9 {
                Ordering::Equal
            } else {
                a.cost.total_cmp(&b.cost)
            }
        })
        .then_with(|| b.min_margin.total_cmp(&a.min_margin))
        .then_with(|| a.changed.cmp(&b.changed))
        .then_with(|| a.index.cmp(&b.index))
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub baseline: Scored,
    /// Every evaluated point, in grid order.
    pub points: Vec<Scored>,
    pub skipped: u32,
    /// Indices into `points`, best first (at most 10).
    pub top: Vec<usize>,
    /// Index into `points` of the proposal: the best point if it ranks strictly above the baseline.
    pub proposal: Option<usize>,
}

fn score(
    index: usize,
    params: &DetectionParams,
    values: Vec<f64>,
    changed: usize,
    rows: &[SearchRow<'_>],
    measured: &[Option<Measurements>],
    grid: &GridFile,
) -> Scored {
    let mut min_margin = f64::INFINITY;
    let verdicts: Vec<Verdict> = rows
        .iter()
        .zip(measured.iter())
        .map(|(row, m)| match m {
            Some(m) => {
                let (verdict, _) = decide(m, params);
                if row.kind == RowKind::Negative && !row.constraint_only && verdict != Verdict::Fake
                {
                    min_margin = min_margin.min(nearest_margin(m, params));
                }
                verdict
            }
            None => row.silent_verdict,
        })
        .collect();
    let mut t = tally(
        rows.iter()
            .zip(verdicts.iter())
            .filter(|(r, _)| !r.constraint_only)
            .map(|(r, v)| (r.kind, r.setting, *v)),
        &grid.must_catch,
    );
    t.confirmed_real += rows
        .iter()
        .zip(verdicts.iter())
        .filter(|(r, v)| r.constraint_only && **v == Verdict::Real)
        .count() as u32;
    Scored {
        index,
        values,
        tally: t,
        cost: t.cost(&grid.weights),
        feasible: t.feasible(),
        min_margin,
        changed,
    }
}

fn measure_rows(rows: &[SearchRow<'_>], params: &DetectionParams) -> Vec<Option<Measurements>> {
    rows.iter()
        .map(|r| r.spectra.map(|s| measure(s, params)))
        .collect()
}

/// Runs the grid search on the baseline parameters and the prepared tune rows.
pub fn search(baseline: &DetectionParams, rows: &[SearchRow<'_>], grid: &GridFile) -> SearchResult {
    let baseline_values: Vec<f64> = grid
        .keys
        .iter()
        .map(|k| get_value(baseline, &k.key).unwrap_or(f64::NAN))
        .collect();
    let base_measured = measure_rows(rows, baseline);
    let base = score(
        0,
        baseline,
        baseline_values.clone(),
        0,
        rows,
        &base_measured,
        grid,
    );

    // Enumerate the points: the first key varies slowest.
    struct Spec {
        values: Vec<f64>,
        params: DetectionParams,
        changed: usize,
    }
    let mut specs: Vec<Spec> = Vec::new();
    let mut skipped = 0_u32;
    let mut counter = vec![0_usize; grid.keys.len()];
    'outer: loop {
        let values: Vec<f64> = grid
            .keys
            .iter()
            .zip(counter.iter())
            .map(|(k, i)| k.values[*i])
            .collect();
        let mut params = baseline.clone();
        for (k, v) in grid.keys.iter().zip(values.iter()) {
            set_value(&mut params, &k.key, *v);
        }
        if invariant_violations(&params).is_empty() {
            let changed = values
                .iter()
                .zip(baseline_values.iter())
                .filter(|(a, b)| a != b)
                .count();
            specs.push(Spec {
                values,
                params,
                changed,
            });
        } else {
            skipped += 1;
        }
        // Odometer: the last key varies fastest.
        let mut pos = grid.keys.len();
        loop {
            if pos == 0 {
                break 'outer;
            }
            pos -= 1;
            counter[pos] += 1;
            if counter[pos] < grid.keys[pos].values.len() {
                break;
            }
            counter[pos] = 0;
        }
    }

    // Measurements once per distinct non-verdict parameter set.
    let mut groups: BTreeMap<Vec<u64>, Vec<usize>> = BTreeMap::new();
    for (i, spec) in specs.iter().enumerate() {
        let key: Vec<u64> = grid
            .keys
            .iter()
            .zip(spec.values.iter())
            .filter(|(k, _)| {
                // Decide-only keys do not change the measurements.
                !(k.key.starts_with("verdict.")
                    || k.key == "full_band.min_level_db"
                    || k.key == "full_band.min_burst_db"
                    || k.key == "full_band.max_lossy_step_db")
            })
            .map(|(_, v)| v.to_bits())
            .collect();
        groups.entry(key).or_default().push(i);
    }
    let mut scored: Vec<Option<Scored>> = vec![None; specs.len()];
    for indices in groups.values() {
        let Some(first) = indices.first() else {
            continue;
        };
        let measured = measure_rows(rows, &specs[*first].params);
        for i in indices {
            let spec = &specs[*i];
            scored[*i] = Some(score(
                i + 1,
                &spec.params,
                spec.values.clone(),
                spec.changed,
                rows,
                &measured,
                grid,
            ));
        }
    }
    let points: Vec<Scored> = scored.into_iter().flatten().collect();

    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|a, b| rank_cmp(&points[*a], &points[*b]));
    let proposal = order
        .first()
        .copied()
        .filter(|i| points[*i].feasible && rank_cmp(&points[*i], &base) == Ordering::Less);
    order.truncate(10);
    SearchResult {
        baseline: base,
        points,
        skipped,
        top: order,
        proposal,
    }
}

// ---------------------------------------------------------------------------------------------
// The proposal file
// ---------------------------------------------------------------------------------------------

/// A proposal: the baseline text with the changed values replaced (evaluation.md section 7).
#[derive(Debug, Clone)]
pub struct Proposal {
    pub file_name: String,
    pub text: String,
    pub params: DetectionParams,
    /// (key, old, new) as written in the `# Changed:` comment.
    pub changed: Vec<(String, String, String)>,
}

pub fn float_token(v: f64) -> String {
    format!("{v:?}")
}

fn token(key: &str, v: f64) -> String {
    if matches!(
        RULE_FIELDS.iter().find(|(k, _)| *k == key),
        Some((_, Kind::Int))
    ) {
        format!("{}", v as u32)
    } else {
        float_token(v)
    }
}

/// Replaces the value of `field` inside `section` (`None`: before the first header) on its own line,
/// keeping the rest of the line (comments included) and the comment column when possible.
fn replace_value(text: &str, section: Option<&str>, field: &str, new: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len() + 16);
    let mut current: Option<String> = None;
    let mut done = false;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let terminator = &line[body.len()..];
        let trimmed = body.trim_start();
        if trimmed.starts_with('[') {
            let name = trimmed
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            current = Some(name);
        }
        let in_section = match (section, &current) {
            (None, None) => true,
            (Some(s), Some(c)) => s == c,
            _ => false,
        };
        if !done && in_section && !trimmed.starts_with('[') {
            if let Some(rest) = trimmed.strip_prefix(field) {
                let after = rest.trim_start();
                if let Some(value_part) = after.strip_prefix('=') {
                    let indent = &body[..body.len() - trimmed.len()];
                    let value_start = value_part.trim_start();
                    let gap = &value_part[..value_part.len() - value_start.len()];
                    let end = value_start
                        .find(|c: char| c.is_whitespace() || c == '#')
                        .unwrap_or(value_start.len());
                    let old = &value_start[..end];
                    let tail = &value_start[end..];
                    let spaces = tail.len() - tail.trim_start().len();
                    let mut tail_out = tail.to_string();
                    if tail.trim_start().starts_with('#') {
                        let wanted = (spaces + old.len()).saturating_sub(new.len()).max(1);
                        tail_out = format!("{}{}", " ".repeat(wanted), tail.trim_start());
                    }
                    out.push_str(&format!(
                        "{indent}{field}{}={gap}{new}{tail_out}{terminator}",
                        &rest[..rest.len() - after.len()]
                    ));
                    done = true;
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    done.then_some(out)
}

/// Builds the proposal file text for the changed values and checks it by parsing it back.
pub fn build_proposal(
    baseline_text: &str,
    baseline: &DetectionParams,
    grid: &GridFile,
    best: &[f64],
    run_id: &str,
    version: u32,
) -> Result<Proposal, ToolError> {
    let mut expected = baseline.clone();
    expected.algorithm_version = version;
    let mut text = baseline_text.to_string();
    let mut changed = Vec::new();
    for (k, new) in grid.keys.iter().zip(best.iter()) {
        let old = get_value(baseline, &k.key).unwrap_or(f64::NAN);
        if old == *new {
            continue;
        }
        let (section, field) = k.key.split_once('.').unwrap_or(("", &k.key));
        let new_token = token(&k.key, *new);
        text = replace_value(&text, Some(section), field, &new_token).ok_or_else(|| {
            ToolError::internal(format!("error: {} not found in the baseline text", k.key))
        })?;
        set_value(&mut expected, &k.key, *new);
        changed.push((k.key.clone(), token(&k.key, old), new_token));
    }
    text =
        replace_value(&text, None, "algorithm_version", &version.to_string()).ok_or_else(|| {
            ToolError::internal("error: algorithm_version not found in the baseline text")
        })?;
    let eol = if baseline_text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let list: Vec<String> = changed
        .iter()
        .map(|(k, o, n)| format!("{k} {o} -> {n}"))
        .collect();
    let text = format!(
        "# PROPOSAL from /calibrate run {run_id}. Not approved: it needs a spec PR (G1).{eol}# Changed: {}{eol}{text}",
        list.join(", ")
    );
    let parsed: DetectionParams = toml::from_str(&text)
        .map_err(|e| ToolError::internal(format!("error: the proposal does not parse: {e}")))?;
    if parsed != expected {
        return Err(ToolError::internal(
            "error: the proposal does not equal the intended parameters",
        ));
    }
    Ok(Proposal {
        file_name: format!("detection-params.v{version}.toml"),
        text,
        params: parsed,
        changed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_fields_are_all_reachable() {
        let mut p = DetectionParams::default();
        for (key, _) in RULE_FIELDS {
            assert!(get_value(&p, key).is_some(), "{key}");
            assert!(set_value(&mut p, key, 1.0), "{key}");
        }
        assert!(!set_value(&mut p, "fft.window", 1.0));
    }

    #[test]
    fn value_replacement_keeps_the_comment() {
        let text = "a = 1\n[verdict]\ngap_fake_db = 6.0      # note\nother = 6.0\n";
        let out = replace_value(text, Some("verdict"), "gap_fake_db", "4.5").expect("found");
        assert_eq!(
            out,
            "a = 1\n[verdict]\ngap_fake_db = 4.5      # note\nother = 6.0\n"
        );
        let out = replace_value(text, None, "a", "2").expect("found");
        assert!(out.starts_with("a = 2\n[verdict]"));
    }

    #[test]
    fn order_prefers_feasible_then_cost() {
        let base = Scored {
            index: 0,
            values: vec![],
            tally: Tally::default(),
            cost: 1.0,
            feasible: true,
            min_margin: f64::INFINITY,
            changed: 0,
        };
        let mut worse = base.clone();
        worse.index = 1;
        worse.feasible = false;
        worse.cost = 0.0;
        assert_eq!(rank_cmp(&base, &worse), Ordering::Less);
        let mut tie = base.clone();
        tie.index = 2;
        assert_eq!(rank_cmp(&base, &tie), Ordering::Less);
    }
}

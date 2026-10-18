//! Structured results of a run (evaluation.md sections 3 to 9). The tool renders `summary.md`,
//! `report.md` and `tracks.csv` from these; [`crate::run::Outcome`] also returns them so that
//! tests check the figures without parsing Markdown.

use fakeflac_core::Verdict;
use std::collections::BTreeMap;

/// Which rows a table covers (inputs.md section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SplitSel {
    All,
    Tune,
    Holdout,
}

/// Verdict counts of a set of tracks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub real: u32,
    pub suspicious: u32,
    pub fake: u32,
    pub error: u32,
}

impl Counts {
    pub fn total(&self) -> u32 {
        self.real + self.suspicious + self.fake + self.error
    }
}

/// Evaluation.md section 4: truth by verdict, over the evaluation set (class `reference` left out).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConfusionMatrix {
    pub real_row: Counts,
    pub fake_row: Counts,
}

/// One row of the per-setting matrix: a setting id of the config, or `confirmed_fake`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingRow {
    pub setting: String,
    pub counts: Counts,
}

/// One row of the false-positive table (evaluation.md section 3). `tracks` excludes `Error` tracks,
/// which are counted only in `error`; `albums` counts albums with at least one such track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FpRow {
    pub origin: String,
    pub class: String,
    pub tracks: u32,
    pub albums: u32,
    pub fake: u32,
    pub suspicious: u32,
    pub error: u32,
    /// Albums with at least one `Fake` track.
    pub albums_fake: u32,
}

/// A `false-positive` track of `confirmed.txt` (evaluation.md section 5). Measurements are `None`
/// when the track has no spectra.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchEntry {
    pub watch_id: String,
    /// Relative to `FAKEFLAC_CORPUS`, `/` separators.
    pub path: String,
    pub verdict: Verdict,
    pub reasons: Vec<fakeflac_core::SuspicionReason>,
    pub gap_db: Option<f64>,
    pub step_db: Option<f64>,
    pub step_hz: Option<f64>,
    pub extent_hz: Option<f64>,
    pub high_frequency_db: Option<f64>,
    pub nearest_fake_margin_db: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetResult {
    pub id: String,
    pub split: SplitSel,
    pub pass: bool,
    pub count: u32,
}

/// Evaluation.md section 9: counts of (baseline verdict, verdict) over the evaluation set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictChanges {
    /// Name of the baseline params file.
    pub against: String,
    /// Keys are (baseline verdict, verdict) as lower-case names (`real`, `suspicious`, `fake`,
    /// `error`); all 16 pairs are present, with 0 when empty.
    pub counts: BTreeMap<(String, String), u32>,
}

/// Everything computed for one params file, in `--params` order.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub params_name: String,
    pub algorithm_version: u32,
    /// 16 hex digits of the extraction fingerprint (generation.md section 4.1).
    pub fingerprint_id: String,
    /// 16 hex digits of the content hash (evaluation.md section 9).
    pub content_hash: String,
    pub matrix: BTreeMap<SplitSel, ConfusionMatrix>,
    /// Config order, then `confirmed_fake` (split `all`).
    pub by_setting: Vec<SettingRow>,
    /// The same for each split.
    pub by_setting_split: BTreeMap<SplitSel, Vec<SettingRow>>,
    /// One row per origin present (the per-class subtotals appear only in the rendered tables).
    pub fp_rows: BTreeMap<SplitSel, Vec<FpRow>>,
    /// `k/n tracks (p %, 95 % CI a–b %), j/m albums` for class `primary`, split `all`.
    pub primary_figure: String,
    pub watch: Vec<WatchEntry>,
    pub targets: Vec<TargetResult>,
    /// `None` for the baseline (the first params file).
    pub changes: Option<VerdictChanges>,
}

/// Cache statistics per extraction fingerprint (generation.md section 4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheStats {
    pub fingerprint_id: String,
    pub hits: u32,
    pub decoded: u32,
    pub transcoded: u32,
    pub generation_failures: u32,
    pub skipped_lines: u32,
}

/// One evaluated point of the grid (evaluation.md section 7). `values` are the grid's
/// `"section.field"` keys with the point's value, in grid file order.
#[derive(Debug, Clone, PartialEq)]
pub struct PointScore {
    pub values: Vec<(String, f64)>,
    pub cost: f64,
    pub feasible: bool,
}

/// Result of `calibrate`.
#[derive(Debug, Clone, PartialEq)]
pub struct GridOutcome {
    pub baseline: PointScore,
    /// Every evaluated point, in grid order (invariant-breaking points are not included).
    pub points: Vec<PointScore>,
    pub skipped: u32,
    /// `detection-params.v<N>.toml` in the report folder, only when a proposal exists.
    pub proposal: Option<std::path::PathBuf>,
}

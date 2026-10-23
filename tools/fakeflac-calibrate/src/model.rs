//! The labelled rows of a run (inputs.md section 3): one per labelled track, corpus or generated.

use crate::cache::Extracted;
use crate::split::Split;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Truth {
    Real,
    Fake,
}

impl Truth {
    pub fn as_str(self) -> &'static str {
        match self {
            Truth::Real => "real",
            Truth::Fake => "fake",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowRoot {
    Corpus,
    Generated,
}

impl RowRoot {
    pub fn as_str(self) -> &'static str {
        match self {
            RowRoot::Corpus => "corpus",
            RowRoot::Generated => "generated",
        }
    }
}

/// A fragment of the source: start and length in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FragmentInfo {
    pub start_s: f64,
    pub duration_s: f64,
}

/// One row of `labels.csv`, with the extraction result it was labelled from.
#[derive(Debug, Clone)]
pub struct LabelRow {
    pub root: RowRoot,
    /// Relative to the root, `/` separators.
    pub path: String,
    pub truth: Truth,
    /// `trusted`, `generated` or `confirmed_fake`.
    pub origin: &'static str,
    pub trust_origin: String,
    /// `primary`, `reviewed`, `clean`, `reference` or `positive`.
    pub class: String,
    pub encoder: String,
    pub setting: String,
    pub bitrate_kbps: Option<u32>,
    pub encoder_version: String,
    pub fragment: Option<FragmentInfo>,
    pub source_path: String,
    pub album_id: String,
    pub split: Split,
    pub watch_id: String,
    pub extracted: Arc<Extracted>,
    pub undetectable: bool,
}

impl LabelRow {
    /// Same on every params file of a run: matches rows across files.
    pub fn identity(&self) -> String {
        format!(
            "{}\u{0}{}\u{0}{}",
            self.root.as_str(),
            self.path,
            self.setting
        )
    }

    /// The evaluation set is every row except class `reference` (evaluation.md section 2).
    pub fn in_evaluation_set(&self) -> bool {
        self.class != "reference"
    }

    pub fn sample_rate(&self) -> Option<u32> {
        match self.extracted.as_ref() {
            Extracted::Probe(p) => Some(p.sample_rate),
            Extracted::Error(_) => None,
        }
    }
}

/// What the lossy extension adds to a row of `labels.csv` (lossy.md section 2). Kept apart from
/// [`LabelRow`] and found through [`LabelRow::identity`], so lossless rows are built as before.
#[derive(Debug, Clone, Default)]
pub struct RowMeta {
    /// `stages`: the stage setting ids joined with `>` (generated lossy rows).
    pub stages: String,
    /// Generated cascades: the lowest bitrate before the last stage; confirmed lossy fakes: the bound.
    pub source_bitrate_kbps: Option<u32>,
    /// `aac`, `opus`, `mp3`, `unknown` (confirmed lossy fakes and generated cascades).
    pub upstream: String,
    pub weak: bool,
    pub chain_id: String,
    pub chain_key: String,
    pub split_unit: String,
    /// `opus decoder pending` for Opus tracks.
    pub not_evaluated: String,
    /// The lossless source of a generated lossy row (for the source-limited test).
    pub source: Option<Arc<Extracted>>,
    /// Honest generated encodes (the rows the source-limited test applies to).
    pub honest_generated: bool,
}

/// The key of a chain: the leading run of 1 to 3 digits of the file stem as an integer, else the
/// stem lower-cased with only its letters and digits kept (lossy.md section 6.5).
pub fn chain_key(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    if (1..=3).contains(&digits.len()) {
        if let Ok(n) = digits.parse::<u32>() {
            return n.to_string();
        }
    }
    stem.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Order of the classes in every table.
pub const CLASSES: [&str; 5] = ["primary", "reviewed", "clean", "reference", "positive"];

/// Precedence of the classes for an album that holds tracks of several (the stronger class decides).
const ALBUM_CLASS_ORDER: [&str; 4] = ["primary", "reviewed", "clean", "reference"];

/// The class of an album for the stratified split, from the classes of its tracks: the strongest
/// of primary, reviewed, clean, reference; `positive` when it holds only confirmed fakes.
pub fn album_class_of<'a>(classes: impl Iterator<Item = &'a str>) -> &'static str {
    let best = classes
        .filter_map(|c| ALBUM_CLASS_ORDER.iter().position(|x| *x == c))
        .min();
    best.map_or("positive", |i| ALBUM_CLASS_ORDER[i])
}

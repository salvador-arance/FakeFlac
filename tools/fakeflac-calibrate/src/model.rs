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

//! Grid search over rule parameters (evaluation.md section 7). Pure functions.

use fakeflac_core::Verdict;

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weights {
    pub fp: f64,
    pub fn_: f64,
    pub suspicious_negative: f64,
    pub suspicious_positive: f64,
}

/// Feasible: no `Real` among the generated positives of the `must_catch` settings, and no confirmed
/// fake classified `Real` (OQ-11).
pub fn is_feasible(_rows: &[PointRow], _must_catch: &[String]) -> bool {
    todo!("RP-11: evaluation.md section 7")
}

/// `fp * #Fake(neg) + fn * #Real(pos) + suspicious_negative * #Suspicious(neg) + suspicious_positive * #Suspicious(pos)`.
/// Positives are generated positives and confirmed fakes.
pub fn cost(_rows: &[PointRow], _weights: &Weights) -> f64 {
    todo!("RP-11: evaluation.md section 7")
}

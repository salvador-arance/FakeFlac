//! Window functions (ADR-0025). Periodic (DFT-even) form, `f64` coefficients.

use fakeflac_core::WindowKind;

/// Coefficients of a periodic window of length `n`. `kaiser_beta` is used only by `Kaiser`.
pub fn coefficients(kind: WindowKind, n: usize, kaiser_beta: f64) -> Vec<f64> {
    let _ = (kind, n, kaiser_beta);
    todo!("RF-03")
}

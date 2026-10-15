//! Window functions (ADR-0025). Periodic (DFT-even) form, `f64` coefficients.

use fakeflac_core::WindowKind;
use std::f64::consts::PI;

/// Coefficients of a periodic window of length `n`. `kaiser_beta` is used only by `Kaiser`.
pub fn coefficients(kind: WindowKind, n: usize, kaiser_beta: f64) -> Vec<f64> {
    let len = n as f64;
    (0..n)
        .map(|i| {
            let x = 2.0 * PI * i as f64 / len;
            match kind {
                WindowKind::Hann => 0.5 - 0.5 * x.cos(),
                WindowKind::BlackmanHarris4 => cosine_sum(x, [0.35875, 0.48829, 0.14128, 0.01168]),
                WindowKind::Nuttall4 => cosine_sum(x, [0.355768, 0.487396, 0.144232, 0.012604]),
                WindowKind::Kaiser => {
                    let r = 2.0 * i as f64 / len - 1.0;
                    bessel_i0(kaiser_beta * (1.0 - r * r).max(0.0).sqrt()) / bessel_i0(kaiser_beta)
                }
            }
        })
        .collect()
}

fn cosine_sum(x: f64, a: [f64; 4]) -> f64 {
    a[0] - a[1] * x.cos() + a[2] * (2.0 * x).cos() - a[3] * (3.0 * x).cos()
}

/// Modified Bessel function of the first kind, order 0 (power series).
fn bessel_i0(x: f64) -> f64 {
    let half_sq = (x / 2.0) * (x / 2.0);
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..200 {
        term *= half_sq / (k as f64 * k as f64);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

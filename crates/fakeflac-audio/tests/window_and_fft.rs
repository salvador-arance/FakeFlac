//! Tests for the FFT window and size (ADR-0025, `specs/detection/rules.md` section 1).
//! Protected after the `test(RF-03,RF-04)` commit.
#![allow(clippy::unwrap_used)]

use fakeflac_audio::{fft_size, window::coefficients};
use fakeflac_core::{DetectionParams, WindowKind};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// rules.md v2 section 1 item 4: N is the power of two nearest to sample_rate / target_bin_hz.
#[test]
fn fft_size_keeps_resolution_near_10_8_hz() {
    let target = DetectionParams::default().fft.target_bin_hz;
    assert_eq!(fft_size(44_100, target), 4096);
    assert_eq!(fft_size(48_000, target), 4096);
    assert_eq!(fft_size(88_200, target), 8192);
    assert_eq!(fft_size(96_000, target), 8192);
    assert_eq!(fft_size(192_000, target), 16_384);
}

#[test]
fn hann_is_periodic() {
    let w = coefficients(WindowKind::Hann, 4, 0.0);
    assert_eq!(w.len(), 4);
    for (got, want) in w.iter().zip([0.0, 0.5, 1.0, 0.5]) {
        assert!(close(*got, want), "{w:?}");
    }
}

#[test]
fn blackman_harris_4_endpoints_and_peak() {
    let w = coefficients(WindowKind::BlackmanHarris4, 8192, 0.0);
    assert_eq!(w.len(), 8192);
    // a0 - a1 + a2 - a3 at n = 0; 1 at the centre.
    assert!(close(w[0], 0.35875 - 0.48829 + 0.14128 - 0.01168));
    assert!(close(w[4096], 1.0));
}

#[test]
fn nuttall_4_peak() {
    let w = coefficients(WindowKind::Nuttall4, 1024, 0.0);
    assert!(close(w[512], 1.0));
    assert!(close(w[0], 0.355768 - 0.487396 + 0.144232 - 0.012604));
}

#[test]
fn kaiser_peak_is_one_and_symmetric() {
    let w = coefficients(WindowKind::Kaiser, 1024, 10.0);
    assert!(close(w[512], 1.0));
    for i in 1..512 {
        assert!(close(w[512 - i], w[512 + i]), "asymmetric at {i}");
    }
}

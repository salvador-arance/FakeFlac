//! Rule-level tests of `specs/detection/rules.md` (sections 2 and 3) on hand-made spectra,
//! and of the supported extensions (RF-14). Protected after the `test(RF-03,RF-04)` commit.

use fakeflac_core::detect::detect;
use fakeflac_core::{AudioFormat, AverageSpectrum, DetectionParams, SuspicionReason, Verdict};

/// Spectrum at 44.1 kHz with bins of 5.383 Hz and level `db(f)` relative to 0 dB.
fn spectrum(db: impl Fn(f64) -> f64) -> AverageSpectrum {
    let sample_rate = 44_100;
    let n = 8192usize;
    let bin_hz = f64::from(sample_rate) / n as f64;
    let power_db = (0..=n / 2).map(|k| db(k as f64 * bin_hz)).collect();
    AverageSpectrum {
        sample_rate,
        bin_hz,
        power_db,
    }
}

fn params() -> DetectionParams {
    DetectionParams::default()
}

// V6: flat full band -> real, cutoff at Nyquist
#[test]
fn flat_full_band_is_real() {
    let d = detect(Some(&spectrum(|_| 0.0)), &params());
    assert_eq!(d.verdict, Verdict::Real);
    assert!(d.reasons.is_empty());
    assert_eq!(d.cutoff.map(|c| c.frequency_hz), Some(22_050.0));
}

// V3: cliff at 16 kHz -> fake
#[test]
fn cliff_at_16k_is_fake() {
    let d = detect(
        Some(&spectrum(|f| if f < 16_000.0 { 0.0 } else { -100.0 })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Fake);
    let c = d.cutoff.expect("cutoff");
    assert!(
        (c.frequency_hz - 16_000.0).abs() <= 200.0,
        "{}",
        c.frequency_hz
    );
    assert!(c.steepness_db_per_khz >= 30.0);
}

// V4: cliff above fake_max_cutoff_hz (20.5 kHz) but below Nyquist - 1 kHz -> suspicious, ambiguous
#[test]
fn cliff_at_20_9k_is_ambiguous() {
    let d = detect(
        Some(&spectrum(|f| if f < 20_900.0 { 0.0 } else { -100.0 })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![SuspicionReason::AmbiguousCutoff]);
}

// V5: gentle roll-off reaching -60 dB below 19 kHz, no cliff -> suspicious, gradual
#[test]
fn gentle_rolloff_is_gradual() {
    let d = detect(
        Some(&spectrum(|f| {
            if f <= 8_000.0 {
                0.0
            } else {
                -200.0 * (f / 8_000.0).log10()
            }
        })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![SuspicionReason::GradualRolloff]);
}

// V2: little high-frequency content wins over a cliff: never fake only because of the cutoff
#[test]
fn low_treble_is_never_fake() {
    let d = detect(
        Some(&spectrum(|f| if f < 6_000.0 { 0.0 } else { -90.0 })),
        &params(),
    );
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![SuspicionReason::LowHighFrequencyContent]);
}

// Section 1: all frames silent
#[test]
fn silence_is_suspicious_low_hf() {
    let d = detect(None, &params());
    assert_eq!(d.verdict, Verdict::Suspicious);
    assert_eq!(d.reasons, vec![SuspicionReason::LowHighFrequencyContent]);
    assert_eq!(d.cutoff, None);
}

// The verdict depends on parameters, not constants (S4)
#[test]
fn thresholds_come_from_params() {
    let s = spectrum(|f| if f < 16_000.0 { 0.0 } else { -100.0 });
    let mut p = params();
    p.verdict.fake_max_cutoff_hz = 15_000.0;
    assert_eq!(detect(Some(&s), &p).verdict, Verdict::Suspicious);
}

// RF-14: supported extensions in H1
#[test]
fn supported_extensions() {
    assert_eq!(AudioFormat::from_extension("flac"), Some(AudioFormat::Flac));
    assert_eq!(AudioFormat::from_extension("FLAC"), Some(AudioFormat::Flac));
    assert_eq!(AudioFormat::from_extension("wav"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("Wav"), Some(AudioFormat::Wav));
    assert_eq!(AudioFormat::from_extension("mp3"), None);
    assert_eq!(AudioFormat::from_extension("txt"), None);
    assert_eq!(AudioFormat::from_extension(""), None);
}

//! Number formatting of the reports (evaluation.md section 1): rates as percentages with 1 decimal,
//! dB with 2 decimals, Hz with no decimals. Never prints `-0.0`.

fn no_negative_zero(s: String) -> String {
    match s.as_str() {
        "-0.0" => "0.0".to_string(),
        "-0.00" => "0.00".to_string(),
        "-0" => "0".to_string(),
        _ => s,
    }
}

/// A fraction as a percentage with one decimal, without the `%` sign.
pub fn pct_value(fraction: f64) -> String {
    no_negative_zero(format!("{:.1}", fraction * 100.0))
}

/// `12.3 %`, or `n/a` when the denominator is 0.
pub fn rate(count: u32, total: u32) -> String {
    if total == 0 {
        "n/a".to_string()
    } else {
        format!("{} %", pct_value(f64::from(count) / f64::from(total)))
    }
}

pub fn db(x: f64) -> String {
    no_negative_zero(format!("{x:.2}"))
}

pub fn hz(x: f64) -> String {
    no_negative_zero(format!("{x:.0}"))
}

/// Hz with 2 decimals: the lossy columns of `tracks.csv`, where margins are differences of Hz values.
pub fn hz2(x: f64) -> String {
    no_negative_zero(format!("{x:.2}"))
}

pub fn opt_hz2(x: Option<f64>) -> String {
    x.map(hz2).unwrap_or_default()
}

pub fn secs3(x: f64) -> String {
    no_negative_zero(format!("{x:.3}"))
}

pub fn opt_db(x: Option<f64>) -> String {
    x.map(db).unwrap_or_default()
}

pub fn opt_hz(x: Option<f64>) -> String {
    x.map(hz).unwrap_or_default()
}

/// Wilson score interval (evaluation.md section 3), bounds clamped to [0, 1]. `None` when `n = 0`.
pub fn wilson(k: u32, n: u32, z: f64) -> Option<(f64, f64)> {
    if n == 0 {
        return None;
    }
    let n = f64::from(n);
    let p = f64::from(k) / n;
    let denom = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denom;
    let half = z / denom * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt();
    Some(((centre - half).max(0.0), (centre + half).min(1.0)))
}

/// `k/n` as `p %, 95 % CI a–b %`, or `n/a`.
pub fn rate_with_interval(k: u32, n: u32, z: f64) -> String {
    match wilson(k, n, z) {
        None => "n/a".to_string(),
        Some((lo, hi)) => format!(
            "{} %, 95 % CI {}–{} %",
            pct_value(f64::from(k) / f64::from(n)),
            pct_value(lo),
            pct_value(hi)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_matches_the_spec_example() {
        assert_eq!(rate_with_interval(0, 2, 1.96), "0.0 %, 95 % CI 0.0–65.8 %");
        assert_eq!(rate_with_interval(0, 0, 1.96), "n/a");
    }

    #[test]
    fn no_negative_zero_is_printed() {
        assert_eq!(db(-0.001), "0.00");
        assert_eq!(hz(-0.2), "0");
        assert_eq!(pct_value(-0.0), "0.0");
    }
}

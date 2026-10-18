//! Albums and the stratified `tune`/`holdout` split (inputs.md section 4).

use crate::config::SplitConfig;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Split {
    Tune,
    Holdout,
}

/// `fmix64(fnv1a64(utf8(seed) ++ [0x0A] ++ utf8(album_id)))`.
pub fn album_hash(_seed: &str, _album_id: &str) -> u64 {
    todo!("RP-11: inputs.md section 4")
}

/// Splits the distinct albums of ONE class: sorted ascending by hash (ties by album id), the first
/// `k` go to `Holdout`, with `k = min(N, max(min_holdout_albums, round(N * holdout_fraction)))` when
/// `N >= 2` and `k = 0` when `N < 2`.
pub fn assign_class(_album_ids: &[String], _config: &SplitConfig) -> BTreeMap<String, Split> {
    todo!("RP-11: inputs.md section 4")
}

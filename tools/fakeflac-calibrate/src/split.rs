//! Albums and the stratified `tune`/`holdout` split (inputs.md section 4).

use crate::config::SplitConfig;
use crate::hash::{fmix64, fnv1a64};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Split {
    Tune,
    Holdout,
}

impl Split {
    pub fn as_str(self) -> &'static str {
        match self {
            Split::Tune => "tune",
            Split::Holdout => "holdout",
        }
    }
}

/// `fmix64(fnv1a64(utf8(seed) ++ [0x0A] ++ utf8(album_id)))`.
pub fn album_hash(seed: &str, album_id: &str) -> u64 {
    let mut bytes = Vec::with_capacity(seed.len() + 1 + album_id.len());
    bytes.extend_from_slice(seed.as_bytes());
    bytes.push(0x0A);
    bytes.extend_from_slice(album_id.as_bytes());
    fmix64(fnv1a64(&bytes))
}

/// Splits the distinct albums of ONE class: sorted ascending by hash (ties by album id), the first
/// `k` go to `Holdout`. With `f = 0` or `N < 2`, `k = round(N * f)` (half away from zero); otherwise
/// `k = min(N, max(min_holdout_albums, round(N * f)))`.
pub fn assign_class(album_ids: &[String], config: &SplitConfig) -> BTreeMap<String, Split> {
    let distinct: BTreeSet<&String> = album_ids.iter().collect();
    let mut hashed: Vec<(u64, &String)> = distinct
        .into_iter()
        .map(|id| (album_hash(&config.seed, id), id))
        .collect();
    hashed.sort();
    let n = hashed.len();
    let rounded = (n as f64 * config.holdout_fraction).round() as usize;
    let k = if config.holdout_fraction == 0.0 || n < 2 {
        rounded
    } else {
        n.min(rounded.max(config.min_holdout_albums as usize))
    };
    hashed
        .into_iter()
        .enumerate()
        .map(|(i, (_, id))| {
            let split = if i < k { Split::Holdout } else { Split::Tune };
            (id.clone(), split)
        })
        .collect()
}

/// Whether a folder name is a disc folder: one of the prefixes (ignoring case), an optional
/// ` `, `-`, `_` or `.`, then 1 to 3 digits, and nothing else.
pub fn is_disc_folder(name: &str, prefixes: &[String]) -> bool {
    let lower = name.to_lowercase();
    prefixes.iter().any(|prefix| {
        let prefix = prefix.to_lowercase();
        let Some(rest) = lower.strip_prefix(prefix.as_str()) else {
            return false;
        };
        let rest = rest.strip_prefix([' ', '-', '_', '.']).unwrap_or(rest);
        (1..=3).contains(&rest.chars().count()) && rest.chars().all(|c| c.is_ascii_digit())
    })
}

/// Album id of a track from its path relative to the corpus (`/` separators): the folder that holds
/// it, or that folder's parent when its name is a disc folder. `.` for the corpus root.
pub fn album_id(rel_path: &str, disc_prefixes: &[String]) -> String {
    let mut folders: Vec<&str> = rel_path.split('/').collect();
    folders.pop();
    if folders
        .last()
        .is_some_and(|name| is_disc_folder(name, disc_prefixes))
    {
        folders.pop();
    }
    if folders.is_empty() {
        ".".to_string()
    } else {
        folders.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefixes() -> Vec<String> {
        ["cd", "disc", "disk", "disco"]
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }

    #[test]
    fn disc_folders() {
        let p = prefixes();
        for name in ["CD1", "cd 2", "Disc-3", "Disco 1", "disk_12", "CD.7"] {
            assert!(is_disc_folder(name, &p), "{name}");
        }
        for name in ["CD1 Bonus", "CD", "CD1234", "Discography", "Album"] {
            assert!(!is_disc_folder(name, &p), "{name}");
        }
    }

    #[test]
    fn album_ids() {
        let p = prefixes();
        assert_eq!(album_id("a/b/CD1/x.flac", &p), "a/b");
        assert_eq!(album_id("a/b/x.flac", &p), "a/b");
        assert_eq!(album_id("x.flac", &p), ".");
        assert_eq!(album_id("CD1/x.flac", &p), ".");
    }
}

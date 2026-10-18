//! Tests derived from `specs/features/calibration.feature` (@RP-11): hashes and the stratified
//! `tune`/`holdout` split (`specs/calibration/inputs.md` section 4). Protected after the
//! `test(RP-11)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::config::SplitConfig;
use fakeflac_calibrate::hash::{fmix64, fnv1a64, id16};
use fakeflac_calibrate::split::{album_hash, assign_class, Split};
use fakeflac_calibrate::Config;

fn split_config() -> SplitConfig {
    Config::parse(&read(&spec_config())).unwrap().split
}

fn ids(prefix: &str, n: usize, width: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}-{i:0width$}")).collect()
}

fn holdout(map: &std::collections::BTreeMap<String, Split>) -> usize {
    map.values().filter(|s| **s == Split::Holdout).count()
}

// Hash functions of inputs.md section 4 (support for the split scenarios; reference values from the
// published FNV-1a and MurmurHash3 definitions).
#[test]
fn rp11_hash_functions_match_their_definitions() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fmix64(0), 0);
    assert_eq!(fmix64(1), 0xb456_bcfc_34c2_cb2c);
    assert_eq!(id16(b"a"), format!("{:016x}", fmix64(fnv1a64(b"a"))));
    assert_eq!(id16(b"a").len(), 16);
}

// Scenario: Deterministic split by album (@RP-11), the hash of an album id: seed, 0x0A, album id.
#[test]
fn rp11_album_hash_uses_seed_newline_album() {
    let seed = "fakeflac-calibration-1";
    assert_eq!(album_hash(seed, "album-00000"), 0xa959_e400_d27d_9b8e);
    assert_eq!(album_hash(seed, "album-10000"), 0x4cb5_610d_eb03_9cb2);
    assert_eq!(album_hash(seed, "."), 0xa2ed_7a34_df8b_a740);
}

// Scenario: Split ratio and stability (@RP-11)
#[test]
fn rp11_split_ratio_and_stability() {
    let config = split_config();
    let albums = ids("album", 10_000, 5);
    let split = assign_class(&albums, &config);
    assert_eq!(split.len(), 10_000);
    assert_eq!(holdout(&split), 3000, "exactly 3000 albums in holdout");

    // Adding "album-10000" inserts it into the hash order; no existing album changes its hash and
    // the holdout size stays round(10001 * 0.30) = 3000. Because the holdout is the first k albums of
    // the order, the new album, when it sorts among them, pushes the album at the boundary out: that
    // is the only possible change (see the report: the scenario text says "none").
    let mut more = albums.clone();
    more.push("album-10000".to_string());
    let after = assign_class(&more, &config);
    assert_eq!(after.len(), 10_001);
    assert_eq!(holdout(&after), 3000);
    let changed: Vec<_> = albums.iter().filter(|a| split[*a] != after[*a]).collect();
    assert!(
        changed.len() <= 1,
        "at most the album at the boundary moves: {changed:?}"
    );
    for a in &changed {
        assert_eq!(split[*a], Split::Holdout);
        assert_eq!(after[*a], Split::Tune);
        assert_eq!(after["album-10000"], Split::Holdout);
    }
    // The order itself never changes: every album keeps the position given by its own hash.
    let seed = &config.seed;
    let mut by_hash = albums.clone();
    by_hash.sort_by_key(|a| (album_hash(seed, a), a.clone()));
    let holdout_set: std::collections::BTreeSet<_> = by_hash.iter().take(3000).collect();
    for a in &albums {
        assert_eq!(
            split[a] == Split::Holdout,
            holdout_set.contains(a),
            "{a} is in holdout iff it is among the 3000 lowest hashes"
        );
    }
}

// Scenario: Stratified split guarantees holdout in small classes (@RP-11)
#[test]
fn rp11_stratified_split_guarantees_holdout_in_small_classes() {
    let config = split_config();
    let primary = assign_class(&ids("primary", 5, 2), &config);
    assert_eq!(holdout(&primary), 2, "5 albums of class primary");
    let reviewed = assign_class(&ids("reviewed", 10, 2), &config);
    assert_eq!(holdout(&reviewed), 3, "10 albums of class reviewed");
}

// Inputs.md section 4, N < 2: a class with a single album has no holdout (support for the
// sample-corpus scenarios).
#[test]
fn rp11_a_class_with_one_album_has_no_holdout() {
    let config = split_config();
    let one = assign_class(&["only".to_string()], &config);
    assert_eq!(holdout(&one), 0);
    assert_eq!(one["only"], Split::Tune);
}

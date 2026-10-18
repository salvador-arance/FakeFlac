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

    // "album-10000" is added and the split computed again.
    let mut more = albums.clone();
    more.push("album-10000".to_string());
    let after = assign_class(&more, &config);
    assert_eq!(after.len(), 10_001);
    assert_eq!(holdout(&after), 3000, "exactly 3000 still in holdout");
    assert_eq!(after["album-10000"], Split::Holdout);
    assert_eq!(split["album-09581"], Split::Holdout);
    assert_eq!(after["album-09581"], Split::Tune);
    let others: Vec<_> = albums
        .iter()
        .filter(|a| *a != "album-09581" && split[*a] != after[*a])
        .collect();
    assert!(
        others.is_empty(),
        "no other album changed split: {others:?}"
    );
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

// Scenario Outline: Extreme holdout fractions ignore the class size (@RP-11), default fraction 0.30:
// a class with a single album stays in tune (round(1 * 0.3) = 0).
#[test]
fn rp11_a_class_with_one_album_has_no_holdout_at_the_default_fraction() {
    let one = assign_class(&["only".to_string()], &split_config());
    assert_eq!(holdout(&one), 0);
    assert_eq!(one["only"], Split::Tune);
}

fn extreme(albums: usize, fraction: f64, expected: usize) {
    let mut config = split_config();
    config.holdout_fraction = fraction;
    let split = assign_class(&ids("primary", albums, 2), &config);
    assert_eq!(split.len(), albums);
    assert_eq!(
        holdout(&split),
        expected,
        "{albums} albums, fraction {fraction}"
    );
}

// Scenario Outline: Extreme holdout fractions ignore the class size (@RP-11), row 1 / 0.0 / 0
#[test]
fn rp11_extreme_fraction_one_album_zero() {
    extreme(1, 0.0, 0);
}

// Scenario Outline: Extreme holdout fractions ignore the class size (@RP-11), row 5 / 0.0 / 0
#[test]
fn rp11_extreme_fraction_five_albums_zero() {
    extreme(5, 0.0, 0);
}

// Scenario Outline: Extreme holdout fractions ignore the class size (@RP-11), row 1 / 0.3 / 0
#[test]
fn rp11_extreme_fraction_one_album_default() {
    extreme(1, 0.3, 0);
}

// Scenario Outline: Extreme holdout fractions ignore the class size (@RP-11), row 1 / 1.0 / 1
#[test]
fn rp11_extreme_fraction_one_album_all() {
    extreme(1, 1.0, 1);
}

// Scenario Outline: Extreme holdout fractions ignore the class size (@RP-11), row 5 / 1.0 / 5
#[test]
fn rp11_extreme_fraction_five_albums_all() {
    extreme(5, 1.0, 5);
}

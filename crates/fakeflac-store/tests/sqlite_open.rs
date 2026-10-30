//! SQLite adapter, section "Opening and failures" of `specs/features/result-cache.feature`
//! (@RF-11 @RNF-04, acceptance C5 of `specs/contracts/result-cache.md` section 11): what `open`
//! does with each file it may find (section 7), and a cache locked during a run (bounds F1 and
//! F2). The verdicts of every run equal those of the same run with `ResultCache::Off`.
//!
//! Readings chosen where the scenarios are open (reported in the test-stage report):
//! - "a FakeFlac cache with user_version 99" is written by the test with `rusqlite`:
//!   `application_id` "FFLC", `user_version` 99 and a table of its own with one row;
//! - "a second connection holds a write transaction" is `BEGIN IMMEDIATE` plus one write
//!   (a table `probe` with one row) on a `rusqlite` connection of the test;
//! - in "The cache is optional", the cache file of the first row is in a folder that does not
//!   exist yet (`open` creates it, section 7 step 1).
//!
//! Protected after the `test(RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::cache::{assert_no_call_after_own_failure, done_by_index, Counting, DecoderSpy, Op};
use common::{
    count, open, pragma_i64, put_file, read_only, run_opening, run_with, sidecar, tables, TempDir,
    FFLC,
};
use fakeflac_core::{CacheStatus, DetectionParams, ProgressEvent, ResultCache, Verdict};
use fakeflac_store::{OpenError, FILE_NAME};
use rusqlite::Connection;
use std::path::Path;
use std::time::Duration;

const MIXED: [&str; 4] = ["fake1.flac", "real1.flac", "real2.flac", "susp1.flac"];

fn mixed(t: &TempDir) -> std::path::PathBuf {
    let dir = t.path().join("Mixed");
    for (i, n) in MIXED.iter().enumerate() {
        put_file(&dir, n, 300 + i, i as u8);
    }
    dir
}

fn verdicts(events: &[ProgressEvent]) -> Vec<(u32, Verdict)> {
    done_by_index(events)
        .into_iter()
        .map(|(i, a)| (i, a.verdict))
        .collect()
}

/// The verdicts of the same run with `ResultCache::Off`.
fn reference(root: &Path) -> Vec<(u32, Verdict)> {
    let run = run_with(
        ResultCache::Off,
        &DecoderSpy::new(),
        &DetectionParams::default(),
        root,
        2,
    );
    verdicts(&run.events)
}

/// A FakeFlac cache of schema version 99, written by the test (journal mode `mode`).
fn newer_cache(file: &Path, mode: &str) {
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let c = Connection::open(file).unwrap();
    c.execute_batch(&format!(
        "PRAGMA journal_mode = {mode};
         PRAGMA application_id = {FFLC};
         PRAGMA user_version = 99;
         CREATE TABLE future (k TEXT PRIMARY KEY, v INTEGER);
         INSERT INTO future VALUES ('a', 1), ('b', 2);"
    ))
    .unwrap();
    c.close().unwrap();
}

/// Every table's schema and rows, as text (for "every table and row is unchanged").
fn dump(file: &Path) -> Vec<String> {
    let c = read_only(file);
    let mut out = Vec::new();
    let mut s = c
        .prepare("SELECT type, name, sql FROM sqlite_schema ORDER BY name")
        .unwrap();
    let schema: Vec<(String, String, Option<String>)> = s
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for (kind, name, sql) in schema {
        out.push(format!("{kind} {name} {sql:?}"));
        if kind == "table" {
            let mut q = c
                .prepare(&format!(
                    "SELECT quote(k), quote(v) FROM \"{name}\" ORDER BY k"
                ))
                .unwrap();
            for row in q
                .query_map([], |r| {
                    Ok(format!(
                        "{}|{}",
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?
                    ))
                })
                .unwrap()
            {
                out.push(row.unwrap());
            }
        }
    }
    out
}

fn assert_ok_run(row: &str, run: &common::Run, root: &Path, status: CacheStatus) {
    assert!(
        matches!(run.events.last(), Some(ProgressEvent::Finished { summary }) if summary.done == 4),
        "{row}: finished with done 4"
    );
    assert_eq!(verdicts(&run.events), reference(root), "{row}: verdicts");
    assert_eq!(run.summary.cache, status, "{row}: cache status");
}

// @RF-11 @RNF-04 Scenario Outline: The cache is optional: verdicts never depend on it
// Row: no cache file exists
#[test]
fn rf11_optional_cache_no_cache_file_exists() {
    let t = TempDir::new("open-none");
    let root = mixed(&t);
    let file = t.path().join("cache").join("not-yet").join(FILE_NAME);
    let (run, opened, _) = run_opening(&file, &DecoderSpy::new(), &root);
    assert_eq!(opened, Ok(()));
    assert_ok_run("no cache file", &run, &root, CacheStatus::Ok);
    assert!(file.is_file());
    assert_eq!(pragma_i64(&file, "application_id"), FFLC);
    assert_eq!(pragma_i64(&file, "user_version"), 1);
}

// Row: the cache file holds 4096 bytes of 0xAB (no SQLite header)
#[test]
fn rf11_optional_cache_garbage_bytes_are_reset() {
    let t = TempDir::new("open-garbage");
    let root = mixed(&t);
    let file = t.cache_file();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, vec![0xABu8; 4096]).unwrap();
    let (run, opened, _) = run_opening(&file, &DecoderSpy::new(), &root);
    assert_eq!(opened, Ok(()));
    assert_ok_run("garbage", &run, &root, CacheStatus::Ok);
    assert_eq!(pragma_i64(&file, "application_id"), FFLC);
    assert_eq!(pragma_i64(&file, "user_version"), 1);
    assert_eq!(count(&file, "track_result"), 4, "holding the 4 results");
}

// Row: the cache file is an SQLite database with another application_id and a table "x"
#[test]
fn rf11_optional_cache_foreign_database_is_reset() {
    let t = TempDir::new("open-foreign");
    let root = mixed(&t);
    let file = t.cache_file();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    {
        let c = Connection::open(&file).unwrap();
        c.execute_batch(
            "PRAGMA application_id = 12345;
             CREATE TABLE x (a INTEGER);
             INSERT INTO x VALUES (1);",
        )
        .unwrap();
    }
    let (run, opened, _) = run_opening(&file, &DecoderSpy::new(), &root);
    assert_eq!(opened, Ok(()));
    assert_ok_run("foreign", &run, &root, CacheStatus::Ok);
    assert_eq!(pragma_i64(&file, "application_id"), FFLC);
    assert!(
        !tables(&file).contains(&"x".to_string()),
        "table x is gone: {:?}",
        tables(&file)
    );
}

// Row: a FakeFlac cache with user_version 99 in rollback-journal mode (Q8)
#[test]
fn rf11_optional_cache_newer_schema_in_rollback_journal_mode_is_untouched() {
    let t = TempDir::new("open-newer-delete");
    let root = mixed(&t);
    let file = t.cache_file();
    newer_cache(&file, "DELETE");
    let before = std::fs::read(&file).unwrap();
    let (run, opened, _) = run_opening(&file, &DecoderSpy::new(), &root);
    assert_eq!(opened, Err(OpenError::NewerSchema { found: 99 }));
    assert_ok_run(
        "newer, rollback journal",
        &run,
        &root,
        CacheStatus::Unavailable,
    );
    assert_eq!(
        std::fs::read(&file).unwrap(),
        before,
        "the cache file is byte-identical"
    );
    assert!(!sidecar(&file, "-wal").exists(), "no -wal file");
    assert!(!sidecar(&file, "-shm").exists(), "no -shm file");
}

// Row: a FakeFlac cache with user_version 99 in WAL mode (Q8)
#[test]
fn rf11_optional_cache_newer_schema_in_wal_mode_is_untouched() {
    let t = TempDir::new("open-newer-wal");
    let root = mixed(&t);
    let file = t.cache_file();
    newer_cache(&file, "WAL");
    let before = dump(&file);
    let (run, opened, _) = run_opening(&file, &DecoderSpy::new(), &root);
    assert_eq!(opened, Err(OpenError::NewerSchema { found: 99 }));
    assert_ok_run("newer, WAL", &run, &root, CacheStatus::Unavailable);
    assert_eq!(pragma_i64(&file, "user_version"), 99);
    assert_eq!(dump(&file), before, "every table and row is unchanged");
}

// Row: the parent of the cache file is a regular file, not a folder
#[test]
fn rf11_optional_cache_parent_is_a_file() {
    let t = TempDir::new("open-parent-file");
    let root = mixed(&t);
    let blocker = t.path().join("blocker");
    std::fs::write(&blocker, b"not a folder").unwrap();
    let file = blocker.join(FILE_NAME);
    let (run, opened, _) = run_opening(&file, &DecoderSpy::new(), &root);
    assert!(matches!(opened, Err(OpenError::Failed(_))), "{opened:?}");
    assert_ok_run("parent is a file", &run, &root, CacheStatus::Unavailable);
    assert_eq!(std::fs::read(&blocker).unwrap(), b"not a folder");
}

// Row: a second connection holds a write transaction on a FakeFlac cache from before the open
// until after the run
#[test]
fn rf11_optional_cache_locked_before_the_open() {
    let t = TempDir::new("open-locked");
    let root = mixed(&t);
    let file = t.cache_file();
    drop(open(&file)); // an empty FakeFlac cache
    let holder = Connection::open(&file).unwrap();
    holder
        .execute_batch(
            "BEGIN IMMEDIATE;
             CREATE TABLE probe (k TEXT, v INTEGER);
             INSERT INTO probe VALUES ('held', 42);",
        )
        .unwrap();

    let (run, opened, open_time) = run_opening(&file, &DecoderSpy::new(), &root);
    assert!(
        opened.is_err(),
        "open fails while the write transaction is held"
    );
    assert!(
        open_time >= Duration::from_millis(1000),
        "open waited busy_timeout: {open_time:?}"
    );
    assert_ok_run("locked", &run, &root, CacheStatus::Unavailable);

    holder.execute_batch("COMMIT;").unwrap();
    drop(holder);
    let v: i64 = read_only(&file)
        .query_row("SELECT v FROM probe WHERE k = 'held'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 42, "the second connection's data is intact");
    assert_eq!(pragma_i64(&file, "application_id"), FFLC);
}

// @RF-11 @RNF-04 Scenario: A cache locked during the run costs at most one timeout per worker
// (bounds F1 and F2 of result-cache.md section 7)
#[test]
fn rf11_a_cache_locked_during_the_run_costs_at_most_one_timeout_per_worker() {
    let t = TempDir::new("locked-during-run");
    let root = t.path().join("Album A");
    let names: Vec<String> = (0..20).map(|i| format!("t{i:02}.flac")).collect();
    for (i, n) in names.iter().enumerate() {
        put_file(&root, n, 300 + i, i as u8);
    }
    let file = t.cache_file();
    let store = open(&file);
    let holder = Connection::open(&file).unwrap();
    holder
        .execute_batch(
            "BEGIN IMMEDIATE;
             CREATE TABLE probe (k TEXT, v INTEGER);
             INSERT INTO probe VALUES ('held', 1);",
        )
        .unwrap();

    let decorator = Counting::new(&store);
    let spy = DecoderSpy::new();
    let run = run_with(
        ResultCache::Store(&decorator),
        &spy,
        &DetectionParams::default(),
        &root,
        2,
    );
    holder.execute_batch("ROLLBACK;").unwrap();
    drop(holder);
    let log = decorator.log();
    drop(decorator);
    drop(store);

    let failed: Vec<_> = log.iter().filter(|c| !c.ok).collect();
    assert!(
        (1..=2).contains(&failed.len()),
        "between 1 and 2 failed calls: {}",
        failed.len()
    );
    for c in &failed {
        assert_eq!(c.op, Op::Put, "only puts fail");
        assert!(
            c.ended.duration_since(c.started) >= Duration::from_millis(1000),
            "a failed call waited busy_timeout: {:?}",
            c.ended.duration_since(c.started)
        );
    }
    assert_no_call_after_own_failure(&log);
    assert_eq!(spy.count(), 20);
    let done = done_by_index(&run.events);
    assert_eq!(done.len(), 20);
    assert!(done.iter().all(|(_, a)| a.verdict == Verdict::Real));
    assert_eq!(run.summary.cache, CacheStatus::Degraded);
}

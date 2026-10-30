//! SQLite adapter, `specs/contracts/result-cache.md` section 7 step 6 (@RF-11 @RNF-04, scenario
//! "The cache is optional: verdicts never depend on it", acceptance C5; ADR-0044 decision 2):
//! another window changes the cache file between the header check of step 4 and the write
//! transaction of step 6. Step 6 reads the header again, rolls back and fails the open: with
//! `NewerSchema` when the file became a newer FakeFlac cache, touching nothing (the WAL switch
//! of step 5 is the only change step 5 may make, and here the file is already in WAL mode), and
//! with a plain failure when it became a foreign file. Step 6 never resets.
//!
//! How the race is reproduced, with no hook in the adapter: the file is in WAL mode, and a second
//! connection (the "other window") holds `BEGIN IMMEDIATE` with its change written but not
//! committed. In WAL mode readers never block, so `open` passes steps 2 to 5 at once and its step
//! 4 sees the state before the change. Its step 6 (`BEGIN IMMEDIATE`) then waits in
//! `busy_timeout` (1000 ms). The other connection commits 300 ms after `open` starts, so step 6
//! gets the lock and reads the changed header. The test checks that `open` did wait (at least
//! 200 ms), which proves that it was already past step 4 when the change was committed.
//!
//! Protected after the `test(RNF-11, RF-11)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{count, open, pragma_i64, read_only, tables, tag, TempDir, FFLC};
use fakeflac_store::{OpenError, SqliteResultStore};
use rusqlite::Connection;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// When the other window commits, counted from the moment `open` is about to be called.
const COMMIT_AFTER: Duration = Duration::from_millis(300);
/// The least time `open` must have waited for the other window's lock.
const MIN_WAIT: Duration = Duration::from_millis(200);

/// Starts a second connection on `file` that runs `change` inside `BEGIN IMMEDIATE` and keeps
/// the transaction open; returns it.
fn other_window_holding(file: &Path, change: &str) -> Connection {
    let other = Connection::open(file).unwrap();
    other.busy_timeout(Duration::from_secs(5)).unwrap();
    other
        .execute_batch(&format!("BEGIN IMMEDIATE;\n{change}"))
        .unwrap();
    other
}

/// Opens the store on another thread while `other` holds its transaction, commits `other`
/// `COMMIT_AFTER` later and closes it, then returns what `open` gave and how long it took.
fn open_racing(file: &Path, other: Connection) -> (Result<SqliteResultStore, OpenError>, Duration) {
    let (ready_tx, ready_rx) = mpsc::channel();
    let path = file.to_path_buf();
    let opener = std::thread::spawn(move || {
        ready_tx.send(()).unwrap();
        let started = Instant::now();
        let opened = SqliteResultStore::open(&path, tag());
        (opened, started.elapsed())
    });
    ready_rx.recv().unwrap();
    std::thread::sleep(COMMIT_AFTER);
    other.execute_batch("COMMIT;").unwrap();
    other.close().unwrap();
    opener.join().expect("open must not panic")
}

fn journal_mode(file: &Path) -> String {
    read_only(file)
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap()
}

fn rows(file: &Path, table: &str) -> Vec<(String, i64)> {
    let c = read_only(file);
    let mut s = c
        .prepare(&format!("SELECT k, v FROM \"{table}\" ORDER BY k"))
        .unwrap();
    let out = s
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    out
}

// @RF-11 @RNF-04 Scenario Outline: The cache is optional: verdicts never depend on it
// (result-cache.md section 7 step 6, "If it finds a newer FakeFlac cache (another window upgraded
// the file after step 4), it rolls back and open fails with NewerSchema without touching the
// schema").
#[test]
fn rf11_step_6_finds_a_newer_schema_written_after_step_4_and_fails_with_newer_schema() {
    let t = TempDir::new("race-newer");
    let file = t.cache_file();
    drop(open(&file)); // a FakeFlac cache of schema version 1, in WAL mode
    assert_eq!(pragma_i64(&file, "user_version"), 1, "precondition");
    assert_eq!(journal_mode(&file), "wal", "precondition: WAL mode");

    let other = other_window_holding(
        &file,
        "CREATE TABLE future (k TEXT PRIMARY KEY, v INTEGER);
         INSERT INTO future VALUES ('a', 1), ('b', 2);
         PRAGMA user_version = 2;",
    );
    let (opened, waited) = open_racing(&file, other);

    assert!(
        waited >= MIN_WAIT,
        "open waited for the other window's transaction, so its step 4 saw version 1: {waited:?}"
    );
    assert_eq!(
        opened.as_ref().err(),
        Some(&OpenError::NewerSchema { found: 2 }),
        "step 6 reads the header again and fails with NewerSchema"
    );
    drop(opened);
    assert_eq!(
        pragma_i64(&file, "user_version"),
        2,
        "the schema is untouched"
    );
    assert_eq!(pragma_i64(&file, "application_id"), FFLC);
    assert_eq!(tables(&file), ["engine", "future", "track_result"]);
    assert_eq!(
        rows(&file, "future"),
        [("a".to_string(), 1), ("b".to_string(), 2)],
        "the other window's rows are intact"
    );
    assert_eq!(count(&file, "engine"), 0, "step 6 wrote nothing");
    assert_eq!(count(&file, "track_result"), 0, "step 6 wrote nothing");
    assert_eq!(journal_mode(&file), "wal");
}

// @RF-11 @RNF-04 Scenario Outline: The cache is optional: verdicts never depend on it
// (result-cache.md section 7 step 6, "step 6 never resets. If it finds a file that is not a
// FakeFlac cache, it rolls back and open fails").
#[test]
fn rf11_step_6_finds_a_foreign_file_written_after_step_4_and_fails_without_a_reset() {
    let t = TempDir::new("race-foreign");
    let file = t.cache_file();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    {
        // An empty database in WAL mode: application_id 0, user_version 0, no table.
        let c = Connection::open(&file).unwrap();
        let mode: String = c
            .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal", "precondition: WAL mode");
        c.close().unwrap();
    }
    assert!(tables(&file).is_empty(), "precondition: an empty database");

    let other = other_window_holding(
        &file,
        "PRAGMA application_id = 12345;
         CREATE TABLE x (k TEXT PRIMARY KEY, v INTEGER);
         INSERT INTO x VALUES ('kept', 7);",
    );
    let (opened, waited) = open_racing(&file, other);

    assert!(
        waited >= MIN_WAIT,
        "open waited for the other window's transaction, so its step 4 saw an empty file: {waited:?}"
    );
    assert!(
        matches!(opened, Err(OpenError::Failed(_))),
        "step 6 fails the open, it does not reset: {:?}",
        opened.as_ref().map(|_| "an opened store")
    );
    drop(opened);
    assert_eq!(
        tables(&file),
        ["x"],
        "nothing was created and nothing was reset"
    );
    assert_eq!(pragma_i64(&file, "application_id"), 12345);
    assert_eq!(pragma_i64(&file, "user_version"), 0);
    assert_eq!(
        rows(&file, "x"),
        [("kept".to_string(), 7)],
        "the foreign file is intact"
    );
}

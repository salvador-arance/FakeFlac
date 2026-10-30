---
id: SPEC-RESULT-CACHE
requirements: [RF-11, RNF-01, RNF-04, RNF-05, RNF-09]
status: proposed
---

# Result cache (RF-11)

> The app keeps every track result it computes in a SQLite database (ADR-0007) in the local app cache folder. A later run reuses the result instead of decoding the file again, when the file is the same (canonical path, size and mtime) and so is the engine that produced the result (every `DetectionParams` value, the decoder's `EXTRACTION_VERSION` and the app version; ADR-0008 as extended by ADR-0044). The cache is an optimisation: when it cannot be used, the run analyses every file as before and says so in its summary. Contract changes: `commands.md` version 7 ("Result cache (version 7)"). UI: `specs/ui/result-cache.md`. Scenarios: `specs/features/result-cache.feature` (tag `@RF-11`).

## 0. Open questions for G1

Each question has a default, and the rest of this spec is written with it. On 2026-10-30 the owner approved the defaults of every question, Q1–Q11. Changing a default changes only the sections, documents and scenarios cited in its row (scenarios are in `features/result-cache.feature`, named by their title and, for an outline, by their example row).

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| Q1 | **Location.** Should the cache live in `%LOCALAPPDATA%\io.github.fakeflac.app\results-cache.sqlite3`, Tauri's `app_cache_dir()`? The planning document says `%APPDATA%\FakeFlac`, but `%APPDATA%` is the roaming profile and `FakeFlac` is not the app identifier. | Yes, `%LOCALAPPDATA%\io.github.fakeflac.app` (ADR-0044) **Approved by the owner (2026-10-30).** | Section 5, M2 (11); `commands.md` (`clear_cache`); `docs/architecture/data-model.md`; ADR-0044 decision 3 |
| Q2 | **Error rows.** Should only `tooShort` and `unsupportedFormat` results be cached? `unreadable` (a lock, permissions, a share that is offline) and `corruptStream` would be analysed again on every run. The audio adapter maps every mid-stream I/O error, and every caught decoder panic, to `corruptStream`, so that kind may be temporary. | Yes: cache `tooShort` and `unsupportedFormat` only **Approved by the owner (2026-10-30).** | 2.2; `commands.md` ("What is not stored"); scenarios "Which analyses are cacheable" (rows with an error) and "Which results are stored" (rows with an error), "A warm run gives the cold run's events and CSV" (`fromCache 3`) |
| Q3 | **App version in the engine.** Should a new release recompute every result once, so that no bug fix shipped without a new `algorithm_version` can leave a stale verdict? | Yes (ADR-0044) **Approved by the owner (2026-10-30).** | 4, 10.2 (`EngineTag.app_version`), C4 (11); ADR-0044 decision 1; scenario "A new engine analyses everything again and keeps only itself", row "an EngineTag with another app_version" |
| Q4 | **"From cache" per row.** Should the table or the detail panel mark which rows came from the cache? That needs a flag in `trackDone` (a further contract change). | No: the status bar gives the count for the run **Approved by the owner (2026-10-30).** | `ui/result-cache.md` section 1 |
| Q5 | **Where is "Clear results cache"?** In the toolbar, in the export group? Alternatives: a settings dialog (RF-13, not specified yet) or a menu (none exists). | Toolbar, after Export CSV **Approved by the owner (2026-10-30).** | `ui/result-cache.md` sections 2 and 6; `ui/sort-filter.md` sections 2 and 13 (cross-references) |
| Q6 | **Confirm before clearing?** Clearing loses only computing time, never user data. | No confirmation **Approved by the owner (2026-10-30).** | `ui/result-cache.md` section 2.2 |
| Q7 | **Rows of files that no longer exist.** They stay until the cache is cleared (about 1 KB per file). Should they be pruned automatically (for example when they are older than N days, or when the file is gone at lookup)? | No pruning in RF-11 **Approved by the owner (2026-10-30).** | 6.4 |
| Q8 | **A cache written by a newer FakeFlac** (a higher schema version). Should it be left untouched (`unavailable`, the user can clear it), or replaced at once? | Left untouched: `open` writes nothing to it (section 7, step 4) **Approved by the owner (2026-10-30).** | 7 (row "newer"), C5 (11); ADR-0044 decision 2; scenario "The cache is optional: verdicts never depend on it", the two rows with `user_version 99` |
| Q9 | **Dependencies of `fakeflac-store`** (ADR-0044 decision 5). **(a)** `rusqlite` with feature `bundled` (ADR-0007 chose it). It adds about 1 MB to the installer and needs a C compiler in CI, which CI already has. **(b)** The `serde_json` feature `float_roundtrip` for `fakeflac-store`. `serde_json` is already a workspace dependency (`Cargo.toml`: `serde_json = "1"`, no features), and `tools/fakeflac-calibrate` already enables this feature for the same reason. Without it, `serde_json`'s float parser is not guaranteed to be correctly rounded, so the bit-for-bit round trip of 2.1 would not be guaranteed. Cargo features are unified per build, so the app build does not get the tool's feature and the store must enable it itself. | (a) **Approved by the owner (2026-10-30).** (b) **Approved by the owner (2026-10-30).** | 2.1, 10.2; ADR-0044 decision 5; scenario "Stored analyses round-trip exactly" |
| Q10 | **A foreign database at the cache path.** At open, a file at `results-cache.sqlite3` that is not a FakeFlac cache is deleted together with its `-wal` and `-shm` files, and a new cache is created. "Not a FakeFlac cache" covers an SQLite database with another `application_id`, one with `application_id` 0 that has tables, and a file SQLite rejects (`SQLITE_NOTADB`, `SQLITE_CORRUPT`). The path is in the app's own cache folder, so such a file can only get there by accident (a copy, another tool, a corruption). Should it be reset? The alternative is to leave it untouched (`unavailable`) until the user clears it, as Q8 does for a newer schema. | Yes: reset (delete and create). **Approved by the owner (2026-10-30).** | 7 (row "Not a FakeFlac cache"), C5 (11); `commands.md` ("it is replaced"); ADR-0044 decision 2; scenario "The cache is optional: verdicts never depend on it", rows "4096 bytes of 0xAB" and "another application_id and a table "x"" |
| Q11 | **Declared changes to protected tests** (section 12): tests that build an `AnalysisSummary` by hand gain the two new fields in `test(RF-11)`. Agree? | Yes **Approved by the owner (2026-10-30).** | 12 |

**Decided defaults (not questions).** They follow from approved decisions or from the scope, and are listed so that G1 sees them. The owner accepted all three on 2026-10-30 (they were questions Q10, Q11 and Q13 before the review round):
- **D1. mtime in whole seconds** (section 3). The key uses the scanner's `FileEntry.modified_unix` (the contract's `modifiedUnix`). A finer mtime would change `FileEntry` and the contract. A file rewritten within the same second, with the same size, is served from the cache: the risk ADR-0008 accepted.
- **D2. No integrity check at open** (section 7). Such a check costs time in proportion to the database size. Corruption found during a run makes it `degraded`. Corruption that SQLite reports at open is reset (Q10). In any other case the user clears the cache.
- **D3. No setting to turn the cache off** (`ui/result-cache.md` section 1). Settings are RF-13.

## 1. Scope

- **In:**
  - looking up and storing per-track results during `start_analysis`;
  - the key and its invalidation;
  - what happens when the database is missing, corrupt, locked, unwritable or written by another schema version;
  - the `clear_cache` command and its button;
  - the cache figures in `AnalysisSummary` and the status bar.
- **Out:**
  - the spectrogram, which is never cached on disk in v1 (`backend.md` section 4, ADR-0009);
  - caching spectra so that a rule-only change needs no decode (ADR-0044, "Alternatives");
  - the calibration tool's own features cache (`calibration/generation.md` section 4), which is unrelated and unchanged;
  - settings (RF-13);
  - pruning (Q7).

## 2. What is cached

### 2.1 The value

The value is the core `TrackAnalysis` exactly as the run produced it, serialised with `serde_json` using the core's `serde` derives (camelCase), nothing removed. On a hit, the use case gets back a value that is equal (`==`) to what it stored, except for two fields: `info.path` and `info.folder`. These are replaced by the current scan entry's path and folder (section 8.2), because the same file can be reached through another root or another spelling of the same path.

**Floats round-trip exactly.** The `f32` fields of `CutoffEstimate` read back bit for bit; for example 16000.4, 42.06, -101.94, 19999.998 and 1e-7 all come back unchanged. `serde_json` writes the shortest representation that round-trips. To read it back exactly, `fakeflac-store` enables `serde_json`'s `float_roundtrip` feature (Q9 b), as `calibration/generation.md` section 4.2 does for the calibration tool. A test checks it (C2).

### 2.2 Cacheable results (decided in the core)

The rule is the pure core function **`is_cacheable(analysis: &TrackAnalysis) -> bool`** (10.1). The use case calls it before each `put`, and its own unit tests check it on hand-built analyses. It returns `true` only when both of these hold:
- the verdict is not `error`, **or** the error is `tooShort` or `unsupportedFormat` (Q2);
- if there is a `cutoff`, its three values are finite. A non-finite value does not survive JSON, and the contract already sends such a value as "no cutoff".

So the use case never stores:
- an `unreadable` result;
- a `corruptStream` result, including the error that comes from a caught decoder panic (ADR-0013).

The next run analyses such files again.

### 2.3 Entries the cache cannot key

These entries are always analysed, and their results are never stored. This is never a cache failure, so it does not make the run `degraded`.
- **In the core:** an entry whose `modified_unix` is 0. The scanner writes 0 when the file system gives no mtime (`fakeflac-fs`), and with no mtime a change cannot be detected.
- **In the adapter:**
  - an entry whose path cannot be canonicalised (for example, the file disappeared);
  - an entry whose path (`entry.path`), or whose canonical path, is not valid Unicode (a Windows name with an unpaired surrogate, RNF-09).

  On these, `get` returns `Ok(None)` and `put` returns `Ok(())` and stores nothing.

**Non-Unicode names (existing behaviour).** Such files already reach the run today, and RF-11 changes nothing about it:
- `fakeflac-fs` keeps a file whose stem is not valid Unicode, as long as its extension is (`to_entry` in `crates/fakeflac-fs/src/lib.rs`);
- the core analyses it like any other;
- the DTO sends its path through `to_string_lossy`, with U+FFFD in place of the surrogate (`path_string` in `src-tauri/src/dto.rs`), as RF-10's tests already check (`src-tauri/tests/album_summary.rs`).

**A non-Unicode path is never serialised into the cache.** The adapter checks `entry.path` and the canonical path before it serialises anything. `analysis.info.path` and `info.folder` are `entry.path` and its parent (`analyze_track` copies them from the entry), so they are then valid Unicode too. Without this check, `serde_json` would fail on the path (a `PathBuf` that is not valid Unicode cannot be serialised), and that error would be a spurious `CacheError`.

## 3. Key and matching

A stored result is a **hit** for a scan entry `e` under params `p` when all four of these match:

| Part | Value | Notes |
|---|---|---|
| Path | `std::fs::canonicalize(e.path)` as a UTF-8 string, compared exactly | Done by the adapter, which does I/O; the core does none. On Windows this gives the `\\?\C:\…` or `\\?\UNC\server\share\…` form with each component spelled as it is on disk. So `C:\Music\album a\x.flac`, `C:\Music\Album A\x.flac` and `\\?\C:\Music\Album A\x.flac` give one key. There is no Unicode normalisation and no case folding of the result: two files whose names differ only in NFC and NFD form are two keys (RNF-09). |
| Size | `e.size_bytes` | From the scan, not read again |
| mtime | `e.modified_unix` (whole Unix seconds, D1) | From the scan |
| Engine | section 4 | |

There is no content hash (ADR-0008).

**Accepted risk (ADR-0008):** a file changed with the same size and the same mtime second is served from the cache.

**A file that changes while a run analyses it:** the result is stored under the size and mtime the scan read. On the next run its new mtime no longer matches, so the file is analysed again: the cache corrects itself.

**The core checks every hit again.** It accepts a returned analysis only if all of these equal the run's values:
- `algorithm_version` equals `p.algorithm_version`;
- `info.size_bytes`, `info.modified_unix` and `info.format` equal those of `e`.

Anything else is a miss, not a failure.

## 4. Engine

The engine is the triple (`params_json`, `extraction_version`, `app_version`):
- `params_json`: `serde_json::to_string(&p)`, the exact text. Its fields come in the order the struct declares them, and `None` sections appear as `null`. It contains `algorithm_version`, so a new version means a new engine (as `rules.md` and `lossy-source.md` already state). So does any changed value, even under the same version, which covers a params file chosen by the user later (RF-13). So does a new optional section, even when it is `None`.
- `extraction_version`: `fakeflac_audio::EXTRACTION_VERSION` (2 today). It is bumped on any adapter change that alters the `AudioProbe`, and on every `symphonia` upgrade (`fakeflac-audio/CLAUDE.md`).
- `app_version`: `env!("CARGO_PKG_VERSION")` of `src-tauri`, "0.0.1" today (Q3).

The composition root passes `extraction_version` and `app_version` when it opens the store (`EngineTag`, section 10.2). `params_json` comes from the `p` that the core passes to every `get` and `put`. So a store can never be opened with one set of params and used with another.

**During development** `app_version` stays "0.0.1" until a release bumps it. So a change to the detection code or to the decoder adapter, made without a new `algorithm_version` or `EXTRACTION_VERSION`, keeps hitting the rows written by the build before it. A developer who runs the app on the same folder then sees the old verdicts. After such a change, clear the cache (the toolbar button, or delete the file). This does not affect:
- the tests, because each one uses its own cache file (section 5);
- `just accept` and `just calibrate`. `tools/fakeflac-calibrate` neither calls `analyze_folder` nor depends on `fakeflac-store`. It keeps its own features cache (`calibration/generation.md` section 4), which RF-11 does not change.

## 5. Location

- **File:** `<app_cache_dir>\results-cache.sqlite3`. `<app_cache_dir>` is Tauri's `PathResolver::app_cache_dir()`, which is `%LOCALAPPDATA%\io.github.fakeflac.app` on Windows (Q1). In WAL mode SQLite adds `results-cache.sqlite3-wal` and `-shm` next to it.
- **The app creates nothing at start-up.** The folder is created, if missing, when a run first opens the store (section 7).
- **RNF-05.** The path is fixed by the composition root and never derived from the analysed root. The app never writes into an analysed folder. Opening, canonicalising and looking up never modify an audio file: `canonicalize` opens a handle without write access.
- **Tests never use the real folder.** Every test passes a cache file inside its own temporary folder (`std::env::temp_dir()`, as the existing tests do), never inside the repository or the audio corpus (ADR-0022). The functions that tests call take the cache file as an argument (section 10.3).
- If `app_cache_dir()` cannot be resolved, every run reports `cache: 'unavailable'` and `clear_cache` fails with `CACHE_FAILED`.

## 6. Storage (`fakeflac-store`)

### 6.1 Schema version 1

```sql
-- PRAGMA application_id = 1179012163;  -- 0x46464C43, "FFLC": marks a FakeFlac cache
-- PRAGMA user_version   = 1;           -- schema version
CREATE TABLE engine (
  id                 INTEGER PRIMARY KEY,
  params_json        TEXT    NOT NULL,          -- section 4
  extraction_version INTEGER NOT NULL,
  app_version        TEXT    NOT NULL,
  algorithm_version  INTEGER NOT NULL,          -- copy of the value inside params_json, for inspection only
  UNIQUE (params_json, extraction_version, app_version)
);
CREATE TABLE track_result (
  path          TEXT    NOT NULL,               -- canonical path (section 3), UTF-8, compared BINARY
  engine_id     INTEGER NOT NULL REFERENCES engine(id) ON DELETE CASCADE,
  size_bytes    INTEGER NOT NULL,
  modified_unix INTEGER NOT NULL,
  analysis_json TEXT    NOT NULL,               -- section 2.1
  stored_unix   INTEGER NOT NULL,               -- when it was stored; for inspection only, not in the contract
  PRIMARY KEY (path, engine_id)
) WITHOUT ROWID;
```

How this differs from the planning schema (section 3.6.1):
- **One JSON value instead of columns.** The planning columns missed `codec`, `channels` and `bitrateMode`, and every change to the contract would need a migration.
- **Size and mtime are outside the primary key.** A changed file replaces its row instead of adding one.
- **The engine is its own table.**
- **The schema version lives in `user_version`, not in a `meta` table.** The version is then stored in the database header, and it is set in the same transaction that creates the tables.

### 6.2 Connection

- **One connection per opened store**, guarded by a `Mutex`. A poisoned mutex is a `CacheError`.
- **Pragmas**, in the order `open` sets them (section 7):
  1. `busy_timeout = 1000` ms, **first**, right after the connection opens and before anything reads the file. So every later step of `open`, the switch to WAL included, waits up to 1000 ms for a lock that another connection holds, and so do `get` and `put`.
  2. After the header check (section 7, step 4), and only for a file that `open` goes on to use:
     - `journal_mode = WAL`. If SQLite keeps another mode (for example on a network file system), the store goes on in that mode;
     - `synchronous = NORMAL`;
     - `foreign_keys = ON`.

  These are settings of the adapter, not detection data.
- **Every `put` is one transaction** (`BEGIN IMMEDIATE`). So a result is durable as soon as its `put` returns, whatever happens afterwards (cancel, `INTERNAL`, a crash).
- **The engine is resolved in every call; the store never keeps an `engine_id` between calls.**
  - Each `put`, inside its own transaction:
    1. looks up the `engine` row of (`params_json`, `extraction_version`, `app_version`);
    2. if there is none, inserts it and deletes every other engine (and, by cascade, its rows);
    3. upserts the row.
  - Each `get` looks the engine up in the same read as the row. A `get` under an engine that is not in the table is a miss and writes nothing.
  - Reason: another FakeFlac window may replace the engine between two calls of this store. An id kept from an earlier call could then name a deleted row, and the insert would fail its foreign key, a spurious `degraded`.
- **Two windows with different engines** (for example two app versions side by side): each `put` of one window deletes the other's engine and rows. So the two windows may analyse again what the other stored, but neither ever reads a result of the other's engine, and neither gets a cache failure from it. Manual check M3 covers only two windows of the same engine.
- **A `put` for a path already stored under the engine** replaces the row (upsert on `(path, engine_id)`).

### 6.3 `get`

1. Canonicalise the path; section 2.3 applies.
2. Look the engine up (6.2: in every call).
3. Look the row up by `(path, engine_id)`. It is a hit only if `size_bytes` and `modified_unix` match.
4. Parse `analysis_json`. A row that does not parse is a miss, not a failure, and the next `put` replaces it.

`get` never writes.

### 6.4 Growth

There is one row per (canonical path, engine), and only one engine at a time. The database holds about 1 KB for each file analysed under the current engine, including files deleted or moved since (Q7). There is no `VACUUM` and no automatic pruning. Clearing the cache (section 9) deletes the files.

## 7. Opening and failures

The store is opened by the composition root **once per run**, after `start_analysis` has taken the analysis slot and before the scan. It is closed (dropped) when the run ends, before the slot is freed. So between runs the app holds no handle on the files, and another FakeFlac window or `clear_cache` can use them.

**`SqliteResultStore::open(file, engine_tag)`** does these steps in order. Nothing before step 5 writes to the database file:
1. `create_dir_all` on the parent folder.
2. Open the connection read-write, creating the file if it is missing. SQLite writes nothing at this point: a new file stays empty, and an existing file is not changed.
3. Set `busy_timeout` (6.2). This writes nothing.
4. **Header check.** In a read transaction, read `application_id`, `user_version` and whether `sqlite_schema` holds any table. Then decide with the table below:
   - a newer FakeFlac cache: close the connection and fail with `NewerSchema`;
   - not a FakeFlac cache: reset;
   - an empty file or a FakeFlac cache of version 1 or older: go on with step 5.

   This step writes nothing to the database file. Reading a file that is in WAL mode needs its `-wal` and `-shm` files, so SQLite creates them if they are missing and removes them when the connection closes. That changes no row and no header value.
5. Set `journal_mode = WAL`, `synchronous = NORMAL` and `foreign_keys = ON` (6.2). The switch to WAL may write the database header.
6. In one write transaction (`BEGIN IMMEDIATE`, so two processes cannot create or migrate at the same time), read `application_id` and `user_version` again, because another window may have created or migrated the file since step 4. Act on what is read, with the same table, with one exception: step 6 never resets. If it finds a file that is not a FakeFlac cache, it rolls back and `open` fails. If it finds a newer FakeFlac cache (another window upgraded the file after step 4), it rolls back and `open` fails with `NewerSchema` without touching the schema; in that race the switch to WAL of step 5 is the only change made to the file. Commit.

| What is found | What `open` does | Status of the run |
|---|---|---|
| No file, or an empty database (`application_id` 0, `user_version` 0, no table) | Step 6 creates schema version 1 and sets `application_id` and `user_version` | `ok` |
| A FakeFlac cache (`application_id` "FFLC") of version 1 | Nothing | `ok` |
| A FakeFlac cache of an older version (none exists before 1; later versions add scripts) | Step 6 runs the migration scripts in order, in its one transaction. If a script fails, the transaction rolls back and the database stays as it was | `ok`, or `unavailable` if the migration failed |
| A FakeFlac cache of a **newer** version | Step 4 closes the connection: nothing is written, and the file is not switched to WAL (Q8). `open` fails with `OpenError::NewerSchema { found }` | `unavailable` |
| Not a FakeFlac cache (Q10): an `application_id` other than 0 and "FFLC"; `application_id` 0 with any table, or with `user_version` greater than 0 (FakeFlac never sets one without the other); or "FFLC" with `user_version` 0, which FakeFlac never writes, because both values are set in one transaction. Also any file for which SQLite reports `SQLITE_NOTADB` or `SQLITE_CORRUPT` during steps 2–5 | **Reset:** close the connection, delete the cache files (section 9, same order), then repeat steps 2–6 once on the new empty file. A second failure fails `open` | `ok` after a reset; `unavailable` if the reset fails (for example another process holds the file open) |
| The folder or the file cannot be created or opened (for example the parent path is a file, permission denied, or the disk is full) | `open` fails | `unavailable` |
| Steps 4–6 cannot get a lock within `busy_timeout` because another connection holds a write transaction (for example another FakeFlac window). In WAL mode only step 6 waits, because readers never block | `open` fails after waiting at least `busy_timeout` (1000 ms) | `unavailable` |

**During the run**, a `CacheError` is any failure of `get` or `put` other than a miss:
- `SQLITE_BUSY` or `SQLITE_LOCKED` after `busy_timeout`;
- an I/O error, `SQLITE_FULL`, `SQLITE_READONLY` or `SQLITE_CORRUPT`;
- a poisoned mutex;
- a panic inside the adapter, which the core catches (8.2).

**After a failure the run stops using the store.** The use case keeps one failure flag per run:
- the worker whose call fails sets the flag before it does anything else;
- every worker reads the flag immediately before each `get` and each `put` (8.2).

Reading the flag and calling the store are two steps, and the adapter serialises calls on its mutex. So when one worker records a failure, another worker may still be in a call, or between its read of the flag and its call. That gives these bounds:
- **F1.** A worker makes no store call after one of its own calls has failed.
- **F2.** Each worker has at most one failed call, so a run has at most `workers` failed calls. A locked database therefore costs the run at most about one `busy_timeout` per worker, plus the wait for the adapter's mutex, not one per track.
- **F3.** Once a failure is recorded, each other worker makes at most one more store call: the one in progress, or the one between its read of the flag and its call.

How the tests check them:
- F1 and F2 are exact with any number of workers: a counting decorator of the port records the thread and the result of every call.
- F3 is checked exactly with one worker (a store fake that fails on its 3rd call gets exactly 3 calls). With more workers, the moment the flag is set cannot be seen from outside the use case, so the tests check only F1 and F2.

The run then reports `degraded`.

**Never an `AppError`.** No cache failure makes `start_analysis` fail, changes a verdict or stops a run (RNF-04). The only report is `AnalysisSummary.cache` (section 8.3), which the status bar shows (`ui/result-cache.md`). The technical detail of a failure is not in the contract. The adapter's error carries it, and the app logs nothing in RF-11.

## 8. Use case (`fakeflac-core`)

### 8.1 Port and parameter

```rust
// ports.rs
/// A failure of the result store (not a miss). The run stops using the store (result-cache.md 7).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("result cache failed: {0}")]
pub struct CacheError(pub String);

/// Result cache (RF-11). Read-only towards audio (RNF-05).
pub trait ResultStore: Send + Sync {
    /// The stored analysis of `entry` under `params`; `Ok(None)` on a miss.
    fn get(&self, entry: &FileEntry, params: &DetectionParams) -> Result<Option<TrackAnalysis>, CacheError>;
    /// Stores `analysis` as the result of `entry` under `params`.
    fn put(&self, entry: &FileEntry, params: &DetectionParams, analysis: &TrackAnalysis) -> Result<(), CacheError>;
}

// analyze_folder.rs
/// How a run uses the result cache.
pub enum ResultCache<'a> {
    /// No cache (status `off`): `analyze_folder`, tests, a future CLI.
    Off,
    /// The composition root could not open the cache (status `unavailable`).
    Unavailable,
    /// Look up and store results (status `ok`, or `degraded` after a `CacheError`).
    Store(&'a dyn ResultStore),
}
```

- The port is the `ResultStore` already listed in `backend.md` section 3 (no new port, `dependencies.md` rule 4). It has no `clear()`: clearing is an adapter function (section 9), because it must work when the database cannot be opened, and it has no domain logic.
- **New function** `analyze_folder_cached(scanner, analyzer, cache: ResultCache<'_>, params, root, workers, cancel, on_progress) -> Result<AnalysisSummary, AppError>`.
- **`analyze_folder` keeps its signature** and behaves exactly as `analyze_folder_cached` with `ResultCache::Off`. So its existing tests do not change, except for the new summary fields (section 12). The calibration tool calls neither function (section 4, "During development").
- **New function** `is_cacheable(analysis: &TrackAnalysis) -> bool` (section 2.2), public so that its rule is tested on its own.

### 8.2 Per track

The worker loop is unchanged except at the analysis step. Taking the index and sending `trackStarted` still happen under the `next` lock; counting, the album update and `trackDone` still happen under the album lock. The analysis step becomes:

1. **Lookup.** If the cache is `Store`, the run's failure flag (section 7) is clear when read just before the call, and `entry.modified_unix != 0`: call `get(entry, params)`, holding no lock of the use case.
   - `Ok(Some(a))` that passes the checks of section 3 is a **hit**. Set `a.info.path = entry.path` and `a.info.folder = entry.folder`; the analysis of the track is `a` and is not computed. Count it in `from_cache`.
   - `Ok(Some(_))` that fails the checks, or `Ok(None)`, is a miss.
   - `Err(_)`: the worker sets the failure flag at once, and the track is a miss.
2. **Miss.** `analyze_track(analyzer, entry, params)`, as today.
3. **Store.** After a miss, if the cache is `Store`, `entry.modified_unix != 0`, `is_cacheable(&analysis)` (2.2), and the failure flag is clear when read just before the call: call `put(entry, params, &analysis)`, holding no lock. On `Err(_)` the worker sets the failure flag at once, and the track is reported normally.
4. Count it and send `trackDone` exactly as today.

Further rules:
- **Panics.** Both calls are panic-guarded like the analyzer (`PanicSafe`, ADR-0013). A panic counts as a `CacheError`.
- **Events and albums.** A hit sends `trackStarted` and then `trackDone` like an analysed track. Its `TrackAnalysis` is equal to the one a fresh analysis would give (2.1). It is counted in its album exactly the same way, so every rule of commands.md "Album summary (version 5)" holds unchanged: order, album lock, totals.
- **Cancel.** The cancel token is checked before each track, as today. A track that was already running finishes its lookup, analysis and `put` before its `trackDone`. So:
  - **every track that has a `trackStarted` gets its `trackDone`**, in a cancelled run too;
  - **every analysed track whose `trackDone` was sent has been stored**, unless it is not cacheable or the cache had failed. That holds for a finished run, a cancelled one, and the tracks reported before an `INTERNAL`;
  - a track with no `trackStarted` is neither looked up nor stored.
- **Concurrency.** Workers call `get` and `put` in parallel, and the adapter serialises them on its connection (6.2). A lookup takes microseconds against a track analysis of about a second, so this lock does not slow a cold run. No use-case lock is ever held during a store call, so the store cannot deadlock with the `next` lock or the album lock.

### 8.3 Summary

`AnalysisSummary` gains two fields:
- **`from_cache: u32`**: tracks whose `trackDone` carried a cached result (`fromCache` in the contract). `from_cache ≤ done`.
- **`cache: CacheStatus`**, a new core enum `Off | Ok | Degraded | Unavailable`, serialised camelCase:
  - `off`: the run used `ResultCache::Off`;
  - `unavailable`: `ResultCache::Unavailable`;
  - `ok`: `Store` and no `CacheError`;
  - `degraded`: `Store` and at least one `CacheError`.

The other fields, `finished` / `cancelled` and `elapsedMs` (from the use case's start, so without opening the store) do not change.

## 9. Clearing (`clear_cache`)

- **`fakeflac_store::clear_cache_files(file: &Path) -> std::io::Result<()>`** deletes, in this order, `<file>-wal`, `<file>-shm` and `<file>`, each only if it exists. It stops at the first error.
  - Deleting the WAL first means a failure part-way leaves a database that is valid and only older.
  - A missing file is not an error, so clearing an absent cache succeeds.
  - It does not open the database. So it also clears a cache that is corrupt or comes from a newer schema.
  - It deletes nothing else: not the folder, and no other file in it.
- **`run::clear_cache(slot, cache_file: Option<&Path>)`** in `src-tauri` does these steps:
  1. Takes the analysis slot for its whole duration. If a run holds it, the call fails with `ANALYSIS_RUNNING` (no run can open the store while the files are deleted).
  2. With `None` (the folder could not be resolved), it fails with `CACHE_FAILED`.
  3. It calls `clear_cache_files`; an I/O error becomes `CACHE_FAILED`, with the OS message in `message`. For example, on Windows another FakeFlac window holding the file open is "being used by another process".
- **Effect.** The next run finds no file and creates an empty cache, so it analyses every file again. Nothing else changes: not the table, not the results kept for the CSV export, not the selection.

## 10. Layers

### 10.1 `fakeflac-core`

- Adds `CacheError`, `ResultStore` (ports.rs), `ResultCache`, `CacheStatus`, `analyze_folder_cached`, `is_cacheable` and the two `AnalysisSummary` fields (analyze_folder.rs).
- Adds `AppErrorCode::CacheFailed` (`CACHE_FAILED`), with `as_str` and serde like the others (error.rs).
- Re-exports the new public items from `lib.rs`.
- Does no I/O and adds no dependency.
- Decides what is cacheable (2.2), whether an entry has an mtime (2.3) and what counts as a hit (3), and keeps the run's failure flag.

### 10.2 `fakeflac-store`

- **New dependencies** (ADR-0044 decision 5):
  - `rusqlite` with feature `bundled` (Q9 a, approved by the owner);
  - `serde_json` (workspace) with feature `float_roundtrip`: `serde_json = { workspace = true, features = ["float_roundtrip"] }`, as in `tools/fakeflac-calibrate/Cargo.toml` (Q9 b, approved by the owner on 2026-10-30). Without it the exact round trip of 2.1 is not guaranteed.
- **Public items:**
  - `pub struct EngineTag { pub app_version: String, pub extraction_version: u32 }`;
  - `pub struct SqliteResultStore` with `pub fn open(file: &Path, engine: EngineTag) -> Result<SqliteResultStore, OpenError>` (section 7);
  - `impl ResultStore for SqliteResultStore`;
  - `pub enum OpenError { NewerSchema { found: u32 }, Failed(String) }`;
  - `pub fn clear_cache_files(file: &Path) -> std::io::Result<()>`;
  - `pub const FILE_NAME: &str = "results-cache.sqlite3"`;
  - `pub const SCHEMA_VERSION: u32 = 1`;
  - `pub const APPLICATION_ID: i32 = 0x4646_4C43`.
- It depends only on `fakeflac-core` (`dependencies.md` rule 2).

### 10.3 `src-tauri`

- **`lib.rs` (setup).** It resolves `app.path().app_cache_dir()` and manages `run::CacheConfig { file: Option<PathBuf> }`, set to `dir.join(fakeflac_store::FILE_NAME)`, or `None` if the folder cannot be resolved. It creates nothing.
- **`run::run_analysis_cached(slot, export, cache_file: Option<&Path>, root, send)`** does these steps:
  1. takes the slot;
  2. opens the store with `EngineTag { app_version: env!("CARGO_PKG_VERSION"), extraction_version: fakeflac_audio::EXTRACTION_VERSION }`;
  3. calls `analyze_folder_cached`, with `ResultCache::Store(&store)` if the open succeeded and `ResultCache::Unavailable` if it failed or `cache_file` is `None`;
  4. records the events for the export as `run_analysis_kept` does;
  5. drops the store, then the slot.

  `run_analysis` and `run_analysis_kept` keep their signatures and use `ResultCache::Off`, so their tests never touch a cache.
- **`run::clear_cache(slot, cache_file)`**: section 9.
- **Commands.**
  - `start_analysis` calls `run_analysis_cached` with the managed `CacheConfig`.
  - The new async command `clear_cache` runs `run::clear_cache` on a blocking thread. A failed task becomes `INTERNAL`, like the other commands.
- **DTOs** (`dto.rs`):
  - `AnalysisSummary` gains `from_cache` and `cache` (DTO enum `CacheStatus`);
  - `AppErrorCode` gains `CacheFailed`.
- It does no cache logic: opening, choosing the `ResultCache` variant and mapping errors is wiring (`src-tauri/CLAUDE.md`).

### 10.4 Frontend

- **Bindings** are regenerated (`just gen`).
- **API client:** `clearCache(): Promise<void>`. It rejects with `ApiError`, like `exportCsv`.
- **Store:** keeps the cache figures of the latest finished or cancelled run (`ui/result-cache.md`).
- **UI:** `specs/ui/result-cache.md`.

## 11. Acceptance (measurable)

Automated (CI, every test in its own temporary folder):

- **C1 (a rerun decodes nothing).** A folder of 6 real files (content irrelevant: the analyzer is a counting fake, the "decoder spy"). Run 1 uses `analyze_folder_cached` with a `SqliteResultStore` in a temporary folder. Run 2 uses a store opened again on the same file, with the same params and `EngineTag`. Run 2 makes **0** `SpectrumAnalyzer::analyze` calls; its summary has `fromCache` 6 and `cache` `ok`; every `trackDone` analysis equals run 1's for the same index. (Store crate integration test; the core use case with fake scanner and fake analyzer, and the real adapter.)
- **C2 (equal results).**
  - The round trip of 2.1 through a `SqliteResultStore` holds for stubbed analyses of every verdict, reason, codec and cacheable error, and for the float values listed there.
  - `is_cacheable` accepts and rejects exactly as 2.2 says, including non-finite cutoff values (core unit level, on hand-built analyses).
  - A real run (`src-tauri`, symphonia, fixtures) analysed cold and then warm gives `trackDone` analyses that are equal field by field.
  - The CSV document of the warm run is byte-identical to the cold run's.
- **C3 (one changed file).**
  - Changing one file's size, or its mtime by +1 s, makes run 2 call the analyzer exactly once, for that file.
  - Reaching the same files through another root (a parent folder) or a differently-cased root (Windows) gives 0 calls, with the paths of the new run in the events.
- **C4 (engine).** Each of these makes run 2 analyse every file again, and afterwards `engine` has exactly 1 row:
  - another `algorithm_version`;
  - one changed params value under the same version;
  - another `extraction_version`;
  - another `app_version`.
- **C5 (failures never cost a verdict).** For each situation of section 7 that can be reproduced in a test, the run's verdicts equal those of the same run with `ResultCache::Off`, and its `cache` status is the one in the table:
  - the folder path is a file;
  - a file of garbage bytes (reset, Q10);
  - a foreign `application_id` (reset, Q10);
  - a newer `user_version` (Q8):
    - in a cache in rollback-journal mode, the file is byte-identical afterwards and no `-wal` or `-shm` file is left;
    - in a cache in WAL mode, `user_version` is still 99 afterwards and every table and row is unchanged;
  - a second connection holding a write transaction from before the open: `unavailable`, and `open` fails after waiting at least `busy_timeout` (1000 ms);
  - a second connection that takes a write transaction after the open: `degraded`. F1 and F2 of section 7 hold: at most `workers` failed calls, and no thread makes a store call after one of its own calls has failed;
  - a store fake that fails on its 3rd call, with 1 worker: exactly 3 store calls (F3).
- **C6 (read-only).** After a cold and a warm run:
  - the audio files' size, mtime and content hash are unchanged;
  - the analysed folder holds exactly the files it held before;
  - the only files created are the cache files in the test's cache folder.
- **C7 (cancel).** A run cancelled after `k` tracks reported; a rerun makes exactly `total − k` analyzer calls, minus any of the `k` that are not cacheable.
- **C8 (clear).** After `clear_cache_files`, the three files are gone, and a rerun analyses every file again. Clearing an absent cache succeeds. `run::clear_cache` while a run holds the slot fails with `ANALYSIS_RUNNING` and deletes nothing.

Manual (release build, local corpus, before the feature PR; recorded in the PR):

- **M1 (indicative speed-up).** Analyse a folder of at least 200 tracks of the local corpus (ADR-0022; only aggregates are recorded), then analyse it again unchanged. The warm run's `elapsedMs` is at most **10 %** of the cold run's (an indicative target: a lookup costs well under 5 ms per track, against about a second to analyse one). Record both figures and the track count.
- **M2.** The cache file is in `%LOCALAPPDATA%\io.github.fakeflac.app`. Nothing is created in `%APPDATA%` or in the analysed folder. Deleting the file while the app is idle is harmless: the next run creates it again.
- **M3.** Two FakeFlac windows of the same build (so the same engine) analysing at the same time both finish with correct verdicts; at worst one of them reports a cache warning. Windows with different engines are not checked: they re-run each other's work by design (6.2).

## 12. Consequences on existing tests (declared)

### 12.1 Edits (Q11)

`AnalysisSummary` gains `from_cache` / `fromCache` and `cache`. The tests below build one field by field, so they stop compiling or type-checking. **Updating them is a declared consequence of this spec.** In the `test(RF-11)` commit the test author adds the two fields (`from_cache: 0` and `cache: CacheStatus::Off`, or `fromCache: 0` and `cache: 'off'` in TS). Nothing else in them changes, and the implementer does not touch them:
- `crates/fakeflac-core/tests/analyze_folder.rs` (`contract_serialization`);
- `src-tauri/tests/contract.rs`: `summary()`, and `summary_json()`, which gains `"fromCache": 0, "cache": "off"`. The `cancelled` event test reuses `summary_json()` and so needs no edit of its own;
- `src-tauri/tests/export_csv.rs` (`summary(…)`);
- `src/lib/testing/fixtures.ts` (`makeSummary`). `summaryOf` in `src/lib/pages/tests/sortFilterHarness.ts` builds on `makeSummary`, so it needs no edit;
- `src/lib/api/index.test.ts` (the `summary` constant).

These are the only edits.

### 12.2 Protected tests checked that need no edit

- **`Toolbar.test.ts`, the `setup` of `Toolbar.filters.test.ts` and `Toolbar.stories.ts`** render `Toolbar` without the new prop. `canClearCache` is optional with default `false`, as `canExport` is (`ui/result-cache.md` section 4), so they type-check unchanged.
- **The tab-order test of `Toolbar.filters.test.ts`** ("open, cancel (when enabled), export, the four verdict buttons, the search field") passes unchanged. It does not set `canClearCache`, so the new button is disabled. A disabled `IconButton` uses the native `disabled` attribute (`IconButton.svelte`), so it is not a tab stop; that is why the test enables Cancel. The tab order with the button enabled is checked by the new test `Toolbar.clearCache.test.ts` (`ui/result-cache.md` section 6).
- **The layout tests** "has a separator between the open/cancel group and the export group" (`Toolbar.test.ts`) and "adds a separator and the verdict group after Export CSV, and the search field last" (`Toolbar.filters.test.ts`) still hold:
  - the new button comes after Export CSV and before the second separator;
  - it adds no separator;
  - the search field stays the last focusable element.
- **`src-tauri/tests/contract.rs` `app_errors_map_every_code`** is an explicit list of pairs, so it still compiles and passes with the new `AppErrorCode::CacheFailed`. The `CacheFailed` → `"CACHE_FAILED"` pair is **new coverage**, in the new `src-tauri/tests/contract_cache.rs`. It is not an edit of this test.
- **`src/App.test.ts` and the `AnalysisPage*.test.ts` files** mock `../api` with a factory that has no `clearCache`. They pass unchanged, provided that `AnalysisPage` reads `clearCache` only in its click handler: a factory mock throws only when a missing export is read. This is a constraint on the implementation, not a test edit.
- **`StatusBar` tests and stories:** `cache` is optional, and nothing is appended without it.
- **`Icon` tests:** no test maps every `IconName`, so the new `clear-cache` breaks none.
- **`src-tauri/tests/bindings.rs`** compares the committed bindings with the generated ones. It passes once `src/lib/api/bindings.ts`, a production file, is regenerated (`just gen`).
- **Tests that call `analyze_folder`, `run_analysis` or `run_analysis_kept`:** those functions keep `ResultCache::Off`.
- **`tools/fakeflac-calibrate` tests:** the tool calls neither `analyze_folder` nor the store (section 4).

Production code that maps the summary (`dto.rs`, the store) changes in stage `implement`, and `src/lib/api/bindings.ts` is regenerated.

## 13. Edits to approved specs and docs (proposed in this PR)

- `contracts/commands.md`: version 7 in the front matter and intro; the `clear_cache` row; `CACHE_FAILED`; the summary fields; the section "Result cache (version 7)".
- `ui/main-screen.md` (intro) and `ui/components.md` (Notes column of `Toolbar` and `StatusBar`): one cross-reference sentence each to `ui/result-cache.md`.
- `ui/sort-filter.md`: one cross-reference sentence each, to `ui/result-cache.md`:
  - in section 2, the toolbar width count (8 buttons with "Clear results cache");
  - in section 13, the tab order (one more stop after Export CSV).
- `ui/export-csv.md` (Accessibility): "when disabled it stays focusable" is corrected to "when disabled it is skipped by Tab", because `IconButton` sets the native `disabled` attribute. This changes no behaviour and no test; the owner asked for it in this PR (2026-10-30).
- `rules.md`, `lossy-source.md` and the params files keep saying that a new `algorithm_version` invalidates the app cache. That stays true, so they are not edited.
- `docs/architecture/data-model.md` (outside `specs/`): the cache section now matches this spec (location, key with the engine, schema version 1 in `user_version`, one engine at a time, opening and failures).
- `docs/architecture/backend.md` (outside `specs/`):
  - the `ResultStore` row reads `get(entry, params)` and `put(entry, params, analysis)`, with no `clear()`, because clearing is `fakeflac_store::clear_cache_files`;
  - the `AnalyzeFolder` use case names `analyze_folder_cached` and the cache's failure policy.

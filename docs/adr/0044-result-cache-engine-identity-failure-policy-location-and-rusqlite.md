# ADR-0044: Result cache: the key names the whole engine, the cache is optional, it lives in the local app cache folder, and `fakeflac-store` uses `rusqlite` (bundled)

- Status: accepted
- Date: 2026-10-30

## Context and rationale

RF-11 asks the app to reuse a track's result when the file has not changed. ADR-0007 chose SQLite through `rusqlite` in WAL mode. ADR-0008 fixed the key as (canonical path, size, mtime, `algorithm_version`). The planning document (section 3.6) sketched a schema and a location (`%APPDATA%\FakeFlac`). Writing the spec (`specs/contracts/result-cache.md`) showed five decisions that those ADRs do not cover:

1. **`algorithm_version` alone does not name everything that makes a result.** A result also depends on:
   - every `DetectionParams` value. Today the app only uses the compiled defaults, and a test keeps them equal to `detection-params.v7.toml`. But a params file the user chooses (planned for the settings, planning section 3.6.2) could change a value without changing the version.
   - the decoder (`fakeflac-audio::EXTRACTION_VERSION`, bumped on any adapter change or `symphonia` upgrade).
   - the rest of the app's code, for example a bug fix in `detect.rs` that ships without a new `algorithm_version`.

   A stale result looks like a good one, so this is the risk that matters most for a tool that gives verdicts.
2. **What happens when the cache fails.** The database can be locked by a second FakeFlac window, unwritable, full, corrupt, or written by a newer FakeFlac. None of these should cost the user an analysis.
3. **Where the file lives.** Planning said `%APPDATA%` (the roaming profile). A cache does not belong in a roaming profile, and the app's identifier is `io.github.fakeflac.app`, not `FakeFlac`.
4. **How the table grows.** Planning's primary key (path, size, mtime, version) adds a row every time a file or the version changes, and never removes the old one.
5. **The dependency.** `rusqlite` is chosen (ADR-0007) but not in the dependency set (ADR-0033), and ADR-0033 asks for an ADR for anything new.

## Decision

1. **Key: same file, same engine.** A cached result is reused only when all of these hold:
   - the canonical path is the same;
   - the size and the mtime (Unix seconds) are the same;
   - the **engine** is the same. The engine is the exact `serde_json` text of the `DetectionParams` the run uses (which contains `algorithm_version`), plus `EXTRACTION_VERSION`, plus the app version (`CARGO_PKG_VERSION`).

   This extends ADR-0008 and does not replace it: path, size and mtime are still the file's identity, and a new `algorithm_version` still invalidates every result. The cost is that each app release recomputes every result once.
2. **The cache is optional.**
   - If the database cannot be opened, the run analyses everything and reports `cache: 'unavailable'`.
   - After the first failed read or write, the run stops using the cache and reports `cache: 'degraded'`. A worker whose call fails makes no further call, and each other worker makes at most one more. So a run has at most one failed call per worker (`result-cache.md` section 7, F1–F3).
   - A cache failure is never an `AppError` of `start_analysis` and never changes a verdict.
   - At open, the header (`application_id`, `user_version`) is read, and the decision taken, before anything writes to the file. Only then does the connection switch to WAL.
   - A file that is not a FakeFlac cache (another `application_id`, or tables without one), or that SQLite reports as corrupt when it opens, is deleted and created again (G1 question Q10 of `result-cache.md`).
   - A cache from a newer schema is not written to at all (`'unavailable'`) until the user clears it.
3. **Location:** `%LOCALAPPDATA%\io.github.fakeflac.app\results-cache.sqlite3`, Tauri's `app_cache_dir()`. It is never derived from the analysed folder (RNF-05). Tauri's NSIS uninstaller already offers to delete this folder.
4. **One row per file and engine.** The primary key is (path, engine).
   - A changed file replaces its row.
   - The first result stored under a new engine deletes every other engine and its rows. The cache therefore holds the results of a single engine, and holds at most one row per file ever analysed.
   - Each `put` resolves, or inserts, its engine row inside its own transaction, and the store never keeps an engine id between calls. Another window may have replaced the engine meanwhile, and a stale id would fail its foreign key, a spurious failure.
   - Two windows with different engines may therefore re-run each other's work. They never read each other's results.
5. **Dependencies of `fakeflac-store`:**
   - `rusqlite`: current major at implementation time, feature `bundled`, so SQLite is compiled in and no system library is needed. The owner approved it on 2026-10-30 (Q9 a of `specs/contracts/result-cache.md`).
   - `serde_json` (already in the workspace) with its feature `float_roundtrip`, so that the `f32` values of a stored result read back bit for bit. `tools/fakeflac-calibrate` already enables this feature, but Cargo unifies features per build, and the app build does not include the tool. The owner approved this feature change on 2026-10-30 (Q9 b).

   `fakeflac-core` gets the `ResultStore` port, already listed in `backend.md`, with `get` and `put` only. Clearing is an adapter function, because it must work when the database cannot be opened. The core adds no dependency.

   The owner approved both dependency changes and accepted the ADR at G1 (#83).

## Alternatives considered

- **ADR-0008's key as it is (algorithm version only).** Rejected. It depends on discipline: every params, decoder or rule change would have to bump the version, and a value chosen by the user would bypass it. Stale verdicts would look correct.
- **A hash of the params instead of their text.** The core has no hasher, and `std`'s `DefaultHasher` is not stable across Rust releases. The text is short, stored once per engine, and cannot collide.
- **Caching the spectra instead of the verdicts**, as the calibration tool does (ADR-0036). Rule changes would then not need a new decode. But the app would store about 2 × 220 floats per track and run the rules on every hit, for a case (rule-only releases) that a release recompute already covers. This can be revisited if releases that only change rules become frequent.
- **A content hash in the key.** Already rejected by ADR-0008: hashing 40 MB costs almost as much as analysing it.
- **Failing the run, or asking the user, when the cache fails.** Rejected. The cache is an optimisation, and RNF-04's spirit is that one bad thing does not stop the analysis.
- **Repairing a corrupt database during a run.** Rejected: the files are open, and Windows does not delete open files. The user can clear the cache (`clear_cache`), and the next open repairs whatever SQLite reports as corrupt.
- **`%APPDATA%\FakeFlac` (planning).** Rejected: a roaming profile syncs the cache between machines, and the folder name does not follow the app identifier that Tauri and the uninstaller use.
- **Keeping old engines' rows** (planning's primary key). Rejected: the rows grow without bound and are never read again.
- **`sqlx` or `sled`.** `sqlx` is async and heavier. `sled` was already rejected by ADR-0007.
- **System SQLite instead of `bundled`.** Windows has no SQLite library the app could rely on.

## Consequences

- A rerun on an unchanged folder decodes no audio (acceptance C1 of `result-cache.md`). Results stay exactly equal to fresh ones, CSV included.
- Every app release, `EXTRACTION_VERSION` bump or params change recomputes each folder once.
- During development the app version stays the same between builds. So a detection or decoder change made without a new `algorithm_version` or `EXTRACTION_VERSION` keeps hitting older rows until the developer clears the cache. Tests use their own cache files. `just accept` and `just calibrate` do not use the app cache at all: the calibration tool links neither `analyze_folder` nor `fakeflac-store`.
- **Contract version 7** (`commands.md`): `AnalysisSummary` gains `fromCache` and `cache`, there is a new command `clear_cache`, and a new `AppError` code `CACHE_FAILED`. Tests that build an `AnalysisSummary` by hand change in `test(RF-11)`; the list is in `result-cache.md` section 12.
- **Size and build:**
  - SQLite compiled in adds roughly 1 MB to the installer (RNF-06 limit: 20 MB) and needs a C compiler in CI. Both Windows (MSVC, already needed by Tauri) and the Linux lint job have one.
  - The licences are MIT for `rusqlite` and `libsqlite3-sys`; SQLite itself is public domain. They are covered by the `deny.toml` allow-list, and the implementer checks this with `cargo deny`.
- Rows of files that were deleted or moved stay until the user clears the cache, about 1 KB per file. Pruning them is left for later (open question Q7 of `result-cache.md`).
- `docs/architecture/data-model.md` (schema, location) and `backend.md` (the `ResultStore` port) are updated in the spec PR, to match `result-cache.md`.
- Amends ADR-0008 (key) and ADR-0033 (dependency set).

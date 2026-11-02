# Data model and cache

> Results are cached in SQLite (`rusqlite`, WAL; ADR-0007, ADR-0044) in `%LOCALAPPDATA%\io.github.fakeflac.app\results-cache.sqlite3` (Tauri's `app_cache_dir()`), never next to the music and never derived from the analysed folder (RNF-05). Key = (canonical path, size, mtime, engine). The engine is the exact JSON of the run's `DetectionParams` (which contains `algorithm_version`), plus `EXTRACTION_VERSION`, plus the app version (ADR-0008 as extended by ADR-0044). Full spec: `specs/contracts/result-cache.md` (RF-11).

## SQLite schema version 1 (ADR-0007, ADR-0044)

```sql
-- PRAGMA application_id = 1179012163;  -- 0x46464C43, "FFLC": marks a FakeFlac cache
-- PRAGMA user_version   = 1;           -- schema version
CREATE TABLE engine (
  id                 INTEGER PRIMARY KEY,
  params_json        TEXT    NOT NULL,          -- serde_json text of DetectionParams
  extraction_version INTEGER NOT NULL,          -- fakeflac_audio::EXTRACTION_VERSION
  app_version        TEXT    NOT NULL,          -- CARGO_PKG_VERSION of src-tauri
  algorithm_version  INTEGER NOT NULL,          -- copy of the value inside params_json, for inspection only
  UNIQUE (params_json, extraction_version, app_version)
);
CREATE TABLE track_result (
  path          TEXT    NOT NULL,               -- canonical path, UTF-8, compared BINARY (RNF-09)
  engine_id     INTEGER NOT NULL REFERENCES engine(id) ON DELETE CASCADE,
  size_bytes    INTEGER NOT NULL,
  modified_unix INTEGER NOT NULL,               -- whole Unix seconds, from the scan
  analysis_json TEXT    NOT NULL,               -- the core TrackAnalysis as serde_json (camelCase)
  stored_unix   INTEGER NOT NULL,               -- inspection only
  PRIMARY KEY (path, engine_id)
) WITHOUT ROWID;
```

- **Hit:** same canonical path and engine, and the row's size and mtime equal the scan's. The core checks the returned analysis again (`algorithm_version`, size, mtime, format).
- **What is stored:** every result except `unreadable` and `corruptStream` errors and results with a non-finite cutoff value (`is_cacheable`, result-cache.md 2.2). Entries with no mtime, or with a path that is not valid Unicode, are never looked up or stored.
- **Invalidation:** a changed file replaces its row. The first result stored under a new engine deletes every other engine and, by cascade, its rows, so the cache holds one engine at a time. There is no pruning of rows of deleted files and no `VACUUM`.
- **Opening:** the header (`application_id`, `user_version`) is read before anything writes. An empty file gets schema version 1. A file that is not a FakeFlac cache, or that SQLite reports as corrupt, is reset. A newer schema is left untouched and the run reports `unavailable`.
- **Migrations:** `user_version` plus sequential scripts, run in one write transaction at open.
- **Failures** never fail a run: the summary reports `cache: 'unavailable'` or `'degraded'` (contract version 7).
- **Clearing:** `clear_cache` deletes `-wal`, `-shm` and the database file, in that order, without opening it.
- **Settings (RF-13, approved at G1 in #107, implemented in #109):** the owner limited RF-13 to Spanish/English with a persistent preference. `Settings { language: "es" | "en" }` is stored as versioned JSON at `app_config_dir()/settings.json`, independent of the result cache ([ADR-0049](../adr/0049-persist-language-preference-in-app-config-json.md), [contract](../../specs/contracts/settings.md)). Sampling and detection-parameter controls from historical planning are deferred. The app has no settings persistence until `/feature RF-13` is merged.

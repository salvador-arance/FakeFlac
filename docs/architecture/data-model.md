# Data model and cache

> Results are cached in SQLite (`rusqlite`, WAL) in `%APPDATA%\FakeFlac`, never next to the music. Key = (path, size, mtime, algorithm_version). Settings are a JSON file in the same folder.

## SQLite schema (ADR-0007, ADR-0008)

```sql
CREATE TABLE track_analysis (
  path              TEXT    NOT NULL,   -- canonical path, UTF-8 (RNF-09)
  size_bytes        INTEGER NOT NULL,
  modified_unix     INTEGER NOT NULL,
  algorithm_version INTEGER NOT NULL,
  verdict           TEXT    NOT NULL,   -- real | suspicious | fake | error
  cutoff_hz         REAL,
  steepness         REAL,
  noise_floor_db    REAL,
  reasons           TEXT,               -- JSON
  format            TEXT    NOT NULL,   -- wav | flac | mp3 | m4a | opus
  declared_bitrate  INTEGER,            -- kbps, lossy formats only
  estimated_bitrate INTEGER,            -- kbps, RF-15
  sample_rate       INTEGER NOT NULL,
  bit_depth         INTEGER,
  duration_ms       INTEGER NOT NULL,
  error             TEXT,
  analyzed_at       INTEGER NOT NULL,
  PRIMARY KEY (path, size_bytes, modified_unix, algorithm_version)
);
CREATE INDEX idx_folder ON track_analysis(path);
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);  -- schema_version
```

- **Invalidation:** a new `algorithm_version` stops matching old rows; they are purged at start-up.
- **Migrations:** `meta.schema_version` plus sequential scripts.
- **Settings:** `Settings { language: "es" | "en", windowsCount, sampleWindows, params: DetectionParams }` as JSON.

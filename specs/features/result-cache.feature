@RF-11
Feature: Reuse the results of unchanged files (H2)
  As a user who analyses the same library again
  I want unchanged files to be served from a cache instead of being decoded again
  So that a second analysis of a folder takes seconds, with the same verdicts

  Spec: specs/contracts/result-cache.md (key, engine, storage, failures,
  layers, acceptance C1 to C8). Contract: specs/contracts/commands.md
  ("Result cache (version 7)"). UI: specs/ui/result-cache.md. Decisions:
  ADR-0007, ADR-0008 as extended by ADR-0044.
  Every cache file in these scenarios is inside the test's own temporary
  folder ("the cache file"), never inside the repository, the audio corpus
  or the real %LOCALAPPDATA% folder. Audio files are generated inside the
  test (specs/detection/fixtures.md) or, where only the path matters, hold
  any bytes.
  "The decoder spy" is a counting fake of the SpectrumAnalyzer port;
  "analyze calls" counts its calls. By default it returns a probe of a
  180 s, 44.1 kHz FLAC with the band spectra of the fakes in
  crates/fakeflac-core/tests/analyze_folder.rs: flat (real), a cliff at
  16 kHz (fake) or a cliff at 20.5 kHz (suspicious). Per path it can be
  told instead to return a probe lasting 500 ms (shorter than
  min_duration_s, 5 s by default), to return a given TrackError, or to
  panic.

  # --- Use case (fakeflac-core, fake scanner, decoder spy, in-memory store fake) ---
  # The store fake keeps results in a map keyed by (path, size, mtime,
  # algorithm_version) and can be told to fail or to panic on its n-th call.
  # These scenarios check the core's rules (result-cache.md sections 2.2 and
  # 8); the SQLite adapter has its own section below.

  Scenario: A hit is not analysed and is reported like an analysed track
    Given a scan of 4 entries "a.flac", "b.flac", "c.flac", "d.flac" with mtimes other than 0
    And the store fake holds a result for "b.flac" and "d.flac" with their size, mtime, format and the run's algorithm_version
    When analyze_folder_cached runs with ResultCache::Store of the fake
    Then the decoder spy is called exactly for "a.flac" and "c.flac"
    And every file gets "trackStarted" and then "trackDone", and the last event is "finished" with done 4
    And the trackDone analyses of "b.flac" and "d.flac" equal the stored ones except info.path and info.folder, which are those of the scan entries
    And the summary has fromCache 2 and cache "ok"
    And the store fake received exactly 2 puts, for "a.flac" and "c.flac", each with the analysis sent in its trackDone

  Scenario: Hits count in their album like analysed tracks
    Given a scan of "Album A" with 3 files whose results are all in the store fake
    When analyze_folder_cached runs
    Then the album updates of "Album A" carry summary.total 1, 2 and 3 in channel order
    And the last update's summary equals summarize_album of the 3 cached analyses

  Scenario Outline: A returned analysis that does not match the entry is a miss
    Given a scan entry "a.flac" with size 1000, mtime 1700000000 and format flac
    And the store fake returns for it an analysis whose <field> is <value>
    When analyze_folder_cached runs with params of algorithm_version 7
    Then the decoder spy is called once for "a.flac"
    And the summary has fromCache 0 and cache "ok"

    Examples:
      | field             | value      |
      | algorithm_version | 6          |
      | info.size_bytes   | 999        |
      | info.modified_unix| 1699999999 |
      | info.format       | wav        |

  Scenario Outline: Which analyses are cacheable
    The rule is the pure core function is_cacheable (result-cache.md 2.2),
    checked on hand-built TrackAnalysis values: the decoder spy cannot
    reliably make the rules produce a non-finite cutoff value.
    Given a TrackAnalysis with <analysis>
    Then is_cacheable returns <cacheable>

    Examples:
      | analysis                                                                   | cacheable |
      | verdict "real" and a finite cutoff                                         | true      |
      | verdict "real" and no cutoff                                               | true      |
      | verdict "fake"                                                             | true      |
      | verdict "suspicious" with reason "streamingSampleRate"                     | true      |
      | the error "tooShort"                                                       | true      |
      | the error "unsupportedFormat"                                              | true      |
      | the error "unreadable"                                                     | false     |
      | the error "corruptStream"                                                  | false     |
      | the error "corruptStream" of a caught decoder panic ("the decoder failed unexpectedly") | false |
      | verdict "real" and a cutoff whose noise floor is NaN                       | false     |
      | verdict "suspicious" and a cutoff whose steepness is +infinity             | false     |
      | verdict "fake" and a cutoff whose frequency is -infinity                   | false     |

  Scenario Outline: Which results are stored
    Given a scan entry "x.flac" for which the decoder spy <behaviour>
    When analyze_folder_cached runs with the store fake
    Then the trackDone of "x.flac" has <outcome>
    And the store fake receives <puts> put for "x.flac"
    And any put for "x.flac" carries the analysis of its trackDone

    Examples:
      | behaviour                                  | outcome                       | puts |
      | returns the flat spectrum                  | the verdict "real"            | 1    |
      | returns the cliff at 16 kHz                | the verdict "fake"            | 1    |
      | returns the cliff at 20.5 kHz              | the verdict "suspicious"      | 1    |
      | returns a probe lasting 500 ms             | the error "tooShort"          | 1    |
      | returns the error UnsupportedFormat        | the error "unsupportedFormat" | 1    |
      | returns the error Unreadable               | the error "unreadable"        | 0    |
      | returns the error CorruptStream            | the error "corruptStream"     | 0    |
      | panics                                     | the error "corruptStream"     | 0    |

  Scenario: An entry without an mtime is never looked up or stored
    Given a scan entry "a.flac" whose modified_unix is 0
    When analyze_folder_cached runs with the store fake
    Then the store fake receives no get and no put for "a.flac"
    And the decoder spy is called once for "a.flac"
    And the summary has cache "ok"

  @RNF-04
  Scenario: The first store failure stops the use of the store for the rest of the run
    With one worker the bound F3 of result-cache.md section 7 is exact.
    Given a scan of 10 entries, none of them stored
    And the store fake fails on its 3rd call
    And the run uses 1 worker
    When analyze_folder_cached runs
    Then the store fake receives exactly 3 calls
    And the decoder spy is called 10 times
    And every file gets its trackDone with the verdict the decoder spy gives it
    And the summary has cache "degraded" and the last event is "finished" with done 10

  @RNF-04
  Scenario: A panic inside the store is a store failure, not a crash
    Given a scan of 3 entries
    And the store fake panics on its first get
    When analyze_folder_cached runs
    Then the run ends with "finished" with done 3
    And the summary has cache "degraded"

  Scenario Outline: The cache status of a run
    Given a scan of 2 entries
    When analyze_folder_cached runs with <cache>
    Then the summary has cache "<status>" and fromCache 0
    And the decoder spy is called 2 times

    Examples:
      | cache                                         | status      |
      | ResultCache::Off                              | off         |
      | ResultCache::Unavailable                      | unavailable |
      | ResultCache::Store of an empty store fake     | ok          |

  Scenario: analyze_folder keeps working without a cache
    Given a scan of 2 entries
    When analyze_folder runs with its existing arguments
    Then its events and summary equal those of analyze_folder_cached with ResultCache::Off
    And the summary has fromCache 0 and cache "off"

  Scenario: Tracks finished before a cancel are stored
    Given a scan of 50 entries, none of them stored, and 2 workers
    And an analysis is cancelled after 10 trackDone events
    When the run ends with "cancelled"
    Then every track that has a trackStarted event also has a trackDone event
    And every track with a trackDone event has been put in the store fake before its trackDone was sent
    And the store fake received no call for a track without a trackStarted event

  # --- Storage (fakeflac-store, real SQLite in a temporary folder) ------------
  # Files are real (canonicalize needs them) but their content does not
  # matter: the decoder spy analyses nothing. "Run 1" and "run 2" each open a
  # new SqliteResultStore on the cache file with the same EngineTag and call
  # analyze_folder_cached with the same DetectionParams, unless the scenario
  # says otherwise.

  @RNF-01
  Scenario: A rerun on an unchanged folder decodes no audio (acceptance C1)
    Given a test folder "Album A" with 6 files
    And run 1 has finished with 6 analyze calls
    When run 2 analyses "Album A"
    Then run 2 makes 0 analyze calls
    And run 2's summary has fromCache 6 and cache "ok"
    And each trackDone analysis of run 2 equals run 1's for the same index

  Scenario Outline: Only the changed file is analysed again (acceptance C3)
    Given a test folder "Album A" with "a.flac", "b.flac" and "c.flac"
    And run 1 has finished
    When <change> and run 2 analyses "Album A"
    Then run 2 makes exactly 1 analyze call, for "b.flac"
    And run 2's summary has fromCache 2

    Examples:
      | change                                                         |
      | "b.flac" gets one more byte                                    |
      | "b.flac" has its modification time moved 1 second later        |
      | "b.flac" is replaced by another file with a different size     |

  Scenario: A file changed with the same size and mtime is served from the cache (accepted risk, ADR-0008)
    Given a test folder "Album A" with "a.flac" of 1000 bytes
    And run 1 has finished
    And "a.flac" is overwritten with 1000 other bytes and its modification time is set back to the original
    When run 2 analyses "Album A"
    Then run 2 makes 0 analyze calls

  Scenario Outline: A new engine analyses everything again and keeps only itself (acceptance C4)
    Given a test folder "Album A" with 3 files
    And run 1 has finished
    When run 2 analyses "Album A" with <difference>
    Then run 2 makes 3 analyze calls and its summary has fromCache 0
    And afterwards the cache's engine table has exactly 1 row and track_result has 3 rows

    Examples:
      | difference                                                         |
      | params of another algorithm_version                                |
      | params equal to run 1's except one threshold, same algorithm_version |
      | params equal to run 1's but with lossy.streaming removed           |
      | an EngineTag with another extraction_version                       |
      | an EngineTag with another app_version                              |

  Scenario: The same file through another root is a hit with the new paths
    Given a test folder "Library" holding a folder "Album A" with "a.flac" and "b.flac"
    And run 1 has analysed "Library"
    When run 2 analyses "Library\Album A"
    Then run 2 makes 0 analyze calls
    And run 2's trackDone paths and folders are those of the scan of "Library\Album A"
    And its album update names start with "Album A"

  @RNF-09
  Scenario: Windows spellings of the same path share one key
    Windows only (#[cfg(windows)]): canonicalize resolves the case, the
    verbatim prefix and 8.3 short names. "<tmp>" is the test's own folder
    inside the system temporary folder, spelled from
    std::fs::canonicalize(std::env::temp_dir()) with the "\\?\" prefix
    removed: a plain "C:\..." path with long names, even when temp_dir()
    gives an 8.3 short name such as "RUNNER~1". "<raw>" is the same folder
    spelled from std::env::temp_dir() as the OS gives it.
    Given a test folder "<tmp>\Album A" with "Track.flac"
    And run 1 has analysed "<tmp>\Album A"
    When run 2 analyses the same folder given as "<tmp>\ALBUM A"
    And run 3 analyses it given as "\\?\<tmp>\Album A"
    And run 4 analyses it given as "<raw>\Album A"
    Then runs 2, 3 and 4 make 0 analyze calls
    And the cache's track_result table has exactly 1 row

  @RNF-09
  Scenario: Unicode names are keys code point for code point
    Windows only. The two "Canción" names differ only in normalisation
    (NFC U+00F3 against NFD "o" + U+0301): NTFS keeps them as two files.
    Given a test folder "Álbum ″Ñandú″" with "01 – Canción ‘x’ 🎵.flac", "02 Canción.flac" in NFC and "02 Canción.flac" in NFD
    And run 1 has finished with 3 analyze calls
    When run 2 analyses "Álbum ″Ñandú″"
    Then run 2 makes 0 analyze calls
    And each trackDone path of run 2 equals the scanned path code point for code point
    And the cache's track_result table has 3 rows

  @RNF-09
  Scenario: A name that is not valid Unicode is analysed every time
    Windows only. The second file is "b" followed by the unpaired surrogate
    U+D800 and ".flac", built with OsString::from_wide. Existing behaviour,
    unchanged by RF-11: fakeflac-fs keeps such a file because its extension
    is valid Unicode, and the DTO sends its path with U+FFFD
    (to_string_lossy). The adapter never serialises its path into the
    cache (result-cache.md 2.3).
    Given a test folder "Album A" with "a.flac" and the non-Unicode file
    And run 1 has finished with 2 analyze calls and cache "ok"
    When run 2 analyses "Album A"
    Then run 2 makes exactly 1 analyze call, for the non-Unicode file
    And run 2's summary has cache "ok" and fromCache 1
    And the cache's track_result table has exactly 1 row, whose path ends with "a.flac"

  Scenario: Stored analyses round-trip exactly (acceptance C2)
    Given stubbed analyses covering every verdict, every reason, every codec, the errors "tooShort" and "unsupportedFormat", and cutoffs 16000.4, 42.06, -101.94, 19999.998 and 1e-7
    When each is put and then got back through a SqliteResultStore
    Then each one read back is equal (==) to the one stored, every f32 bit for bit

  Scenario: A row that does not parse is a miss and is replaced
    Given run 1 has stored "a.flac"
    And its analysis_json is overwritten with "{not json"
    When run 2 analyses the folder
    Then run 2 makes 1 analyze call and its summary has cache "ok"
    And afterwards the row of "a.flac" parses again

  Scenario: One row per file
    Given a test folder with "a.flac"
    When "a.flac" is analysed, changed in size, analysed, changed in mtime and analysed again
    Then the cache's track_result table has exactly 1 row

  # --- Opening and failures (fakeflac-store + use case, acceptance C5) ---------

  @RNF-04
  Scenario Outline: The cache is optional: verdicts never depend on it
    "Rollback-journal mode" means the test wrote the file with SQLite's
    default journal_mode (DELETE); "WAL mode" means journal_mode WAL and a
    clean close. busy_timeout is 1000 ms (result-cache.md 6.2).
    Given a test folder "Mixed" with 4 files
    And <situation>
    When a run opens the store on the cache file and analyses "Mixed"
    Then the run ends with "finished" with done 4
    And its trackDone verdicts equal those of a run with ResultCache::Off
    And its summary has cache "<status>"
    And <after>

    Examples:
      | situation                                                                  | status      | after                                                                        |
      | no cache file exists                                                       | ok          | the cache file now exists with application_id "FFLC" and user_version 1      |
      | the cache file holds 4096 bytes of 0xAB (no SQLite header)                 | ok          | the cache file is now a FakeFlac cache of schema version 1 holding the 4 results (reset, Q10) |
      | the cache file is an SQLite database with another application_id and a table "x" | ok   | the cache file is now a FakeFlac cache and table "x" is gone (reset, Q10)    |
      | the cache file is a FakeFlac cache with user_version 99 in rollback-journal mode | unavailable | the cache file is byte-identical to before and no "-wal" or "-shm" file exists (Q8) |
      | the cache file is a FakeFlac cache with user_version 99 in WAL mode        | unavailable | its user_version is still 99 and every table and row is unchanged (Q8)       |
      | the parent of the cache file is a regular file, not a folder               | unavailable | the regular file is unchanged                                                |
      | a second connection holds a write transaction on a FakeFlac cache from before the open until after the run | unavailable | the store's open failed after waiting at least 1000 ms, and the second connection's data is intact after it commits |

  @RNF-04
  Scenario: A cache locked during the run costs at most one timeout per worker
    The store is wrapped in a counting decorator of the ResultStore port
    that records, for every call, the calling thread, its start and end
    times and its result. Bounds F1 and F2 of result-cache.md section 7.
    Given a test folder with 20 files and an empty FakeFlac cache
    And the store has been opened on the cache file
    And then a second connection begins a write transaction on it and holds it until the run ends
    When a run with 2 workers analyses the 20 files with that store
    Then the decorator records between 1 and 2 failed calls, all of them puts
    And each failed call ended at least 1000 ms after it started
    And no thread made a store call after one of its own calls had failed
    And the decoder spy is called 20 times
    And every file gets its trackDone with the verdict the decoder spy gives it
    And the run's summary has cache "degraded"

  # --- App (src-tauri: run::run_analysis_cached, run::clear_cache) ------------
  # Real runs with the symphonia analyzer and the fixtures of
  # specs/detection/fixtures.md; the cache file is in the test's temporary
  # folder. "<id>.wav" holds the samples of fixture <id> as 16-bit WAV, so
  # two WAV fixtures of the same duration, rate and channels have the same
  # size.

  @RNF-04
  Scenario: A warm run gives the cold run's events and CSV (acceptance C2)
    Given a test folder "Mixed" holding "full_band.flac", "corrupt.flac", "lp16k.flac" and "short.flac"
    And run::run_analysis_cached has finished on "Mixed" with the cache file
    When it runs again on "Mixed"
    Then each trackDone analysis of the second run equals the first run's for the same index
    And the second summary has fromCache 3 and cache "ok": "corrupt.flac" is analysed again and "short.flac" (tooShort) is not
    And the CSV document built from the second run's results is byte-identical to the first run's

  Scenario: The cache, not the decoder, gives a warm run's verdict
    Given a test folder "Album A" holding "x.wav" with the samples of lp16k
    And run::run_analysis_cached has finished on "Album A" and "x.wav" was "fake"
    And "x.wav" is overwritten with the samples of full_band as a WAV of the same size, and its modification time is set back to the original
    When run::run_analysis_cached runs again on "Album A"
    Then "x.wav" is still "fake" and the summary has fromCache 1
    When run::clear_cache is called with the cache file
    And run::run_analysis_cached runs again on "Album A"
    Then "x.wav" is "real" and the summary has fromCache 0

  Scenario: No cache folder means no cache, not an error
    Given a test folder "Album A" with 2 FLAC files
    When run::run_analysis_cached runs on "Album A" with no cache file (None)
    Then the run ends with "finished" with done 2 and cache "unavailable"

  Scenario: The existing run functions use no cache
    Given a test folder "Album A" with 2 FLAC files
    When run::run_analysis and run::run_analysis_kept run on "Album A"
    Then their summaries have cache "off" and fromCache 0

  @RNF-05
  Scenario: The cache never touches the analysed folder (acceptance C6)
    Given a test folder "Album A" holding "full_band.flac" and "lp16k.flac" and a separate cache folder
    When run::run_analysis_cached runs twice on "Album A"
    Then the size, modification time and content hash of both audio files are unchanged
    And "Album A" holds exactly the 2 files it held before
    And the only files created are "results-cache.sqlite3" and, at most, its "-wal" and "-shm" files in the cache folder

  @RNF-09
  Scenario: Unicode names in a real warm run
    Given a test folder "Álbum ″Ñandú″" with "01 – Canción ‘x’.flac" and "02 «Ça va» 🎵.flac", each holding the samples of full_band
    When run::run_analysis_cached runs twice on it
    Then the second run's summary has fromCache 2
    And its trackDone paths and its album update name equal the scanned ones code point for code point

  Scenario Outline: Clearing the cache (acceptance C8)
    Given <before>
    When run::clear_cache is called with the cache file
    Then <outcome>

    Examples:
      | before                                                                    | outcome                                                                             |
      | a run has filled the cache                                                | it succeeds and "results-cache.sqlite3", its "-wal" and its "-shm" do not exist     |
      | no cache file exists                                                      | it succeeds                                                                         |
      | the cache file holds random bytes                                         | it succeeds and the file is gone                                                    |
      | the cache file is a FakeFlac cache with user_version 99                   | it succeeds and the file is gone, so the next run has cache "ok"                    |
      | the cache folder holds another file "notes.txt"                           | it succeeds and "notes.txt" is unchanged                                            |
      | a run holds the analysis slot                                             | it fails with "ANALYSIS_RUNNING" and the cache files are unchanged                  |
      | another connection keeps the cache file open (Windows)                    | it fails with "CACHE_FAILED" and the cache is still a valid FakeFlac cache          |

  Scenario: Clearing changes no result kept for the export
    Given run::run_analysis_cached has finished on "Album A" with 3 files
    When run::clear_cache succeeds
    Then run::begin_export returns the same 3 tracks as before

  Scenario: No analysis starts while the cache is being cleared
    Given run::clear_cache holds the analysis slot
    When start_analysis is called
    Then it fails with error "ANALYSIS_RUNNING"

  Scenario: The contract of version 7
    Then a core AnalysisSummary with from_cache 2 and cache Degraded serialises through the DTO with "fromCache": 2 and "cache": "degraded"
    And the four CacheStatus values serialise as "off", "ok", "degraded" and "unavailable"
    And the AppError code CacheFailed serialises as "CACHE_FAILED"
    And the generated bindings contain the command clearCache, the type CacheStatus and the two AnalysisSummary fields

  # --- Interface (stubbed API client; specs/ui/result-cache.md) ----------------

  Scenario Outline: Cache figures in the status bar
    Given an analysis has ended "<end>" with <done> of 31 tracks, fromCache <n> and cache "<status>"
    Then the status bar shows "<text>"

    Examples:
      | end       | done | n  | status      | text                                                                                                   |
      | finished  | 31   | 31 | ok          | 31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 31 from cache                                 |
      | finished  | 31   | 0  | ok          | 31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors                                                 |
      | finished  | 31   | 0  | unavailable | 31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · Results cache unavailable                     |
      | finished  | 31   | 10 | degraded    | 31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 10 from cache · Results cache error: some results were not saved |
      | finished  | 31   | 0  | off         | 31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors                                                 |
      | cancelled | 18   | 12 | ok          | Cancelled · 18 of 31 · 12 from cache                                                                   |

  Scenario: Cache figures come before the shown count and the notice
    Given an analysis has finished with 31 tracks, fromCache 25 and cache "ok"
    And a verdict filter shows 3 rows and an export showed "Saved 31 rows to a.csv"
    Then the status bar shows "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 25 from cache · 3 shown · Saved 31 rows to a.csv"

  Scenario: No cache figures while analysing or after an unexpected failure
    Given an analysis is running
    Then the status bar shows no "from cache" text
    When the run fails with error "INTERNAL"
    Then the status bar shows only "The analysis stopped because of an unexpected error"

  Scenario: A drop that starts nothing keeps the previous figures
    Given an analysis has finished with 31 tracks, fromCache 31 and cache "ok"
    When I drop two files and start_analysis rejects with "NOT_A_FOLDER" for both
    Then the status bar still ends with "· 31 from cache"

  Scenario: A new run replaces the figures
    Given an analysis has finished with fromCache 31 and the status bar ends with "· 31 from cache"
    When the next run starts
    Then the status bar shows "Scanning…" without "from cache"
    When that run finishes with fromCache 0 and cache "ok"
    Then the status bar shows no "from cache" text

  Scenario Outline: When "Clear results cache" is enabled
    Given the screen is in state "<state>"
    Then the toolbar button "Clear results cache" is <enabled>

    Examples:
      | state                        | enabled  |
      | Empty                        | enabled  |
      | Scanning                     | disabled |
      | Analysing                    | disabled |
      | Finished                     | enabled  |
      | Cancelled                    | enabled  |
      | Error                        | enabled  |
      | Finished, while exporting    | enabled  |

  Scenario: While clearing, nothing else can start
    Given an analysis has finished
    When I click "Clear results cache" and clearCache has not resolved yet
    Then the toolbar buttons "Clear results cache" and "Open folder" are disabled
    And dropping a folder on the window does not call startAnalysis
    And both buttons are enabled again when clearCache resolves

  Scenario Outline: The outcome of clearing is a notice
    Given an analysis has finished with 31 tracks and fromCache 31
    When I click "Clear results cache" and clearCache <outcome>
    Then the status bar ends with "· <text>"
    And the table, the selection, the filters and the Albums panel are unchanged

    Examples:
      | outcome                         | text                              |
      | resolves                        | Results cache cleared             |
      | rejects with "CACHE_FAILED"     | Cannot clear the results cache    |
      | rejects with "INTERNAL"         | Cannot clear the results cache    |
      | rejects with "ANALYSIS_RUNNING" | An analysis is already running    |

  Scenario: Clearing from the empty screen
    Given the application has just started
    When I click "Clear results cache" and clearCache resolves
    Then the status bar shows "Ready · Results cache cleared"

  @RNF-08
  Scenario: The button and the warning are named in text
    Then the toolbar button with the new "clear-cache" icon has the accessible name and tooltip "Clear results cache"
    And the cache warnings are status bar text in the role "status" region, never colour alone

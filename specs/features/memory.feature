@RNF-02
Feature: Streaming decode with a bounded working set per track (H1)
  As a user who checks long recordings and whole albums
  I want each track analysed without loading it whole into memory
  So that a one-hour file costs no more memory than a three-minute song

  Spec: specs/contracts/memory.md (criterion, metric, fixtures, suites,
  report, pure functions, levers, acceptance A1 to A10, manual checks M1 and
  M2, G1 questions Q1 to Q9, decided defaults D1 to D8, figures F1 to F7:
  F1 to F5 measured before G1 and backing the bounds, F6 (the long suite)
  and F7 (the whole app) measured later in the feature PR). Related:
  specs/contracts/performance.md (RNF-01, D7), ADR-0002, ADR-0010, ADR-0019,
  ADR-0022, ADR-0046, and a new ADR extending ADR-0046 that must exist before
  the feature PR opens (memory.md Q9).

  How the scenarios run (memory.md section 5):
    "The suite" is crates/fakeflac-audio/tests/memory.rs. Every fixture is
    written and every child is started by one serial driver test:
    memory_gate for the gate suite, memory_long for the long suite. Scenarios
    marked "(long)" belong to memory_long, an ignored test run only through
    "just memory" in a release build on the owner's machine; their report
    goes in the feature PR. Every other scenario is a normal test that
    "just test" and CI's test-rust job run; the ones that read memory peaks
    run on Windows only.
  "A child" is the test binary re-executed to run the ignored test
  memory_child, which makes exactly one call (analyze or spectrogram, with
  the default parameters) on one file, or no call for "a baseline child",
  then reads its own peaks with:
    powershell.exe -NoProfile -NonInteractive -Command "$p = Get-Process -Id <pid>; '{0} {1}' -f $p.PeakWorkingSet64, $p.PeakPagedMemorySize64"
  "Private" is the peak private bytes (PeakPagefileUsage) and "working set"
  the peak working set (PeakWorkingSetSize) of that child, in bytes.
  "The growth" of a pair is the long file's peak minus the short file's peak
  (0 if negative); "over the baseline" is a child's peak minus the first
  baseline child's peak. 1 MiB = 1 048 576 bytes.
  Fixtures (memory.md section 4) are written with a fixed seed under
  CARGO_TARGET_TMPDIR/memory/<driver>-<pid>, in a folder "Memoria ″Ñandú″",
  named "NN – Canción ‘x’.<ext>", one case group at a time, and deleted when
  the driver ends. "Short" is about 30 s and "long" about 300 s of the same
  format; the long suite adds 3600 s.
  "Within 1 ms of the length" is the length tolerance of memory.md section 4:
  1 ms for every case except mp3 (one codec frame: 1152 frames, 27 ms) and
  m4a_aac (one codec frame: 1024 frames, 24 ms); each scenario that applies
  it states the exception.

  # --- The criterion ---

  Scenario Outline: Peak memory does not grow with the track's length
    Given the suite has written the short and the long "<case>" fixture
    When a child runs <op> on the short file and another child on the long file
    Then the growth of private is at most 1048576 bytes
    And the growth of working set is at most 1048576 bytes

    Examples:
      | case                | op          |
      | wav                 | analyze     |
      | flac                | analyze     |
      | flac_unknown_length | analyze     |
      | mp3                 | analyze     |
      | m4a_aac             | analyze     |
      | m4a_alac            | analyze     |
      | opus                | analyze     |
      | flac                | spectrogram |

  Scenario Outline: One track is analysed within a fixed ceiling
    The ceiling applies to every measured child, short and long files alike
    (memory.md section 5.1 rows 3 to 18, and A4). The error and empty cases
    (rows 19 to 21) have scenarios of their own below.
    Given the suite has run two baseline children
    When a child runs <op> on the <length> "<case>" fixture
    Then its private is at most 33554432 bytes over the baseline
    And its working set is at most 33554432 bytes over the baseline

    Examples:
      | case                | op          | length |
      | wav                 | analyze     | short  |
      | wav                 | analyze     | long   |
      | flac                | analyze     | short  |
      | flac                | analyze     | long   |
      | flac_unknown_length | analyze     | short  |
      | flac_unknown_length | analyze     | long   |
      | mp3                 | analyze     | short  |
      | mp3                 | analyze     | long   |
      | m4a_aac             | analyze     | short  |
      | m4a_aac             | analyze     | long   |
      | m4a_alac            | analyze     | short  |
      | m4a_alac            | analyze     | long   |
      | opus                | analyze     | short  |
      | opus                | analyze     | long   |
      | flac                | spectrogram | short  |
      | flac                | spectrogram | long   |

  Scenario: An Opus file's memory does not grow with its packet count
    The Opus reader keeps one index entry per 20 ms packet today (memory.md
    section 11). Measured against today's code (memory.md F1), the peaks grow
    from 30 s to 300 s by 2142208 bytes of working set and 2248704 bytes of
    private, about twice the bound, so this scenario is red until the reader
    is made two-pass (Q4). Its results must stay identical (memory.md
    section 8).
    Given the short "opus" fixture has 1501 packets and the long one 15001 packets
    When a child runs analyze on each
    Then the growth of private is at most 1048576 bytes
    And the opus fixtures and errors of opus.feature give the same results as before the change

  Scenario Outline: Decoding hands the analysis bounded chunks
    Checked through the hidden seam decode_chunks_for_test(path) of
    fakeflac-audio (memory.md section 3.5), on every platform. The tolerance
    is 1 ms (floor(sample_rate / 1000) frames) except for mp3 and m4a_aac,
    which are allowed one codec frame.
    Given the suite has written the short and the long "<case>" fixture
    When decode_chunks_for_test runs on each file
    Then each returns a largest chunk of at most 65535 frames
    And the long file's largest chunk equals the short file's
    And each returns frames within <tolerance> of its fixture's length

    Examples:
      | case                | tolerance   |
      | wav                 | 1 ms        |
      | flac                | 1 ms        |
      | flac_unknown_length | 1 ms        |
      | mp3                 | 1152 frames |
      | m4a_aac             | 1024 frames |
      | m4a_alac            | 1 ms        |
      | opus                | 1 ms        |

  Scenario: The fixtures make whole-track buffering visible
    Given the suite has written every pair of memory.md section 4.1, one case group at a time
    Then every fixture has the format, sample rate, channels and frames or packets of its row
    And for every pair check_fixture_growth(case, short bytes, long bytes, 4194304) returns Ok
    And no child runs on a case group before that group has passed these checks

  Scenario: Every measured call does real work
    When the suite runs its children
    Then every analyze child on an intact fixture returns ok with a duration within 1 ms of its fixture's length, except mp3 within 27 ms (one codec frame of 1152 samples) and m4a_aac within 24 ms (one codec frame of 1024 samples)
    And every spectrogram child returns ok with width 1200 and height 512
    And a child that panics, exits non-zero or prints no result fails the suite

  # --- How memory is measured ---

  Scenario: Each measurement runs in a process of its own
    When the suite measures a call
    Then the call runs in a fresh child process that makes no other call
    And the child reads its peaks after the call returns, while it is still running
    And the parent's own memory is never measured

  Scenario: The gate suite runs serially with one case group on disk at a time
    The suite is safe both under "cargo test" (tests on threads of one
    process) and under "cargo nextest run" (one process per test), so it
    does not rely on a mutex (memory.md section 5.1.1).
    Given memory_gate is the only test of memory.rs that writes a fixture or starts a child
    When memory_gate runs under cargo test or under cargo nextest
    Then its fixture root is CARGO_TARGET_TMPDIR/memory/gate-<pid>, with <pid> the driver's own process id
    And a folder of that name left by an earlier run is deleted before the first group is written
    And it starts each child only after the previous child has exited
    And before it writes each case group its fixture folder holds no file
    And it deletes each case group before it writes the next one
    And its fixture root is deleted when it ends, whether it passes or fails

  Scenario: Unreadable peaks fail the suite
    Given the peaks command of a child prints "12 x"
    Then parse_peaks returns Err "memory: cannot read the peaks from \"12 x\""
    And the suite fails instead of skipping the measurement

  Scenario: Reading the peaks
    When parse_peaks is called with "123 456\r\n"
    Then it returns Ok (123, 456)

  Scenario Outline: Peaks that cannot be read
    When parse_peaks is called with "<text>"
    Then it returns Err "memory: cannot read the peaks from \"<text>\""

    Examples:
      | text |
      | 123  |
      |      |
      | 12 x |

  Scenario: The growth check gives an exact message
    When check_growth("opus analyze", "private", 1000000, 3000000, 1048576) is called
    Then it returns Err "memory: opus analyze private grew by 2000000 bytes, bound 1048576"

  Scenario Outline: The growth check passes at its bound and on a negative difference
    When check_growth("opus analyze", "private", <short>, <long>, 1048576) is called
    Then it returns Ok

    Examples:
      | short   | long    | why                   |
      | 1000000 | 2048576 | the bound, inclusive  |
      | 3000000 | 1000000 | a negative difference |

  Scenario: The ceiling check gives an exact message
    When check_ceiling("flac analyze", "working-set", 50000000, 10000000, 33554432) is called
    Then it returns Err "memory: flac analyze working-set is 40000000 bytes over the baseline, ceiling 33554432"

  Scenario: The ceiling check passes at its bound
    When check_ceiling("flac analyze", "working-set", 43554432, 10000000, 33554432) is called
    Then it returns Ok

  Scenario: The fixture growth check gives an exact message
    When check_fixture_growth("opus", 9000000, 12000000, 4194304) is called
    Then it returns Err "memory: fixture opus grows by 3000000 bytes, need at least 4194304"

  Scenario Outline: Which figures fail the suite
    The report is built by one function from the figures (memory.md section
    6), so the suite checks it with given figures.
    Given the report function is given every figure within its bound except <figure>
    Then both tables of the report are printed in full
    And the last report line is "<last line>"

    Examples:
      | figure                                                           | last line                                               |
      | an opus analyze private growth of 2000000 bytes                  | result: FAIL (opus analyze growth private)              |
      | none                                                             | result: pass                                            |
      | a flac_huge_length private peak 33554433 bytes over the baseline | result: FAIL (flac_huge_length analyze ceiling private) |

  Scenario: The report has a fixed shape
    When the gate suite prints its report
    Then it starts with the lines "RNF-02 memory report (specs/contracts/memory.md)", "build: debug", "suite: gate" and a "baseline:" line
    And its first table has one row per measured child in the order of memory.md section 5.1
    And its second table has a "private" and a "working-set" row for every growth pair
    And its last line starts with "result: "

  # --- Empty state ---

  Scenario: A file with no audio frames is measured like any other
    The adapter returns ok for an empty data chunk (memory.md section 4.1,
    Empty state); the app later reports it as tooShort through
    fakeflac-core, which is outside this call.
    Given a WAV "wav_empty" with a valid 16-bit, 44100 Hz, stereo header and a data chunk of 0 bytes
    When a child runs analyze on it
    Then the result is "ok" with duration_ms 0
    And its private and its working set are each at most 33554432 bytes over the baseline
    And decode_chunks_for_test on it returns 0 chunks, 0 frames and a largest chunk of 0 frames

  # --- Reliability (RNF-04) ---

  @RNF-04
  Scenario: A truncated long track fails without growing memory
    Given "flac_truncated" is the long "flac" fixture cut to the first half of its bytes
    When a child runs analyze on it
    Then the result is "corruptStream"
    And its growth against the short "flac" analyze child is at most 1048576 bytes for private and for working set
    And it is at most 33554432 bytes over the baseline for both metrics
    And decode_chunks_for_test on it returns a CorruptStream error

  @RNF-04
  Scenario: A stream that declares a huge length allocates nothing for it
    Given "flac_huge_length" is the short "flac" fixture with its STREAMINFO total samples set to 68719476735
    When a child runs analyze on it
    Then the result is "corruptStream"
    And its private and its working set are each at most 33554432 bytes over the baseline

  # --- Unicode names (RNF-09) ---

  @RNF-09
  Scenario: Unicode names are measured like any other name
    Given every fixture lives in "Memoria ″Ñandú″" and is named "01 – Canción ‘x’.<ext>" or "02 – Canción ‘x’.<ext>"
    When the children run on them
    Then every child opens its file by its exact path, passed as an OsString
    And every intact fixture's result is "ok"

  # --- Long tracks (opt-in) ---

  Scenario Outline: A one-hour track costs no more memory than a 30-second one (long)
    Given "just memory" has written the 30 s and the 3600 s "<case>" fixture
    When a child runs <op> on each
    Then the growth of private and of working set is at most 1048576 bytes
    And the 3600 s child is at most 33554432 bytes over its baseline for both metrics

    Examples:
      | case    | op          |
      | flac_1h | analyze     |
      | flac_1h | spectrogram |
      | opus_1h | analyze     |

  Scenario: A 24-bit 96 kHz track stays within its own ceiling (long)
    The 64 MiB ceiling is provisional: it is not measured yet (memory.md F6).
    The first "just memory" report in the feature PR confirms it, or the spec
    returns to G1 with that report.
    Given "just memory" has written the 30 s and the 300 s "flac_hires" fixture (24-bit, 96000 Hz, stereo)
    When a child runs analyze on each
    Then the growth of private and of working set is at most 1048576 bytes
    And the 300 s child is at most 67108864 bytes over its baseline for both metrics

  Scenario: The long suite is opt-in and never runs in CI
    When the suite reads the justfile and the workflows as text
    Then the justfile has a "memory" recipe whose body holds "--release", "--ignored", "--test memory" and "--test-threads=1"
    And none of the recipes "ci", "check", "check-fast", "test" and "check-app" names "memory" as a whole word (\bmemory\b) in its dependency list or body
    And neither ".github/workflows/ci.yml" nor ".github/workflows/release.yml" contains "just memory"

  # --- The whole app (manual, indicative) ---

  Scenario: The app's peak memory on a real folder is reported
    Checked by hand (memory.md M1); aggregates only (ADR-0022). The targets
    are indicative and never fail.
    Given the release app has just started and its peaks P0 are read with Get-Process
    When the owner clears the result cache and analyses a real folder holding a 15-track album and a track of at least 40 minutes
    And reads the peaks P1, then opens the spectrogram of the longest track and reads the peaks P2
    Then the worker count, the track count, the folder size, P1 − P0 and P2 − P1 are recorded in the feature PR
    And P1 − P0 is compared with workers × 32 MiB + 64 MiB and P2 − P1 with 32 MiB

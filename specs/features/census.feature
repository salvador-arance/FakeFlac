@RP-12
Feature: Library census
  The census of specs/calibration/census.md: the detector's verdicts on a whole
  folder, in aggregates, through the app's own use case. Checked on the census
  sample folder of census.md section 8 (synthetic fixtures, 15 tracks in 5
  albums, Unicode names) in a fresh temporary folder outside the repository.
  Unless a scenario says otherwise the params are the defaults (detection v8).
  "The report builder" is the pure function that turns the tracks and albums
  of a run into summary.md (census.md section 9); its scenarios give it
  analyses built in the test, so they need no audio.

  Background:
    Given the census sample folder of census.md section 8

  # --- The run on the sample folder ---

  @RNF-04
  Scenario: A census of the sample folder reports its verdicts by codec
    When "census" runs on the sample folder
    Then it exits 0
    And stdout is summary.md followed by one line "elapsed: <seconds> s"
    And the header says "detection: algorithm_version 8, params default" and "tracks 15 | albums 5"
    And the verdict table is
      | codec | tracks | real | suspicious | fake | error |
      | flac  | 14     | 5    | 1          | 6    | 2     |
      | pcm   | 1      | 1    | 0          | 0    | 0     |
      | total | 15     | 6    | 1          | 6    | 2     |
    And the error table has the single row "flac" with corruptStream 1, tooShort 1, unreadable 0 and unsupportedFormat 0

  Scenario: The evidence of the Fake verdicts is counted by its kinds
    When "census" runs on the sample folder
    Then the evidence table is
      | group    | evidence               | tracks |
      | lossless | gapStep                | 3      |
      | lossless | cutoffStep             | 3      |
      | lossless | gapStep + cutoffStep   | 0      |
      | lossless | other                  | 0      |
      | lossy    | shortfall + cutoffStep | 0      |
      | lossy    | other                  | 0      |

  Scenario: Albums with a Fake are grouped by the share of their analysed tracks that is Fake
    When "census" runs on the sample folder
    Then the album table is
      | stratum  | albums | fake tracks | lossless | lossy |
      | whole    | 1      | 4           | 4        | 0     |
      | mixed    | 1      | 1           | 1        | 0     |
      | isolated | 1      | 1           | 1        | 0     |
      | total    | 3      | 6           | 6        | 0     |
    And the line after it is "albums with exactly one Fake among 2 or more analysed tracks: 2"
    And the margin table's column totals are whole 4, mixed 1, isolated 1 and total 6, with "none" 0 in every column

  Scenario: The reasons of the Suspicious verdicts are counted
    When "census" runs on the sample folder
    Then the reason table has "lowHighFrequencyContent" with lossless 1 and lossy 0
    And every other row of the reason table, "none" included, is 0 and 0

  @RNF-09
  Scenario: The per-track file is sorted, quoted and relative to the folder
    When "census" runs on the sample folder
    Then tracks.csv has the header of census.md section 5 and 15 rows sorted by path
    And the row of "Álbum B, aislado/05 – Falsa.flac" has its path quoted, verdict "fake", evidence "cutoffStep" and stratum "isolated"
    And that row has a cutoff_step_margin_db and empty gap_step_margin_db and shortfall_margin_hz
    And the row of "Álbum C/CD2/02 – Roto.flac" has verdict "error", error "corruptStream" and empty measurement fields
    And no field of tracks.csv holds the absolute path of the sample folder

  @RNF-04
  Scenario: An empty field of the per-track file means the value was not measured
    When "census" runs on the sample folder
    Then the row of "Álbum C/CD2/02 – Roto.flac" has empty sample_rate, bit_depth, channels, declared_bitrate_kbps and duration_ms
    And the row of "Álbum C/CD2/03 – Corto.flac" has verdict "error" and error "tooShort"
    And that row has a sample_rate, a channels and a duration_ms that are neither empty nor "0"

  @RNF-09
  Scenario: The summary names no folder and no file
    When "census" runs on the sample folder
    Then neither summary.md nor stdout contains the root's name "Biblioteca ″Ñandú″ & co"
    And neither contains any folder name or file name of the sample
    And neither contains any component of 4 or more characters of the root's absolute path

  @RNF-05
  Scenario: The census leaves the folder untouched
    When "census" runs on the sample folder
    Then every file of the sample folder keeps its size, modification time and content hash
    And the sample folder holds no new file
    And the only folder created is "target/census/<run>/" with exactly "summary.md" and "tracks.csv"

  Scenario: Two censuses of an unchanged folder are identical
    When "census" runs twice on the sample folder
    Then the two tracks.csv files are byte-identical
    And the two summary.md files differ only in their "run:" line

  Scenario: Two runs started in the same second get different folders
    Given a census run whose folder is "target/census/<run>/"
    When a second census starts in the same second
    Then its folder is "target/census/<run>-2/"

  # --- Other params ---

  Scenario: A copy of the shipped params is reported as custom and changes nothing
    When "census" runs on the sample folder with --params set to a copy of the shipped params file
    Then the header says "params custom <id>" with the 16-character content hash of that file
    And every table equals the one of the default run
    And the name of the params file does not appear in summary.md

  Scenario: Another params file changes the census
    When "census" runs on the sample folder with a params file whose gap_fake_db is 100
    Then the tracks made from "sfb21_gaps" are not Fake
    And the row "gapStep" of the evidence table is 0

  # --- The tables, from given analyses ---

  Scenario Outline: A lossless Fake is banded by its largest margin
    Given a lossless Fake track whose largest evidence margin is <margin> dB
    When the report builder runs
    Then the track is counted in the margin band "<band>"

    Examples:
      | margin | band   |
      | 0.0    | < 1    |
      | 0.99   | < 1    |
      | 1.0    | 1 – 3  |
      | 2.99   | 1 – 3  |
      | 3.0    | 3 – 6  |
      | 5.99   | 3 – 6  |
      | 6.0    | 6 – 12 |
      | 11.99  | 6 – 12 |
      | 12.0   | ≥ 12   |
      | 40.0   | ≥ 12   |

  Scenario: A track with both kinds is banded by the larger margin
    Given a lossless Fake track with a gapStep margin of 2.0 dB and a cutoffStep margin of 7.5 dB
    When the report builder runs
    Then the track is counted in the margin band "6 – 12"
    And in the evidence row "gapStep + cutoffStep"

  Scenario Outline: The stratum of an album compares integers exactly
    Given an album with <fake> Fake tracks among <analysed> analysed tracks
    When the report builder runs
    Then the album is counted in the stratum "<stratum>"

    Examples:
      | fake | analysed | stratum  |
      | 3    | 4        | whole    |
      | 1    | 1        | whole    |
      | 2    | 3        | mixed    |
      | 1    | 4        | mixed    |
      | 1    | 5        | isolated |
      | 2    | 9        | isolated |

  @RNF-04
  Scenario: Error tracks are left out of an album's share
    Given an album with 1 Fake track and 3 error tracks
    When the report builder runs
    Then the album is counted in the stratum "whole"
    And it is not counted in "albums with exactly one Fake among 2 or more analysed tracks"

  Scenario: Lossy fakes are counted but not banded
    Given Fake tracks with the codecs mp3, aac and opus, each with a shortfall item and a cutoffStep item
    When the report builder runs
    Then the evidence row "lossy | shortfall + cutoffStep" is 3
    And the "lossy" column of the album table counts them
    And no cell of the margin table counts them

  Scenario: Evidence that the rules do not allow is shown, not hidden
    Given a lossless Fake track with empty evidence
    When the report builder runs
    Then it is counted in the evidence row "lossless | other" and in the margin row "none"

  Scenario: A Suspicious track is counted once per reason
    Given a lossy Suspicious track with the reasons cutoffBelowDeclared and gradualRolloff
    And a lossless Suspicious track without reasons
    When the report builder runs
    Then the rows "cutoffBelowDeclared" and "gradualRolloff" each count 1 lossy
    And the row "none" counts 1 lossless

  @RNF-04
  Scenario: A track without a codec is counted as unknown
    Given an M4A error track without a codec and with the error unsupportedFormat
    When the report builder runs
    Then the verdict table has a row "unknown" with 1 track and 1 error
    And the error table has a row "unknown" with unsupportedFormat 1

  Scenario: A folder without errors says so
    Given analyses without any error track
    When the report builder runs
    Then the errors section is the single line "errors: none"

  # --- Refused runs (empty and error states) ---

  Scenario Outline: A wrong command line prints the usage and writes nothing
    When "census" runs with <arguments>
    Then it exits 2
    And stderr is "usage: fakeflac-calibrate census FOLDER [--params FILE]"
    And stdout is empty and no folder is created under "target/census/"

    Examples:
      | arguments                               |
      | no argument                             |
      | two folders                             |
      | a folder and --params without a value   |
      | a folder and --params twice             |
      | a folder and an unknown option          |

  Scenario Outline: A folder that cannot be censused is refused without naming it
    When "census" runs on <folder>
    Then it exits 2
    And stderr is "<message>"
    And stderr does not contain the folder's path
    And stdout is empty and no folder is created under "target/census/"

    Examples:
      | folder                                   | message                                                      |
      | a folder that does not exist             | error: census: the folder does not exist or is not a folder  |
      | a file instead of a folder               | error: census: the folder does not exist or is not a folder  |
      | a folder that holds only "notas.txt"     | error: census: no supported audio files in the folder        |

  Scenario: An invalid params file is refused as accept refuses it
    When "census" runs on the sample folder with a params file that "accept" rejects
    Then it exits 2 with the message "accept" gives for that file
    And no folder is created under "target/census/"

  # --- Wiring ---

  Scenario: The census is opt-in and never runs in CI
    Then the justfile has a "census" recipe whose body holds "fakeflac-calibrate" and "census"
    And none of the recipes "ci", "check", "check-fast", "test" and "check-app" names "census" as a whole word in its dependency list or body
    And neither ".github/workflows/ci.yml" nor ".github/workflows/release.yml" contains "just census"

  # --- The owner's library (manual) ---

  Scenario: The census of the owner's library matches the app's run
    Checked by hand (census.md MC1 and MC2); aggregates only (ADR-0022).
    Given the library of the 2026-11-14 diagnosis, unchanged, and detection v8
    When the owner runs "just census" on it
    Then summary.md and the elapsed time are recorded in the feature PR
    And the totals are 11396 tracks: 9003 real, 1173 suspicious, 1156 fake and 64 errors
    And the lossless evidence rows are gapStep 95, cutoffStep 188 and both 218
    And the library's file count, sizes and modification times are unchanged

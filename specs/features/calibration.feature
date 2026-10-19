@RP-11
Feature: Calibration and acceptance against a labelled corpus
  The calibration tool of specs/calibration/calibration.md, checked on the
  sample corpus of specs/calibration/sample-corpus.md (synthetic fixtures,
  fixture transcoder, no ffmpeg) in two fresh temporary folders used as
  FAKEFLAC_CORPUS and FAKEFLAC_CALIBRATION. Unless a scenario says otherwise,
  the params file is specs/detection/detection-params.v2.toml.
  The v4 expectations on fxv0 (sfb21_gaps) rest on a step of 3 to 10 dB in the
  lossy range (fixtures.md, "Verdicts with v4": derived, and measured once
  before G1 at D 6.15 dB, fs 16050 Hz); the test stage confirms it first and
  stops for G1 if it does not hold.

  # --- Environment -------------------------------------------------------

  @RP-11
  Scenario Outline: Missing environment variables
    Given FAKEFLAC_CORPUS is <corpus> and FAKEFLAC_CALIBRATION is <calibration>
    When the binary runs "accept"
    Then the exit code is 2
    And the error output names <missing>
    And nothing is written anywhere

    Examples:
      | corpus | calibration | missing                                    |
      | unset  | unset       | FAKEFLAC_CORPUS and FAKEFLAC_CALIBRATION   |
      | unset  | set         | FAKEFLAC_CORPUS                            |
      | set    | unset       | FAKEFLAC_CALIBRATION                       |

  @RP-11
  Scenario Outline: Invalid location of the working folder
    Given FAKEFLAC_CALIBRATION is <where>
    When the binary runs "accept"
    Then the exit code is 2
    And nothing is written anywhere

    Examples:
      | where                                   |
      | a relative path                         |
      | a folder inside the repository          |
      | a folder inside FAKEFLAC_CORPUS         |
      | a folder that contains FAKEFLAC_CORPUS  |

  # --- Acceptance on the sample corpus -----------------------------------

  @RP-11 @RNF-04
  Scenario: Global confusion matrix
    The corrupt and the too-short tracks are counted as Error and the run continues.
    When "accept" runs on the sample corpus
    Then the exit code is 0
    And the global confusion matrix for split "all" is:
      | truth | Real | Suspicious | Fake | Error | total |
      | real  | 4    | 1          | 0    | 2     | 7     |
      | fake  | 0    | 6          | 19   | 0     | 25    |
    And labels.csv has 33 rows

  @RP-11
  Scenario: Matrix per encoder setting
    When "accept" runs on the sample corpus
    Then the matrix per setting is:
      | setting        | Real | Suspicious | Fake | Error |
      | fx128          | 0    | 0          | 6    | 0     |
      | fxv0           | 0    | 0          | 6    | 0     |
      | fx320          | 0    | 0          | 6    | 0     |
      | fxweak         | 0    | 6          | 0    | 0     |
      | confirmed_fake | 0    | 0          | 1    | 0     |

  @RP-11
  Scenario: False positives by origin and the primary figure
    When "accept" runs on the sample corpus
    Then the false-positive table for split "all" is:
      | origin          | class     | tracks | albums | Fake | Suspicious | Error |
      | tienda          | primary   | 2      | 1      | 0    | 0          | 0     |
      | revisado        | reviewed  | 1      | 1      | 0    | 0          | 2     |
      | analisis-limpio | clean     | 2      | 1      | 0    | 1          | 0     |
      | analisis        | reference | 1      | 1      | 0    | 0          | 0     |
    And the primary figure is "0/2 tracks (0.0 %, 95 % CI 0.0–65.8 %), 0/1 albums"
    And the primary figure carries the small-sample warning
    And targets T1 and T2 are "PASS"

  @RP-11
  Scenario: Watch list
    When "accept" runs on the sample corpus
    Then the watch list has exactly one entry "W1" with verdict "real" and its G, D, fs, fe, H and nearest_fake_margin_db
    And report.md shows the path of W1
    And summary.md shows "W1" without its path

  @RP-11
  Scenario: Margins against every threshold
    When "accept" runs on the sample corpus
    Then tracks.csv has 33 rows, one per labelled track
    And it has the columns m_low_hf_db, m_gap_fake_db, m_gap_suspicious_db, m_step_fake_db, m_step_suspicious_db, m_step_low_db, m_step_high_db, m_gradual_hz, fs_fake_range_hz and nearest_fake_margin_db
    And for every row with spectra m_gap_fake_db equals gap_fake_db minus G
    And the 2 rows of the Error tracks have empty measurements and margins

  @RP-11
  Scenario: Undetectable transcodes on band-limited sources
    When a generated transcode has a source whose measured bandwidth extent is below or equal to the setting lowpass
    Then the transcode is classified as undetectable
    And it is counted in the Undetectable column of the confusion matrices
    And it is listed in the undetectable section of report.md

  # --- Split -------------------------------------------------------------

  @RP-11
  Scenario: Deterministic split by album
    When "accept" runs on the sample corpus in two fresh FAKEFLAC_CALIBRATION folders
    Then both labels.csv files are byte-identical
    And "Colección/Álbum Ü/CD1/01 – Uno.flac" and "Colección/Álbum Ü/CD2/01 – Dos.flac" have album_id "Colección/Álbum Ü"
    And every generated row has the album_id and the split of its source_path

  @RP-11
  Scenario: Split ratio and stability
    Given the album ids "album-00000" to "album-09999" in one class and the seed of calibration.v1.toml
    When the split is computed
    Then exactly 3000 albums are in holdout
    When the album id "album-10000" is added and the split is computed again
    Then exactly 3000 albums are still in holdout
    And "album-10000" is in holdout and "album-09581" has moved to tune
    And no other album has changed split

  @RP-11
  Scenario: Stratified split guarantees holdout in small classes
    Given 5 albums in class "primary" and 10 albums in class "reviewed"
    When the stratified split is computed
    Then exactly 2 albums of class "primary" are assigned to holdout
    And exactly 3 albums of class "reviewed" are assigned to holdout

  @RP-11
  Scenario Outline: Extreme holdout fractions ignore the class size
    Given <albums> albums in class "primary" and split.holdout_fraction is <fraction>
    When the stratified split is computed
    Then exactly <holdout> albums of class "primary" are assigned to holdout

    Examples:
      | albums | fraction | holdout |
      | 1      | 0.0      | 0       |
      | 5      | 0.0      | 0       |
      | 1      | 0.3      | 0       |
      | 1      | 1.0      | 1       |
      | 5      | 1.0      | 5       |

  # --- Features cache ----------------------------------------------------

  @RP-11
  Scenario: A cold run decodes every labelled track once
    When "accept" runs on the sample corpus with an empty features folder
    Then 33 files are decoded and 24 entries are transcoded

  @RP-11
  Scenario: A second run decodes nothing
    Given "accept" has run once on the sample corpus
    When "accept" runs again with no change
    Then 0 files are decoded and 0 entries are transcoded
    And labels.csv, tracks.csv, report.md and summary.md are equal to the first run's, except the run id, the date and the cache statistics

  @RP-11
  Scenario: Cached results equal fresh results
    Given "accept" has run once on the sample corpus
    When "accept" runs again with an empty features folder
    Then tracks.csv is equal to the first run's

  @RP-11
  Scenario: A rule-only change reuses the cache
    Given "accept" has run once on the sample corpus
    When "accept" runs with the params file candidate.toml (the next free algorithm_version, gap_fake_db 4.0)
    Then 0 files are decoded and 0 entries are transcoded

  @RP-11
  Scenario: An extraction change invalidates the cache
    Given "accept" has run once on the sample corpus
    When "accept" runs with detection-params.v2.toml changed to window "blackman_harris_4"
    Then a second features file with a different fingerprint id exists
    And 33 files are decoded and 24 entries are transcoded

  @RP-11
  Scenario: A changed source is extracted and transcoded again
    Given "accept" has run once on the sample corpus
    And track 1 is written again with a different modification time
    When "accept" runs again
    Then 5 files are decoded and 4 entries are transcoded

  @RP-11
  Scenario: Generated audio is deleted once cached
    When "accept" runs on the sample corpus
    Then FAKEFLAC_CALIBRATION/generated holds no file

  @RP-11
  Scenario: --no-generate leaves labels.csv untouched
    Given "accept" has run once on the sample corpus
    And the contents of labels.csv are recorded
    When "accept" runs with "--no-generate"
    Then labels.csv is byte-identical to the recorded file
    And missing generated entries are reported as not generated

  # --- Several parameter files -------------------------------------------

  @RP-11
  Scenario: v2 and a candidate side by side in one run
    When "accept" runs on the sample corpus with an empty features folder and the params files detection-params.v2.toml and candidate.toml
    Then 33 files are decoded and 24 entries are transcoded
    And the side-by-side table has one column per params file
    And for candidate.toml the fake row of the global matrix is 0 Real, 0 Suspicious, 25 Fake, 0 Error
    And the real row is the same for both files
    And the verdict-change table against v2 has exactly 6 changes, all "suspicious" to "fake" on positives

  @RP-11
  Scenario: A v1 params file is rejected
    When "accept" runs with the params file specs/detection/detection-params.v1.toml
    Then the exit code is 2
    And the error output says "detection-params.v1.toml: not a DetectionParams file of the v2 rule family"

  @RP-11 @RF-04
  Scenario: v2 and v3 side by side in one run
    Detection v3 adds the full-band guard (specs/detection/rules.md). The fxv0
    fixture (sfb21_gaps) has gated content up to Nyquist, so it is full band.
    When "accept" runs on the sample corpus with an empty features folder and the params files detection-params.v2.toml and detection-params.v3.toml
    Then 33 files are decoded and 24 entries are transcoded
    And for detection-params.v3.toml the fake row of the global matrix is 0 Real, 12 Suspicious, 13 Fake, 0 Error
    And the real row is the same for both files
    And the verdict-change table against v2 has exactly 6 changes, all "fake" to "suspicious" on fxv0 positives
    And the per-setting change table shows those 6 changes on fxv0 and none on any other setting
    And the caught rate of every setting is the same for both files
    And the per-setting table of detection-params.v3.toml shows a full_band count and the median, 95th percentile and maximum of B for every setting
    And the watch list entry "W1" of the sample corpus shows U, B and full_band "true" for detection-params.v3.toml and empty U, B and full_band for detection-params.v2.toml

  @RP-11 @RF-04
  Scenario: Full-band columns in tracks.csv
    When "accept" runs on the sample corpus with the params files detection-params.v2.toml and detection-params.v3.toml
    Then tracks.csv has the columns U, B, full_band, m_full_band_level_db, m_full_band_burst_db and full_band_margin_db
    And they are empty on every row of detection-params.v2.toml
    And on every row of detection-params.v3.toml with spectra, full_band is "true" exactly when full_band_margin_db is at least 0
    And on those rows nearest_fake_margin_db equals the larger of min(m_gap_fake_db, m_step_fake_db) and full_band_margin_db when full_band is "true", and min(m_gap_fake_db, m_step_fake_db) otherwise

  @RP-11 @RF-04
  Scenario Outline: A params file that fails DetectionParams::validate is rejected
    Given a copy of detection-params.v3.toml with <change>
    When "accept" runs with that params file
    Then the exit code is 2
    And the error output names the file and "<inequality>"

    Examples:
      | change                                 | inequality                            |
      | full_band.low_hz set to 20000.0        | fake_max_cutoff_hz < full_band.low_hz |
      | full_band.high_hz set to 20700.0       | full_band.low_hz < full_band.high_hz  |
      | verdict.gap_suspicious_db set to 6.5   | gap_suspicious_db ≤ gap_fake_db       |
      | verdict.step_suspicious_db set to 10.5 | step_suspicious_db ≤ step_fake_db     |

  @RP-11 @RF-04
  Scenario: A [full_band] section with a missing field is a TOML error
    Given a copy of detection-params.v3.toml without the min_burst_db line of [full_band]
    When "accept" runs with that params file
    Then the exit code is 2
    And the error output gives the parser's message, which names "min_burst_db"

  @RP-11 @RF-04
  Scenario: v2, v3 and v4 side by side in one run
    Detection v4 (specs/detection/rules.md, ADR-0039) adds the lossy-step
    condition to the full-band guard. The fxv0 fixture (sfb21_gaps) passes the
    U and B thresholds, but its deepest step (about 6 dB at 16 kHz) lies in the
    lossy range, so with v4 it is not full band and stays fake, as with v2
    (specs/detection/fixtures.md, "Verdicts with v4"). The verdict-change tables
    compare each file with the first one, v2.
    When "accept" runs on the sample corpus with an empty features folder and the params files detection-params.v2.toml, detection-params.v3.toml and detection-params.v4.toml
    Then 33 files are decoded and 24 entries are transcoded
    And the side-by-side table has one column per params file, with algorithm_version 2, 3 and 4
    And for detection-params.v4.toml the fake row of the global matrix is 0 Real, 6 Suspicious, 19 Fake, 0 Error
    And the real row is the same for the three files
    And the verdict-change table of detection-params.v4.toml against v2 has no change
    And the verdict-change table of detection-params.v3.toml against v2 has exactly 6 changes, all "fake" to "suspicious" on fxv0 positives
    And the caught rate of every setting is the same for the three files
    And the per-setting table of detection-params.v4.toml shows a full_band count of 0 for every setting, and the median, 95th percentile and maximum of B for every setting
    And the watch list entry "W1" of the sample corpus shows U, B and full_band "true" for detection-params.v3.toml and for detection-params.v4.toml

  @RP-11 @RF-04
  Scenario: Full-band columns in tracks.csv with detection v4
    m_full_band_step_db is the margin of the lossy-step condition
    (specs/calibration/evaluation.md section 8): defined only when the params
    have max_lossy_step_db and fs lies in the lossy range.
    When "accept" runs on the sample corpus with the params files detection-params.v2.toml, detection-params.v3.toml and detection-params.v4.toml
    Then tracks.csv has the column m_full_band_step_db
    And it is empty on every row of detection-params.v2.toml and of detection-params.v3.toml
    And on every row of detection-params.v4.toml with spectra, m_full_band_step_db is filled exactly when fs is in [14500, 20300), and then equals 3.0 minus D
    And on those rows full_band is "true" exactly when full_band_margin_db is at least 0 and m_full_band_step_db is empty or greater than 0
    And on those rows nearest_fake_margin_db equals the larger of min(m_gap_fake_db, m_step_fake_db) and min(full_band_margin_db, m_full_band_step_db), ignoring an empty term, when full_band is "true", and min(m_gap_fake_db, m_step_fake_db) otherwise
    And the fxv0 rows of detection-params.v4.toml have m_full_band_step_db at most 0

  @RP-11 @RF-04
  Scenario Outline: A v4 params file with an invalid max_lossy_step_db is rejected
    Given a copy of detection-params.v4.toml with <change>
    When "accept" runs with that params file
    Then the exit code is 2
    And the error output names the file and "<inequality>"

    Examples:
      | change                                          | inequality                                 |
      | full_band.max_lossy_step_db set to -0.5         | full_band.max_lossy_step_db ≥ 0            |
      | full_band.max_lossy_step_db set to 10.5         | full_band.max_lossy_step_db ≤ step_fake_db |

  # --- trusted.txt and confirmed.txt -------------------------------------

  @RP-11
  Scenario Outline: Errors in trusted.txt stop the run with the line number
    Given line 6 of the sample trusted.txt is replaced by "<line>"
    When "accept" runs on the sample corpus
    Then the exit code is 2
    And the error output is "trusted.txt:6: <message>"
    And nothing is written in FAKEFLAC_CALIBRATION

    Examples:
      | line                                                     | message                                    |
      | Colección\Álbum Ü \| comprado                            | unknown origin 'comprado' (expected one of: cd-log, accuraterip, bandcamp, qobuz, tienda, revisado, usuario, analisis-limpio, analisis) |
      | Colección\No existe \| revisado                          | path does not exist: Colección\No existe   |
      | Colección\Álbum Ü                                        | missing origin                             |
      | Colección\Álbum Ü\folder.jpg \| revisado                 | not a FLAC or WAV file                     |
      | ..\fuera \| revisado                                     | '..' is not allowed                        |
      | Compras\Ñandú – Ácido, Vol. 1 (2024) \| tienda           | already listed at line 4                   |
      | !Colección\Álbum Ü\CD1 \| x                              | exclusion is not inside any listed folder  |
      | library_root = C:\otra                                   | library_root is already set at line 2      |

  @RP-11
  Scenario: A folder with no lossless audio is an error
    Given a folder "Vacío" under FAKEFLAC_CORPUS that holds only "folder.jpg"
    And line 6 of the sample trusted.txt is replaced by "Vacío | revisado"
    When "accept" runs on the sample corpus
    Then the exit code is 2
    And the error output is "trusted.txt:6: no FLAC or WAV files under this folder"

  @RP-11
  Scenario: Every error is reported, in line order
    Given line 6 of the sample trusted.txt is replaced by "Colección\Álbum Ü | comprado"
    And line 8 of the sample trusted.txt is replaced by "Sueltas\Mixto\05 – Cuatro.flac"
    When "accept" runs on the sample corpus
    Then the exit code is 2
    And the error output has exactly 2 lines, the first starting with "trusted.txt:6:" and the second with "trusted.txt:8:"

  @RP-11
  Scenario: library_root does not match FAKEFLAC_CORPUS
    Given line 2 of the sample trusted.txt is "library_root = " followed by another existing folder
    When "accept" runs on the sample corpus
    Then the exit code is 2
    And the error output is "trusted.txt:2: library_root does not match FAKEFLAC_CORPUS"
    And nothing is written in FAKEFLAC_CALIBRATION

  @RP-11
  Scenario: library_root with a trailing separator matches
    Given line 2 of the sample trusted.txt is "library_root = " followed by FAKEFLAC_CORPUS and a trailing "\"
    When "accept" runs on the sample corpus
    Then the exit code is 0

  @RP-11
  Scenario: Empty trusted list
    Given trusted.txt holds only lines 1 and 2 of the sample
    When "accept" runs on the sample corpus
    Then the exit code is 2
    And the error output is "trusted.txt: no tracks listed"

  @RP-11
  Scenario: The most specific line wins
    Given a line "Colección\Álbum Ü\CD2\01 – Dos.flac | revisado" is added to the sample trusted.txt
    When "accept" runs on the sample corpus
    Then the labels.csv row of "Colección/Álbum Ü/CD2/01 – Dos.flac" has trust_origin "revisado" and class "reviewed"
    And the labels.csv row of "Colección/Álbum Ü/CD1/01 – Uno.flac" has trust_origin "analisis-limpio"
    And "Compras/Ñandú – Ácido, Vol. 1 (2024)/03 – Bonus.flac" has no labels.csv row

  @RP-11
  Scenario Outline: Errors in confirmed.txt
    Given line 2 of the sample confirmed.txt is replaced by "<line>"
    When "accept" runs on the sample corpus
    Then the exit code is 2
    And the error output is "confirmed.txt:2: <message>"

    Examples:
      | line                                                        | message                                         |
      | Revisión\Ørsted\01 – Tres.wav \| fake                       | also trusted (trusted.txt line 7)               |
      | Sueltas\Mixto\06 – Falso.flac \| false-positive             | not a trusted track                             |
      | Sueltas\Mixto \| false-positive                             | must be a file                                  |
      | Sueltas\Mixto\06 – Falso.flac \| dudoso                     | unknown kind 'dudoso' (expected fake or false-positive) |

  # --- Unicode, privacy and read-only corpus ------------------------------

  @RP-11 @RNF-09
  Scenario: Unicode names in labels.csv
    When "accept" runs on the sample corpus
    Then labels.csv is UTF-8 without BOM with CRLF line ends
    And it has a row whose path is "Compras/Ñandú – Ácido, Vol. 1 (2024)/01 – Intro ’A’.flac", written between double quotes because of the comma
    And reading labels.csv back as RFC 4180 gives that path unchanged

  @RP-11
  Scenario: summary.md holds no corpus names or paths
    When "accept" runs on the sample corpus with the params files detection-params.v2.toml and candidate.toml
    Then summary.md does not contain the value of FAKEFLAC_CORPUS or of FAKEFLAC_CALIBRATION, with either separator
    And summary.md does not contain any folder name or file stem of the sample corpus
    And summary.md does not contain any note of trusted.txt or confirmed.txt
    And labels.csv, tracks.csv and report.md do not contain the value of FAKEFLAC_CORPUS or of FAKEFLAC_CALIBRATION

  @RP-11
  Scenario: privacy-check exits 1 when a changed file cannot be read
    Given a changed file in the diff that cannot be read
    When "privacy-check" runs
    Then the exit code is 1
    And the error output reports the unreadable file

  @RP-11 @RNF-05
  Scenario: The corpus is not modified
    Given the list of every file under FAKEFLAC_CORPUS with its size and modification time
    When "accept" runs on the sample corpus
    Then the list is unchanged

  # --- Grid search (/calibrate) -------------------------------------------

  @RP-11
  Scenario: The grid search proposes new parameters from tune data
    Given split.holdout_fraction is 0.0
    And the grid of sample-corpus.md section 5
    When "calibrate" runs on the sample corpus
    Then the exit code is 0
    And the baseline cost is 1.1 and the cost of "gap_fake_db = 4.0" is 0.5
    And the report folder holds detection-params.v5.toml
    And that file parses to detection-params.v2.toml with algorithm_version 5 and gap_fake_db 4.0
    And its first two lines are the "PROPOSAL" and "Changed: verdict.gap_fake_db 6.0 -> 4.0" comments
    And no file in the repository has changed

  @RP-11
  Scenario: Holdout data never chooses the parameters
    Given split.holdout_fraction is 1.0
    And the grid of sample-corpus.md section 5
    When "calibrate" runs on the sample corpus
    Then the exit code is 0
    And the output says "no change proposed"
    And the report folder holds no detection-params file

  @RP-11
  Scenario: Extraction parameters cannot be searched
    Given a grid whose values include "fft.window"
    When "calibrate" runs on the sample corpus
    Then the exit code is 2
    And the error output says "extraction parameter; compare params files with accept instead"

  @RP-11 @RF-04
  Scenario: A full_band key needs a baseline with a [full_band] section
    Given a grid with baseline detection-params.v2.toml whose values include "full_band.min_burst_db"
    When "calibrate" runs on the sample corpus
    Then the exit code is 2
    And the error output says "full_band.min_burst_db: the baseline has no [full_band] section"

  @RP-11 @RF-04
  Scenario: The full-band guard can be searched from a v3 baseline
    The sample corpus has no full-band negative with a gap, so on it the guard
    only costs detections and the search proposes to switch it off. On the real
    corpus the confirmed false positives FP1-FP3 weigh against that (fp = 10 each).
    Given split.holdout_fraction is 0.0
    And a grid with baseline detection-params.v3.toml, must_catch ["fx128"] and values "full_band.min_burst_db" = [3.0, 100.0]
    When "calibrate" runs on the sample corpus
    Then the exit code is 0
    And with the point "full_band.min_burst_db = 100.0" the fxv0 entries are "fake" instead of "suspicious", and its cost is lower than the baseline cost
    And the report folder holds detection-params.v5.toml with algorithm_version 5 and full_band.min_burst_db 100.0
    And its second line is the comment "# Changed: full_band.min_burst_db 3.0 -> 100.0"

  @RP-11 @RF-04
  Scenario: The lossy-step condition can be searched from a v4 baseline
    On the sample corpus the step condition is what keeps fxv0 Fake under v4
    (specs/detection/fixtures.md, "Verdicts with v4"): with max_lossy_step_db
    10.0 its 16 kHz step (about 6 dB) no longer removes the guard. 12.0 breaks
    max_lossy_step_db ≤ step_fake_db and is skipped.
    Given split.holdout_fraction is 0.0
    And a grid with baseline detection-params.v4.toml, must_catch ["fx128"] and values "full_band.max_lossy_step_db" = [3.0, 10.0, 12.0]
    When "calibrate" runs on the sample corpus
    Then the exit code is 0
    And 1 point is skipped by the invariants
    And the baseline cost is 1.1 and the cost of "full_band.max_lossy_step_db = 10.0" is 1.7
    And with that point the fxv0 entries are "suspicious" instead of "fake"
    And the output says "no change proposed"
    And the report folder holds no detection-params file

  @RP-11 @RF-04
  Scenario: A max_lossy_step_db key needs a baseline that has the field
    Given a grid with baseline detection-params.v3.toml whose values include "full_band.max_lossy_step_db"
    When "calibrate" runs on the sample corpus
    Then the exit code is 2
    And the error output says "full_band.max_lossy_step_db: the baseline's [full_band] section has no max_lossy_step_db"

  @RP-11
  Scenario: Undetectable transcodes are excluded from must_catch feasibility and cost
    Given a must_catch setting with an undetectable transcode that receives verdict "real"
    When the grid search evaluates candidate parameters
    Then the undetectable transcode does not cause must_catch feasibility to fail
    And it does not contribute to the grid cost

  @RP-11
  Scenario: Feasibility requires that no confirmed fake is Real
    Given a candidate parameter set under which a confirmed fake track receives verdict "real"
    When feasibility is evaluated during grid search
    Then the candidate point is marked infeasible

  @RP-11
  Scenario: A confirmed fake in holdout still constrains the search
    Given a candidate parameter set under which a confirmed fake track in holdout receives verdict "real"
    And on the tune rows that candidate costs less than the baseline
    When the grid search ranks the points
    Then the candidate point is marked infeasible
    And it is not proposed

  @RP-11
  Scenario: A mixed-class album takes the strongest class
    Given an album with tracks of class "clean" in "CD1" and of class "reviewed" in "CD2"
    And a confirmed fake in "CD2"
    When the stratified split is computed
    Then the album is counted once, in class "reviewed"
    And every track of the album, the confirmed fake included, has the split of the album

  # --- ffmpeg (no ffmpeg needed) ------------------------------------------

  @RP-11
  Scenario: ffmpeg command lines
    Given the setting "cbr128" of calibration.v1.toml and the source "Compras/Ñandú – Ácido, Vol. 1 (2024)/01 – Intro ’A’.flac" with a fragment from 90.500 s of 60.000 s
    When the ffmpeg calls are built
    Then the encode call is ffmpeg with the arguments "-nostdin -hide_banner -loglevel error -ss 90.500 -t 60.000 -i <source> -map 0:a:0 -map_metadata -1 -c:a libmp3lame -b:a 128k -y <generated>/<id>.mp3", each one a separate argument and <source> passed unchanged
    And the decode call is ffmpeg with the arguments "-nostdin -hide_banner -loglevel error -i <generated>/<id>.mp3 -map 0:a:0 -map_metadata -1 -c:a flac -sample_fmt s16 -y <generated>/<id>.flac"

  @RP-11
  Scenario: ffmpeg encode command with full track default
    Given the setting "cbr128" of calibration.v1.toml and the source "Compras/Ñandú – Ácido, Vol. 1 (2024)/01 – Intro ’A’.flac" with full track default (fragment_s omitted)
    When the ffmpeg calls are built
    Then the encode call arguments omit "-ss" and "-t"

  @RP-11
  Scenario: ffmpeg missing when a transcode is needed
    Given the sample corpus with the settings of calibration.v1.toml and an empty features folder
    And FAKEFLAC_FFMPEG points to a file that does not exist
    When the binary runs "accept"
    Then the exit code is 3
    And the error output says "ffmpeg not found"

  @RP-11
  Scenario: The binary rejects the fixture transcoder
    Given a config with a setting whose encoder is "fixture"
    When the binary runs "accept"
    Then the exit code is 2

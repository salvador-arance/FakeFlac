@RP-11
Feature: Calibration and acceptance against a labelled corpus
  The calibration tool of specs/calibration/calibration.md, checked on the
  sample corpus of specs/calibration/sample-corpus.md (synthetic fixtures,
  fixture transcoder, no ffmpeg) in two fresh temporary folders used as
  FAKEFLAC_CORPUS and FAKEFLAC_CALIBRATION. Unless a scenario says otherwise,
  the params file is specs/detection/detection-params.v2.toml.

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
    And adding the album id "album-10000" only inserts it into the hash order without changing the split of other albums
 
  @RP-11
  Scenario: Stratified split guarantees holdout in small classes
    Given 5 albums in class "primary" and 10 albums in class "reviewed"
    When the stratified split is computed
    Then exactly 2 albums of class "primary" are assigned to holdout
    And exactly 3 albums of class "reviewed" are assigned to holdout

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
    When "accept" runs with the params file candidate.toml (algorithm_version 3, gap_fake_db 4.0)
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
    And the report folder holds detection-params.v3.toml
    And that file parses to detection-params.v2.toml with algorithm_version 3 and gap_fake_db 4.0
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

  @RP-11
  Scenario: Feasibility requires that no confirmed fake is Real
    Given a candidate parameter set under which a confirmed fake track receives verdict "real"
    When feasibility is evaluated during grid search
    Then the candidate point is marked infeasible

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

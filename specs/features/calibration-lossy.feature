@RP-11 @RF-15
Feature: Calibration and acceptance for lossy files
  The lossy extensions of specs/calibration/lossy.md, checked on the lossy
  variant of the sample corpus (lossy.md section 9): the base sample corpus of
  specs/calibration/sample-corpus.md plus three fixture settings, three lossy
  entries (l128, l320, l128_320), four lossy corpus files and the chain C1, in
  two fresh temporary folders, with the fixture transcoder (no ffmpeg). Lossy
  fixtures: specs/detection/lossy-fixtures.md. Unless a scenario says
  otherwise, the run uses the params files detection-params.v2.toml and
  detection-params.v5.toml, in that order. "The sample lossy trusted.txt" and
  "the sample lossy confirmed.txt" are the files of lossy.md section 9.
  Changed for detection v7 (specs/detection/lossy-source.md, Q25 (h); RF-16):
  the per-group table (lossy.md 6.3) gains the column "streaming", three fc
  percentile columns and one sub-row per decoded sample rate, labelled
  "<group> @ <bucket>" (lossy.md 6.8, C13). No scenario text changes. "Lossy
  table per group" says "contains the rows": its rows are still present with
  the same counts, the new sub-rows are extra rows (only "@ 44100" ones, as
  every lossy fixture of the variant is 44.1 kHz) and the step compares only
  the columns it names, so the new columns are not compared; with v2 and v5
  (no [lossy.streaming]) "streaming" is 0 in every row anyway. A derived test
  that compares the whole table or summary.md as exact text gains the new
  columns and sub-rows in the test stage (declared consequence, lossy.md 6.8
  and lossy-source.md Q25 (h), (j)).

  # --- Lossless rows do not change -------------------------------------------

  @RP-11 @RF-15
  Scenario: The lossless tables equal those of the base sample corpus
    When "accept" runs on the lossy variant of the sample corpus
    Then the exit code is 0
    And for both params files every lossless table of split "all" equals the one of the base sample corpus
    And the lossless global confusion matrix has no lossy row in it

  @RP-11 @RF-15
  Scenario: Lossy rows are labelled
    When "accept" runs on the lossy variant of the sample corpus
    Then labels.csv has 55 rows
    And the 18 generated lossy rows have domain "lossy", root "generated" and setting "l128", "l320" or "l128_320"
    And the "l128_320" rows have truth "fake", class "positive", stages "fxmp3_128>fxmp3_320_src128" and source_bitrate_kbps 128
    And the "l128" and "l320" rows have truth "real" and class "generated"
    And the row of "Pérdidas/Vídeo/08 – Señal ’AAC’.m4a" has truth "real", origin "confirmed_real", class "reviewed", codec "aac" and declared_bitrate_kbps 128
    And the row of "Pérdidas/Vídeo/07 – Falso 320.mp3" has truth "fake", origin "confirmed_fake", upstream "aac" and source_bitrate_kbps 128
    And the row of "Pérdidas/Vídeo/09 – Ópera.opus" has not_evaluated "opus decoder pending"

  @RP-11 @RF-15
  Scenario: A cold run decodes every lossy file once, except Opus
    When "accept" runs on the lossy variant of the sample corpus with an empty features folder
    Then 54 files are decoded
    And a second run decodes 0 files and transcodes 0 entries

  # --- Lossy evaluation ---------------------------------------------------------

  @RP-11 @RF-15
  Scenario: Params without a [lossy] section do not evaluate lossy rows
    When "accept" runs on the lossy variant of the sample corpus
    Then the header says that detection-params.v2.toml evaluates no lossy row (22 not evaluated)
    And the header says that detection-params.v5.toml leaves 1 lossy row not evaluated
    And no lossy row is counted in any Error column

  @RP-11 @RF-15
  Scenario: Lossy confusion matrix
    When "accept" runs on the lossy variant of the sample corpus
    Then the lossy confusion matrix of detection-params.v5.toml for split "all" is:
      | truth | Real | Suspicious | Fake | Undetectable | Source-limited | Error | total |
      | real        | 11 | 0 | 0 | — | 2 | 0 | 13 |
      | fake        | 0  | 0 | 7 | 1 | — | 0 | 8  |
      | fake (weak) | 0  | 0 | 0 | — | — | 0 | 0  |

  @RP-11 @RF-15
  Scenario: Lossy table per group
    Groups with no evaluated row (trusted lossy classes, other upstream
    values, other sample rates, weak labels) may be listed with zeros or left
    out; the rows below must be present with these counts.
    When "accept" runs on the lossy variant of the sample corpus
    Then the lossy per-group table of detection-params.v5.toml contains the rows:
      | group                | Real | Suspicious | Fake | Undetectable | Source-limited | Error |
      | l128                 | 5    | 0          | 0    | —            | 1              | 0     |
      | l320                 | 5    | 0          | 0    | —            | 1              | 0     |
      | l128_320             | 0    | 0          | 5    | 1            | —              | 0     |
      | confirmed_real       | 1    | 0          | 0    | —            | 0              | 0     |
      | confirmed_fake aac   | 0    | 0          | 2    | 0            | —              | 0     |
    And the median estimate of the Fake rows of "l128_320" is 128
    And for the confirmed fakes with a source bitrate, 1 of 1 estimates is at most that bitrate

  @RP-11 @RF-15
  Scenario: Lossy columns in tracks.csv
    When "accept" runs on the lossy variant of the sample corpus
    Then every v5 row of a lossy track with spectra has codec, declared_bitrate_kbps, fc_hz, c_exp_hz, shortfall_hz, m_lossy_suspicious_hz, m_lossy_fake_hz, m_lossy_step_db and estimate_kbps filled in
    And for those rows m_lossy_fake_hz equals lossy.fake_shortfall_hz minus shortfall_hz
    And those rows leave the lossless margin columns empty
    And lossless rows leave the lossy columns empty

  # --- Chains, groups and split ---------------------------------------------------

  @RP-11 @RF-15
  Scenario: Tracks of one master are paired across versions
    When "accept" runs on the lossy variant of the sample corpus
    Then chain "C1" has keys 1, 2 and 3 and 1 pair group
    And in pair group 1 the "pcm" version is "real" and the "mp3" version is "fake" with estimate 128
    And report.md shows the paths of the pair group
    And summary.md shows "C1" with its counts and no path or file name

  @RP-11 @RF-15
  Scenario: A chain is one split unit
    When "accept" runs on the lossy variant of the sample corpus
    Then every track of "Revisión/Ørsted" and of "Cadena/MP3 – Ørsted" has split_unit "chain:C1" and the same split

  @RP-11 @RF-15
  Scenario: A group makes two albums one split unit
    Given line 4 of the sample lossy trusted.txt is "Compras\Ñandú – Ácido, Vol. 1 (2024) | tienda group=G1 | comprado"
    And line 6 of the sample lossy trusted.txt is "Colección\Álbum Ü | analisis-limpio group=G1"
    When "accept" runs on the lossy variant of the sample corpus
    Then every labelled track of "Compras/Ñandú – Ácido, Vol. 1 (2024)" and of "Colección/Álbum Ü", generated rows included, has split_unit "G1" and the same split
    And the watch track W1 ("02 – Canción.flac", a false-positive line without attributes) has split_unit "G1", from its trusted.txt line
    And the unit "G1" is counted once, in class "primary"

  @RP-11 @RF-15
  Scenario: A false-positive line takes no attributes
    Given line 3 of the sample lossy confirmed.txt is "Compras\Ñandú – Ácido, Vol. 1 (2024)\02 – Canción.flac | false-positive group=G1 | detector antiguo"
    When "accept" runs on the lossy variant of the sample corpus
    Then the exit code is 2
    And the error output is "confirmed.txt:3: false-positive lines take no attributes; write them on the trusted.txt line"

  @RP-11 @RF-15
  Scenario: An album split over two units is an error
    Line 8 decides only track 10; track 11 of the same album keeps the album id
    as its unit.
    Given line 8 of the sample lossy trusted.txt is "Sueltas\Mixto\05 – Cuatro.flac | analisis group=G1"
    When "accept" runs on the lossy variant of the sample corpus
    Then the exit code is 2
    And the error output is "album has tracks in several split units (trusted.txt:8, confirmed.txt:2)"

  @RP-11 @RF-15
  Scenario: Adding lossy-only labels does not move a lossless album
    Given the split of every lossless album in a run of the base sample corpus
    When "accept" runs on the base sample corpus with the lossy corpus files and lines 4 to 6 of the sample lossy confirmed.txt added
    Then every lossless album has the same split as before

  # --- Labels: grammar ------------------------------------------------------------

  @RP-11 @RF-15
  Scenario: A trusted lossy file is an honest negative, not a transcode source
    Given line 5 of the sample lossy confirmed.txt is removed
    And the line "Pérdidas\Vídeo\08 – Señal ’AAC’.m4a | tienda" is added to the sample lossy trusted.txt
    When "accept" runs on the lossy variant of the sample corpus
    Then its labels.csv row has truth "real", origin "trusted", class "primary" and domain "lossy"
    And no generated entry has it as source_path

  @RP-11 @RF-15
  Scenario: A folder line with formats=lossy includes MP3 and M4A files only
    Given lines 4 to 6 of the sample lossy confirmed.txt are replaced by the single line "Pérdidas\Vídeo | real formats=lossy"
    When "accept" runs on the lossy variant of the sample corpus
    Then "Pérdidas/Vídeo/07 – Falso 320.mp3" and "Pérdidas/Vídeo/08 – Señal ’AAC’.m4a" have truth "real" and origin "confirmed_real"
    And "Pérdidas/Vídeo/09 – Ópera.opus" has no labels.csv row

  @RP-11 @RF-15
  Scenario Outline: Errors in the attributes of confirmed.txt
    Given line 4 of the sample lossy confirmed.txt is replaced by "<line>"
    When "accept" runs on the lossy variant of the sample corpus
    Then the exit code is 2
    And the error output is "confirmed.txt:4: <message>"
    And nothing is written in FAKEFLAC_CALIBRATION

    Examples:
      | line                                                                     | message                                                                     |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| fake source=aac                     | unknown attribute 'source'                                                  |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| fake weak weak                      | attribute 'weak' given twice                                                |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| fake upstream=vorbis                | invalid value 'vorbis' for upstream (expected aac, opus, mp3, unknown or by-rate) |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| fake source_max_kbps=12.5           | source_max_kbps must be an integer from 8 to 640                            |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| fake formats=lossy                  | formats= applies to folders only                                            |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| fake chain=Álbum                    | invalid id 'Álbum' for chain (1 to 16 letters, digits, '_' or '-')          |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| real upstream=aac                   | upstream=, source_max_kbps= and weak apply to fake lines only               |
      | Compras\Ñandú – Ácido, Vol. 1 (2024)\03 – Bonus.flac \| fake upstream=aac | upstream=, source_max_kbps= and weak apply to lossy files only             |
      | Pérdidas\Vídeo\07 – Falso 320.mp3 \| dudoso                              | unknown kind 'dudoso' (expected fake, real or false-positive)               |
      | Pérdidas\Vídeo \| real formats=all                                       | real applies to lossy files only                                            |
      | Compras\Ñandú – Ácido, Vol. 1 (2024)\03 – Bonus.flac \| real             | real applies to lossy files only                                            |
      | Colección\Álbum Ü\folder.jpg \| fake                                     | not a supported audio file (FLAC, WAV, MP3, M4A or Opus)                    |

  @RP-11 @RF-15
  Scenario: A real label on a lossless file is excluded after extraction
    Given an ALAC fixture "m4a_alac_full_band" saved as "Pérdidas/Vídeo/10 – Sin pérdida.m4a"
    And a line "Pérdidas\Vídeo\10 – Sin pérdida.m4a | real" added to the sample lossy confirmed.txt as line 8
    When "accept" runs on the lossy variant of the sample corpus
    Then the exit code is 0
    And the file has no labels.csv row
    And the report header says "confirmed.txt line 8: real applies to lossy files; the track holds ALAC (excluded)"

  @RP-11 @RF-15
  Scenario: A chain split over two groups is an error
    Given line 7 of the sample lossy confirmed.txt is "Cadena\MP3 – Ørsted\01 – Tres.mp3 | fake upstream=aac chain=C1 group=G2"
    When "accept" runs on the lossy variant of the sample corpus
    Then the exit code is 2
    And the error output says "chain C1 spans split units G2 and chain:C1"

  @RP-11 @RF-15
  Scenario: upstream=by-rate is resolved from the sample rate
    Given line 4 of the sample lossy confirmed.txt is "Pérdidas\Vídeo\07 – Falso 320.mp3 | fake upstream=by-rate"
    When "accept" runs on the lossy variant of the sample corpus
    Then its labels.csv row has upstream "aac" and source_bitrate_kbps 128

  # --- Configuration and grid ------------------------------------------------------

  @RP-11 @RF-15
  Scenario Outline: Invalid lossy entries in the config
    Given specs/calibration/calibration.v2.toml with <change>
    When the binary runs "accept"
    Then the exit code is 2
    And the error output says "<message>"
    And nothing is written in FAKEFLAC_CALIBRATION

    Examples:
      | change                                          | message                                          |
      | a lossy entry "x" with stages ["v0", "cbr320"]  | lossy entry x: stage v0 has no bitrate_kbps      |
      | a lossy entry "y" with stages ["nope"]          | lossy entry y: unknown stage nope                |
      | a lossy entry "w" with stages []                | lossy entry w: no stages                         |
      | a lossy entry "z" with stages ["opus128"]       | lossy entry z: Opus output is not supported yet  |
      | a second lossy entry "cbr128"                   | lossy entry cbr128 is defined twice              |

  @RP-11 @RF-15
  Scenario: Lossy parameters cannot be searched
    Given a grid with baseline detection-params.v5.toml whose values include "lossy.fake_shortfall_hz"
    When "calibrate" runs on the sample corpus
    Then the exit code is 2
    And the error output says "lossy.fake_shortfall_hz: lossy parameters cannot be searched yet"

  @RP-11 @RF-15
  Scenario: Cascade command lines
    Given the lossy entry "aac128_cbr320" of calibration.v2.toml and the source "Compras/Ñandú – Ácido, Vol. 1 (2024)/01 – Intro ’A’.flac" with full track default
    When the ffmpeg calls are built
    Then there are 2 encode calls and no decode call
    And the first is "-nostdin -hide_banner -loglevel error -i <source> -map 0:a:0 -map_metadata -1 -c:a aac -b:a 128k -y <generated>/<id>.s1.m4a"
    And the second is "-nostdin -hide_banner -loglevel error -i <generated>/<id>.s1.m4a -map 0:a:0 -map_metadata -1 -c:a libmp3lame -b:a 320k -y <generated>/<id>.mp3"
    And each one is a list of separate arguments with <source> passed unchanged

  @RP-11 @RF-15
  Scenario: The default config is calibration.v2.toml
    Given no --config argument
    When "accept" starts
    Then it reads specs/calibration/calibration.v2.toml

  @RP-11 @RF-15 @RNF-05
  Scenario: The corpus is not modified by lossy evaluation
    Given the list of every file under FAKEFLAC_CORPUS with its size and modification time
    When "accept" runs on the lossy variant of the sample corpus
    Then the list is unchanged

  @RP-11 @RF-15
  Scenario: summary.md holds no lossy corpus names
    When "accept" runs on the lossy variant of the sample corpus
    Then summary.md does not contain any folder name or file stem of the lossy variant
    And summary.md does not contain any note of the sample lossy confirmed.txt

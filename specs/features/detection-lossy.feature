@RF-15
Feature: Source bitrate of lossy files (detection v5)
  The lossy rules of specs/detection/lossy-source.md with DetectionParams v5
  (detection-params.v5.toml; since detection v6 the defaults are
  detection-params.v6.toml, whose [lossy] section and every other value used
  here are v5's, rules.md Q22 (c), so under detection v6 the tests could use
  the defaults for v5's values. Changed for detection v7 (lossy-source.md
  Q25 (c), RF-16): the defaults are now detection-params.v7.toml, which adds
  [lossy.streaming]; with it the row "48 kHz MP3 from Opus (known limitation)"
  and the fixture mp3_320_48k_lp20k are suspicious (streamingSampleRate,
  detection-streaming-source.feature). So in this feature "DetectionParams v5"
  means a params set without [lossy.streaming], the parameters of
  detection-params.v5.toml or detection-params.v6.toml, never the v7 defaults;
  only the algorithmVersion they carry differs): the content edge fc (the reported
  cutoff: fs when D >= step_suspicious_db = 7, otherwise fe), the expected
  cutoff c_exp = min(table row of the declared bitrate, LAME tag low-pass,
  nyquist - 400), the shortfall s = c_exp - fc, and rules L0-L6:
  L3 Fake when s >= 2500 Hz and D >= 10 dB; L4 Suspicious (cutoffBelowDeclared)
  when s >= 1000 Hz and D >= 7 dB; L5 Suspicious (gradualRolloff) when
  s >= 1000 Hz; L2 Suspicious (lowHighFrequencyContent) when H < -60 dB.
  The estimate is the declared bitrate when no shortfall rule fires, otherwise
  min(declared, the table row nearest to fc; ties to the lower bitrate).
  MP3 table (kbps -> Hz): 32 5500, 40 7000, 48 7500, 56 10000, 64 11000,
  80 13500, 96 15100, 112 15600, 128 17000, 160 17500, 192 18600, 224 19400,
  256 19700, 320 20500. AAC table: 64 11000, 96 13000, 128 15500, 160 16000,
  192 17000, 256 19000, 320 19500. These values are indicative, not measured
  (lossy-source.md Q2); acceptance B decides.
  In the measurement tables, fs "—" means fs is undefined (D = 0), a tag
  low-pass "—" means the stream declares none, c_exp "none" means the
  expected cutoff is undefined and an estimate "none" means no estimated
  source bitrate is reported (null).
  Audio-level scenarios use the fixtures of specs/detection/lossy-fixtures.md;
  the test stage first confirms its section 3 and returns to G1 if a check fails.

  # --- Verdicts on lossy fixtures ----------------------------------------------

  @RF-15
  Scenario Outline: Verdict and estimated source bitrate of lossy fixtures
    Given the lossy fixture "<fixture>"
    When it is analysed with DetectionParams v5
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>
    And the estimated cutoff is <cutoff> Hz ± 200 Hz

    Examples:
      | fixture             | verdict    | reasons             | estimate | cutoff |
      | mp3_320_lp20k5      | real       |                     | 320      | 20500  |
      | mp3_320_lp17k       | fake       |                     | 128      | 17000  |
      | mp3_128_lp17k       | real       |                     | 128      | 17000  |
      | mp3_192_lp17k_notag | suspicious | cutoffBelowDeclared | 128      | 17000  |
      | mp3_320_lp18k5      | suspicious | cutoffBelowDeclared | 192      | 18500  |
      | mp3_320_tag16k      | real       |                     | 320      | 16000  |
      | mp3_vbr_lp19k5      | real       |                     | 240      | 19500  |
      | mp3_abr_lp18k6      | real       |                     | 192      | 18600  |
      | mp3_320_48k_lp20k   | real       |                     | 320      | 20000  |
      | m4a_aac_128_lp16k   | real       |                     | 128      | 16000  |
      | m4a_aac_256_lp15k3  | fake       |                     | 128      | 15300  |

  @RF-15
  Scenario: Little treble gives no estimate
    Given the lossy fixture "mp3_320_low_hf"
    When it is analysed with DetectionParams v5
    Then the verdict is "suspicious"
    And the reasons are "lowHighFrequencyContent"
    And no estimated source bitrate is reported

  @RF-15
  Scenario: A silent lossy file reports no cutoff and no estimate
    Given the lossy fixture "mp3_silence"
    When it is analysed with DetectionParams v5
    Then the verdict is "suspicious"
    And the reasons are "lowHighFrequencyContent"
    And no cutoff is reported
    And no estimated source bitrate is reported

  @RF-15
  Scenario: Deterministic result
    Given the lossy fixture "mp3_320_lp17k"
    When it is analysed twice with DetectionParams v5
    Then both analyses are equal

  # --- Rules from given measurements -------------------------------------------

  @RF-15
  Scenario Outline: Lossy verdict from stream information and measurements
    Given a lossy track with codec "<codec>", sample rate <rate> Hz, declared bitrate <b> kbps and tag low-pass <tag> Hz
    And the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz and fe = <fe> Hz
    When the lossy rules of DetectionParams v5 are applied
    Then the expected cutoff is <c_exp> Hz
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>

    Examples:
      | case                                          | codec | rate  | b   | tag   | H   | G  | D    | fs    | fe    | c_exp | verdict    | reasons                 | estimate |
      | honest 320                                    | mp3   | 44100 | 320 | —     | -20 | 0  | 40   | 20450 | 22050 | 20500 | real       |                         | 320      |
      | 320 from 128                                  | mp3   | 44100 | 320 | 20500 | -20 | 0  | 40   | 17050 | 22050 | 20500 | fake       |                         | 128      |
      | 192 from 128                                  | mp3   | 44100 | 192 | —     | -20 | 0  | 40   | 17050 | 22050 | 18600 | suspicious | cutoffBelowDeclared     | 128      |
      | shortfall at fake_shortfall_hz                | mp3   | 44100 | 320 | —     | -20 | 0  | 10.0 | 18000 | 22050 | 20500 | fake       |                         | 160      |
      | same, step just below fake_min_step_db        | mp3   | 44100 | 320 | —     | -20 | 0  | 9.99 | 18000 | 22050 | 20500 | suspicious | cutoffBelowDeclared     | 160      |
      | shortfall just below fake_shortfall_hz (tie)  | mp3   | 44100 | 320 | —     | -20 | 0  | 40   | 18050 | 22050 | 20500 | suspicious | cutoffBelowDeclared     | 160      |
      | shortfall at suspicious_shortfall_hz          | mp3   | 44100 | 320 | —     | -20 | 0  | 40   | 19500 | 22050 | 20500 | suspicious | cutoffBelowDeclared     | 224      |
      | shortfall just below suspicious_shortfall_hz  | mp3   | 44100 | 320 | —     | -20 | 0  | 40   | 19550 | 22050 | 20500 | real       |                         | 320      |
      | step at step_suspicious_db: not deep enough   | mp3   | 44100 | 320 | —     | -20 | 0  | 7.0  | 17050 | 22050 | 20500 | suspicious | cutoffBelowDeclared     | 128      |
      | weak step ignored, content to Nyquist         | mp3   | 44100 | 320 | —     | -20 | 0  | 6.99 | 17050 | 22050 | 20500 | real       |                         | 320      |
      | no step, content fades early                  | mp3   | 44100 | 320 | —     | -20 | 0  | 3    | 18050 | 17000 | 20500 | suspicious | gradualRolloff          | 128      |
      | no step, content ends below the first row     | mp3   | 44100 | 320 | —     | -20 | 0  | 0    | —     | 4000  | 20500 | suspicious | gradualRolloff          | 32       |
      | little treble comes first                     | mp3   | 44100 | 320 | —     | -65 | 0  | 40   | 17050 | 22050 | 20500 | suspicious | lowHighFrequencyContent | none     |
      | gap step is not used                          | mp3   | 44100 | 320 | —     | -20 | 15 | 40   | 20450 | 22050 | 20500 | real       |                         | 320      |
      | tag low-pass lowers c_exp                     | mp3   | 44100 | 320 | 16000 | -20 | 0  | 40   | 16050 | 22050 | 16000 | real       |                         | 320      |
      | tag low-pass above the table is ignored       | mp3   | 44100 | 128 | 20000 | -20 | 0  | 40   | 17050 | 22050 | 17000 | real       |                         | 128      |
      | tag low-pass below the first row is ignored   | mp3   | 44100 | 320 | 100   | -20 | 0  | 40   | 17050 | 22050 | 20500 | fake       |                         | 128      |
      | average between rows takes the lower row      | mp3   | 44100 | 300 | —     | -20 | 0  | 40   | 19750 | 22050 | 19700 | real       |                         | 300      |
      | average between rows, early edge              | mp3   | 44100 | 300 | —     | -20 | 0  | 40   | 18050 | 22050 | 19700 | suspicious | cutoffBelowDeclared     | 160      |
      | declared below the table: nothing expected    | mp3   | 22050 | 24  | —     | -20 | 0  | 0    | —     | 8000  | none  | real       |                         | 24       |
      | 32 kHz: c_exp capped at nyquist - 400         | mp3   | 32000 | 320 | —     | -20 | 0  | 0    | —     | 16000 | 15600 | real       |                         | 320      |
      | 32 kHz: early edge                            | mp3   | 32000 | 320 | —     | -20 | 0  | 40   | 13050 | 16000 | 15600 | fake       |                         | 80       |
      | 48 kHz MP3 from Opus (known limitation)       | mp3   | 48000 | 320 | 20500 | -20 | 0  | 40   | 20050 | 24000 | 20500 | real       |                         | 320      |
      | honest AAC 128                                | aac   | 44100 | 128 | —     | -20 | 0  | 40   | 16050 | 22050 | 15500 | real       |                         | 128      |
      | AAC 256 from 128                              | aac   | 44100 | 256 | —     | -20 | 0  | 40   | 15550 | 22050 | 19000 | fake       |                         | 128      |
      | AAC 192, moderate shortfall                   | aac   | 44100 | 192 | —     | -20 | 0  | 40   | 15850 | 22050 | 17000 | suspicious | cutoffBelowDeclared     | 160      |

  @RF-15
  Scenario: The lossless rules never apply to a lossy codec
    An honest MP3 has a low-pass by design: rules.md V4 would make it fake.
    Given a lossy track with codec "mp3", sample rate 44100 Hz, declared bitrate 128 kbps and tag low-pass — Hz
    And the measurements H = -20 dB, G = 15 dB, D = 30 dB, fs = 17050 Hz and fe = 22050 Hz
    When the verdict of DetectionParams v5 is computed for the track
    Then the verdict is "real"
    And the estimated source bitrate is 128

  @RF-15
  Scenario: The lossy rules never apply to a lossless codec
    Given a track with codec "alac" and the measurements of the lossless fixture "lp16k"
    When the verdict of DetectionParams v5 is computed for the track
    Then the verdict is "fake" as for "lp16k" saved as FLAC
    And no estimated source bitrate is reported

  @RF-15
  Scenario: A lossy codec without table rows cannot be judged
    detection-params.v5.toml has no Opus row; in v5 the scanner never lists an
    Opus file (specs/pending/opus.feature), so only the core meets this case.
    Given a lossy track with codec "opus", sample rate 48000 Hz, declared bitrate 128 kbps and tag low-pass — Hz
    When the verdict of DetectionParams v5 is computed for the track
    Then the verdict is "error" with error "unsupportedFormat"
    And no estimated source bitrate is reported

  # --- Older params files --------------------------------------------------------

  @RF-15
  Scenario Outline: Params without a [lossy] section do not judge lossy files
    Given the lossy fixture "mp3_320_lp17k"
    When it is analysed with the parameters of <params>
    Then the verdict is "error" with error "unsupportedFormat" and detail "these DetectionParams have no [lossy] section"
    And the TrackAnalysis carries algorithmVersion <version>

    Examples:
      | params                                  | version |
      | specs/detection/detection-params.v4.toml | 4       |
      | specs/detection/detection-params.v2.toml | 2       |

  @RF-15 @RF-04
  Scenario: v5 gives the v4 lossless verdicts
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC
    When each one is analysed with the parameters of specs/detection/detection-params.v4.toml and with DetectionParams v5 (the parameters of specs/detection/detection-params.v5.toml, not the v6 defaults: edge_bump19k3 differs with v6)
    Then the two results have the same verdict, reasons and cutoff for every signal
    And they carry algorithmVersion 4 and 5

  # --- Parameter files (lossy-source.md section 6) -----------------------------

  @RF-15
  Scenario Outline: Valid parameter files and their [lossy] section
    Given <params>
    When it is validated, after being deserialised into DetectionParams if it is a file
    Then it is accepted
    And its [lossy] section is <lossy>

    Examples:
      | params                                               | lossy   |
      | the text of specs/detection/detection-params.v5.toml | present |
      | the text of specs/detection/detection-params.v4.toml | absent  |
      | the default DetectionParams of the code              | present |

  @RF-15
  Scenario: The defaults hold the v5 lossy values
    Given the default DetectionParams of the code
    Then lossy.suspicious_shortfall_hz is 1000, lossy.fake_shortfall_hz is 2500 and lossy.fake_min_step_db is 10
    And lossy.cutoffs has 14 mp3 rows, 7 aac rows and no opus row

  @RF-15
  Scenario Outline: Invalid [lossy] sections are rejected
    The shape is checked when the file is deserialised; the values by
    DetectionParams::validate, which every loader calls.
    Given the text of specs/detection/detection-params.v5.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is rejected by <layer>
    And the error names "<what>"

    Examples:
      | change                                                               | layer           | what                                                 |
      | the suspicious_shortfall_hz line of [lossy] removed                  | deserialisation | suspicious_shortfall_hz                              |
      | an extra line "max_shortfall_hz = 1.0" in [lossy]                    | deserialisation | max_shortfall_hz                                     |
      | the aac array of [lossy.cutoffs] removed                             | deserialisation | aac                                                  |
      | an extra line "vorbis = []" in [lossy.cutoffs]                       | deserialisation | vorbis                                               |
      | the cutoff_hz of the mp3 row for 128 kbps removed                    | deserialisation | cutoff_hz                                            |
      | lossy.suspicious_shortfall_hz set to 0.0                             | validation      | lossy.suspicious_shortfall_hz > 0                    |
      | lossy.suspicious_shortfall_hz set to 3000.0 (above fake_shortfall_hz) | validation     | lossy.suspicious_shortfall_hz ≤ lossy.fake_shortfall_hz |
      | lossy.fake_shortfall_hz set to nan                                   | validation      | lossy.fake_shortfall_hz                              |
      | lossy.fake_min_step_db set to 6.5 (below step_suspicious_db)         | validation      | step_suspicious_db ≤ lossy.fake_min_step_db          |
      | the mp3 array set to []                                              | validation      | lossy.cutoffs.mp3 is not empty                       |
      | an "opus = []" line added to [lossy.cutoffs]                         | validation      | lossy.cutoffs.opus is not empty                      |
      | the mp3 rows for 112 and 128 kbps swapped                            | validation      | lossy.cutoffs.mp3: bitrate_kbps increases            |
      | a second mp3 row for 128 kbps after the first                        | validation      | lossy.cutoffs.mp3: bitrate_kbps increases            |
      | the cutoff_hz of the mp3 row for 160 kbps set to 16500.0             | validation      | lossy.cutoffs.mp3: cutoff_hz does not decrease       |
      | the bitrate_kbps of the first aac row set to 0                       | validation      | lossy.cutoffs.aac: bitrate_kbps > 0                  |
      | the cutoff_hz of the first aac row set to 0.0                        | validation      | lossy.cutoffs.aac: cutoff_hz > 0                     |

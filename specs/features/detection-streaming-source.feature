@RF-16
Feature: Streaming sample rate of lossy files (detection v7)
  Rule L6a of specs/detection/lossy-source.md section 11 with DetectionParams v7
  (the defaults, detection-params.v7.toml): a lossy MP3 or AAC track that rule
  L6 would call real becomes suspicious with the reason streamingSampleRate and
  no estimated source bitrate when its codec is listed in
  lossy.streaming.codecs = [mp3, aac], its decoded sample rate is listed in
  lossy.streaming.sample_rates_hz = [48000], its declared bitrate is at least
  lossy.streaming.min_declared_bitrate_kbps = 192 and its reported cutoff fc
  lies in [lossy.streaming.min_cutoff_hz, lossy.streaming.max_cutoff_hz) =
  [19500, 21000) Hz. L6a is evaluated after L1a-L5, so it never overrides an
  error, a fake or a suspicious verdict of an earlier rule, and it never gives
  fake. These values are indicative, not measured (lossy-source.md Q21);
  acceptance K decides.
  Every other value is v6's (= v5's lossy values, detection-lossy.feature):
  fc = fs when D >= 7 dB, otherwise fe; c_exp = min(table row of the declared
  bitrate, LAME tag low-pass, nyquist - 400); s = c_exp - fc; L3 fake when
  s >= 2500 Hz and D >= 10 dB; L4 suspicious (cutoffBelowDeclared) when
  s >= 1000 Hz and D >= 7 dB; L5 suspicious (gradualRolloff) when s >= 1000 Hz;
  L2 suspicious (lowHighFrequencyContent) when H < -60 dB. MP3 table (kbps -> Hz):
  ... 128 17000, 160 17500, 192 18600, 224 19400, 256 19700, 320 20500.
  AAC table: ... 128 15500, 160 16000, 192 17000, 256 19000, 320 19500.
  In the measurement tables, fs "—" means fs is undefined (D = 0), a tag
  low-pass "—" means the stream declares none, and an estimate "none" means no
  estimated source bitrate is reported (null).
  Audio-level scenarios use the fixtures of specs/detection/lossy-fixtures.md
  (the test stage first confirms its section 3 for the new fixtures and returns
  to G1 if a check fails).

  # --- Verdicts on lossy fixtures ----------------------------------------------

  @RF-16
  Scenario Outline: Verdict of lossy fixtures with v7
    Given the lossy fixture "<fixture>"
    When it is analysed with DetectionParams v7
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>
    And the estimated cutoff is <cutoff> Hz ± 200 Hz
    And the TrackAnalysis carries algorithmVersion 7

    Examples:
      | fixture               | verdict    | reasons             | estimate | cutoff |
      | mp3_320_48k_lp20k     | suspicious | streamingSampleRate | none     | 20000  |
      | mp3_320_lp20k         | real       |                     | 320      | 20000  |
      | m4a_aac_256_48k_lp20k | suspicious | streamingSampleRate | none     | 20000  |
      | mp3_320_lp20k5        | real       |                     | 320      | 20500  |
      | mp3_320_lp17k         | fake       |                     | 128      | 17000  |

  @RF-16
  Scenario Outline: The same 48 kHz fixtures are real with params without [lossy.streaming]
    Given the lossy fixture "<fixture>"
    When it is analysed with the parameters of <params>
    Then the verdict is "real"
    And the reasons are ""
    And the estimated source bitrate is <estimate>
    And the TrackAnalysis carries algorithmVersion <version>

    Examples:
      | fixture               | params                                   | estimate | version |
      | mp3_320_48k_lp20k     | specs/detection/detection-params.v6.toml | 320      | 6       |
      | m4a_aac_256_48k_lp20k | specs/detection/detection-params.v6.toml | 256      | 6       |
      | mp3_320_48k_lp20k     | specs/detection/detection-params.v5.toml | 320      | 5       |

  @RF-16 @RF-15
  Scenario: v7 changes no other lossy fixture
    Given every lossy fixture of specs/detection/lossy-fixtures.md section 2 except "mp3_320_48k_lp20k" and "m4a_aac_256_48k_lp20k"
    When each one is analysed with the parameters of specs/detection/detection-params.v6.toml and with DetectionParams v7
    Then the two results have the same verdict, reasons, error, cutoff and estimated source bitrate for every fixture
    And they carry algorithmVersion 6 and 7

  # --- Rule L6a from given measurements ------------------------------------------

  @RF-16
  Scenario Outline: Lossy verdict at streaming sample rates
    Given a lossy track with codec "<codec>", sample rate <rate> Hz, declared bitrate <b> kbps and tag low-pass <tag> Hz
    And the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz and fe = <fe> Hz
    When the lossy rules of DetectionParams v7 are applied
    Then the expected cutoff is <c_exp> Hz
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>

    Examples:
      | case                                                 | codec | rate  | b   | tag   | H   | G | D    | fs    | fe    | c_exp | verdict    | reasons                 | estimate |
      | the motivating case: MP3 320 at 48 kHz, edge 20050   | mp3   | 48000 | 320 | 20500 | -20 | 0 | 40   | 20050 | 24000 | 20500 | suspicious | streamingSampleRate     | none     |
      | the same at 44.1 kHz                                 | mp3   | 44100 | 320 | 20500 | -20 | 0 | 40   | 20050 | 22050 | 20500 | real       |                         | 320      |
      | honest LAME 320 of a 48 kHz master (by design)       | mp3   | 48000 | 320 | 20500 | -20 | 0 | 40   | 20450 | 24000 | 20500 | suspicious | streamingSampleRate     | none     |
      | 48 kHz 320 from 128: L3 comes first                  | mp3   | 48000 | 320 | 20500 | -20 | 0 | 40   | 17050 | 24000 | 20500 | fake       |                         | 128      |
      | 48 kHz 320 from 192: L4 comes first                  | mp3   | 48000 | 320 | —     | -20 | 0 | 40   | 18550 | 24000 | 20500 | suspicious | cutoffBelowDeclared     | 192      |
      | 48 kHz large shortfall, step not deep: L4            | mp3   | 48000 | 320 | —     | -20 | 0 | 9    | 17050 | 24000 | 20500 | suspicious | cutoffBelowDeclared     | 128      |
      | 48 kHz no step, content fades early: L5              | mp3   | 48000 | 320 | —     | -20 | 0 | 3    | —     | 17000 | 20500 | suspicious | gradualRolloff          | 128      |
      | 48 kHz little treble: L2 comes first                 | mp3   | 48000 | 320 | —     | -65 | 0 | 40   | 20050 | 24000 | 20500 | suspicious | lowHighFrequencyContent | none     |
      | fc at min_cutoff_hz                                  | mp3   | 48000 | 256 | —     | -20 | 0 | 40   | 19500 | 24000 | 19700 | suspicious | streamingSampleRate     | none     |
      | fc just below min_cutoff_hz                          | mp3   | 48000 | 256 | —     | -20 | 0 | 40   | 19499 | 24000 | 19700 | real       |                         | 256      |
      | fc just below max_cutoff_hz                          | mp3   | 48000 | 320 | —     | -20 | 0 | 40   | 20999 | 24000 | 20500 | suspicious | streamingSampleRate     | none     |
      | fc at max_cutoff_hz                                  | mp3   | 48000 | 320 | —     | -20 | 0 | 40   | 21000 | 24000 | 20500 | real       |                         | 320      |
      | no low-pass: content to Nyquist (fc = fe)            | mp3   | 48000 | 320 | —     | -20 | 0 | 6.99 | 17050 | 24000 | 20500 | real       |                         | 320      |
      | declared at min_declared_bitrate_kbps                | mp3   | 48000 | 192 | —     | -20 | 0 | 40   | 19550 | 24000 | 18600 | suspicious | streamingSampleRate     | none     |
      | declared below min_declared_bitrate_kbps             | mp3   | 48000 | 160 | —     | -20 | 0 | 40   | 19550 | 24000 | 17500 | real       |                         | 160      |
      | VBR average between rows                             | mp3   | 48000 | 245 | —     | -20 | 0 | 40   | 19950 | 24000 | 19400 | suspicious | streamingSampleRate     | none     |
      | tag low-pass lowers c_exp, fc still in the window    | mp3   | 48000 | 320 | 19600 | -20 | 0 | 40   | 19650 | 24000 | 19600 | suspicious | streamingSampleRate     | none     |
      | 32 kHz is not a listed rate                          | mp3   | 32000 | 320 | —     | -20 | 0 | 0    | —     | 16000 | 15600 | real       |                         | 320      |
      | AAC 256 at 48 kHz, edge near full band               | aac   | 48000 | 256 | —     | -20 | 0 | 40   | 19950 | 24000 | 19000 | suspicious | streamingSampleRate     | none     |
      | the same AAC at 44.1 kHz                             | aac   | 44100 | 256 | —     | -20 | 0 | 40   | 19950 | 22050 | 19000 | real       |                         | 256      |
      | AAC 128 at 48 kHz: below the bitrate and the window  | aac   | 48000 | 128 | —     | -20 | 0 | 40   | 16050 | 24000 | 15500 | real       |                         | 128      |
      | AAC at 96 kHz is not a listed rate                   | aac   | 96000 | 320 | —     | -20 | 0 | 40   | 19950 | 48000 | 19500 | real       |                         | 320      |
      | AAC 256 at 48 kHz from 128: L3 comes first           | aac   | 48000 | 256 | —     | -20 | 0 | 40   | 15550 | 24000 | 19000 | fake       |                         | 128      |

  @RF-16
  Scenario Outline: Params without [lossy.streaming] never apply L6a
    Given a lossy track with codec "<codec>", sample rate 48000 Hz, declared bitrate <b> kbps and tag low-pass — Hz
    And the measurements H = -20 dB, G = 0 dB, D = 40 dB, fs = <fs> Hz and fe = 24000 Hz
    When the lossy rules of the parameters of <params> are applied
    Then the verdict is "real"
    And the reasons are ""
    And the estimated source bitrate is <b>

    Examples:
      | codec | b   | fs    | params                                   |
      | mp3   | 320 | 20050 | specs/detection/detection-params.v6.toml |
      | mp3   | 320 | 20450 | specs/detection/detection-params.v6.toml |
      | aac   | 256 | 19950 | specs/detection/detection-params.v6.toml |
      | mp3   | 320 | 20050 | specs/detection/detection-params.v5.toml |

  # --- Out of scope: Opus and lossless codecs ------------------------------------

  @RF-16
  Scenario: Opus tracks are not judged by L6a even when the params have Opus rows
    Opus is natively 48 kHz, and "opus" cannot be listed in lossy.streaming.codecs
    (next scenario outline). In the defaults an Opus track is an error by L0
    (no Opus rows, detection-lossy.feature); here Opus rows are added so that
    the track reaches the shortfall rules.
    Given the text of specs/detection/detection-params.v7.toml with an "opus" array in [lossy.cutoffs] equal to its "mp3" array
    And a lossy track with codec "opus", sample rate 48000 Hz, declared bitrate 160 kbps and tag low-pass — Hz
    And the measurements H = -20 dB, G = 0 dB, D = 40 dB, fs = 20050 Hz and fe = 24000 Hz
    When the lossy rules of those parameters are applied
    Then the verdict is "real"
    And the reasons are ""
    And the estimated source bitrate is 160

  @RF-16
  Scenario: An Opus track with the defaults is still an error, not streamingSampleRate
    Given a lossy track with codec "opus", sample rate 48000 Hz, declared bitrate 128 kbps and tag low-pass — Hz
    When the verdict of DetectionParams v7 is computed for the track
    Then the verdict is "error" with error "unsupportedFormat" and detail "no [lossy.cutoffs] rows for opus"
    And no estimated source bitrate is reported

  @RF-16 @RF-04
  Scenario Outline: Lossless codecs at 48 kHz are never judged by L6a
    Given a track with codec "<codec>", sample rate 48000 Hz and the measurements of the lossless fixture "full_band"
    When the verdict of DetectionParams v7 is computed for the track
    Then the verdict, reasons and cutoff equal those that the parameters of specs/detection/detection-params.v6.toml give the same track
    And the reasons do not contain "streamingSampleRate"
    And no estimated source bitrate is reported

    Examples:
      | codec |
      | pcm   |
      | flac  |
      | alac  |

  @RF-16 @RF-04
  Scenario: v7 gives the v6 lossless verdicts
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC
    When each one is analysed with the parameters of specs/detection/detection-params.v6.toml and with DetectionParams v7
    Then the two results have the same verdict, reasons and cutoff for every signal
    And they carry algorithmVersion 6 and 7

  # --- Errors (RNF-04) --------------------------------------------------------

  @RF-16 @RNF-04
  Scenario Outline: Broken lossy files stay errors with v7
    Given the lossy fixture "<fixture>"
    When it is analysed with DetectionParams v7
    Then the verdict is "error" with error "<error>"
    And the reasons are ""
    And no cutoff and no estimated source bitrate are reported

    Examples:
      | fixture           | error         |
      | mp3_320_48k_short | tooShort      |
      | mp3_corrupt       | corruptStream |
      | m4a_corrupt       | corruptStream |

  # --- Parameter files (lossy-source.md section 11.3) -----------------------------

  @RF-16
  Scenario Outline: Valid parameter files and their [lossy.streaming] section
    Given <params>
    When it is validated, after being deserialised into DetectionParams if it is a file
    Then it is accepted
    And its [lossy.streaming] section is <streaming>

    Examples:
      | params                                               | streaming |
      | the text of specs/detection/detection-params.v7.toml | present   |
      | the text of specs/detection/detection-params.v6.toml | absent    |
      | the text of specs/detection/detection-params.v5.toml | absent    |
      | the default DetectionParams of the code              | present   |

  @RF-16
  Scenario: The defaults are the v7 file
    Given the default DetectionParams of the code
    Then they are equal to specs/detection/detection-params.v7.toml
    And their sections other than [lossy.streaming] are equal to those of specs/detection/detection-params.v6.toml
    And lossy.streaming.sample_rates_hz is [48000], lossy.streaming.codecs is [mp3, aac], lossy.streaming.min_declared_bitrate_kbps is 192, lossy.streaming.min_cutoff_hz is 19500 and lossy.streaming.max_cutoff_hz is 21000
    And every TrackAnalysis computed with them carries algorithmVersion 7

  @RF-16
  Scenario Outline: Values of [lossy.streaming] at the edge of validity are accepted
    Given the text of specs/detection/detection-params.v7.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is accepted

    Examples:
      | change                                                     |
      | lossy.streaming.sample_rates_hz set to [44100, 48000]      |
      | lossy.streaming.codecs set to ["aac"]                      |
      | lossy.streaming.codecs set to ["aac", "mp3"]               |
      | lossy.streaming.min_declared_bitrate_kbps set to 1         |
      | lossy.streaming.min_cutoff_hz set to 20999.0               |

  @RF-16
  Scenario Outline: An invalid [lossy.streaming] section is rejected
    The shape is checked when the file is deserialised; the values by
    DetectionParams::validate, which every loader calls, after the checks of
    lossy-source.md section 6. A NaN fails every comparison.
    Given the text of specs/detection/detection-params.v7.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is rejected by <layer>
    And the error names "<what>"

    Examples:
      | change                                                                  | layer           | what                                                             |
      | the min_cutoff_hz line of [lossy.streaming] removed                     | deserialisation | min_cutoff_hz                                                    |
      | the codecs line of [lossy.streaming] removed                            | deserialisation | codecs                                                           |
      | the sample_rates_hz line of [lossy.streaming] removed                   | deserialisation | sample_rates_hz                                                  |
      | an extra line "max_declared_bitrate_kbps = 320" in [lossy.streaming]     | deserialisation | max_declared_bitrate_kbps                                        |
      | lossy.streaming.codecs set to ["mp3", "opus"]                           | deserialisation | opus                                                             |
      | lossy.streaming.codecs set to ["mp3", "flac"]                           | deserialisation | flac                                                             |
      | lossy.streaming.sample_rates_hz set to [48000.0]                        | deserialisation | sample_rates_hz                                                  |
      | every line of [lossy] and [lossy.cutoffs] removed, [lossy.streaming] kept | deserialisation | missing field                                                  |
      | lossy.streaming.sample_rates_hz set to []                               | validation      | lossy.streaming.sample_rates_hz is not empty                     |
      | lossy.streaming.sample_rates_hz set to [0]                              | validation      | lossy.streaming.sample_rates_hz: sample rate > 0                 |
      | lossy.streaming.sample_rates_hz set to [48000, 44100]                   | validation      | lossy.streaming.sample_rates_hz increases                        |
      | lossy.streaming.sample_rates_hz set to [48000, 48000]                   | validation      | lossy.streaming.sample_rates_hz increases                        |
      | lossy.streaming.codecs set to []                                        | validation      | lossy.streaming.codecs is not empty                              |
      | lossy.streaming.codecs set to ["mp3", "mp3"]                            | validation      | lossy.streaming.codecs has no duplicates                         |
      | lossy.streaming.min_declared_bitrate_kbps set to 0                      | validation      | lossy.streaming.min_declared_bitrate_kbps > 0                    |
      | lossy.streaming.min_cutoff_hz set to 0.0                                | validation      | lossy.streaming.min_cutoff_hz > 0                                |
      | lossy.streaming.min_cutoff_hz set to nan                                | validation      | lossy.streaming.min_cutoff_hz > 0                                |
      | lossy.streaming.min_cutoff_hz set to 21000.0 (equal to max_cutoff_hz)   | validation      | lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz    |
      | lossy.streaming.max_cutoff_hz set to nan                                | validation      | lossy.streaming.min_cutoff_hz < lossy.streaming.max_cutoff_hz    |
      | lossy.suspicious_shortfall_hz set to 0.0 and lossy.streaming.codecs set to [] | validation | lossy.suspicious_shortfall_hz > 0                             |

  # --- In the app: real run, CSV and interface ------------------------------------

  @RF-16 @RNF-09 @RNF-05
  Scenario: A 48 kHz rip with a Unicode name is flagged in a real run and left unchanged
    Given a folder "Álbum ″Ñandú″" with the lossy fixture "mp3_320_48k_lp20k" saved as "01 – Canción ‘x’.mp3" and the lossy fixture "mp3_320_lp20k" saved as "02 – Ópera «Ça va» 🎵.mp3"
    When I analyse "Álbum ″Ñandú″" with the default parameters
    Then the trackDone event of "01 – Canción ‘x’.mp3" carries verdict "suspicious", reasons ["streamingSampleRate"], sample rate 48000, declared bitrate 320, no estimated source bitrate and algorithmVersion 7
    And the trackDone event of "02 – Ópera «Ça va» 🎵.mp3" carries verdict "real", no reasons, sample rate 44100 and estimated source bitrate 320
    And both paths in the events equal the names on disk code point for code point
    And the album update of "Álbum ″Ñandú″" after both tracks has verdict "suspicious"
    And the size, modification time and content hash of both files are unchanged

  @RF-16 @RF-09 @RNF-09
  Scenario: The CSV record of a flagged track
    The reasons column holds the contract identifier; the estimate cell is empty.
    Given a stubbed TrackAnalysis with path "C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’.mp3", folder "C:\Music\Álbum ″Ñandú″", format "mp3", codec "mp3", sample rate 48000, no bit depth, 2 channels, duration 240000 ms, size 9600000 bytes, declared bitrate 320 kbps, bitrate mode "cbr", no estimated source bitrate, cutoff 20050 Hz with steepness 85.0 dB/kHz and noise floor -95.0 dB, verdict "suspicious", reasons "streamingSampleRate", no error and algorithmVersion 7
    When the CSV document of that track is built
    Then its second record is exactly:
      """
      suspicious,01 – Canción ‘x’.mp3,C:\Music\Álbum ″Ñandú″,mp3,48000,,240000,20050,320,cbr,,streamingSampleRate,,,mp3,2,9600000,85.0,-95.0,7,C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’.mp3
      """

  @RF-16 @RF-06
  Scenario: Table cells and tooltip of a flagged track
    Given a stubbed TrackAnalysis of "a.mp3" with format "mp3", codec "mp3", sample rate 48000, declared bitrate 320 kbps, bitrate mode "cbr", no estimated source bitrate, cutoff 20100 Hz, verdict "suspicious" and reasons "streamingSampleRate"
    When the results table shows it
    Then "a.mp3" has state "Suspicious", samplerate "48000", cutoff "20.1 kHz", bitrate "320 kbps" and "—" in its real bitrate cell
    And hovering its state cell shows a tooltip that contains "Full band at 48 kHz: possibly ripped from a streaming site such as YouTube. Genuine 48 kHz releases look the same."

  @RF-16 @RF-07
  Scenario: Detail line of a flagged track
    Given a stubbed TrackAnalysis with format "mp3", sample rate 48000, declared bitrate 320 kbps, bitrate mode "cbr", no estimated source bitrate, cutoff 20100 Hz, verdict "suspicious" and reasons "streamingSampleRate"
    When I select its row
    Then the panel shows "Cutoff 20.1 kHz · 320 kbps · Suspicious · Full band at 48 kHz: possibly ripped from a streaming site such as YouTube. Genuine 48 kHz releases look the same."

  # --- Calibration reports (specs/calibration/lossy.md section 6.8) ----------------

  @RF-16 @RP-11
  Scenario: Lossy verdict changes, streaming counts and sample-rate rows
    The evaluation step of the calibration tool, given evaluated lossy rows
    (no audio). Group rows and columns are those of lossy.md sections 6.3 and 6.8.
    Rows 1 to 6 belong to generated entries ([[generate.lossy]] ids); row 7 is a
    confirmed lossy fake with upstream "opus" (written "confirmed_fake opus", as
    in calibration-lossy.feature): it is counted in the 6.3 row of confirmed
    lossy fakes with upstream = opus (the "upstream = opus" row of K4) and in the
    6.3 row of confirmed lossy fakes at sample rate 48000.
    Given these lossy rows, each evaluated with detection-params.v6.toml and detection-params.v7.toml
      | row | group               | truth | rate  | fc    | v6 verdict | v6 reasons          | v7 verdict | v7 reasons          |
      | 1   | opus128_cbr320      | fake  | 48000 | 20050 | real       |                     | suspicious | streamingSampleRate |
      | 2   | opus128_cbr320      | fake  | 48000 | 20150 | real       |                     | suspicious | streamingSampleRate |
      | 3   | opus128_cbr320      | fake  | 48000 | 17050 | fake       |                     | fake       |                     |
      | 4   | cbr320              | real  | 48000 | 20450 | real       |                     | suspicious | streamingSampleRate |
      | 5   | cbr320              | real  | 44100 | 20450 | real       |                     | real       |                     |
      | 6   | cbr320              | real  | 44100 | 18550 | suspicious | cutoffBelowDeclared | suspicious | cutoffBelowDeclared |
      | 7   | confirmed_fake opus | fake  | 48000 | 20050 | real       |                     | suspicious | streamingSampleRate |
    When the lossy tables are written
    Then the lossy verdict-change table v6 × v7 has "real → suspicious" 4 and every other off-diagonal count 0
    And its "of which streamingSampleRate" count is 4
    And with v7 the group row "opus128_cbr320" has streaming 2, caught 3 of 3 and fc median 20050 Hz
    And with v7 the row "cbr320 @ 48000" has 1 row, streaming 1 and Suspicious 1
    And with v7 the row "cbr320 @ 44100" has 2 rows, streaming 0, Real 1 and Suspicious 1
    And with v6 every streaming count is 0
    And summary.md names no file, folder or artist

  @RF-16 @RP-11
  Scenario: The lossy sample corpus with v6 and v7
    The sample corpus's lossy fixtures are all 44.1 kHz, so v7 changes nothing there.
    Given the lossy variant of the sample corpus (specs/calibration/lossy.md section 9)
    When the tool runs with --params detection-params.v6.toml --params detection-params.v7.toml
    Then the lossy verdict-change table v6 × v7 is diagonal and its "of which streamingSampleRate" count is 0
    And the lossy matrix of v6 and of v7 on split "all" each equals the v5 matrix of lossy.md section 9
    And every streaming count is 0 for both files

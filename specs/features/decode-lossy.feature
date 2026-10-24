@RF-14
Feature: Decode MP3 and M4A files (H2)
  MP3 and M4A (AAC-LC or ALAC) are decoded with symphonia, without external
  tools, and give the same TrackAnalysis contract as WAV and FLAC
  (specs/contracts/commands.md, version 3). Stream information:
  specs/detection/lossy-source.md section 2. Test files: the synthetic lossy
  fixtures of specs/detection/lossy-fixtures.md, written inside the tests
  (ADR-0040); the test stage first confirms the checks of its section 3.
  Opus is specified in specs/pending/opus.feature and not tested yet.
  Parameters: the default DetectionParams (detection-params.v6.toml since
  detection v6, rules.md Q22 (c): v5's values plus [edge], algorithmVersion 6).

  # --- Scan ----------------------------------------------------------------

  @RF-02 @RF-14
  Scenario: MP3 and M4A files are found; Opus files are not yet
    Given a folder "Library" containing "a.flac", "b.mp3", "c.M4A", "d.opus", "e.ogg", "f.mp4" and "notes.txt", and a subfolder "CD2" containing "g.MP3"
    When I analyse "Library"
    Then 4 files are scanned: "a.flac", "b.mp3", "c.M4A" and "CD2/g.MP3"

  @RF-02 @RF-14
  Scenario: A folder with only Opus files has no supported audio yet
    Given a folder "Opus only" containing only "a.opus"
    When I analyse "Opus only"
    Then the analysis fails with error "NO_AUDIO_FOUND"

  # --- Stream information --------------------------------------------------

  @RF-14
  Scenario Outline: Stream information of lossy fixtures
    Given the lossy fixture "<fixture>"
    When it is analysed
    Then the TrackInfo has format "<format>", codec "<codec>", sample rate <rate> Hz, 1 channel and no bit depth
    And its declared bitrate is <kbps> kbps with bitrate mode "<mode>"
    And its duration is 30 s within one frame

    Examples:
      | fixture             | format | codec | rate  | kbps | mode |
      | mp3_320_lp20k5      | mp3    | mp3   | 44100 | 320  | cbr  |
      | mp3_128_lp17k       | mp3    | mp3   | 44100 | 128  | cbr  |
      | mp3_192_lp17k_notag | mp3    | mp3   | 44100 | 192  | cbr  |
      | mp3_vbr_lp19k5      | mp3    | mp3   | 44100 | 240  | vbr  |
      | mp3_abr_lp18k6      | mp3    | mp3   | 44100 | 192  | abr  |
      | mp3_320_48k_lp20k   | mp3    | mp3   | 48000 | 320  | cbr  |

  @RF-14
  Scenario: Stream information of an AAC file in M4A
    The declared bitrate is the average over the track's samples; AAC does
    not signal its bitrate mode.
    Given the lossy fixture "m4a_aac_128_lp16k"
    When it is analysed
    Then the TrackInfo has format "m4a", codec "aac", sample rate 44100 Hz, 1 channel and no bit depth
    And its declared bitrate is exactly 128 kbps
    And it has no bitrate mode

  @RF-14
  Scenario Outline: ALAC in M4A is lossless and judged by the lossless rules
    Given the lossy fixture "<fixture>"
    When it is analysed
    Then the TrackInfo has format "m4a", codec "alac", bit depth 16, no declared bitrate and no bitrate mode
    And the verdict is "<verdict>" with the reasons and cutoff that specs/detection/rules.md gives the lossless signal "<signal>" saved as FLAC
    And no estimated source bitrate is reported

    Examples:
      | fixture            | signal    | verdict |
      | m4a_alac_full_band | full_band | real    |
      | m4a_alac_lp16k     | lp16k     | fake    |

  @RF-14
  Scenario: The LAME tag is read from the tag frame
    The encoder string and the low-pass of the LAME extension are read from the
    tag frame; the tag frame is not counted as audio. The low-pass can only
    lower the expected cutoff (detection-lossy.feature).
    Given the lossy fixture "mp3_320_lp17k"
    When its AudioProbe is taken
    Then its encoder is "LAME3.100" and its encoder low-pass is 20500 Hz
    And its duration is 1148 × 1152 / 44100 s = 29989 ms within 1 ms: the 1148 audio frames of the fixture, the tag frame excluded

  @RF-14
  Scenario: A zero low-pass byte means no low-pass is declared
    Given the lossy fixture "mp3_abr_lp18k6"
    When its AudioProbe is taken
    Then it has no encoder low-pass

  @RF-14
  Scenario: WAV and FLAC report their codec and no bitrate
    Given the synthetic signal "full_band" saved as FLAC and as WAV
    When both are analysed
    Then the FLAC has codec "flac" and the WAV has codec "pcm"
    And neither has a declared bitrate, a bitrate mode or an estimated source bitrate

  # --- Same result contract --------------------------------------------------

  @RF-14 @RF-06
  Scenario: A lossy row has the same fields as a lossless row
    Given a folder "Mixed" with "full_band.flac" and the lossy fixtures "mp3_320_lp17k.mp3" and "m4a_aac_128_lp16k.m4a"
    When I analyse "Mixed"
    Then every trackDone event carries a complete TrackAnalysis with algorithmVersion 6
    And the table shows "mp3_320_lp17k.mp3" with format "MP3", bitrate "320 kbps", real bitrate "≈128 kbps" and state "Fake", and a cutoff within 200 Hz of 17.0 kHz
    And the table shows "m4a_aac_128_lp16k.m4a" with format "M4A", bitrate "128 kbps", real bitrate "128 kbps" and state "Real"
    And the table shows "full_band.flac" with "—" in its bitrate and real bitrate cells

  @RF-14 @RF-06
  Scenario: A variable bitrate is marked in the table
    Given a folder "VBR" with the lossy fixture "mp3_vbr_lp19k5.mp3"
    When I analyse "VBR"
    Then the table shows its bitrate as "240 kbps VBR"

  # --- Presentation (stubbed analyses; specs/ui/lossy-formats.md) -------------
  # These scenarios give the TrackAnalysis directly (no audio), so every value
  # shown is exact.

  @RF-06 @RF-15
  Scenario Outline: BitrateValue states
    Given a BitrateValue with kbps <kbps>, mode <mode> and approx <approx>
    Then it shows "<text>"
    And its accessible name is "<name>"

    Examples:
      | kbps | mode | approx | text         | name         |
      | 320  | cbr  | false  | 320 kbps     | 320 kbps     |
      | 245  | vbr  | false  | 245 kbps VBR | 245 kbps VBR |
      | 192  | abr  | false  | 192 kbps ABR | 192 kbps ABR |
      | 128  | none | true   | ≈128 kbps    | ≈128 kbps    |
      | none | none | false  | —            | No bitrate   |

  @RF-06 @RF-15
  Scenario: Table cells of a stubbed lossy row
    Given a stubbed TrackAnalysis of "a.mp3" with format "mp3", codec "mp3", declared bitrate 320 kbps, bitrate mode "cbr", estimated source bitrate 128 kbps, cutoff 17000 Hz and verdict "fake"
    And a stubbed TrackAnalysis of "b.m4a" with format "m4a", codec "alac", no declared bitrate, cutoff 22050 Hz and verdict "real"
    When the results table shows them
    Then "a.mp3" has format "MP3", cutoff "17.0 kHz", bitrate "320 kbps" and real bitrate "≈128 kbps"
    And "b.m4a" has format "M4A (ALAC)" and "—" in its bitrate and real bitrate cells

  @RF-06 @RF-15
  Scenario: The Real bitrate column explains the estimate
    When the results table is shown
    Then the "Real bitrate" column header has the tooltip "The real bitrate is an estimate from the cutoff, in terms of the file's own codec. Files from sources with little treble, from encoders without a low-pass, or re-encoded from a close bitrate may not be detected."

  @RF-07 @RF-15
  Scenario Outline: Detail line of a lossy track
    Given a stubbed TrackAnalysis with format "mp3", declared bitrate <declared> kbps, bitrate mode "<mode>", estimated source bitrate <estimate> kbps, cutoff <cutoff> Hz, verdict "<verdict>" and reasons "<reasons>"
    When I select its row
    Then the panel shows "<line>"

    Examples:
      | declared | mode | estimate | cutoff | verdict    | reasons             | line                                                                                                   |
      | 320      | cbr  | 128      | 17000  | fake       |                     | Cutoff 17.0 kHz · 320 kbps · ≈128 kbps source · Fake                                                    |
      | 320      | cbr  | 192      | 18500  | suspicious | cutoffBelowDeclared | Cutoff 18.5 kHz · 320 kbps · ≈192 kbps source · Suspicious · Cutoff lower than expected for its bitrate |
      | 320      | cbr  | 320      | 20400  | real       |                     | Cutoff 20.4 kHz · 320 kbps · Genuine                                                                    |
      | 240      | vbr  | 240      | 19500  | real       |                     | Cutoff 19.5 kHz · 240 kbps VBR · Genuine                                                                |

  @RF-07 @RF-14
  Scenario: An unsupported lossy file shows its error text
    Given a stubbed TrackAnalysis with format "m4a", verdict "error" and error "unsupportedFormat"
    When I select its row
    Then the panel shows "Unsupported audio format" instead of a spectrogram

  @RF-14 @RF-07
  Scenario: The spectrogram of an MP3 file
    Given the lossy fixture "mp3_320_lp17k" saved as "a.mp3"
    When its spectrogram is requested
    Then it has height 512 and a width of at most 1200
    And maxFrequencyHz is 22050
    And the rows above 17.2 kHz are near the bottom of dbRange

  # --- Errors (RNF-04) ---------------------------------------------------------

  @RF-14 @RNF-04
  Scenario Outline: Broken or unsupported lossy files become error tracks
    Given the lossy fixture "<fixture>"
    When it is analysed
    Then the verdict is "error" with error "<error>"
    And no cutoff and no estimated source bitrate are reported

    Examples:
      | fixture     | error             |
      | mp3_corrupt | corruptStream     |
      | m4a_corrupt | corruptStream     |
      | m4a_he_aac  | unsupportedFormat |
      | mp3_short   | tooShort          |

  @RNF-04
  Scenario: A corrupt lossy file does not stop the analysis
    Given a folder "Mixed" with "mp3_320_lp20k5.mp3", "mp3_corrupt.mp3" and "m4a_aac_128_lp16k.m4a"
    When I analyse "Mixed"
    Then "mp3_corrupt.mp3" has state "Error" with the reason "Corrupt stream"
    And the other two files are analysed normally
    And the last event is "finished" with errors 1

  # --- Read-only and Unicode -------------------------------------------------

  @RNF-05
  Scenario: Analysed lossy files are never modified
    Given a folder "Album A" with the lossy fixtures "mp3_320_lp20k5.mp3" and "m4a_aac_128_lp16k.m4a"
    When I analyse "Album A"
    Then the size, modification time and content hash of both files are unchanged

  @RNF-09
  Scenario: Unicode names of lossy files
    Given a folder "Álbum ″Ñandú″" with the lossy fixture "mp3_320_lp20k5" saved as "01 – Canción ‘x’.mp3"
    When I analyse "Álbum ″Ñandú″"
    Then the table shows the file name "01 – Canción ‘x’.mp3" exactly
    And its verdict is "real"

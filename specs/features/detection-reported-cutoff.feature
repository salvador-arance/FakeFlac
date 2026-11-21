@RF-04
Feature: The reported cutoff of a lossless file is not a narrow content edge (detection v9)
  As a user who reads the Cutoff column
  I want the cutoff of a genuine lossless file to be where its content ends
  So that a short narrow sound below the treble is not shown as the cutoff

  Spec: specs/detection/rules.md section 2 ("Reported cutoff", ContentEdge,
  f_step), section 3 ("Relation to v8 (v9)"), section 4 and section 9
  (acceptance R1-R4, M1); G1 questions Q24-Q32, written with their defaults.
  Fixtures: specs/detection/fixtures.md, "Reported cutoff". ADR-0052
  (Proposed). Proposed for G1.
  "DetectionParams v9" and "the defaults" mean detection-params.v9.toml, the
  proposed defaults; "v8" means the parameters of
  specs/detection/detection-params.v8.toml, which has no
  edge.content_edge_cutoff. Thresholds: step_suspicious_db = 7,
  step_low_suspicious_db = 10, step_high_suspicious_db = 12,
  step_fake_db = 10, gap_fake_db = 6, gap_suspicious_db = 4, lossy range
  [14500, 20300) Hz, ambiguous_max_cutoff_hz = 20800, edge.max_median_step_db
  = 3, edge.min_residual_db = 15, edge.max_step_db = 16, step_window_bands = 3
  and band_hz = 100, so the steepness of a reported step is D / 0.4 dB/kHz,
  and fs is a band centre (an odd multiple of 50 Hz): 20850 Hz is the lowest
  fs at or above ambiguous_max_cutoff_hz.
  What counts is the rule that decided the verdict (the first row of rules.md
  section 3 that holds), not which of V6-V8 would hold on their own: in the
  row "V3 decides although V7 would hold" the condition of V7 holds too, and
  in the row "fs >= ambiguous_max_cutoff_hz" D reaches
  step_high_suspicious_db but fs is outside V8's range; both report fe.
  A track is a content edge (ContentEdge) when fs is defined, D_Q < 3,
  R >= 15 and D < 16, wherever fs lies. With v9 a lossless track reports fe
  instead of fs, with steepness 0, when D >= 7, it is a content edge and the
  verdict was not decided by V6, V7 or V8 (Q27 (2): a "suspicious" verdict
  with "ambiguousCutoff" keeps fs). No verdict, reason or evidence changes.
  Lossy files keep their cutoff fc = f_step exactly as in v8 (Q26).
  The owner's track of the 2026-11-21 report is given by its measurements
  only; it is never named. U = -90 dB and B = 0.2 dB (not full band) unless a
  step says otherwise. Rule-level values are compared within 0.01.
  Audio files are generated inside the tests; every cache file is inside the
  test's own temporary folder.

  # --- The rule from given measurements (rules.md section 2) -------------------

  Scenario Outline: A content edge is reported as the content extent, and no verdict changes
    Given a lossless track with codec "flac" and sample rate 44100 Hz
    And the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = -90 dB, B = 0.2 dB, D_Q = <D_Q> dB and R = <R> dB
    When the verdict rules and the evidence of the parameters of specs/detection/detection-params.v8.toml ("v8") and of DetectionParams v9 are applied
    Then the track is <edge>
    And both verdicts are "<verdict>" with the reasons "<reasons>" and the same evidence
    And the reported cutoff is <v8 cutoff> Hz with v8 and <v9 cutoff> Hz with v9
    And the steepness with v9 is <v9 steepness> dB/kHz

    Examples:
      | case                                         | H     | G     | D     | fs    | fe    | D_Q  | R     | edge                 | verdict    | reasons                 | v8 cutoff | v9 cutoff | v9 steepness |
      | the owner's track (2026-11-21)               | -15.6 | 0     | 7.4   | 12750 | 22050 | 2.3  | 28.5  | a content edge       | real       |                         | 12750     | 22050     | 0            |
      | D_Q at max_median_step_db                    | -15.6 | 0     | 7.4   | 12750 | 22050 | 3.0  | 28.5  | not a content edge   | real       |                         | 12750     | 12750     | 18.5         |
      | D_Q just below max_median_step_db            | -15.6 | 0     | 7.4   | 12750 | 22050 | 2.99 | 28.5  | a content edge       | real       |                         | 12750     | 22050     | 0            |
      | R just below min_residual_db                 | -15.6 | 0     | 7.4   | 12750 | 22050 | 2.3  | 14.99 | not a content edge   | real       |                         | 12750     | 12750     | 18.5         |
      | R at min_residual_db                         | -15.6 | 0     | 7.4   | 12750 | 22050 | 2.3  | 15.0  | a content edge       | real       |                         | 12750     | 22050     | 0            |
      | D at step_suspicious_db                      | -15.6 | 0     | 7.0   | 12750 | 22050 | 2.3  | 28.5  | a content edge       | real       |                         | 12750     | 22050     | 0            |
      | D just below step_suspicious_db: no step     | -15.6 | 0     | 6.99  | 12750 | 22050 | 2.3  | 28.5  | a content edge       | real       |                         | 22050     | 22050     | 0            |
      | V7 decides: the step keeps its cutoff        | -15.6 | 0     | 10.0  | 12750 | 22050 | 2.3  | 28.5  | a content edge       | suspicious | ambiguousCutoff         | 12750     | 12750     | 25.0         |
      | D at max_step_db (V7)                        | -15.6 | 0     | 16.0  | 12750 | 22050 | 2.3  | 28.5  | not a content edge   | suspicious | ambiguousCutoff         | 12750     | 12750     | 40.0         |
      | the gap rule V3 decides, fe below Nyquist    | -15.6 | 8     | 7.4   | 12750 | 18700 | 2.3  | 28.5  | a content edge       | fake       |                         | 12750     | 18700     | 0            |
      | V3 decides although V7 would hold            | -15.6 | 8     | 10.5  | 12750 | 22050 | 2.3  | 28.5  | a content edge       | fake       |                         | 12750     | 22050     | 0            |
      | weak gap: V5 decides                         | -15.6 | 5     | 7.4   | 12750 | 22050 | 2.3  | 28.5  | a content edge       | suspicious | highBandGaps            | 12750     | 22050     | 0            |
      | little treble: V2 decides                    | -65   | 0     | 7.4   | 12750 | 22050 | 2.3  | 28.5  | a content edge       | suspicious | lowHighFrequencyContent | 12750     | 22050     | 0            |
      | W4: EdgeStep, V6 decides                     | -20   | -0.25 | 14.22 | 19450 | 22050 | 0.32 | 17.48 | a content edge       | suspicious | ambiguousCutoff         | 19450     | 19450     | 35.55        |
      | W5: EdgeStep, V6 decides                     | -20   | -2.51 | 9.88  | 20150 | 22050 | 1.02 | 22.35 | a content edge       | suspicious | ambiguousCutoff         | 20150     | 20150     | 24.7         |
      | above the lossy range, V8 does not fire      | -20   | 0     | 11.0  | 20450 | 22050 | 1.0  | 16.0  | a content edge       | real       |                         | 20450     | 22050     | 0            |
      | above the lossy range, V8 decides            | -20   | 0     | 12.0  | 20450 | 22050 | 1.0  | 16.0  | a content edge       | suspicious | ambiguousCutoff         | 20450     | 20450     | 30.0         |
      | fs >= ambiguous_max_cutoff_hz: V10 decides   | -20   | 0     | 12.0  | 20850 | 22050 | 1.0  | 16.0  | a content edge       | real       |                         | 20850     | 22050     | 0            |
      | a low-pass below the lossy range             | -15.6 | 0     | 60    | 12750 | 12600 | 55   | 0.2   | not a content edge   | suspicious | ambiguousCutoff         | 12750     | 12750     | 150.0        |

  Scenario: The v3 gap evidence of a content edge does not change
    Given a lossless track with codec "flac" and sample rate 44100 Hz
    And the measurements H = -15.6 dB, G = 8 dB, D = 7.4 dB, fs = 12750 Hz, fe = 18700 Hz, U = -90 dB, B = 0.2 dB, D_Q = 2.3 dB and R = 28.5 dB
    When the verdict rules and the evidence of DetectionParams v9 are applied
    Then the verdict is "fake" with no reasons
    And the evidence is "gapStep 8.00/6.00/2.00"
    And the reported cutoff is 18700 Hz

  Scenario Outline: A content edge needs the edge measurements
    Without an [edge] section D_Q and R are undefined; without a floor window
    (at 32 kHz, Nyquist 16 kHz) R is undefined. Then the track is not a
    content edge and the cutoff is f_step, as with v8.
    Given a lossless track with codec "flac" and sample rate <rate> Hz
    And the measurements H = -15.6 dB, G = 0 dB, D = 7.4 dB, fs = 12750 Hz, fe = <fe> Hz, U = -90 dB, B = 0.2 dB, D_Q = <D_Q> and R = <R>
    When the verdict rules of <params> are applied
    Then the track is not a content edge
    And the verdict is "real" with no reasons
    And the reported cutoff is 12750 Hz with steepness 18.5 dB/kHz

    Examples:
      | rate  | fe    | D_Q       | R         | params                                                     |
      | 32000 | 16000 | 2.3 dB    | undefined | DetectionParams v9                                         |
      | 44100 | 22050 | undefined | undefined | the parameters of specs/detection/detection-params.v5.toml |

  Scenario Outline: Older params files and a disabled key keep the v8 cutoff
    Given a lossless track with codec "flac" and sample rate 44100 Hz
    And the measurements H = -15.6 dB, G = 0 dB, D = 7.4 dB, fs = 12750 Hz, fe = 22050 Hz, U = -90 dB, B = 0.2 dB, D_Q = 2.3 dB and R = 28.5 dB
    When the verdict rules of <params> are applied
    Then the track is a content edge
    And the verdict is "real" with no reasons
    And the reported cutoff is <cutoff> Hz

    Examples:
      | params                                                                                   | cutoff |
      | DetectionParams v9                                                                       | 22050  |
      | the parameters of specs/detection/detection-params.v8.toml                               | 12750  |
      | the parameters of specs/detection/detection-params.v6.toml                               | 12750  |
      | the text of specs/detection/detection-params.v9.toml with edge.content_edge_cutoff set to false | 12750  |
      | the text of specs/detection/detection-params.v9.toml with the content_edge_cutoff line removed  | 12750  |

  Scenario Outline: The edge step is the content edge inside the lossy range
    rules.md section 2: EdgeStep = ContentEdge and InRange(fs), the same
    predicate as in v6.
    Given the measurements H = -20 dB, G = -0.25 dB, D = 14.22 dB, fs = <fs> Hz, fe = 22050 Hz, U = -47.06 dB, B = 2.86 dB, D_Q = 0.32 dB and R = 17.48 dB
    When they are evaluated with DetectionParams v9 and with the parameters of specs/detection/detection-params.v6.toml
    Then with both the track is a content edge
    And with both the track is <edge step>

    Examples:
      | fs    | edge step          |
      | 12750 | not an edge step   |
      | 14450 | not an edge step   |
      | 14500 | an edge step       |
      | 19450 | an edge step       |
      | 20250 | an edge step       |
      | 20300 | not an edge step   |
      | 20450 | not an edge step   |

  # --- Lossy files keep fc (lossy-source.md section 3; Q26) ---------------------

  @RF-15
  Scenario Outline: A lossy file with a content edge keeps its v8 cutoff and verdict
    Given a lossy track with codec "<codec>", sample rate <rate> Hz, declared bitrate <b> kbps and tag low-pass — Hz
    And the measurements H = -15.6 dB, G = 0 dB, D = 7.4 dB, fs = 12750 Hz, fe = <fe> Hz, D_Q = 2.3 dB and R = 28.5 dB
    When the lossy rules of the parameters of specs/detection/detection-params.v8.toml ("v8") and of DetectionParams v9 are applied
    Then the track is a content edge
    And with both the expected cutoff is <c_exp> Hz
    And with both the verdict is "suspicious" with the reasons "cutoffBelowDeclared" and an empty evidence
    And with both the estimated source bitrate is <estimate>
    And with both the reported cutoff is 12750 Hz with steepness 18.5 dB/kHz

    Examples:
      | codec | rate  | b   | fe    | c_exp | estimate |
      | mp3   | 44100 | 320 | 22050 | 20500 | 80       |
      | aac   | 44100 | 256 | 22050 | 19000 | 96       |
      | opus  | 48000 | 160 | 24000 | 20000 | none     |

  # --- Synthetic signals (fixtures.md, "Reported cutoff") ----------------------

  Scenario Outline: Reported-cutoff fixtures
    Given the synthetic signal "<id>" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v8.toml ("v8") and with DetectionParams v9
    Then both verdicts are "<verdict>" with the reasons "<reasons>" and the same evidence
    And the v8 cutoff is <v8 cutoff> Hz ± 200 Hz
    And the v9 cutoff is <v9 cutoff> Hz ± 200 Hz

    Examples:
      | id                   | verdict    | reasons                 | v8 cutoff | v9 cutoff |
      | edge_bump12k5        | real       |                         | 12750     | 22050     |
      | edge_bump12k5_lp12k7 | suspicious | lowHighFrequencyContent | 12750     | 12750     |
      | shelf12k7            | real       |                         | 12750     | 12750     |
      | edge_bump19k3        | suspicious | ambiguousCutoff         | 19450     | 19450     |

  Scenario: The content-edge fixture is a content edge below the lossy range
    Derived in fixtures.md (D about 8.8 dB, D_Q about 0.5 to 1.5 dB, R about
    27.5 dB). The test stage confirms it first; if it does not hold, it stops
    for G1.
    Given the synthetic signal "edge_bump12k5" saved as FLAC
    When it is analysed with DetectionParams v9
    Then fs is between 12650 and 12850 Hz
    And D is at least 7 dB and below 10 dB
    And D_Q is below 3 dB
    And R is at least 15 dB
    And fe is 22050 Hz
    And the reported cutoff is 22050 Hz with steepness 0 dB/kHz

  Scenario Outline: The control fixtures are not content edges
    The test stage confirms these measurements first; if they do not hold, it
    stops for G1.
    Given the synthetic signal "<id>" saved as FLAC
    When it is analysed with DetectionParams v9
    Then fs is between 12650 and 12850 Hz
    And <condition>
    And the track is not a content edge

    Examples:
      | id                   | condition                                    |
      | shelf12k7            | D is at least 7 dB and below 10 dB, and D_Q is at least 3 dB |
      | edge_bump12k5_lp12k7 | D is at least 16 dB                          |

  @RF-14 @RF-15
  Scenario: v9 gives the v8 results on every other fixture
    Given every synthetic signal of specs/detection/fixtures.md except "edge_bump12k5" saved as FLAC, and every lossy fixture of specs/detection/lossy-fixtures.md sections 2 and 2.2
    When each one is analysed with the parameters of specs/detection/detection-params.v8.toml ("v8") and with DetectionParams v9
    Then the two results have the same verdict, reasons, evidence, error, cutoff and estimated source bitrate for every file
    And they carry algorithmVersion 8 and 9
    And "edge_bump12k5" has the same verdict, reasons, evidence and error with both, and only its cutoff differs

  # --- Empty state and errors (RNF-04) -----------------------------------------

  @RNF-04
  Scenario Outline: Without spectra there is no cutoff to report
    Given the <file>
    When it is analysed with DetectionParams v9
    Then the verdict is "<verdict>"
    And <outcome>
    And no cutoff is reported

    Examples:
      | file                                            | verdict    | outcome                                   |
      | synthetic signal "silence" saved as FLAC        | suspicious | the reasons are "lowHighFrequencyContent" |
      | file "corrupt" (FLAC truncated after the header) | error      | the error is "corruptStream"              |
      | synthetic signal "short" saved as FLAC          | error      | the error is "tooShort"                   |

  # --- Parameter files (rules.md section 4) ------------------------------------

  Scenario: Parameters come from the versioned file
    Given the default DetectionParams of the code
    Then they are equal to specs/detection/detection-params.v9.toml
    And their sections and values other than edge.content_edge_cutoff are equal to those of specs/detection/detection-params.v8.toml
    And every TrackAnalysis computed with them carries algorithmVersion 9

  Scenario Outline: Valid parameter files deserialise and validate
    Given <params>
    When it is validated, after being deserialised into DetectionParams if it is a file
    Then it is accepted
    And its edge.content_edge_cutoff is <value>

    Examples:
      | params                                                                                     | value                  |
      | the text of specs/detection/detection-params.v9.toml                                       | true                   |
      | the text of specs/detection/detection-params.v8.toml                                       | false (absent)         |
      | the text of specs/detection/detection-params.v6.toml                                       | false (absent)         |
      | the text of specs/detection/detection-params.v5.toml                                       | false (no [edge])      |
      | the text of specs/detection/detection-params.v9.toml with edge.content_edge_cutoff set to false | false             |
      | the default DetectionParams of the code                                                    | true                   |

  Scenario Outline: An invalid content_edge_cutoff is rejected when the file is deserialised
    Given the text of specs/detection/detection-params.v9.toml with <change>
    When it is deserialised into DetectionParams
    Then it is rejected by deserialisation
    And the error names "<what>"

    Examples:
      | change                                                              | what                |
      | edge.content_edge_cutoff set to 1                                   | content_edge_cutoff |
      | edge.content_edge_cutoff set to "yes"                               | content_edge_cutoff |
      | the line "content_edge_cutoff = true" moved from [edge] to [verdict] | content_edge_cutoff |
      | the line "content_edge_cutoff = true" renamed to "content_edge_cutof = true" | content_edge_cutof |

  # --- In the app: a real run (src-tauri) ---------------------------------------

  @RNF-09 @RNF-05
  Scenario: A content edge in a real run with Unicode names
    Given a folder "Álbum ″Ñandú″" with the synthetic signal "edge_bump12k5" saved as "01 – Canción ‘x’.flac" and the synthetic signal "edge_bump12k5_lp12k7" saved as "02 – Ópera «Ça va» 🎵.flac"
    When I analyse "Álbum ″Ñandú″" with the default parameters
    Then the trackDone event of "01 – Canción ‘x’.flac" carries verdict "real", no reasons, a cutoff with frequencyHz 22050 and steepnessDbPerKhz 0, and algorithmVersion 9
    And the trackDone event of "02 – Ópera «Ça va» 🎵.flac" carries a cutoff with frequencyHz 12750 ± 200 and algorithmVersion 9
    And the CSV exported from the run has cutoff_hz "22050" in the row of "01 – Canción ‘x’.flac"
    And every path in the events equals the name on disk code point for code point
    And the size, modification time and content hash of every file are unchanged

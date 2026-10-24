@RF-04
Feature: Edge step on the step rule V4 (detection v6)
  Rule-level checks of specs/detection/rules.md sections 2 to 4 with
  DetectionParams v6 (the defaults, detection-params.v6.toml), and the
  edge-step fixtures of specs/detection/fixtures.md. The thresholds are:
  edge.max_median_step_db = 3, edge.min_residual_db = 15, edge.max_step_db = 16,
  edge.floor_min_db = -104 (absolute), floor window [20900, 22000) Hz,
  step_fake_db = 10, step_suspicious_db = 7, gap_fake_db = 6,
  gap_suspicious_db = 4, lossy range [14500, 20300) Hz.
  A track is an edge step when fs is in the lossy range, D_Q < 3, R >= 15 and
  D < 16. An edge step only skips V4.
  In the measurement tables, fs "—" means fs is undefined (no step: D = 0).
  W4 is the confirmed false positive FP4; W5 and W6 are two other primary
  tracks of its album (rules.md, "Names used here"). Their measurements are
  those of rules.md, "Why v5's lossless rules were replaced"; H is illustrative.

  # --- Measurements --------------------------------------------------------

  @RF-04
  Scenario: Median step, top floor and post-step residual at the step of the mean spectrum
    Levels in this scenario are absolute dB, before L_ref is subtracted. The
    deepest step of M is at 19450 Hz; the smaller drop at 20.9 kHz is not fs.
    Given band spectra at 44.1 kHz with 100 Hz bands and bit depth 16
    And M = -30 dB on every band centred below 19450 Hz, -45 dB at 19450 Hz, -55 dB from 19550 to 20850 Hz and -70 dB from 20950 Hz up
    And Q = -50 dB on the bands centred from 18850 to 19350 Hz and -51 dB on the bands centred from 19550 to 19750 Hz
    When the measurements of DetectionParams v6 are taken
    Then L_ref = -30 dB
    And D = 25 dB at fs = 19450 Hz
    And D_Q = 1 dB
    And F_top = -40 dB
    And R = 15 dB
    And the track is not an edge step

  @RF-04
  Scenario Outline: Above 16 bits the top floor is never below the 16-bit rounding floor
    Levels in this scenario are absolute dB. -104 dB absolute is -74 dB
    relative to L_ref = -30 dB.
    Given band spectra at 44.1 kHz with 100 Hz bands and bit depth <bit depth>
    And M = -30 dB on every band centred below 19450 Hz, -75 dB at 19450 Hz, -120 dB from 19550 to 20850 Hz and -140 dB from 20950 Hz up
    When the measurements of DetectionParams v6 are taken
    Then D = 90 dB at fs = 19450 Hz
    And F_top = <F_top> dB
    And R = <R> dB

    Examples:
      | bit depth | F_top | R   |
      | 16        | -110  | 20  |
      | 8         | -110  | 20  |
      | 24        | -74   | -16 |
      | 32        | -74   | -16 |
      | undefined | -74   | -16 |

  @RF-04
  Scenario Outline: The edge measurements are undefined without a step, a floor window or an [edge] section
    Given band spectra at <sample rate> Hz with 100 Hz bands and <step>
    When the measurements of <params> are taken
    Then D_Q is <D_Q>
    And F_top and R are undefined
    And the track is not an edge step

    Examples:
      | sample rate | step                         | params                                                     | D_Q       |
      | 32000       | a cutoff step at 15050 Hz    | DetectionParams v6                                         | defined   |
      | 44100       | no cutoff step (fs —)        | DetectionParams v6                                         | undefined |
      | 44100       | a cutoff step at 19450 Hz    | the parameters of specs/detection/detection-params.v5.toml | undefined |

  # --- The conjunction and its boundaries ------------------------------------

  @RF-04
  Scenario Outline: A lossy-range step is an edge step only when the three conditions hold
    Given the measurements H = -20 dB, G = -0.25 dB, D = <D> dB, fs = <fs> Hz, fe = 22050 Hz, U = -47.06 dB, B = 2.86 dB, D_Q = <D_Q> dB and R = <R> dB
    When the verdict rules of DetectionParams v6 are applied
    Then the track is <edge>
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | case                                          | D     | fs    | D_Q   | R     | edge               | verdict    | reasons         |
      | W4                                            | 14.22 | 19450 | 0.32  | 17.48 | an edge step       | suspicious | ambiguousCutoff |
      | D_Q at max_median_step_db                     | 14.22 | 19450 | 3.0   | 17.48 | not an edge step   | fake       |                 |
      | D_Q just below max_median_step_db             | 14.22 | 19450 | 2.99  | 17.48 | an edge step       | suspicious | ambiguousCutoff |
      | R at min_residual_db                          | 14.22 | 19450 | 0.32  | 15.0  | an edge step       | suspicious | ambiguousCutoff |
      | R just below min_residual_db                  | 14.22 | 19450 | 0.32  | 14.99 | not an edge step   | fake       |                 |
      | D at max_step_db                              | 16.0  | 19450 | 0.32  | 17.48 | not an edge step   | fake       |                 |
      | D just below max_step_db                      | 15.99 | 19450 | 0.32  | 17.48 | an edge step       | suspicious | ambiguousCutoff |
      | fs just inside the upper edge of the range    | 14.22 | 20250 | 0.32  | 17.48 | an edge step       | suspicious | ambiguousCutoff |
      | fs at the upper edge of the range (V8)        | 14.22 | 20300 | 0.32  | 17.48 | not an edge step   | suspicious | ambiguousCutoff |
      | fs just below the range (V7)                  | 14.22 | 14450 | 0.32  | 17.48 | not an edge step   | suspicious | ambiguousCutoff |
      | low-pass: the floor right above the step      | 30    | 19450 | -5    | 0.5   | not an edge step   | fake       |                 |
      | low-pass with quantiser holes below it        | 13    | 19450 | 0     | 0.5   | not an edge step   | fake       |                 |
      | low-pass that shows in the median spectrum    | 13    | 19450 | 12    | 20    | not an edge step   | fake       |                 |
      | deep low-pass with a residue above it         | 35    | 19450 | -8    | 22    | not an edge step   | fake       |                 |

  # --- Watch list and what the edge step never changes ----------------------

  @RF-04
  Scenario Outline: An edge step only turns Fake into Suspicious
    Given the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = 22050 Hz, U = <U> dB, B = <B> dB, D_Q = <D_Q> dB and R = <R> dB
    When the verdict rules of DetectionParams v6 are applied
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | case                                     | H   | G     | D     | fs    | U      | B     | D_Q   | R     | verdict    | reasons                 |
      | edge step with a gap: V3 comes first     | -20 | 8     | 14.22 | 19450 | -47.06 | 2.86  | 0.32  | 17.48 | fake       |                         |
      | edge step with a weak gap: V5            | -20 | 5     | 14.22 | 19450 | -47.06 | 2.86  | 0.32  | 17.48 | suspicious | highBandGaps            |
      | edge step with little treble: V2         | -65 | -0.25 | 14.22 | 19450 | -47.06 | 2.86  | 0.32  | 17.48 | suspicious | lowHighFrequencyContent |
      | W5: edge step below step_fake_db (V6)    | -20 | -2.51 | 9.88  | 20150 | -32.64 | 17.29 | 1.02  | 22.35 | suspicious | ambiguousCutoff         |
      | W6: verdict from the gap rule V5         | -20 | 5.65  | 7.39  | 16550 | -51.13 | 2.37  | -0.24 | 26.72 | suspicious | highBandGaps            |
      | W1: deepest step above the lossy range   | -20 | 18.39 | 8.93  | 20450 | -56.64 | 9.53  | 1.11  | 14.15 | suspicious | highBandGaps            |
      | small lossy step, no gap: nothing fires  | -20 | 1     | 5.0   | 18050 | -47.06 | 2.86  | 0.32  | 17.48 | real       |                         |

  @RF-04
  Scenario: Without an [edge] section the edge step is disabled whatever D_Q and R are
    Given the measurements H = -20 dB, G = -0.25 dB, D = 14.22 dB, fs = 19450 Hz, fe = 22050 Hz, U = -47.06 dB, B = 2.86 dB, D_Q = 0.32 dB and R = 17.48 dB
    When the verdict rules of the parameters of specs/detection/detection-params.v5.toml are applied
    Then the track is not an edge step
    And the verdict is "fake"
    And the reasons are ""

  # --- Synthetic signals (fixtures.md, "Edge step") --------------------------

  @RF-04
  Scenario Outline: Edge-step fixtures
    Given the synthetic signal "<id>" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v5.toml and with DetectionParams v6
    Then the v5 verdict is "fake" with reasons ""
    And the v6 verdict is "<v6 verdict>" with reasons "<v6 reasons>"
    And with v6 the track is <edge>
    And both estimated cutoffs are <cutoff> Hz ± 200 Hz

    Examples:
      | id                   | v6 verdict | v6 reasons      | edge             | cutoff |
      | edge_bump19k3        | suspicious | ambiguousCutoff | an edge step     | 19450  |
      | edge_bump19k3_lp19k5 | fake       |                 | not an edge step | 19500  |
      | holes_lp19k5         | fake       |                 | not an edge step | 19500  |
      | shelf19k5            | fake       |                 | not an edge step | 19500  |

  @RF-04
  Scenario: The content-edge fixture meets the three conditions
    Derived in fixtures.md (D about 11.8 dB, D_Q about 1.2 dB, R about 20.7 dB).
    The test stage confirms it first; if it does not hold, it stops for G1.
    Given the synthetic signal "edge_bump19k3" saved as FLAC
    When it is analysed with DetectionParams v6
    Then fs is between 19350 and 19550 Hz
    And D is at least 10 dB and below 16 dB
    And D_Q is below 3 dB
    And R is at least 15 dB

  @RF-04
  Scenario: A low-pass with quantiser holes is stopped by the residual alone
    The median step and the cutoff step pass; only R keeps the track Fake, so
    a rule without R would clear it (fixtures.md, holes_lp19k5). The test stage
    confirms the measurements first; if they do not hold, it stops for G1.
    Given the synthetic signal "holes_lp19k5" saved as FLAC
    When it is analysed with DetectionParams v6
    Then fs is in the lossy range
    And D is at least 10 dB and below 16 dB
    And D_Q is below 3 dB
    And R is below 15 dB
    And the verdict is "fake"

  @RF-04
  Scenario: v6 gives the v5 verdicts on every other fixture
    Given every synthetic signal of specs/detection/fixtures.md except "edge_bump19k3" saved as FLAC
    When each one is analysed with the parameters of specs/detection/detection-params.v5.toml and with DetectionParams v6
    Then the two results have the same verdict, reasons and cutoff for every signal
    And they carry algorithmVersion 5 and 6

  # --- Parameter files (rules.md section 4) ---------------------------------

  @RF-04
  Scenario Outline: Valid parameter files deserialise and validate
    Given <params>
    When it is validated, after being deserialised into DetectionParams if it is a file
    Then it is accepted
    And its [edge] section is <edge section>

    Examples:
      | params                                               | edge section |
      | the text of specs/detection/detection-params.v6.toml | present      |
      | the text of specs/detection/detection-params.v5.toml | absent       |
      | the text of specs/detection/detection-params.v2.toml | absent       |
      | the default DetectionParams of the code              | present      |

  @RF-04
  Scenario Outline: Values of [edge] at the edge of validity are accepted
    Given the text of specs/detection/detection-params.v6.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is accepted

    Examples:
      | change                                                       |
      | edge.floor_low_hz set to 20600.0 (fake_max_cutoff_hz + 3 · 100) |
      | edge.max_step_db set to 10.01 (just above step_fake_db)      |
      | edge.max_median_step_db set to -5.0                          |

  @RF-04
  Scenario Outline: An invalid [edge] section is rejected
    The shape is checked when the file is deserialised; the values by
    DetectionParams::validate, which every loader calls. A NaN fails every
    comparison.
    Given the text of specs/detection/detection-params.v6.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is rejected by <layer>
    And the error names "<what>"

    Examples:
      | change                                                  | layer           | what                                                                  |
      | the floor_low_hz line of [edge] removed                 | deserialisation | floor_low_hz                                                          |
      | the floor_high_hz line of [edge] removed                | deserialisation | floor_high_hz                                                         |
      | the floor_min_db line of [edge] removed                 | deserialisation | floor_min_db                                                          |
      | the max_median_step_db line of [edge] removed           | deserialisation | max_median_step_db                                                    |
      | the min_residual_db line of [edge] removed              | deserialisation | min_residual_db                                                       |
      | the max_step_db line of [edge] removed                  | deserialisation | max_step_db                                                           |
      | an extra line "min_step_db = 0.0" in [edge]             | deserialisation | min_step_db                                                           |
      | edge.floor_low_hz set to 20599.0                        | validation      | edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz |
      | step.window_bands set to 7                              | validation      | edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz |
      | edge.floor_high_hz set to 20900.0 (equal to floor_low_hz) | validation    | edge.floor_low_hz < edge.floor_high_hz                                |
      | edge.floor_min_db set to 0.0                            | validation      | edge.floor_min_db < 0                                                 |
      | edge.max_median_step_db set to nan                      | validation      | edge.max_median_step_db is finite                                     |
      | edge.min_residual_db set to 0.0                         | validation      | edge.min_residual_db > 0                                              |
      | edge.max_step_db set to 10.0 (equal to step_fake_db)    | validation      | edge.max_step_db > step_fake_db                                       |
      | edge.min_residual_db set to nan                         | validation      | edge.min_residual_db > 0                                              |

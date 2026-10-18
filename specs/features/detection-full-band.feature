@RF-04
Feature: Full-band guard on the gap and step rules (detection v3)
  Rule-level checks of specs/detection/rules.md sections 2 to 4 with
  DetectionParams v3: band spectra, measurements or params files are given
  directly, without audio. The thresholds are those of detection-params.v3.toml:
  full_band.min_level_db = -70, full_band.min_burst_db = 3, gap_fake_db = 6,
  gap_suspicious_db = 4, step_fake_db = 10, step_suspicious_db = 7, lossy
  range [14500, 20300) Hz.
  In the measurement tables, fs "—" means fs is undefined (no step: D = 0).
  FP1-FP4 are the confirmed false positives of v2 (rules.md, "Names used here").
  Tests derived from v2 that these rules change are listed in ADR-0038.

  # --- Measurements --------------------------------------------------------

  @RF-04
  Scenario: Upper level and burstiness are medians over the upper window
    Levels in this scenario are absolute dB, before L_ref is subtracted.
    Given band spectra at 44.1 kHz with 100 Hz bands and L_ref = -30 dB
    And the bands centred at 20750, 20850 and 20950 Hz have M = -70, -80 and -75 dB and Q = -80, -82 and -80 dB
    And the bands centred at 20650 and 21050 Hz have M = -30 dB and Q = -90 dB
    When the measurements of DetectionParams v3 are taken
    Then U = -45 dB
    And B = 5 dB
    And the track is full band

  @RF-04
  Scenario: An even number of window bands takes the upper middle value
    With full_band.low_hz = 20800 the window holds 2 bands; the median of 2
    values is the larger one (rules.md section 2, "Medians").
    Levels in this scenario are absolute dB, before L_ref is subtracted.
    Given band spectra at 44.1 kHz with 100 Hz bands and L_ref = -30 dB
    And the bands centred at 20850 and 20950 Hz have M = -80 and -70 dB and Q = -82 and -74 dB
    When the measurements of DetectionParams v3 with full_band.low_hz = 20800 are taken
    Then the upper window holds the bands centred at 20850 and 20950 Hz
    And U = -40 dB
    And B = 4 dB

  @RF-04
  Scenario Outline: Upper level and burstiness are undefined without an upper window
    Given band spectra at <sample rate> Hz with 100 Hz bands
    When the measurements of <params> are taken
    Then U and B are undefined
    And the track is not full band

    Examples:
      | sample rate | params                                                     |
      | 32000       | DetectionParams v3                                         |
      | 44100       | the parameters of specs/detection/detection-params.v2.toml |

  # --- Confirmed false positives of v2 --------------------------------------

  @RF-04
  Scenario Outline: Confirmed false positives with content above the lossy range are not Fake
    G, D, fs and fe are the values measured on the real tracks with v2
    (rules.md, "Why v2 was replaced"). U and B are illustrative: the real
    values are reported by acceptance criterion A8 and judged by A1.
    Given the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v3 are applied
    Then the verdict is "suspicious"
    And the reasons are "<reasons>"

    Examples:
      | id  | H   | G     | D     | fs    | fe    | U   | B | reasons         |
      | FP1 | -20 | 18.39 | 8.93  | 20450 | 20900 | -45 | 8 | highBandGaps    |
      | FP2 | -20 | 13.18 | 14.79 | 21450 | 21400 | -40 | 8 | highBandGaps    |
      | FP3 | -20 | 10.26 | 14.70 | 21250 | 21100 | -40 | 8 | highBandGaps    |
      | FP4 | -20 | -0.25 | 14.22 | 19450 | 22050 | -45 | 8 | ambiguousCutoff |

  @RF-04
  Scenario Outline: The same evidence without content above the lossy range is still Fake
    A transcode has only a floor above its low-pass. A floor keeps B under
    0.5 dB whatever its level (first five rows). When what lies there is near
    digital silence, U falls below min_level_db and B does not matter (last row).
    Given the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v3 are applied
    Then the verdict is "fake"
    And the reasons are ""

    Examples:
      | pattern                          | H   | G     | D     | fs    | fe    | U   | B   |
      | FP1 gap over a floor             | -20 | 18.39 | 8.93  | 20450 | 22050 | -50 | 0.3 |
      | FP4 step over a floor            | -20 | -0.25 | 14.22 | 19450 | 22050 | -45 | 0.3 |
      | 320 kbps: gap, step at 20.5 kHz  | -20 | 12.00 | 25.00 | 20450 | 22050 | -55 | 0.3 |
      | 128 kbps: gap, step at 16 kHz    | -20 | 15.00 | 30.00 | 16050 | 22050 | -48 | 0.2 |
      | AAC/Opus: step only at 20 kHz    | -20 | -0.50 | 20.00 | 19950 | 22050 | -60 | 0.4 |
      | gap, near digital silence above  | -20 | 9.00  | 40.00 | 20450 | 20600 | -95 | 6.0 |

  # --- Boundaries at the thresholds -----------------------------------------

  @RF-04
  Scenario Outline: The guard holds at its thresholds and not below them
    Given the measurements H = -20 dB, G = 10 dB, D = 0 dB, fs = — Hz, fe = 22050 Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v3 are applied
    Then the track is <full band>
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | U     | B   | full band     | verdict    | reasons      |
      | -70.0 | 3.0 | full band     | suspicious | highBandGaps |
      | -70.1 | 3.0 | not full band | fake       |              |
      | -70.0 | 2.9 | not full band | fake       |              |
      | -20.0 | 2.9 | not full band | fake       |              |
      | -90.0 | 9.0 | not full band | fake       |              |

  @RF-04
  Scenario Outline: The gap and step thresholds keep their v2 boundaries when the track is not full band
    Given the measurements H = -20 dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = 22050 Hz, U = -50 dB and B = 0.3 dB
    When the verdict rules of DetectionParams v3 are applied
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | G    | D    | fs    | verdict    | reasons         |
      | 6.0  | 0    | —     | fake       |                 |
      | 5.99 | 0    | —     | suspicious | highBandGaps    |
      | 0    | 10.0 | 20250 | fake       |                 |
      | 0    | 9.99 | 20250 | suspicious | ambiguousCutoff |
      | 0    | 10.0 | 20300 | real       |                 |

  # --- What the guard never changes -----------------------------------------

  @RF-04
  Scenario Outline: The guard only turns Fake into Suspicious
    Given the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = -40 dB and B = 8 dB
    When the verdict rules of DetectionParams v3 are applied
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | case                               | H   | G  | D  | fs    | fe    | verdict    | reasons                 |
      | gap and lossy step: V5 comes first | -20 | 12 | 20 | 19450 | 22050 | suspicious | highBandGaps            |
      | weak gap (V5 as in v2)             | -20 | 5  | 0  | —     | 22050 | suspicious | highBandGaps            |
      | step above the lossy range (V8)    | -20 | 0  | 13 | 20450 | 22050 | suspicious | ambiguousCutoff         |
      | anti-aliasing step at 21.5 kHz     | -20 | 0  | 30 | 21550 | 21500 | real       |                         |
      | little treble (V2 comes first)     | -65 | 12 | 20 | 19450 | 22050 | suspicious | lowHighFrequencyContent |
      | no evidence                        | -20 | 1  | 3  | 18050 | 22050 | real       |                         |

  @RF-04
  Scenario: Without a [full_band] section the guard is disabled whatever U and B are
    FullBand is decided from the params, not only from the measurements: with
    params that have no [full_band] section it is false even when U and B are
    given and would pass the v3 thresholds.
    Given the measurements H = -20 dB, G = 18.39 dB, D = 8.93 dB, fs = 20450 Hz, fe = 20900 Hz, U = -45 dB and B = 8 dB
    When the verdict rules of the parameters of specs/detection/detection-params.v2.toml are applied
    Then the track is not full band
    And the verdict is "fake"
    And the reasons are ""

  # --- Parameter files (rules.md section 4) ---------------------------------

  @RF-04
  Scenario Outline: Valid parameter files deserialise and validate
    Given <params>
    When it is validated, after being deserialised into DetectionParams if it is a file
    Then it is accepted
    And its [full_band] section is <full band section>

    Examples:
      | params                                                  | full band section |
      | the text of specs/detection/detection-params.v3.toml    | present           |
      | the text of specs/detection/detection-params.v2.toml    | absent            |
      | the default DetectionParams of the code                 | present           |

  @RF-04
  Scenario Outline: Invalid parameter files are rejected
    The shape is checked when the file is deserialised; the order of the
    thresholds by DetectionParams::validate, which every loader calls.
    Given the text of specs/detection/detection-params.v3.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is rejected by <layer>
    And the error names "<what>"

    Examples:
      | change                                                   | layer           | what                                 |
      | the low_hz line of [full_band] removed                   | deserialisation | low_hz                               |
      | the high_hz line of [full_band] removed                  | deserialisation | high_hz                              |
      | the min_level_db line of [full_band] removed             | deserialisation | min_level_db                         |
      | the min_burst_db line of [full_band] removed             | deserialisation | min_burst_db                         |
      | an extra line "max_level_db = 0.0" in [full_band]        | deserialisation | max_level_db                         |
      | full_band.high_hz set to 20700.0 (equal to low_hz)       | validation      | full_band.low_hz < full_band.high_hz |
      | full_band.low_hz set to 20300.0 (equal to fake_max_cutoff_hz) | validation | fake_max_cutoff_hz < full_band.low_hz |
      | verdict.gap_suspicious_db set to 6.5                     | validation      | gap_suspicious_db ≤ gap_fake_db      |
      | verdict.step_suspicious_db set to 10.5                   | validation      | step_suspicious_db ≤ step_fake_db    |

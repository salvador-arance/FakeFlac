@RF-04
Feature: Full-band guard on the gap rule (detection v4)
  Rule-level checks of specs/detection/rules.md sections 2 to 4 with
  DetectionParams v4: band spectra, measurements or params files are given
  directly, without audio. The thresholds are those of detection-params.v4.toml:
  full_band.min_level_db = -70, full_band.min_burst_db = 3,
  full_band.max_lossy_step_db = 3, gap_fake_db = 6, gap_suspicious_db = 4,
  step_fake_db = 10, step_suspicious_db = 7, lossy range [14500, 20300) Hz.
  A track is full band when U >= -70, B >= 3 and not (fs in the lossy range and
  D >= 3). detection-params.v3.toml has the same values without
  max_lossy_step_db, so its guard has no step condition.
  In the measurement tables, fs "—" means fs is undefined (no step: D = 0).
  FP1-FP4 are the confirmed false positives of v2, watch ids W1-W4 of the
  acceptance runs (rules.md, "Names used here").
  Tests derived from v3 that these rules change are listed in ADR-0039.

  # --- Measurements --------------------------------------------------------

  @RF-04
  Scenario: Upper level and burstiness are medians over the upper window
    Levels in this scenario are absolute dB, before L_ref is subtracted. The
    dip in the window lies above the lossy range, so it gives no cutoff step
    in the lossy range.
    Given band spectra at 44.1 kHz with 100 Hz bands and L_ref = -30 dB
    And the bands centred at 20750, 20850 and 20950 Hz have M = -70, -80 and -75 dB and Q = -80, -82 and -80 dB
    And the bands centred at 20650 and 21050 Hz have M = -30 dB and Q = -90 dB
    When the measurements of DetectionParams v4 are taken
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
    When the measurements of DetectionParams v4 with full_band.low_hz = 20800 are taken
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
      | 32000       | DetectionParams v4                                         |
      | 44100       | the parameters of specs/detection/detection-params.v2.toml |

  # --- Confirmed false positives of v2 --------------------------------------

  @RF-04
  Scenario Outline: Confirmed false positives whose deepest step is above the lossy range are not Fake
    G, D, fs and fe were measured on the real tracks with v2 (rules.md, "Why
    v2 was replaced"), U and B with v3 (rules.md, "Why v3 was replaced"; same
    window in v4). H is illustrative. Acceptance criterion A1 judges the real
    tracks.
    Given the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v4 are applied
    Then the track is full band
    And the verdict is "suspicious"
    And the reasons are "highBandGaps"

    Examples:
      | id  | H   | G     | D     | fs    | fe    | U      | B     |
      | FP1 | -20 | 18.39 | 8.93  | 20450 | 20900 | -56.64 | 9.53  |
      | FP2 | -20 | 13.18 | 14.79 | 21450 | 21400 | -44.74 | 30.60 |
      | FP3 | -20 | 10.26 | 14.70 | 21250 | 21100 | -51.46 | 10.76 |

  @RF-04
  Scenario Outline: FP4 stays Fake (known limitation)
    Its deepest step lies in the lossy range, where a cbr256 transcode puts
    its own, so v4 never guards it, whatever its upper window holds (rules.md
    section 6, Q7). The first row has the U and B measured on the real track,
    the second illustrative bursty content.
    Given the measurements H = -20 dB, G = -0.25 dB, D = 14.22 dB, fs = 19450 Hz, fe = 22050 Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v4 are applied
    Then the track is not full band
    And the verdict is "fake"
    And the reasons are ""

    Examples:
      | U      | B    |
      | -47.06 | 2.86 |
      | -45.00 | 8.00 |

  @RF-04
  Scenario Outline: The same evidence without content above the lossy range is still Fake
    A transcode has only a floor above its low-pass. A floor keeps B under
    0.5 dB whatever its level (first five rows). When what lies there is near
    digital silence, U falls below min_level_db and B does not matter (last row).
    Given the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v4 are applied
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

  # --- The lossy-step condition (v4) ----------------------------------------

  @RF-04
  Scenario Outline: A cutoff step in the lossy range removes the guard
    U and B pass the thresholds in every row; only D and fs decide whether the
    track is full band. The confirmed-fake row has the smallest lossy-range
    step among the confirmed fakes that v3 guarded (D measured; G and fs
    illustrative).
    Given the measurements H = -20 dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = 22050 Hz, U = -40 dB and B = 8 dB
    When the verdict rules of DetectionParams v4 are applied
    Then the track is <full band>
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | case                                              | G     | D     | fs    | full band     | verdict    | reasons      |
      | gap, no cutoff step                               | 10    | 0     | —     | full band     | suspicious | highBandGaps |
      | gap, lossy step at max_lossy_step_db              | 10    | 3.0   | 18050 | not full band | fake       |              |
      | gap, lossy step just below max_lossy_step_db      | 10    | 2.99  | 18050 | full band     | suspicious | highBandGaps |
      | gap, step at the lower edge of the lossy range    | 10    | 5.0   | 14500 | not full band | fake       |              |
      | gap, step just below the lossy range              | 10    | 5.0   | 14450 | full band     | suspicious | highBandGaps |
      | gap, step just inside the upper edge of the range | 10    | 5.0   | 20250 | not full band | fake       |              |
      | gap, step at the upper edge of the range          | 10    | 5.0   | 20300 | full band     | suspicious | highBandGaps |
      | FP1 pattern: deepest step 150 Hz above the range  | 18.39 | 8.93  | 20450 | full band     | suspicious | highBandGaps |
      | gap and a deep lossy step                         | 12    | 20    | 19450 | not full band | fake       |              |
      | confirmed fake with the smallest lossy step       | 8     | 5.63  | 17050 | not full band | fake       |              |
      | deep lossy step, no gap: V4 is never guarded      | -0.25 | 14.22 | 19450 | not full band | fake       |              |
      | small lossy step, no gap: nothing fires           | 1     | 5.0   | 18050 | not full band | real       |              |

  # --- Boundaries at the thresholds -----------------------------------------

  @RF-04
  Scenario Outline: The guard holds at its thresholds and not below them
    Given the measurements H = -20 dB, G = 10 dB, D = 0 dB, fs = — Hz, fe = 22050 Hz, U = <U> dB and B = <B> dB
    When the verdict rules of DetectionParams v4 are applied
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
    When the verdict rules of DetectionParams v4 are applied
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
    When the verdict rules of DetectionParams v4 are applied
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | case                               | H   | G  | D  | fs    | fe    | verdict    | reasons                 |
      | weak gap (V5 as in v2)             | -20 | 5  | 0  | —     | 22050 | suspicious | highBandGaps            |
      | step above the lossy range (V8)    | -20 | 0  | 13 | 20450 | 22050 | suspicious | ambiguousCutoff         |
      | anti-aliasing step at 21.5 kHz     | -20 | 0  | 30 | 21550 | 21500 | real       |                         |
      | little treble (V2 comes first)     | -65 | 12 | 20 | 19450 | 22050 | suspicious | lowHighFrequencyContent |
      | no evidence                        | -20 | 1  | 3  | 18050 | 22050 | real       |                         |

  @RF-04
  Scenario: Without a [full_band] section the guard is disabled whatever U and B are
    FullBand is decided from the params, not only from the measurements: with
    params that have no [full_band] section it is false even when U and B are
    given and would pass the thresholds.
    Given the measurements H = -20 dB, G = 18.39 dB, D = 8.93 dB, fs = 20450 Hz, fe = 20900 Hz, U = -45 dB and B = 8 dB
    When the verdict rules of the parameters of specs/detection/detection-params.v2.toml are applied
    Then the track is not full band
    And the verdict is "fake"
    And the reasons are ""

  @RF-04
  Scenario Outline: A v3 params file still gives the v3 verdicts
    detection-params.v3.toml has no max_lossy_step_db, so its guard has no
    step condition and also covers V4 (rules.md sections 3 and 4). The two
    files differ only where a cutoff step in the lossy range of 3 dB or more
    lies under a guarded track.
    Given the measurements H = -20 dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = <U> dB and B = <B> dB
    When the verdict rules of the parameters of specs/detection/detection-params.v3.toml are applied
    Then the verdict is "<v3 verdict>"
    And the reasons are "<v3 reasons>"
    When the verdict rules of DetectionParams v4 are applied
    Then the verdict is "<v4 verdict>"
    And the reasons are "<v4 reasons>"

    Examples:
      | case                                | G     | D     | fs    | fe    | U      | B    | v3 verdict | v3 reasons      | v4 verdict | v4 reasons   |
      | FP4 pattern with bursty content     | -0.25 | 14.22 | 19450 | 22050 | -45    | 8    | suspicious | ambiguousCutoff | fake       |              |
      | gap and a deep lossy step           | 12    | 20    | 19450 | 22050 | -40    | 8    | suspicious | highBandGaps    | fake       |              |
      | gap and a small lossy step          | 10    | 5     | 16050 | 22050 | -6     | 13   | suspicious | highBandGaps    | fake       |              |
      | FP1                                 | 18.39 | 8.93  | 20450 | 20900 | -56.64 | 9.53 | suspicious | highBandGaps    | suspicious | highBandGaps |
      | FP1 gap over a floor                | 18.39 | 8.93  | 20450 | 22050 | -50    | 0.3  | fake       |                 | fake       |              |

  # --- Parameter files (rules.md section 4) ---------------------------------

  @RF-04
  Scenario Outline: Valid parameter files deserialise and validate
    Given <params>
    When it is validated, after being deserialised into DetectionParams if it is a file
    Then it is accepted
    And its [full_band] section is <full band section>
    And its full_band.max_lossy_step_db is <lossy step>

    Examples:
      | params                                                  | full band section | lossy step |
      | the text of specs/detection/detection-params.v4.toml    | present           | 3.0        |
      | the text of specs/detection/detection-params.v3.toml    | present           | absent     |
      | the text of specs/detection/detection-params.v2.toml    | absent            | absent     |
      | the default DetectionParams of the code                 | present           | 3.0        |

  @RF-04
  Scenario Outline: The bounds of max_lossy_step_db are inclusive
    Given the text of specs/detection/detection-params.v4.toml with full_band.max_lossy_step_db set to <value>
    When it is deserialised into DetectionParams and validated
    Then it is accepted

    Examples:
      | value | note                     |
      | 0.0   | the lower bound          |
      | 10.0  | equal to step_fake_db    |

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

  @RF-04
  Scenario Outline: An invalid max_lossy_step_db is rejected by validation
    A NaN fails both inequalities; the error names the field either way.
    Given the text of specs/detection/detection-params.v4.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is rejected by validation
    And the error names "<what>"

    Examples:
      | change                                                                                         | what                                       |
      | full_band.max_lossy_step_db set to -0.5                                                        | full_band.max_lossy_step_db ≥ 0            |
      | full_band.max_lossy_step_db set to 10.5 (above step_fake_db)                                   | full_band.max_lossy_step_db ≤ step_fake_db |
      | verdict.step_fake_db and verdict.step_suspicious_db set to 2.5 (below max_lossy_step_db)       | full_band.max_lossy_step_db ≤ step_fake_db |
      | full_band.max_lossy_step_db set to nan                                                         | full_band.max_lossy_step_db                |

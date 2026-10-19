@RF-03 @RF-04 @RF-14
Feature: Verdict for lossless files (WAV, FLAC)
  The detection rules of specs/detection/rules.md, checked with the synthetic
  signals of specs/detection/fixtures.md and DetectionParams v4 (the defaults).
  Cutoff tolerance: ± 200 Hz.
  Changed for v4 (the test author updates the derived tests listed in ADR-0039,
  "Consequences"): sfb21_gaps and shelf19k5_bursts are fake again (their deepest
  step lies in the lossy range, so they are not full band); only
  sfb21_gaps_aa21k stays guarded; a v3 params file still gives the v3 verdicts;
  the defaults are compared with detection-params.v4.toml and carry
  algorithmVersion 4.
  sfb21_gaps has D of at least 3 dB with fs in the lossy range, and hf_bursts
  has no such step (fixtures.md, "Verdicts with v4": derived, and measured once
  before G1 at D 6.15 dB / 16050 Hz and D 0.48 dB / 20950 Hz). The test stage
  confirms them first; if either does not hold, it stops and returns to G1.

  @RF-04
  Scenario Outline: Verdict by signal
    Given the synthetic signal "<signal>" saved as <format>
    When it is analysed with DetectionParams v4
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | signal            | format | verdict    | reasons                 |
      | full_band         | FLAC   | real       |                         |
      | full_band         | WAV    | real       |                         |
      | lp16k             | FLAC   | fake       |                         |
      | lp19k             | FLAC   | fake       |                         |
      | lp20k             | WAV    | fake       |                         |
      | lp16k_96k         | FLAC   | fake       |                         |
      | bass_tone_lp16k   | FLAC   | fake       |                         |
      | gradual           | FLAC   | suspicious | gradualRolloff          |
      | low_hf            | FLAC   | suspicious | lowHighFrequencyContent |
      | silence           | FLAC   | suspicious | lowHighFrequencyContent |
      | lp16k_dither      | FLAC   | fake       |                         |
      | lp20k_floor       | FLAC   | fake       |                         |
      | sfb21_gaps        | FLAC   | fake       |                         |
      | sfb21_gaps_weak   | FLAC   | suspicious | highBandGaps            |
      | hf_bursts         | FLAC   | real       |                         |
      | sfb21_gaps_lp20k5 | FLAC   | fake       |                         |
      | sfb21_gaps_aa21k  | FLAC   | suspicious | highBandGaps            |
      | shelf19k5         | FLAC   | fake       |                         |
      | shelf19k5_bursts  | FLAC   | fake       |                         |

  @RF-03
  Scenario Outline: Cutoff estimate
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the estimated cutoff is <cutoff> Hz ± 200 Hz

    Examples:
      | signal            | cutoff |
      | lp16k             | 16000  |
      | lp19k             | 19000  |
      | lp20k             | 20000  |
      | lp16k_96k         | 16000  |
      | bass_tone_lp16k   | 16000  |
      | lp16k_dither      | 16000  |
      | lp20k_floor       | 20000  |
      | sfb21_gaps_lp20k5 | 20500  |
      | sfb21_gaps_aa21k  | 21000  |
      | shelf19k5         | 19500  |
      | shelf19k5_bursts  | 19500  |

  @RF-03
  Scenario Outline: Signal whose deepest step is below step_suspicious_db reports the Nyquist frequency as cutoff
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the estimated cutoff is 22050 Hz

    Examples:
      | signal     |
      | full_band  |
      | sfb21_gaps |
      | hf_bursts  |

  @RF-04
  Scenario Outline: Noise above the low-pass does not hide a transcode (v1 regression)
    The noise left above the cutoff of a decoded MP3 written at 16 bits made
    DetectionParams v1 report these signals as real.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the verdict is not "real"

    Examples:
      | signal       |
      | lp16k_dither |
      | lp20k_floor  |
      | sfb21_gaps   |

  @RF-04
  Scenario Outline: A stationary floor above the low-pass is not full band
    The floor of a decoded transcode can sit less than 60 dB below the reference
    level, but it does not come and go like music. The full-band guard must not
    clear these transcodes.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the track is not full band
    And the verdict is "fake"

    Examples:
      | signal            |
      | lp16k_dither      |
      | lp20k_floor       |
      | sfb21_gaps_lp20k5 |
      | shelf19k5         |

  @RF-04
  Scenario Outline: Intermittent content above the lossy range, with no step of max_lossy_step_db or more inside it, is full band
    sfb21_gaps_aa21k has its deepest step at 21 kHz, above the lossy range;
    hf_bursts has no step of 3 dB or more in the lossy range (its gate starts
    at 8 kHz, below the step search range).
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the track is full band
    And the verdict is "<verdict>"

    Examples:
      | signal           | verdict    |
      | sfb21_gaps_aa21k | suspicious |
      | hf_bursts        | real       |

  @RF-04
  Scenario Outline: Intermittent content above a cutoff step in the lossy range is not full band (v4)
    U and B pass the thresholds (the track is full band with the v3
    parameters), but its deepest cutoff step lies in the lossy range with
    D >= max_lossy_step_db (specs/detection/fixtures.md, "Verdicts with v4").
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v3.toml
    Then the track is full band
    When it is analysed with DetectionParams v4
    Then the track is not full band
    And the cutoff step lies in the lossy range with D of at least 3 dB
    And the verdict is "fake"

    Examples:
      | signal           |
      | sfb21_gaps       |
      | shelf19k5_bursts |

  @RF-04
  Scenario Outline: A v2 params file still gives the v2 verdicts
    DetectionParams v2 has no [full_band] section, so the guard is disabled and
    the calibration tool can compare v2, v3 and v4 in one run.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v2.toml
    Then the verdict is "fake"
    And the TrackAnalysis carries algorithmVersion 2

    Examples:
      | signal           |
      | sfb21_gaps       |
      | sfb21_gaps_aa21k |
      | shelf19k5_bursts |
      | lp16k_dither     |

  @RF-04
  Scenario Outline: A v3 params file still gives the v3 verdicts
    DetectionParams v3 has a [full_band] section without max_lossy_step_db, so
    its guard has no step condition.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v3.toml
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the TrackAnalysis carries algorithmVersion 3

    Examples:
      | signal           | verdict    | reasons         |
      | sfb21_gaps       | suspicious | highBandGaps    |
      | sfb21_gaps_aa21k | suspicious | highBandGaps    |
      | shelf19k5_bursts | suspicious | ambiguousCutoff |
      | lp16k_dither     | fake       |                 |

  @RF-04
  Scenario: A v2 params file changes only the guarded verdicts
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC
    When each one is analysed with the parameters of specs/detection/detection-params.v2.toml and with DetectionParams v4
    Then the two results have the same verdict, reasons and cutoff for every signal except "sfb21_gaps_aa21k"
    And that one is "fake" with no reasons with the v2 parameters and "suspicious" with "highBandGaps" with v4, with the same cutoff

  @RF-04
  Scenario: A v3 params file differs only where a lossy-range step removes the guard
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC
    When each one is analysed with the parameters of specs/detection/detection-params.v3.toml and with DetectionParams v4
    Then the two results have the same verdict, reasons and cutoff for every signal except "sfb21_gaps" and "shelf19k5_bursts"
    And those two are "suspicious" with the v3 parameters and "fake" with no reasons with v4, with the same cutoff

  @RNF-04
  Scenario: Too short
    Given the synthetic signal "short" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the verdict is "error" with error "tooShort"

  @RNF-04
  Scenario: Corrupt file
    Given the file "corrupt" (FLAC truncated after the header)
    When it is analysed with DetectionParams v4
    Then the verdict is "error" with error "corruptStream"

  @RF-04
  Scenario: Deterministic result
    Given the synthetic signal "shelf19k5_bursts" saved as FLAC
    When it is analysed twice with DetectionParams v4
    Then both analyses are equal

  @RF-04
  Scenario: Parameters come from the versioned file
    Given the default DetectionParams of the code
    Then they are equal to specs/detection/detection-params.v4.toml
    And every TrackAnalysis carries algorithmVersion 4

  @RF-03 @RNF-04
  Scenario: Stream without a declared length
    A FLAC from a streaming encoder may leave total_samples at 0. The length is
    counted by decoding, so the track is analysed instead of reported as silent.
    Given the file "lp16k_unknown_length" (lp16k as FLAC with total_samples 0 in STREAMINFO)
    When it is analysed with DetectionParams v4
    Then the verdict is "fake"
    And the estimated cutoff is 16000 Hz ± 200 Hz

  @RF-03
  Scenario: A silent track reports no cutoff
    Given the synthetic signal "silence" saved as FLAC
    When it is analysed with DetectionParams v4
    Then the verdict is "suspicious"
    And the reasons are "lowHighFrequencyContent"
    And no cutoff is reported
